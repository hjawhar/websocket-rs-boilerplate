use std::{
    mem,
    sync::Arc,
    thread,
    time::{Duration, SystemTime},
};

use async_recursion::async_recursion;
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use serde_json::json;
use tokio::sync::Mutex;
use tokio::{net::TcpStream, task::JoinHandle};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{self, http::Request, protocol::Message, Error},
    MaybeTlsStream, WebSocketStream,
};

pub struct WsClient {
    pub write: Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>,
    pub read: Mutex<SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>>,
}

impl WsClient {
    #[async_recursion]
    pub async fn new(bx_ws: &str) -> WsClient {
        println!("Connecting to {}", bx_ws);
        let request = new_request(bx_ws);
        match connect_async(request).await {
            Ok((stream, _)) => {
                let (write, read) = stream.split();
                println!("Successfully connected to {}", bx_ws);
                return WsClient {
                    write: Mutex::new(write),
                    read: Mutex::new(read),
                };
            }
            Err(err) => {
                println!("Error connecting to websocket {}", err);
                thread::sleep(Duration::from_millis(5000));
                Self::new(bx_ws).await
            }
        }
    }

    pub async fn send(&mut self, command: String) {
        let cloned_msg = command.clone();
        let result: Result<(), Error> = self.write.lock().await.send(Message::Text(command)).await;
        match result {
            Ok(_) => {
                println!("Successfully sent {}", cloned_msg);
            }
            Err(err) => match err {
                Error::ConnectionClosed | Error::Protocol(_) | Error::Utf8 => {
                    print!("{:#?}", err);
                }
                err => println!("test: {}", err),
            },
        }
    }

    pub async fn ping(&mut self) {
        self.send(
            json!({
                "ping":1
            })
            .to_string(),
        )
        .await;
    }
}

pub fn new_request(client_endpoint: &str) -> Request<()> {
    let url = client_endpoint;
    let req = Request::builder()
        .method("GET")
        .header("Host", "localhost")
        .header("Connection", "Upgrade")
        .header("Upgrade", "websocket")
        .header("Sec-WebSocket-Version", "13")
        .header(
            "Sec-WebSocket-Key",
            tungstenite::handshake::client::generate_key(),
        )
        // .header("Authorization", client_auth_header)
        .uri(url)
        .body(())
        .unwrap();
    return req;
}

#[derive(Debug)]
pub struct CustomErr {
    pub msg: String,
}

#[tokio::main]
async fn main() -> Result<(), CustomErr> {
    println!("Starting app...");
    let client: WsClient = WsClient::new("ws://localhost:3000").await;
    let ws_client_clone1: Arc<Mutex<WsClient>> = Arc::new(Mutex::new(client));
    let mut ws_client_clone2: Arc<Mutex<WsClient>> = ws_client_clone1.clone();

    let mut thread_handles: Vec<JoinHandle<()>> = vec![];
    thread_handles.push(tokio::spawn(async move {
        while let Some(response) = ws_client_clone1.lock().await.read.lock().await.next().await {
            match response {
                Ok(line) => {
                    println!("{}", line);
                    ws_client_clone2
                        .lock()
                        .await
                        .send("command".to_string())
                        .await;
                }
                Err(err) => {
                    println!("Yolo {}", err);
                    let client: WsClient = WsClient::new("ws://localhost:3000").await;
                    *ws_client_clone1.lock().await = client;
                    mem::replace(&mut ws_client_clone2, ws_client_clone1.clone());
                }
            }
        }
    }));

    thread_handles.push(tokio::spawn(async move {
        // thread::sleep(Duration::from_millis(10000));
        // println!("yolo 123");

        println!("Test");
    }));

    let join_res = futures::future::join_all(thread_handles).await;
    for e in join_res {
        if e.is_err() {
            return Err(CustomErr {
                msg: e.err().unwrap().to_string(),
            });
        }
    }

    Ok(()) // Result ok.
}
