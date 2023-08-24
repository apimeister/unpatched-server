use crate::{
    db::utc_to_str,
    execution::Execution,
    host::{Host, ScheduleState},
    schedule::Timer,
};
use axum::{
    extract::connect_info::ConnectInfo,
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Error, Router,
};
use axum_server::tls_rustls::RustlsConfig;
use chrono::{prelude::*, Days};
use clap::Parser;
use futures::{sink::SinkExt, stream::StreamExt};
use futures_util::{future::join_all, stream::SplitSink};
use headers::HeaderMap;
use host::get_hosts_from_db;
use include_dir::{include_dir, Dir};
use schedule::Schedule;
use serde::{Deserialize, Serialize};
use sqlx::{pool::PoolConnection, sqlite::SqlitePool, Sqlite};
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
    /// deactivate tls
    #[arg(long)]
    no_tls: bool,
    /// auto-accept new agents
    #[arg(long)]
    //FIXME: make this work!
    auto_accept_agents: bool,
    /// Sets the certificate folder
    #[arg(long, value_name = "FOLDER", default_value = "./self-signed-certs")]
    cert_folder: PathBuf,
}

const UPDATE_RATE: Duration = Duration::new(5, 0);
const SQLITE_DB: &str = "sqlite:unpatched_server_internal.sqlite";
const TLS_CERT: &str = "unpatched.server.crt";
const TLS_KEY: &str = "unpatched.server.key";

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
            "/api/v1/hosts/:id/approve",
            post(host::approve_one_host_api).with_state(pool.clone()),
        )
        .route(
            "/api/v1/hosts/:id/lock",
            post(host::lock_one_host_api).with_state(pool.clone()),
        )
        .route(
            "/api/v1/hosts/:id/schedules",
            get(schedule::get_host_schedules_api)
                .post(schedule::post_host_schedules_api)
                .with_state(pool.clone()),
        )
        .route(
            "/api/v1/hosts/:id/executions",
            get(execution::get_host_executions_api).with_state(pool.clone()),
        )
        .route(
            "/api/v1/hosts/:id",
            get(host::get_one_host_api)
                .patch(host::update_one_host_api)
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
    let tls_cert_path = tls_folder.join(TLS_CERT);
    let tls_key_path = tls_folder.join(TLS_KEY);
    let config = match RustlsConfig::from_pem_file(&tls_cert_path, &tls_key_path).await {
        Ok(tls) => tls,
        Err(e) => match e.kind() {
            ErrorKind::NotFound => panic!(
                "TLS certificates not found under:\n{}\n{}",
                tls_cert_path.to_str().unwrap(),
                tls_key_path.to_str().unwrap()
            ),
            _ => panic!("{e}"),
        },
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
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    // use Websocket
    // TODO: Add api-key-logic of seeding keys to agent_id as well
    // remove hardcode
    let auto_accept_agents = false;
    let go_on = if auto_accept_agents {
        Some(Uuid::nil())
    } else {
        agent_auth(headers, &addr, pool.clone()).await
    };
    if let Some(key) = go_on {
        // authenticated agent
        if key.is_nil() {
            ws.on_upgrade(move |socket| handle_socket(socket, addr, pool))
        } else {
            // new agent, send key for future auth
            ws.on_upgrade(move |socket| handle_first(socket, addr, key))
        }
    } else {
        StatusCode::UNAUTHORIZED.into_response()
    }
}

/// Actual websocket statemachine for api_key exchange (one will be spawned per connection)
async fn handle_first(socket: WebSocket, who: SocketAddr, key: Uuid) {
    let _sender_handle = tokio::spawn(async move {
        // split websocket stream so we can have both directions working independently
        let (sender, _) = socket.split();
        let arc_sink = Arc::new(Mutex::new(sender));
        info!("Connection established to agent: {}", who);

        let _sent_script = send_message(&arc_sink, Message::Text(format!("api_key:{key}"))).await;
    })
    .await;
}

/// Actual websocket statemachine (one will be spawned per connection)
async fn handle_socket(socket: WebSocket, who: SocketAddr, pool: SqlitePool) {
    let this_host: Option<Host> = None;
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
            let now = utc_to_str(Utc::now());

            let host = {
                let Some(ref host) = &*general_arc_this_host.lock().await else { continue };
                host.clone()
            };

            let executable_schedules = host
                .get_all_schedules(general_pool.acquire().await.unwrap(), ScheduleState::Active)
                .await;

            // Generate Executions from the schedules
            for sched in executable_schedules {
                // Get future executions for this schedule
                let exec_filter = format!(
                    "host_id='{}' AND sched_id='{}' AND request > '{now}'",
                    host.id, sched.id
                );
                let execs = execution::get_executions_from_db(
                    Some(&exec_filter),
                    general_pool.acquire().await.unwrap(),
                )
                .await;
                debug!("Found executions for {}: {execs:?}", host.alias);

                // get execution trigger timestamp
                let Some(trigger) =
                    generate_execution_timestamp(&sched, general_pool.acquire().await.unwrap())
                        .await else { continue };
                let execs: Vec<Execution> = execs
                    .into_iter()
                    .filter(|ex| ex.request <= trigger)
                    .collect();
                if !execs.is_empty() {
                    debug!("Execution with a closer datetime exists, skip");
                    continue;
                }
                let exe = Execution {
                    id: Uuid::new_v4(),
                    request: trigger,
                    host_id: host.id,
                    sched_id: sched.id,
                    ..Default::default()
                };
                let res = exe
                    .insert_into_db(general_pool.acquire().await.unwrap())
                    .await;
                debug!("Created new Execution: {:?}", res);
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
            let now = utc_to_str(Utc::now());
            let exec_filter = format!(
                "request < '{now}'
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
                let filter = format!("id = '{}'", exe.sched_id);
                let schedules = schedule::get_schedules_from_db(
                    Some(&filter),
                    sender_pool.acquire().await.unwrap(),
                )
                .await;
                let Some(schedule) = schedules.first() else {
                        warn!(
                            "execution {} did not find a schedule with id {}. Execution Skipped",
                            exe.id, exe.sched_id
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
                            "Schedule not found, execution skipped".into(),
                            sender_pool.acquire().await.unwrap(),
                        )
                        .await;
                        continue;
                    };
                let filter = format!("id = '{}'", schedule.script_id);
                let scripts = script::get_scripts_from_db(
                    Some(&filter),
                    sender_pool.acquire().await.unwrap(),
                )
                .await;
                debug!("{:?}", scripts);
                let Some(script) = scripts.first() else {
                    warn!(
                        "execution {} did not find a script with id {}. Execution Skipped",
                        exe.id, exe.sched_id
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
                };
                let script_exec = ScriptExec {
                    id: exe.id,
                    script: script.to_owned(),
                };
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
                            let host: Host = serde_json::from_str(v).unwrap();
                            host::update_text_field(
                                host.id,
                                "alias",
                                host.alias,
                                receiver_pool.acquire().await.unwrap(),
                            )
                            .await;
                            host::update_text_field(
                                host.id,
                                "attributes",
                                serde_json::to_string(&host.attributes).unwrap_or("".to_string()),
                                receiver_pool.acquire().await.unwrap(),
                            )
                            .await;
                            host::update_text_field(
                                host.id,
                                "seed_key",
                                host.seed_key.to_string(),
                                receiver_pool.acquire().await.unwrap(),
                            )
                            .await;
                            let filter = format!("id='{}'", host.id);
                            let central_host = host::get_hosts_from_db(
                                Some(&filter),
                                receiver_pool.acquire().await.unwrap(),
                            )
                            .await;

                            debug!("{:?}", central_host);

                            let mut this_host = recv_arc_this_host.lock().await;
                            *this_host = central_host.first().cloned();
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

async fn agent_auth(headers: HeaderMap, who: &SocketAddr, pool: SqlitePool) -> Option<Uuid> {
    let who = who.to_string();

    // make sure header is present and a uuid, otherwise instant reject
    let Some(api_key) = headers.get("X_API_KEY") else { return None };
    let Ok(api_key) = api_key.to_str() else { return None };
    let Ok(api_key) = api_key.parse::<Uuid>() else { return None };
    debug!("X_API_KEY: {}", api_key);

    // make sure header is present and a uuid, otherwise instant reject
    let Some(seed_key) = headers.get("X_SEED_KEY") else { return None };
    let Ok(seed_key) = seed_key.to_str() else { return None };
    let Ok(seed_key) = seed_key.parse::<Uuid>() else { return None };
    debug!("X_SEED_KEY: {}", seed_key);

    // make sure header is present and a uuid, otherwise instant reject
    let Some(agent_id) = headers.get("X_AGENT_ID") else { return None };
    let Ok(agent_id) = agent_id.to_str() else { return None };
    let Ok(agent_id) = agent_id.parse::<Uuid>() else { return None };
    debug!("X_AGENT_ID: {}", agent_id);

    // make sure header is present and a uuid, otherwise instant reject
    let Some(agent_alias) = headers.get("X_AGENT_ALIAS") else { return None };
    let Ok(agent_alias) = agent_alias.to_str() else { return None };
    debug!("X_AGENT_ALIAS: {}", agent_alias);

    // let's see if the host is already present
    let filter = format!("id='{agent_id}'",);
    let host_vec = get_hosts_from_db(Some(&filter), pool.acquire().await.unwrap()).await;
    let host = match host_vec.first().cloned() {
        Some(h) => h,
        None => {
            // if host is not in DB yet, create placeholder to track approval process
            info!(
                "New agent {agent_alias} ({agent_id}) from host {who} trying to connect to Server"
            );
            let new_host = Host {
                id: agent_id,
                alias: agent_alias.into(),
                attributes: vec!["placeholder".to_string()],
                ip: who.clone(),
                seed_key,
                api_key: None,
                api_key_ttl: None,
                last_pong: None,
            };
            let _ins = new_host
                .clone()
                .insert_into_db(pool.acquire().await.unwrap())
                .await;
            new_host
        }
    };

    // database says no approval yet, get out
    let Some(db_key) = host.api_key else {
        warn!("Api key for agent {agent_alias} ({agent_id}) from host {who} is not approved. Closing connection");
        return None;
    };

    // database says no ttl yet, get out
    let Some(db_ttl) = host.api_key_ttl else {
        warn!("Api key for agent {agent_alias} ({agent_id}) from host {who} has no ttl. Closing connection");
        return None;
    };

    // agent is locked, get out
    if db_key.is_nil() {
        warn!("Api key for agent {agent_alias} ({agent_id}) from host {who} is locked. Closing connection");
        return None;
    };

    // agent has outdated key, get out
    if db_ttl < Utc::now() {
        warn!("Api key for agent {agent_alias} ({agent_id}) from host {who} is outdated. Closing connection");
        return None;
    };

    // invalid key
    if db_key != api_key {
        debug!("{db_key} - db key");
        debug!("{api_key} - host key");
        warn!("Api key for agent {agent_alias} ({agent_id}) from host {who} is invalid. Closing connection");
        return None;
    }

    // api_key is known and valid
    if db_key == api_key && db_ttl > Utc::now() {
        let ttl_warn = Utc::now().checked_add_days(Days::new(7)).unwrap();
        if db_ttl < ttl_warn {
            warn!("Api key for agent {agent_alias} ({agent_id}) from host {who} has less than 7 days to live. Please refresh!");
        }
        return Some(Uuid::nil());
    };

    if seed_key == host.seed_key && api_key.is_nil() && db_ttl > Utc::now() {
        info!("Connection approved, sending API key to {agent_alias} ({agent_id})");
        return Some(db_key);
    }

    // TODO: Implement update/outdated APIKEYS
    // TODO: Handle agent restart while approving keys
    warn!("unknown condition, check code!");
    None
}

async fn generate_execution_timestamp(
    schedule: &Schedule,
    connection: PoolConnection<Sqlite>,
) -> Option<DateTime<Utc>> {
    debug!("Generating execution for schedule {}", schedule.id);
    match &schedule.timer {
        Timer::Cron(c) => {
            let cron_iter = c.split_ascii_whitespace();
            let cron = match cron_iter.count() {
                5 => format!("0 {} *", c),
                7 => c.to_owned(),
                _ => {
                    error!(
                        "Schedule {}: Cron {c} has wrong format needs to be 5 part or 7 part cron. Skipped",
                        schedule.id
                    );
                    return None;
                }
            };
            let cron_schedule = match cron.parse::<cron::Schedule>() {
                Ok(cc) => cc,
                Err(e) => {
                    error!(
                        "Schedule {}: Cron {cron} parsing err. Skipped \n {e}",
                        schedule.id
                    );
                    return None;
                }
            };
            let ts_vec: Vec<DateTime<Utc>> = cron_schedule.upcoming(Utc).take(1).collect();
            return ts_vec.first().copied();
        }
        Timer::Timestamp(ts) => {
            schedule::update_text_field(schedule.id, "active", "0".into(), connection).await;
            Some(*ts)
        }
    }
}
