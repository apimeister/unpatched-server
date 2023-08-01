use axum::{
    extract::connect_info::ConnectInfo,
    extract::State,
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path,
    },
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use clap::Parser;
use futures::{sink::SinkExt, stream::StreamExt};
use include_dir::{include_dir, Dir};
use serde::{Deserialize, Serialize};
use sqlx::{query, sqlite::SqlitePool, Row};
use std::net::SocketAddr;
use std::time::Duration;
use std::vec;
use systemctl::Unit;
use tower_http::{
    services::ServeDir,
    trace::{DefaultMakeSpan, TraceLayer},
};
use tracing::{debug, error, info};
use tracing_subscriber::{
    fmt::{self},
    layer::SubscriberExt,
    registry,
    util::SubscriberInitExt,
    EnvFilter,
};
use uuid::Uuid;

mod db;
mod host;
mod script;

static WEBPAGE: Dir = include_dir!("$CARGO_MANIFEST_DIR/target/site");

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

const UPDATE_RATE: Duration = Duration::new(5, 0);
const SQLITE_DB: &str = "sqlite:monitor_server_internal.sqlite";

#[tokio::main]
async fn main() {
    let args = Args::parse();
    registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(fmt::layer())
        .init();
    let pool = db::create_datase().await;

    let web_page = ServeDir::new(WEBPAGE.path().join("target").join("site"))
        .append_index_html_on_directories(true);

    // build our application with some routes
    let app = Router::new()
        .fallback_service(web_page)
        .route("/ws", get(ws_handler).with_state(pool.clone()))
        .route(
            "/api/v1/agents/:id",
            get(single_agent_api).with_state(pool.clone()),
        )
        .route("/api/v1/agents", get(agents_api).with_state(pool.clone()))
        .route(
            "/api/v1/scripts",
            get(script::get_scripts_api).with_state(pool.clone()),
        )
        .route(
            "/api/v1/hosts",
            get(host::get_hosts_api).with_state(pool.clone()),
        )
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        );

    let addr: SocketAddr = format!("{}:{}", args.bind, args.port).parse().unwrap();
    info!("listening on http://{addr}/");
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

    // ##################
    // ALL THE SEND STUFF
    // ##################

    let _sender_handle = tokio::spawn(async move {
        let mut sink = sender;

        loop {
            let _ping = sink.send(Message::Ping("Hello, Client!".into())).await;
            tokio::time::sleep(UPDATE_RATE).await;
        }
    });
    // #####################
    // ALL THE RECEIVE STUFF
    // #####################

    let recv_handle = tokio::spawn(async move {
        let mut id: Option<String> = None;
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Close(_) => break,
                // Ping Message is only send on first connection by agent
                Message::Ping(v) => {
                    let alias = std::str::from_utf8(&v).unwrap_or("");
                    debug!("Received a Ping!");
                    info!("Client {alias} Connected from {who}");
                }
                // Pong Message is sent by client on any Ping from server (alive status)
                Message::Pong(_v) => {
                    debug!("Received a Pong!");

                    // without ID, skip and wait for next update cycle
                    if let Some(uuid) = &id {
                        debug!("Update agent as alive (hosts table -> last pong)!");
                        let _ = query(r#"UPDATE hosts SET last_pong = datetime() WHERE id = ?"#)
                            .bind(uuid)
                            .execute(&mut *pool.acquire().await.unwrap())
                            .await
                            .unwrap();
                    }
                }
                Message::Text(t) => {
                    let (k, v) = t.split_once(':').unwrap_or(("", ""));
                    let data = match k {
                        "id" => {
                            if id.is_none() {
                                id = Some(v.into());
                                let id_res = query(r#"INSERT INTO hosts(id) VALUES (?)"#)
                                    .bind(v)
                                    .execute(&mut *pool.acquire().await.unwrap())
                                    .await;
                                // FIXME: This should be some real error handling
                                if id_res.is_err() {
                                    debug!("Agent with id {v} already known")
                                }
                            }
                            continue;
                        }
                        "alias" => v.to_string(),
                        "attributes" => v.to_string(),
                        "script" => v.to_string(),
                        // ignore all unknown fields
                        x => {
                            warn!("{x} is unsupported!");
                            continue;
                        }
                    };
                    // without ID, skip and wait for next update cycle
                    if let Some(uuid) = &id {
                        debug!("Received some data for {k}: {data}");
                        let stmt = format!("UPDATE hosts SET {k} = ? WHERE id = ?");
                        let _ = query(&stmt)
                            .bind(data)
                            .bind(uuid)
                            .execute(&mut *pool.acquire().await.unwrap())
                            .await
                            .unwrap();
                    }
                }
                Message::Binary(_) => error!("Binary is unsupported!"),
            };
            // FIXME: implement something with this who
            let _who = who;
        }
    });

    let _ = recv_handle.await;
}

async fn agents_api(State(pool): State<SqlitePool>) -> (StatusCode, Json<Vec<AgentData>>) {
    let mut conn = pool.acquire().await.unwrap();
    let show_data = match query("SELECT id, name, uptime, os_release, memory FROM data")
        .fetch_all(&mut *conn)
        .await
    {
        Ok(d) => d,
        Err(_) => return (StatusCode::OK, Json(Vec::new())),
    };

    let mut agents_vec: Vec<AgentData> = vec![];

    for d in show_data {
        let agent = AgentData {
            id: d.get::<String, _>("id"),
            alias: d.get::<String, _>("name"),
            uptime: d.get::<i64, _>("uptime"),
            os_release: d.get::<String, _>("os_release"),
            ..Default::default()
        };
        agents_vec.push(agent);
    }

    (StatusCode::OK, Json(agents_vec))
}

async fn single_agent_api(
    Path(id): Path<Uuid>,
    State(pool): State<SqlitePool>,
) -> (StatusCode, Json<AgentData>) {
    let mut conn = pool.acquire().await.unwrap();
    let show_data = match query("SELECT * FROM data WHERE id = ?")
        .bind(id.to_string())
        .fetch_one(&mut *conn)
        .await
    {
        Ok(d) => d,
        Err(_) => return (StatusCode::NOT_FOUND, Json(AgentData::default())),
    };
    let single_agent = AgentData {
        id: show_data.get::<String, _>("id"),
        alias: show_data.get::<String, _>("name"),
        uptime: show_data.get::<i64, _>("uptime"),
        os_release: show_data.get::<String, _>("os_release"),
        memory: serde_json::from_str(show_data.get::<String, _>("memory").as_str()).unwrap(),
        units: serde_json::from_str(show_data.get::<String, _>("units").as_str()).unwrap(),
    };
    (StatusCode::OK, Json(single_agent))
}

fn new_id() -> String {
    let id = Uuid::new_v4();
    let string_id = format!("{}", id.as_hyphenated());
    string_id
}
