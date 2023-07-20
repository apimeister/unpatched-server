use axum::{
    extract::connect_info::ConnectInfo,
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use clap::Parser;
use futures::{sink::SinkExt, stream::StreamExt};
use include_dir::{include_dir, Dir};
use serde::{Deserialize, Serialize};
use sqlx::{
    query,
    sqlite::{SqliteConnectOptions, SqlitePool},
    Row,
};
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;
use std::vec;
use systemctl::Unit;
use tower_http::{
    services::ServeDir,
    trace::{DefaultMakeSpan, TraceLayer},
};
use tracing::{debug, error, info};
use tracing_subscriber::{fmt, layer::SubscriberExt, registry, util::SubscriberInitExt, EnvFilter};

static WEBPAGE: Dir = include_dir!("$CARGO_MANIFEST_DIR/target/page");

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "127.0.0.1")]
    bind: String,
    #[arg(short, long, default_value = "3000")]
    port: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
struct AgentData {
    id: String,
    alias: String,
    os_release: String,
    uptime: i64,
    memory: AgentDataMemory,
    units: Vec<Unit>,
}
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
struct AgentDataMemory {
    used_mem: u64,
    free_mem: u64,
    av_mem: u64,
    total_mem: u64,
}

enum AllowedFields {
    Id(String),
    Alias(String),
    OsRelease(String),
    // FIXME: uptime could be f32 or u32 to have a real check
    Uptime(String),
    Memory(AgentDataMemory),
    Units(Vec<Unit>),
    Discard,
}

const UPDATE_RATE: Duration = Duration::new(5, 0);
const SQLITE_DB: &str = "sqlite:monitor_server_internal.sqlite";

#[tokio::main]
async fn main() {
    let args = Args::parse();
    registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(fmt::layer())
        .init();
    info!("Starting unpatched server...");
    let pool = create_datase().await;

    let web_page = ServeDir::new(WEBPAGE.path().join("target").join("page"))
        .append_index_html_on_directories(true);

    // build our application with some routes
    let app = Router::new()
        .fallback_service(web_page)
        .route("/ws", get(ws_handler).with_state(pool.clone()))
        .route("/api", get(stats_api).with_state(pool))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        );

    let addr: SocketAddr = format!("{}:{}", args.bind, args.port).parse().unwrap();
    debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();
}

/// The handler for the HTTP request (this gets called when the HTTP GET lands at the start
/// of websocket negotiation). After this completes, the actual switching from HTTP to
/// websocket protocol will occur.
/// This is the last point where we can extract TCP/IP metadata such as IP address of the client
/// as well as things from HTTP headers such as user-agent of the browser etc.
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(pool): State<SqlitePool>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    // use Websocket
    ws.on_upgrade(move |socket| handle_socket(socket, addr, pool))
}

/// Actual websocket statemachine (one will be spawned per connection)
async fn handle_socket(socket: WebSocket, who: SocketAddr, pool: SqlitePool) {
    // split websocket stream so we can have both directions working independently
    let (sender, mut receiver) = socket.split();

    let _sender_handle = tokio::spawn(async move {
        let mut sink = sender;
        let _ping = sink.send(Message::Ping("Hello, Client!".into())).await;

        // ALL THE SEND STUFF
        loop {
            let _ping = sink.send(Message::Ping("Hello, Client!".into())).await;
            tokio::time::sleep(UPDATE_RATE).await;
        }
    });

    let recv_handle = tokio::spawn(async move {
        // ALL THE RECEIVE STUFF
        let mut id: Option<AllowedFields> = None;
        while let Some(Ok(msg)) = receiver.next().await {
            // cnt += 1;
            let mut data = AllowedFields::Discard;
            match msg {
                Message::Close(_) => break,
                Message::Ping(v) => {
                    let alias = std::str::from_utf8(&v).unwrap_or("");
                    debug!("Received a Ping!");
                    info!("Client {alias} Connected from {who}");
                }
                Message::Pong(_) => debug!("Received a Pong!"),
                Message::Text(t) => {
                    let (k, v) = t.split_once(':').unwrap();
                    data = match k {
                        "uuid" => {
                            id = Some(AllowedFields::Id(v.to_string()));
                            let mut conn = pool.acquire().await.unwrap();
                            let id_res = query(r#"INSERT INTO data( id, name, uptime, os_release, memory, units ) VALUES ( ?, "", 0, "", "", "" )"#).bind(v).execute(&mut *conn).await;
                            // TO-DO: This should be some real error handling
                            if id_res.is_err() {
                                debug!("Agent with Id {v} already known")
                            }
                            AllowedFields::Id(v.to_string())
                        }
                        "alias" => AllowedFields::Alias(v.to_string()),
                        "os" => AllowedFields::OsRelease(v.to_string()),
                        "uptime" => {
                            let (time, _) = v.split_once('.').unwrap_or((v, v));
                            AllowedFields::Uptime(time.to_string())
                        }
                        "memory" => AllowedFields::Memory(serde_json::from_str(v).unwrap()),
                        "units" => AllowedFields::Units(serde_json::from_str(v).unwrap()),
                        _ => AllowedFields::Discard,
                    };
                }
                Message::Binary(_) => error!("Binary is unsupported!"),
            };
            // FIXME: implement something with this who
            let _who = who;

            // without ID, skip and wait for next update cycle
            if let Some(AllowedFields::Id(agent_id)) = &id {
                let mut conn = pool.acquire().await.unwrap();
                let (field, value) = match data {
                    AllowedFields::Alias(v) => ("name", v),
                    AllowedFields::Uptime(v) => ("uptime", v),
                    AllowedFields::OsRelease(v) => ("os_release", v),
                    AllowedFields::Units(v) => ("units", serde_json::to_string(&v).unwrap()),
                    AllowedFields::Memory(v) => ("memory", serde_json::to_string(&v).unwrap()),
                    _ => continue,
                };

                debug!("Received some data for {field}: {value}");

                let _ = query(format!(r#"UPDATE data SET {field} = ? WHERE id = ?"#).as_str())
                    .bind(value)
                    .bind(agent_id)
                    .execute(&mut *conn)
                    .await
                    .unwrap();
            }
        }
    });

    let _ = recv_handle.await;
}

async fn stats_api(State(pool): State<SqlitePool>) -> (StatusCode, Json<Vec<AgentData>>) {
    let mut conn = pool.acquire().await.unwrap();
    let show_data = match query("SELECT * FROM data").fetch_all(&mut *conn).await {
        Ok(d) => d,
        Err(_) => return (StatusCode::NOT_FOUND, Json(Vec::new())),
    };

    let mut agents_vec: Vec<AgentData> = vec![];

    for d in show_data {
        let agent = AgentData {
            id: d.get::<String, _>("id"),
            alias: d.get::<String, _>("name"),
            uptime: d.get::<i64, _>("uptime"),
            os_release: d.get::<String, _>("os_release"),
            memory: serde_json::from_str(d.get::<String, _>("memory").as_str()).unwrap(),
            units: serde_json::from_str(d.get::<String, _>("units").as_str()).unwrap(),
        };
        agents_vec.push(agent);
    }

    (StatusCode::OK, Json(agents_vec))
}

/// create database
/// sqlite::memory: - Open an in-memory database.
/// sqlite:data.db - Open the file data.db in the current directory.
/// sqlite://data.db - Open the file data.db in the current directory.
/// sqlite:///data.db - Open the file data.db from the root (/) directory.
/// sqlite://data.db?mode=ro - Open the file data.db for read-only access.
async fn create_datase() -> SqlitePool {
    let connection_options = SqliteConnectOptions::from_str(SQLITE_DB)
        .unwrap()
        .create_if_missing(true);
    let pool = SqlitePool::connect_with(connection_options).await.unwrap();
    {
        let mut conn = pool.acquire().await.unwrap();
        let _create_table = query(
            r#"CREATE TABLE IF NOT EXISTS 
            data(
                id VARCHAR(36) PRIMARY KEY NOT NULL,
                name VARCHAR(255),
                uptime INT,
                os_release VARCHAR(255),
                memory VARCHAR(255),
                units VARCHAR(255)
            )"#,
        )
        .execute(&mut *conn)
        .await
        .unwrap();
    }
    pool
}
