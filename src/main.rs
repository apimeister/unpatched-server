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
use email_address::EmailAddress;
use futures::{sink::SinkExt, stream::StreamExt};
use futures_util::{future::join_all, stream::SplitSink};
use headers::HeaderMap;
use host::get_hosts_from_db;
use include_dir::{include_dir, Dir};
use jwt::KEYS;
use once_cell::sync::OnceCell;
use schedule::Schedule;
use serde::{Deserialize, Serialize};
use sqlx::{pool::PoolConnection, sqlite::SqlitePool, Sqlite};
use std::{fs::File, io::ErrorKind, path::PathBuf, time::Duration};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::Mutex;
use tower_http::trace::{DefaultMakeSpan, TraceLayer};
use tracing::{debug, error, info, warn};
use tracing_subscriber::{fmt, layer::SubscriberExt, registry, util::SubscriberInitExt, EnvFilter};
use uuid::Uuid;

mod db;
mod execution;
mod host;
mod jwt;
mod schedule;
mod script;
mod swagger;
mod user;
mod webpage;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Default)]
struct ScriptExec {
    pub id: Uuid,
    pub script: script::Script,
}

static WEBPAGE: Dir = include_dir!("$CARGO_MANIFEST_DIR/target/site");
static API_YAML: &[u8] = include_bytes!("../api.yaml");

type SenderSinkArc = Arc<Mutex<SplitSink<WebSocket, Message>>>;

/// A bash first monitoring solution
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// bind adress for frontend and agent websockets, v6 example [::1]
    #[arg(short, long, default_value = "127.0.0.1")]
    bind: String,
    /// bind port for frontend and agent websockets
    #[arg(short, long, default_value = "3000")]
    port: String,
    /// deactivate tls
    #[arg(long)]
    no_tls: bool,
    /// use 7 part instead of 5 part cron pattern
    #[arg(long)]
    seven_part_cron: bool,
    /// Sets the certificate folder
    #[arg(long, value_name = "FOLDER", default_value = "./self-signed-certs")]
    cert_folder: PathBuf,
    /// Email of first user to initialize the server with
    #[arg(long)]
    init_user: Option<EmailAddress>,
    /// Password of first user to initialize the server with
    #[arg(long)]
    init_password: Option<String>,
}

const UPDATE_RATE: Duration = Duration::new(5, 0);
const SQLITE_DB: &str = "sqlite:unpatched_server_internal.sqlite";
const TLS_CERT: &str = "unpatched.server.crt";
const TLS_KEY: &str = "unpatched.server.key";
const JWT_SECRET: &str = "jwt.secret";
const API_KEY_LOGIN_TTL: u64 = 30;

static CRON: OnceCell<bool> = OnceCell::new();

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
    let creds = if args.init_user.is_some() && args.init_password.is_some() {
        Some((args.init_user.unwrap(), args.init_password.unwrap()))
    } else {
        None
    };
    db::init_database(&pool, creds)
        .await
        .expect("Unable to initialize database!");

    // cron
    CRON.set(args.seven_part_cron)
        .expect("Error configuring cron format!");

    // JWT secret
    let _init_jwt = &KEYS;

    // build our application with some routes
    let app = Router::new()
        .route("/protected", get(jwt::protected))
        .route("/logout", get(jwt::logout))
        .route("/loginstatus", get(jwt::login_status))
        .route(
            "/api/v1/executions/:id",
            get(execution::get_one_execution_api).delete(execution::delete_one_execution_api),
        )
        .route(
            "/api/v1/executions",
            get(execution::get_executions_api).delete(execution::delete_executions_api),
        )
        .route(
            "/api/v1/scripts/:id",
            get(script::get_one_script_api).delete(script::delete_one_script_api),
        )
        .route(
            "/api/v1/scripts",
            get(script::get_scripts_api)
                .delete(script::delete_scripts_api)
                .post(script::post_scripts_api),
        )
        .route(
            "/api/v1/hosts/:id/deactivate",
            post(host::deactivate_one_host_api),
        )
        .route(
            "/api/v1/hosts/:id/activate",
            post(host::activate_one_host_api),
        )
        .route(
            "/api/v1/hosts/:id/schedules",
            get(schedule::get_host_schedules_api).post(schedule::post_host_schedules_api),
        )
        .route(
            "/api/v1/hosts/:id/executions",
            get(execution::get_host_executions_api),
        )
        .route(
            "/api/v1/hosts/:id",
            get(host::get_one_host_api)
                .patch(host::update_one_host_api)
                .delete(host::delete_one_host_api),
        )
        .route(
            "/api/v1/hosts",
            get(host::get_hosts_api).delete(host::delete_hosts_api),
        )
        .route("/api/v1/hosts/new", post(host::post_hosts_api))
        .route(
            "/api/v1/schedules/:id/executions",
            get(execution::get_schedule_executions_api),
        )
        .route(
            "/api/v1/schedules/:id",
            get(schedule::get_one_schedule_api).delete(schedule::delete_one_schedule_api),
        )
        .route(
            "/api/v1/schedules",
            get(schedule::get_schedules_api)
                .delete(schedule::delete_schedules_api)
                .post(schedule::post_schedules_api),
        )
        .route(
            "/api/v1/users/:id",
            get(user::get_one_user_api)
                .patch(user::update_one_user_api)
                .delete(user::delete_one_user_api)
                .with_state(pool.clone()),
        )
        .route(
            "/api/v1/users",
            get(user::get_users_api)
                .delete(user::delete_users_api)
                .post(user::post_users_api)
                .with_state(pool.clone()),
        )
        // Swagger API
        .route("/api", get(swagger::api_ui))
        .route("/api/v1", get(swagger::api_ui))
        .route("/api/api.yaml", get(swagger::api_def))
        // .route_layer(AuthLayer::verify())
        .route("/api/v1/authorize", post(jwt::api_authorize_user))
        .route(
            "/api/v1/unblock/:id",
            post(jwt::remove_ip_from_blacklist_api),
        )
        // Websocket for Agents
        .route("/ws", get(ws_handler))
        .fallback(webpage::web_page)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        )
        .with_state(pool.clone());

    let addr: SocketAddr = format!("{}:{}", args.bind, args.port).parse().unwrap();

    // spawn http or https depending on --no-tls
    if args.no_tls {
        http_server(app, addr).await;
    } else {
        //TODO check for exiting cert
        let cert_file = File::open(args.cert_folder.join(TLS_CERT));
        let key_file = File::open(args.cert_folder.join(TLS_KEY));
        if cert_file.is_ok() && key_file.is_ok() {
            // use exiting files
            https_server(app, addr, args.cert_folder).await;
        } else {
            info!("no existing TLS certificate found ({TLS_CERT},{TLS_KEY}), generating self signed certificate...");
            https_server_self_signed(app, addr).await;
        }
    }
}

async fn http_server(app: Router, addr: SocketAddr) {
    info!("listening on http://{addr}/");
    match axum_server::bind(addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
    {
        Ok(()) => (),
        Err(e) => error!("{e}"),
    }
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
    match axum_server::bind_rustls(addr, config)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
    {
        Ok(()) => (),
        Err(e) => error!("{e}"),
    }
}

async fn https_server_self_signed(app: Router, addr: SocketAddr) {
    use rcgen::generate_simple_self_signed;
    let subject_alt_names = vec!["hello.world.example".to_string(), "localhost".to_string()];

    let cert = generate_simple_self_signed(subject_alt_names).unwrap();
    let config = RustlsConfig::from_pem(
        cert.serialize_pem().unwrap().as_bytes().to_vec(),
        cert.serialize_private_key_pem().as_bytes().to_vec(),
    )
    .await
    .unwrap();
    info!("listening on https://{addr}/");
    match axum_server::bind_rustls(addr, config)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
    {
        Ok(()) => (),
        Err(e) => error!("{e}"),
    }
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
    match agent_auth(headers, &addr, pool.clone()).await {
        Ok(()) => ws.on_upgrade(move |socket| handle_socket(socket, addr, pool)),
        Err(e) => {
            error!("{e}");
            StatusCode::UNAUTHORIZED.into_response()
        }
    }
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
                let Some(ref host) = &*general_arc_this_host.lock().await else {
                    continue;
                };
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
                let Some(trigger) = generate_execution_timestamp(
                    &sched,
                    general_pool.acquire().await.unwrap(),
                    *CRON.get().unwrap_or(&false),
                )
                .await
                else {
                    continue;
                };
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
                let Some(ref host) = &*sender_arc_this_host.lock().await else {
                    continue;
                };
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
                // lock execution via timestamp 1970
                execution::update_text_field(
                    exe.id,
                    "response",
                    utc_to_str("1970-01-01T00:00:00.000Z".parse::<DateTime<Utc>>().unwrap()),
                    sender_pool.acquire().await.unwrap(),
                )
                .await;
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
                            "Update agent {} as alive (hosts table -> last_checkin)!",
                            host.id
                        );
                        host::update_text_field(
                            host.id,
                            "last_checkin",
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
                                "ip",
                                who.to_string(),
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

async fn agent_auth(headers: HeaderMap, who: &SocketAddr, pool: SqlitePool) -> Result<(), Error> {
    let who = who.to_string();

    // make sure header is present and a uuid, otherwise instant reject
    let agent_id = headers
        .get("X_API_KEY")
        .ok_or(Error::new("Header X_API_KEY not found"))?;
    let agent_id = agent_id
        .to_str()
        .map_err(|_| Error::new("Can't parse X_API_KEY header"))?;
    let agent_id = agent_id
        .parse::<Uuid>()
        .map_err(|_| Error::new("Can't parse X_API_KEY header"))?;
    debug!("X_API_KEY: {}", agent_id);

    // let's see if the host is already present
    let filter = format!("id='{agent_id}'",);
    let host_vec = get_hosts_from_db(Some(&filter), pool.acquire().await.unwrap()).await;
    // unknown ID/API_KEY -> kick
    let host = host_vec
        .first()
        .cloned()
        .ok_or(Error::new("No agent found with this ID"))?;

    // agent is deactivated, get out
    if !host.active {
        warn!(
            "Agent {} ({}) on host {who} is marked as inactive on Server. Closing connection",
            host.alias, host.id
        );
        return Err(Error::new(""));
    };

    if let Some(check_in) = host.last_checkin {
        if check_in
            < Utc::now()
                .checked_sub_days(Days::new(API_KEY_LOGIN_TTL))
                .unwrap()
        {
            warn!(
                "Agent {} ({}) on host {who} tries to use outdated API_KEY, older than {API_KEY_LOGIN_TTL} days. Closing connection",
                host.alias, host.id
            );
            return Err(Error::new(""));
        }
    };

    Ok(())
}

async fn generate_execution_timestamp(
    schedule: &Schedule,
    connection: PoolConnection<Sqlite>,
    seven_part_cron: bool,
) -> Option<DateTime<Utc>> {
    debug!("Generating execution for schedule {}", schedule.id);
    match &schedule.timer {
        Timer::Cron(c) => {
            let cron = if seven_part_cron {
                c.to_string()
            } else {
                format!("0 {} *", c)
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

#[cfg(test)]
mod tests {
    use crate::db::{create_database, init_database};

    use super::*;
    use chrono::Days;
    use tracing_subscriber::{
        fmt, layer::SubscriberExt, registry, util::SubscriberInitExt, EnvFilter,
    };

    #[tokio::test]
    async fn test_generate_execution_timestamp() {
        registry()
            .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "debug".into()))
            .with(fmt::layer())
            .try_init()
            .unwrap_or(());

        // prepare DB with user
        let pool = create_database("sqlite::memory:").await.unwrap();
        init_database(&pool, None).await.unwrap();

        let schedule = Schedule {
            timer: Timer::Cron("0 0 * * *".into()),
            ..Default::default()
        };
        let exe =
            generate_execution_timestamp(&schedule, pool.acquire().await.unwrap(), false).await;
        assert!(exe.is_some());
        assert_eq!(
            format!("{}", exe.unwrap()),
            format!(
                "{} 00:00:00 UTC",
                Utc::now()
                    .date_naive()
                    .checked_add_days(Days::new(1))
                    .unwrap()
            )
        );

        let schedule = Schedule {
            timer: Timer::Cron("".into()),
            ..Default::default()
        };
        let exe =
            generate_execution_timestamp(&schedule, pool.acquire().await.unwrap(), false).await;
        assert!(exe.is_none());
    }
}
