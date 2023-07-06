use std::str::FromStr;
use std::time::Duration;
use std::vec;
use std::{net::SocketAddr, path::PathBuf};

use axum::{
    extract::connect_info::ConnectInfo,
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use axum_extra::TypedHeader;
use clap::Parser;
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use sqlx::{
    query,
    sqlite::{SqliteConnectOptions, SqlitePool},
    Row,
};
use tower_http::{
    services::ServeDir,
    trace::{DefaultMakeSpan, TraceLayer},
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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
    uptime: u32,
}

impl AgentData {
    fn new() -> AgentData {
        AgentData {
            ..Default::default()
        }
    }
}

enum AllowedFields {
    Id(String),
    Alias(String),
    OsRelease(String),
    Uptime(String),
    Discard,
}

const UPDATE_RATE: Duration = Duration::new(0, 5000);

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "example_websockets=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let assets_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");

    // create database
    // sqlite::memory:	Open an in-memory database.
    // sqlite:data.db	Open the file data.db in the current directory.
    // sqlite://data.db	Open the file data.db in the current directory.
    // sqlite:///data.db	Open the file data.db from the root (/) directory.
    // sqlite://data.db?mode=ro	Open the file data.db for read-only access.
    let connection_options = SqliteConnectOptions::from_str("sqlite:monitor_server_internal.db")
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
                os_release VARCHAR(255)
            )"#,
        )
        .execute(&mut *conn)
        .await
        .unwrap();
        // let _first_data = query(
        //     r#"INSERT INTO data(id, name, uptime, os_release) VALUES (?, "test", ?, "test")"#,
        // )
        // .bind(uuid)
        // .bind(100)
        // .fetch_optional(&mut *conn)
        // .await
        // .unwrap();
        // let first_data = query(r#"INSERT INTO data(id, data) VALUES (123, 'cool-data-2')"#).fetch_all(&mut *conn).await.unwrap();
        // let show_data = query("SELECT * FROM data")
        //     .fetch_all(&mut *conn)
        //     .await
        //     .unwrap();
        // for data in show_data {
        //     println!(
        //         "{} | {} | {} | {}",
        //         data.get::<String, _>("id"),
        //         data.get::<String, _>("name"),
        //         data.get::<i32, _>("uptime"),
        //         data.get::<String, _>("os_release")
        //     );
        // }
    }

    // build our application with some routes
    let app = Router::new()
        .fallback_service(ServeDir::new(assets_dir).append_index_html_on_directories(true))
        .route("/ws", get(ws_handler).with_state(pool.clone()))
        .route("/api", get(stats_api).with_state(pool))
        // logging so we can see whats going on
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        );

    // run it with hyper
    let args = Args::parse();
    let listener = tokio::net::TcpListener::bind(args.bind + ":" + args.port.as_str())
        .await
        .unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
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
    user_agent: Option<TypedHeader<headers::UserAgent>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let user_agent = if let Some(TypedHeader(user_agent)) = user_agent {
        user_agent.to_string()
    } else {
        String::from("Unknown browser")
    };

    // let _ = GLOBAL_CACHE.lock().unwrap().insert(
    //     addr.to_string(),
    //     AgentData {
    //         uptime: 0,
    //         os_release: "".into(),
    //         host_name: host_name.clone(),
    //     },
    // );

    println!("`{user_agent}` at {addr} connected.");
    // finalize the upgrade process by returning upgrade callback.
    // we can customize the callback by sending additional info such as address.
    ws.on_upgrade(move |socket| handle_socket(socket, addr, pool))
}

// fn parse_json() -> String {
//     let mut json: String = String::new();
//     for (_, v) in GLOBAL_CACHE.lock().unwrap().iter() {
//         json = format!(
//             "{{\"hostname\":\"{}\",\"os-release\":{:?},\"uptime\":\"{}\"}}",
//             v.host_name, v.os_release, v.uptime
//         );
//     }
//     json
// }

/// Actual websocket statemachine (one will be spawned per connection)
async fn handle_socket(mut socket: WebSocket, who: SocketAddr, pool: SqlitePool) {
    // let mut agent_data = AgentData::new();
    //send a ping (unsupported by some browsers) just to kick things off and get a response
    if socket.send(Message::Ping(vec![1, 2, 3])).await.is_ok() {
        println!("Pinged {}...", who);
    } else {
        println!("Could not send ping {}!", who);
        // no Error here since the only thing we can do is to close the connection.
        // If we can not send messages, there is no way to salvage the statemachine anyway.
        return;
    }

    // receive single message from a client (we can either receive or send with socket).
    // this will likely be the Pong for our Ping or a hello message from client.
    // waiting for message from a client will block this task, but will not block other client's
    // connections.
    // if let Some(msg) = socket.recv().await {
    //     if let Ok(msg) = msg {
    //         if process_message(msg, who, None).is_break() {
    //             return;
    //         }
    //     } else {
    //         println!("client {who} abruptly disconnected");
    //         return;
    //     }
    // }

    // Since each client gets individual statemachine, we can pause handling
    // when necessary to wait for some external event (in this case illustrated by sleeping).
    // Waiting for this client to finish getting its greetings does not prevent other clients from
    // connecting to server and receiving their greetings.
    // for i in 1..5 {
    //     if socket
    //         .send(Message::Text(format!("Hi {i} times!")))
    //         .await
    //         .is_err()
    //     {
    //         println!("client {who} abruptly disconnected");
    //         return;
    //     }
    //     tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
    // }

    // By splitting socket we can send and receive at the same time. In this example we will send
    // unsolicited messages to client based on some sort of server's internal event (i.e .timer).
    let (mut sender, mut receiver) = socket.split();

    // Spawn a task that will push several messages to the client (does not matter what client does)
    let _send_task = tokio::spawn(async move {
        let n_msg = 20;
        for i in 0..n_msg {
            // In case of any websocket error, we exit.
            if sender
                .send(Message::Text(format!("Server message {i} ...")))
                .await
                .is_err()
            {
                return i;
            }
            tokio::time::sleep(UPDATE_RATE).await;
        }

        // println!("Sending close to {who}...");
        // if let Err(e) = sender
        //     .send(Message::Close(Some(CloseFrame {
        //         code: axum::extract::ws::close_code::NORMAL,
        //         reason: Cow::from("Goodbye"),
        //     })))
        //     .await
        // {
        //     println!("Could not send Close due to {}, probably it is ok?", e);
        // }
        n_msg
    });

    // This second task will receive messages from client and print them on server console
    let _recv_task = tokio::spawn(async move {
        // let mut cnt = 0;
        let mut id: Option<AllowedFields> = None;
        while let Some(Ok(msg)) = receiver.next().await {
            // cnt += 1;
            let mut data = AllowedFields::Discard;
            match msg {
                Message::Close(_) => break,
                Message::Ping(_) => println!("Received a Ping!"),
                Message::Pong(_) => println!("Received a Pong!"),
                Message::Text(t) => {
                    let (k, v) = t.split_once(':').unwrap();
                    data = match k {
                        "uuid" => {
                            id = Some(AllowedFields::Id(v.to_string()));
                            let mut conn = pool.acquire().await.unwrap();
                            let id_res = query(r#"INSERT INTO data(id, name, uptime, os_release) VALUES (?, "", 0, "")"#).bind(v).execute(&mut *conn).await;
                            // TO-DO: This should be some real error handling
                            if id_res.is_err() {
                                println!("Agent with Id {v} already known")
                            }
                            AllowedFields::Id(v.to_string())
                        }
                        "alias" => AllowedFields::Alias(v.to_string()),
                        "os" => AllowedFields::OsRelease(v.to_string()),
                        "uptime" => {
                            let (time, _) = v.split_once(".").unwrap_or((v, v));
                            AllowedFields::Uptime(time.to_string())
                        }
                        _ => AllowedFields::Discard,
                    };
                }
                Message::Binary(_) => println!("Binary is unsupported!"),
            };

            // without ID, skip and wait for next update cycle
            if let Some(AllowedFields::Id(agent_id)) = &id {
                let mut conn = pool.acquire().await.unwrap();
                let (field, value) = match data {
                    AllowedFields::Alias(v) => ("name", v),
                    AllowedFields::Uptime(v) => ("uptime", v),
                    AllowedFields::OsRelease(v) => ("os_release", v),
                    _ => continue,
                };

                println!("Received some data for {field}: {value}");

                let _ = query(
                    format!(
                        r#"UPDATE data SET {field} = ?
                WHERE id = ?"#
                    )
                    .as_str(),
                )
                .bind(value)
                .bind(agent_id)
                .execute(&mut *conn)
                .await
                .unwrap();

                // let res = query(r#"SELECT * FROM data WHERE id = ?"#).bind(agent_id).fetch_all(&mut *conn).await.unwrap();
                // for r in res {
                //     println!("{}", r.get::<String,_>("id"));
                //     println!("{}", r.get::<String,_>("name"));
                //     println!("{}", r.get::<i32,_>("uptime"));
                //     println!("{}", r.get::<String,_>("os_release"));
                // }
            }

            // if let Message::Close(_) = msg {
            //     break;
            // } else {}

            // // print message and break if instructed to do so
            // if process_message(msg, who, Some(&mut agent_data)).is_break() {
            //     break;
            // }
        }
    });

    // let db_task = tokio::spawn(async move {
    //     if !&agent_data.id.is_empty() {
    //         let mut conn = pool.acquire().await.unwrap();
    //         // make sure ID is there, otherwise create
    //         let _first_data = query(
    //             r#"REPLACE INTO data(id, name, uptime, os_release) VALUES (?, ?, ?, ?)"#,
    //         )
    //         .bind(agent_data.id)
    //         .bind(agent_data.alias)
    //         .bind(agent_data.uptime)
    //         .bind(agent_data.os_release)
    //         // .bind(100)
    //         .fetch_optional(&mut *conn)
    //         .await
    //         .unwrap();

    //     }

    // });

    // If any one of the tasks exit, abort the other.
    // tokio::select! {
    //     rv_a = (&mut send_task) => {
    //         match rv_a {
    //             Ok(a) => println!("{} messages sent to {}", a, who),
    //             Err(a) => println!("Error sending messages {:?}", a)
    //         }
    //         recv_task.abort();
    //     },
    //     rv_b = (&mut recv_task) => {
    //         match rv_b {
    //             Ok(b) => println!("Received {} messages", b),
    //             Err(b) => println!("Error receiving messages {:?}", b)
    //         }
    //         send_task.abort();
    //     }
    // }

    // returning from the handler closes the websocket connection
    println!("Websocket context {} destroyed", who);
}

/// helper to print contents of messages to stdout. Has special treatment for Close.
// fn process_message(msg: Message, who: SocketAddr, mut agent_data: Option<&AgentData>) -> ControlFlow<(), ()> {
//     println!("++++starting processing!");
//     match msg {
//         Message::Text(t) => {
//             if let Some(mut agent_data) = agent_data {
//                 let (k, v) = t.split_once(':').unwrap();
//                 match k {
//                     "uuid" => agent_data.id = v.to_string(),
//                     "alias" => agent_data.alias = v.to_string(),
//                     "os" => agent_data.os_release = v.to_string(),
//                     "uptime" => agent_data.uptime = {
//                         let float_uptime: f32 = v.parse().unwrap_or(0.0);
//                         float_uptime as u32
//                     },
//                     _ => {}
//                 };
//                 println!(">>> {} sent str: {:?}", who, t);
//             }

//         }
//         Message::Binary(d) => {
//             println!(">>> {} sent {} bytes: {:?}", who, d.len(), d);
//         }
//         Message::Close(c) => {
//             if let Some(cf) = c {
//                 println!(
//                     ">>> {} sent close with code {} and reason `{}`",
//                     who, cf.code, cf.reason
//                 );
//             } else {
//                 println!(">>> {} somehow sent close message without CloseFrame", who);
//             }
//             return ControlFlow::Break(());
//         }

//         Message::Pong(v) => {
//             println!(">>> {} sent pong with {:?}", who, v);
//         }
//         // You should never need to manually handle Message::Ping, as axum's websocket library
//         // will do so for you automagically by replying with Pong and copying the v according to
//         // spec. But if you need the contents of the pings you can see them here.
//         Message::Ping(v) => {
//             println!(">>> {} sent ping with {:?}", who, v);
//         }
//     }

//     ControlFlow::Continue(())
// }

async fn stats_api(State(pool): State<SqlitePool>) -> (StatusCode, Json<Vec<AgentData>>) {
    let mut conn = pool.acquire().await.unwrap();
    let show_data = match query("SELECT * FROM data").fetch_all(&mut *conn).await {
        Ok(d) => d,
        Err(_) => return (StatusCode::NOT_FOUND, Json(Vec::new())),
    };

    let mut agents_vec: Vec<AgentData> = vec![];

    for d in show_data {
        let mut agent = AgentData::new();
        agent.id = d.get::<String, _>("id");
        agent.alias = d.get::<String, _>("name");
        agent.uptime = d.get::<u32, _>("uptime");
        agent.os_release = d.get::<String, _>("os_release");
        agents_vec.push(agent);
    }

    (StatusCode::OK, Json(agents_vec))

    // match GLOBAL_CACHE.lock() {
    //     Ok(res) => {
    //         let v_ad = res.values().cloned().collect::<Vec<AgentData>>();
    //         (StatusCode::OK, Json(v_ad))
    //     }
    //     Err(_) => (StatusCode::NOT_FOUND, Json(Vec::new())),
    // }
}
