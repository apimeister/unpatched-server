//! Example websocket server.
//!
//! Run the server with
//! ```not_rust
//! cargo run -p example-websockets --bin example-websockets
//! ```
//!
//! Run a browser client with
//! ```not_rust
//! firefox http://localhost:3000
//! ```
//!
//! Alternatively you can run the rust client (showing two
//! concurrent websocket connections being established) with
//! ```not_rust
//! cargo run -p example-websockets --bin example-client
//! ```

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
    Router, Json, http::StatusCode,
};
use axum_extra::TypedHeader;
use serde::{Deserialize, Serialize};

use std::{borrow::Cow, time::Duration};
use std::ops::ControlFlow;
use std::{net::SocketAddr, path::PathBuf};
use tower_http::{
    services::ServeDir,
    trace::{DefaultMakeSpan, TraceLayer},
};

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

//allows to extract the IP of connecting user
use axum::extract::connect_info::ConnectInfo;
use axum::extract::ws::CloseFrame;

//allows to split the websocket stream into separate TX and RX branches
use futures::{sink::SinkExt, stream::StreamExt};

use once_cell::sync::Lazy;
use std::{collections::HashMap, sync::Mutex};

use clap::Parser;

const UPDATE_RATE: Duration = Duration::new(0, 5000);

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "127.0.0.1")]
    server: String,
    #[arg(short, long, default_value = "3000")]
    port: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
struct AgentData {
    host_name: String,
    os_release: String,
    uptime: u32,
}

static GLOBAL_CACHE: Lazy<Mutex<HashMap<String, AgentData>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

enum User {
    Web,
    Agent,
}

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

    // build our application with some routes
    let app = Router::new()
        .fallback_service(ServeDir::new(assets_dir).append_index_html_on_directories(true))
        .route("/ws", get(ws_handler))
        .route("/api", get(stats_api))
        // logging so we can see whats going on
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        );

    // run it with hyper
    let args = Args::parse();
    let listener = tokio::net::TcpListener::bind(args.server + ":" + args.port.as_str())
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
    user_agent: Option<TypedHeader<headers::UserAgent>>,
    host_name: Option<TypedHeader<headers::Host>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let user_agent = if let Some(TypedHeader(user_agent)) = user_agent {
        user_agent.to_string()
    } else {
        String::from("Unknown browser")
    };
    let host_name = if let Some(TypedHeader(host_name)) = host_name {
        host_name.to_string()
    } else {
        String::from("Unknown Host")
    };
    let client_kind: User;
    if user_agent == "internal-monitoring-agent/1.0" {
        client_kind = User::Agent;
        let _ = GLOBAL_CACHE.lock().unwrap().insert(
            addr.to_string(),
            AgentData {
                uptime: 0,
                os_release: "".into(),
                host_name: host_name.clone(),
            },
        );
    } else {
        client_kind = User::Web;
    }
    println!("`{user_agent}` for {host_name} at {addr} connected.");
    // finalize the upgrade process by returning upgrade callback.
    // we can customize the callback by sending additional info such as address.
    ws.on_upgrade(move |socket| handle_socket(socket, addr, client_kind))
}

fn parse_json() -> String {
    let mut json: String = String::new();
    for (_, v) in GLOBAL_CACHE.lock().unwrap().iter() {
        json = format!(
            "{{\"hostname\":\"{}\",\"os-release\":{:?},\"uptime\":\"{}\"}}",
            v.host_name, v.os_release, v.uptime
        );
    }
    json
}

/// Actual websocket statemachine (one will be spawned per connection)
async fn handle_socket(mut socket: WebSocket, who: SocketAddr, client_kind: User) {
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
    if let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            if process_message(msg, who, &client_kind).is_break() {
                return;
            }
        } else {
            println!("client {who} abruptly disconnected");
            return;
        }
    }

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

    match client_kind {
        User::Web => {
            // let mut yy: String = "".into();
            // for (k,v) in GLOBAL_CACHE.lock().unwrap().iter() {
            //     yy = yy + format!("host: {k}, {:?}", v).as_str();
            // };
            if socket.send(Message::Text(parse_json())).await.is_err() {
                println!("client {who} abruptly disconnected");
                return;
            }
        }
        User::Agent => {
            // By splitting socket we can send and receive at the same time. In this example we will send
            // unsolicited messages to client based on some sort of server's internal event (i.e .timer).
            let (mut sender, mut receiver) = socket.split();

            // Spawn a task that will push several messages to the client (does not matter what client does)
            let send_task = tokio::spawn(async move {
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
            let recv_task = tokio::spawn(async move {
                let mut cnt = 0;
                while let Some(Ok(msg)) = receiver.next().await {
                    cnt += 1;
                    // print message and break if instructed to do so
                    if process_message(msg, who, &client_kind).is_break() {
                        break;
                    }
                }
                cnt
            });

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
        }
    }

    // returning from the handler closes the websocket connection
    println!("Websocket context {} destroyed", who);
}

/// helper to print contents of messages to stdout. Has special treatment for Close.
fn process_message(msg: Message, who: SocketAddr, client_kind: &User) -> ControlFlow<(), ()> {
    match client_kind {
        User::Web => {
            println!("++++starting the web processing!");
            match msg {
                Message::Text(t) => {
                    println!(">>> {} sent str: {:?}", who, t);
                }
                Message::Binary(d) => {
                    println!(">>> {} sent {} bytes: {:?}", who, d.len(), d);
                }
                Message::Close(c) => {
                    if let Some(cf) = c {
                        println!(
                            ">>> {} sent close with code {} and reason `{}`",
                            who, cf.code, cf.reason
                        );
                    } else {
                        println!(">>> {} somehow sent close message without CloseFrame", who);
                    }
                    return ControlFlow::Break(());
                }

                Message::Pong(v) => {
                    println!(">>> {} sent pong with {:?}", who, v);
                }
                // You should never need to manually handle Message::Ping, as axum's websocket library
                // will do so for you automagically by replying with Pong and copying the v according to
                // spec. But if you need the contents of the pings you can see them here.
                Message::Ping(v) => {
                    println!(">>> {} sent ping with {:?}", who, v);
                }
            }
        }
        User::Agent => {
            println!("++++starting the web processing!");
            match msg {
                Message::Text(t) => {
                    let (k, v) = t.split_once(':').unwrap();
                    let _ = match k {
                        "os" => {
                            GLOBAL_CACHE
                                .lock()
                                .unwrap()
                                .entry(who.to_string())
                                .and_modify(|e| e.os_release = v.into());
                            ()
                        }
                        "uptime" => {
                            let uptime: f32 = v.parse().unwrap_or(0.0);
                            GLOBAL_CACHE
                                .lock()
                                .unwrap()
                                .entry(who.to_string())
                                .and_modify(|e| e.uptime = uptime as u32);
                            ()
                        }
                        _ => {}
                    };
                    println!(">>> {} sent str: {:?}", who, t);
                }
                Message::Binary(d) => {
                    println!(">>> {} sent {} bytes: {:?}", who, d.len(), d);
                }
                Message::Close(c) => {
                    if let Some(cf) = c {
                        println!(
                            ">>> {} sent close with code {} and reason `{}`",
                            who, cf.code, cf.reason
                        );
                    } else {
                        println!(">>> {} somehow sent close message without CloseFrame", who);
                    }
                    return ControlFlow::Break(());
                }

                Message::Pong(v) => {
                    println!(">>> {} sent pong with {:?}", who, v);
                }
                // You should never need to manually handle Message::Ping, as axum's websocket library
                // will do so for you automagically by replying with Pong and copying the v according to
                // spec. But if you need the contents of the pings you can see them here.
                Message::Ping(v) => {
                    println!(">>> {} sent ping with {:?}", who, v);
                }
            }
        }
    }

    ControlFlow::Continue(())
}

async fn stats_api() -> (StatusCode, Json<Vec<AgentData>>) {
    match GLOBAL_CACHE.lock() {
        Ok(res) => {
            let v_ad = res.values().cloned().collect::<Vec<AgentData>>();
            ( StatusCode::OK, Json(v_ad))
        }
        Err(_) => (StatusCode::NOT_FOUND, Json(Vec::new()))
    }}

