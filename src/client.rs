use std::{sync::Arc, thread, time::Duration};

use async_recursion::async_recursion;
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use tokio::net::TcpStream;
use tokio::sync::{mpsc::Sender, Mutex};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{self, http::Request, protocol::Message, Error},
    MaybeTlsStream, WebSocketStream,
};

#[derive(Debug)]
pub struct WsClient {
    pub write: Option<Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>,
    pub read: Option<Mutex<SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>>>,
    pub tx: Arc<Mutex<Sender<bool>>>,
    pub ws_endpoint: &'static str,
}

impl WsClient {
    #[async_recursion]
    pub async fn init(&mut self) {
        println!("Connecting to {}", self.ws_endpoint);
        let request = self.new_request(self.ws_endpoint);
        match connect_async(request).await {
            Ok((stream, _)) => {
                let (write, read) = stream.split();
                println!("Successfully connected to {}", self.ws_endpoint);
                self.write = Some(Mutex::new(write));
                self.read = Some(Mutex::new(read));
                self.tx.lock().await.send(true).await.unwrap();
            }
            Err(err) => {
                println!("Error connecting to websocket {}", err);
                thread::sleep(Duration::from_millis(1500));
                self.init().await
            }
        }
    }

    pub async fn send(&mut self, command: String) {
        let cloned_msg = command.clone();
        if let Some(r) = &self.write {
            let result = r.lock().await.send(Message::Text(command)).await;
            match result {
                Ok(_) => {
                    println!("Successfully sent {}", cloned_msg);
                }
                Err(err) => match err {
                    Error::ConnectionClosed | Error::Protocol(_) | Error::Utf8 => {
                        println!("{:#?}", err);
                        self.init().await;
                    }
                    err => {
                        println!("test: {}", err);
                        self.init().await;
                    }
                },
            }
        }
    }

    pub fn new_request(&self, client_endpoint: &str) -> Request<()> {
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
}
