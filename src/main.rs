use crate::{
    db::{new_id, utc_from_str, utc_to_str},
    execution::Execution,
};
use axum::{
    extract::connect_info::ConnectInfo,
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::State,
    response::IntoResponse,
    routing::get,
    Error, Router,
};
use axum_server::tls_rustls::RustlsConfig;
use chrono::prelude::*;
use clap::Parser;
use futures::{sink::SinkExt, stream::StreamExt};
use futures_util::{future::join_all, stream::SplitSink};
use include_dir::{include_dir, Dir};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;
use std::{io::ErrorKind, path::PathBuf, time::Duration};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::Mutex;
use tower_http::{
    services::ServeDir,
    trace::{DefaultMakeSpan, TraceLayer},
};
use tracing::{debug, error, info, warn};
use tracing_subscriber::{fmt, layer::SubscriberExt, registry, util::SubscriberInitExt, EnvFilter};
use uuid::Uuid;

mod db;
mod execution;
mod host;
mod schedule;
mod script;
mod swagger;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Default)]
struct ScriptExec {
    pub id: Uuid,
    pub script: script::Script,
}

static WEBPAGE: Dir = include_dir!("$CARGO_MANIFEST_DIR/target/site");

type SenderSinkArc = Arc<Mutex<SplitSink<WebSocket, Message>>>;

/// A bash first monitoring solution
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// bind adress for frontend and agent websockets
    #[arg(short, long, default_value = "127.0.0.1")]
    bind: String,
    /// bind port for frontend and agent websockets
    #[arg(short, long, default_value = "3000")]
    port: String,
    /// deactivate tls for frontend
    #[arg(long)]
    no_tls: bool,
    /// Sets the certificate folder
    #[arg(long, value_name = "FOLDER", default_value = "./self-signed-certs")]
    cert_folder: PathBuf,
}

const UPDATE_RATE: Duration = Duration::new(5, 0);
const SQLITE_DB: &str = "sqlite:unpatched_server_internal.sqlite";

enum Trigger {
    Cron(String),
    Once(DateTime<Utc>),
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(fmt::layer())
        .init();

    let pool = db::create_database(SQLITE_DB)
        .await
        .expect("Unable to create database connection!");
    db::init_database(&pool)
        .await
        .expect("Unable to initialize database!");

    // Frontend
    let web_page = ServeDir::new(WEBPAGE.path().join("target").join("site"))
        .append_index_html_on_directories(true);

    // build our application with some routes
    let app = Router::new()
        .fallback_service(web_page)
        .route(
            "/api/v1/executions/:id",
            get(execution::get_one_execution_api)
                .delete(execution::delete_one_execution_api)
                .with_state(pool.clone()),
        )
        .route(
            "/api/v1/executions",
            get(execution::get_executions_api)
                .delete(execution::delete_executions_api)
                .post(execution::post_executions_api)
                .with_state(pool.clone()),
        )
        .route(
            "/api/v1/scripts/:id",
            get(script::get_one_script_api)
                .delete(script::delete_one_script_api)
                .with_state(pool.clone()),
        )
        .route(
            "/api/v1/scripts",
            get(script::get_scripts_api)
                .delete(script::delete_scripts_api)
                .post(script::post_scripts_api)
                .with_state(pool.clone()),
        )
        .route(
            "/api/v1/hosts/:id",
            get(host::get_one_host_api)
                .delete(host::delete_one_host_api)
                .with_state(pool.clone()),
        )
        .route(
            "/api/v1/hosts",
            get(host::get_hosts_api)
                .delete(host::delete_hosts_api)
                .post(host::post_hosts_api)
                .with_state(pool.clone()),
        )
        .route(
            "/api/v1/schedules/:id",
            get(schedule::get_one_schedule_api)
                .delete(schedule::delete_one_schedule_api)
                .with_state(pool.clone()),
        )
        .route(
            "/api/v1/schedules",
            get(schedule::get_schedules_api)
                .delete(schedule::delete_schedules_api)
                .post(schedule::post_schedules_api)
                .with_state(pool.clone()),
        )
        // Swagger API
        .route("/api", get(swagger::api_ui))
        .route("/api/v1", get(swagger::api_ui))
        .route("/api/api.yaml", get(swagger::api_def))
        // Websocket for Agents
        .route("/ws", get(ws_handler).with_state(pool.clone()))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        );
    let addr: SocketAddr = format!("{}:{}", args.bind, args.port).parse().unwrap();

    // spawn http or https depending on --no-tls
    if args.no_tls {
        let _http = tokio::spawn(http_server(app, addr)).await;
    } else {
        let _https = tokio::spawn(https_server(app, addr, args.cert_folder)).await;
    }
}

async fn http_server(app: Router, addr: SocketAddr) {
    info!("listening on http://{addr}/");
    axum_server::bind(addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap()
}

async fn https_server(app: Router, addr: SocketAddr, tls_folder: PathBuf) {
    let config =
    match RustlsConfig::from_pem_file(tls_folder.join("unpatched.server.crt"), tls_folder.join("unpatched.server.key"))
        .await
        {
            Ok(tls) => tls,
            Err(e) => {
                match e.kind() {
                    ErrorKind::NotFound => panic!("TLS certificates not found under \"self-signed-certs/cert.pem\", \"self-signed-certs/key.pem\""),
                    _ => panic!("{e}")
                }
            }
        };
    info!("listening on https://{addr}/");
    axum_server::bind_rustls(addr, config)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap()
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
    let this_host: Option<host::Host> = None;
    let arc_this_host = Arc::new(Mutex::new(this_host));
    // split websocket stream so we can have both directions working independently
    let (sender, mut receiver) = socket.split();
    let arc_sink = Arc::new(Mutex::new(sender));
    info!("Connection established to agent: {}", who);

    // ##################
    // General tasks per Connection
    // ##################
    let general_pool = pool.clone();
    let general_arc_this_host = Arc::clone(&arc_this_host);
    let general_handle = tokio::spawn(async move {
        loop {
            tokio::time::sleep(UPDATE_RATE).await;

            let host = {
                let Some(ref host) = &*general_arc_this_host.lock().await else { continue };
                host.clone()
            };

            let schedules = schedule::get_schedules_from_db(
                Some("active = 1"),
                general_pool.acquire().await.unwrap(),
            )
            .await;
            let mut executable_schedules = Vec::new();
            let mut host_attributes = host.attributes.clone();
            host_attributes.sort();
            for mut sched in schedules {
                sched.attributes.sort();
                if host_attributes.contains(&sched.attributes()) {
                    executable_schedules.push(sched);
                } else {
                    continue;
                }
            }

            if !executable_schedules.is_empty() {
                debug!(
                    "Found schedules for {}: {:?}",
                    host.alias, executable_schedules
                );
            }

            for sched in executable_schedules {
                // Generate Executions from the schedules
                let exec_filter = format!(
                    "host_id='{}' AND sched_id='{}' AND request > dateTime('now')",
                    host.id, sched.id
                );
                let execs = execution::get_executions_from_db(
                    Some(&exec_filter),
                    general_pool.acquire().await.unwrap(),
                )
                .await;
                debug!("Found executions for {}: {:?}", host.alias, execs);
                let cron_iter = sched.cron.split_ascii_whitespace();
                let trigger = match cron_iter.count() {
                    2 => Trigger::Once(utc_from_str(&sched.cron)),
                    5 => Trigger::Cron(format!("0 {} *", sched.cron)),
                    7 => Trigger::Cron(sched.cron.clone()),
                    _ => {
                        error!(
                            "Schedule {}: Cron {} has wrong format needs to be 5 part or 7 part cron, or a timestamp. Skipped",
                            sched.id, sched.cron
                        );
                        continue;
                    }
                };
                let triggers: Vec<DateTime<Utc>> = match trigger {
                    Trigger::Cron(c) => {
                        let cron_schedule = match c.parse::<cron::Schedule>() {
                            Ok(cc) => cc,
                            Err(e) => {
                                error!(
                                    "Schedule {}: Cron {c} parsing err. Skipped \n {e}",
                                    sched.id
                                );
                                continue;
                            }
                        };
                        cron_schedule.upcoming(Utc).take(1).collect()
                    }
                    Trigger::Once(o) => {
                        schedule::update_text_field(
                            sched.id,
                            "active",
                            "0".into(),
                            general_pool.acquire().await.unwrap(),
                        )
                        .await;
                        vec![o]
                    }
                };

                if execs.is_empty() {
                    // greenfield, just add three new executions
                    for datetime in triggers {
                        let exe = Execution {
                            id: new_id(),
                            request: datetime,
                            host_id: host.id,
                            sched_id: sched.id,
                            script_id: sched.script_id,
                            ..Default::default()
                        };
                        let res = exe
                            .insert_into_db(general_pool.acquire().await.unwrap())
                            .await;
                        debug!("Created new Execution: {:?}", res);
                    }
                }
            }
        }
    });

    // ##################
    // ALL THE SEND STUFF
    // ##################
    let sender_pool = pool.clone();
    let sender_arc_sink = Arc::clone(&arc_sink);
    let sender_arc_this_host = Arc::clone(&arc_this_host);
    let sender_handle = tokio::spawn(async move {
        loop {
            tokio::time::sleep(UPDATE_RATE).await;
            let host = {
                let Some(ref host) = &*sender_arc_this_host.lock().await else { continue };
                host.clone()
            };
            let ping_msg = format!("Agent {} you there?", host.alias).into_bytes();
            let _ping = send_message(&sender_arc_sink, Message::Ping(ping_msg)).await;
            // 1. get all executions where start date + timeout + x secs < now
            // 2. get linked script
            // 3. send script with execution id
            // 4. update execution on return with timestamp
            // FIXME: maybe add retries? and after some show as failed
            // TODO: Implement skip when multiple execs from history would be executed (should only actually exec the newest one)
            let exec_filter = format!(
                "request < datetime('now')
                AND response IS NULL
                AND host_id='{}'",
                host.id
            );
            // FIXME: Filter out overdue executions (now() + x)
            let execs = execution::get_executions_from_db(
                Some(&exec_filter),
                sender_pool.acquire().await.unwrap(),
            )
            .await;
            debug!("{:?}", execs);
            let mut script_exec_vec = Vec::new();
            for exe in execs {
                let script_filter = format!("id = '{}'", exe.script_id);
                let scripts = script::get_scripts_from_db(
                    Some(&script_filter),
                    sender_pool.acquire().await.unwrap(),
                )
                .await;
                debug!("{:?}", scripts);
                let script = match scripts.first() {
                    Some(s) => s.clone(),
                    None => {
                        warn!(
                            "execution {} did not find a script with id {}. Execution Skipped",
                            exe.id, exe.script_id
                        );
                        execution::update_text_field(
                            exe.id,
                            "response",
                            utc_to_str(Utc::now()),
                            sender_pool.acquire().await.unwrap(),
                        )
                        .await;
                        execution::update_text_field(
                            exe.id,
                            "output",
                            "Script not found, execution skipped".into(),
                            sender_pool.acquire().await.unwrap(),
                        )
                        .await;
                        continue;
                    }
                };

                let script_exec = ScriptExec { id: exe.id, script };
                script_exec_vec.push(script_exec)
            }
            for script_exec in script_exec_vec {
                let json_script = match serde_json::to_string(&script_exec) {
                    Ok(json) => json,
                    Err(e) => {
                        error!("Could not transform script {} to json\n{e}", script_exec.id);
                        continue;
                    }
                };
                let _sent_script = send_message(
                    &sender_arc_sink,
                    Message::Text(format!("script:{json_script}")),
                )
                .await;
            }
        }
    });
    // #####################
    // ALL THE RECEIVE STUFF
    // #####################
    let receiver_pool = pool.clone();
    let recv_arc_sink = Arc::clone(&arc_sink);
    let recv_arc_this_host = Arc::clone(&arc_this_host);

    let recv_handle = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Close(_) => break,

                // Heartbeat test from Agent to check if Server is alive
                Message::Ping(v) => {
                    debug!(
                        "Got ping with {}",
                        std::str::from_utf8(&v).unwrap_or("utf-8 error, not parsable")
                    );
                    let _pong =
                        send_message(&recv_arc_sink, Message::Pong("still here".into())).await;
                }

                // Heartbeat from Agent
                Message::Pong(v) => {
                    debug!(
                        "Got pong with {}",
                        std::str::from_utf8(&v).unwrap_or("utf-8 error, not parsable")
                    );

                    // without ID, skip and wait for next update cycle
                    let host_lock = &*recv_arc_this_host.lock().await;
                    if let Some(host) = host_lock {
                        debug!(
                            "Update agent {} as alive (hosts table -> last pong)!",
                            host.id
                        );
                        host::update_text_field(
                            host.id,
                            "last_pong",
                            utc_to_str(Utc::now()),
                            receiver_pool.acquire().await.unwrap(),
                        )
                        .await;
                    }
                }

                Message::Text(t) => {
                    let (k, v) = t.split_once(':').unwrap_or(("", ""));
                    match k {
                        "host" => {
                            let mut host: host::Host = serde_json::from_str(v).unwrap();
                            host.ip = who.to_string();
                            debug!("{:?}", host);

                            let mut this_host = recv_arc_this_host.lock().await;
                            *this_host = Some(host.clone());
                            host.insert_into_db(receiver_pool.acquire().await.unwrap())
                                .await;
                            continue;
                        }
                        "script" => {
                            let script_exec: ScriptExec = serde_json::from_str(v).unwrap();
                            debug!("{:?}", script_exec);
                            execution::update_text_field(
                                script_exec.id,
                                "response",
                                utc_to_str(Utc::now()),
                                receiver_pool.acquire().await.unwrap(),
                            )
                            .await;
                            execution::update_text_field(
                                script_exec.id,
                                "output",
                                script_exec.script.script_content,
                                receiver_pool.acquire().await.unwrap(),
                            )
                            .await;
                            continue;
                        }
                        // ignore all unknown fields
                        x => {
                            warn!("{x} is unsupported!");
                            continue;
                        }
                    }
                }
                Message::Binary(_) => error!("Binary is unsupported!"),
            };
            // FIXME: implement something with this who
            let _who = who;
        }
    });

    // await all tasks
    let handle_vec = vec![general_handle, sender_handle, recv_handle];
    join_all(handle_vec).await;
}

// /// Get ARC to Splitsink and push message onto it
// /// Will not actually flush any data, needs another send event
// /// either via .close() or .flush()
// async fn sink_message(arc: &SenderSinkArc, m: Message) -> Result<(), Error> {
//     let mut x = arc.lock().await;
//     debug!("feeding sink: {:?}", m);
//     x.feed(m).await
// }

/// Get ARC to Splitsink and push message onto it and flush them
async fn send_message(arc: &SenderSinkArc, m: Message) -> Result<(), Error> {
    let mut x = arc.lock().await;
    match m {
        Message::Ping(_) => debug!(
            "sending ping: {:?}",
            m.clone().into_text().unwrap_or("".into())
        ),
        Message::Pong(_) => debug!(
            "sending pong: {:?}",
            m.clone().into_text().unwrap_or("".into())
        ),
        _ => debug!("sending: {:?}", m),
    }
    x.send(m).await
}
