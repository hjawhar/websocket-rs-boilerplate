use std::{sync::Arc, thread, time::Duration};
pub mod client;
use futures_util::StreamExt;
use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    Mutex,
};
use tokio::task::JoinHandle;

use crate::client::WsClient;
#[derive(Debug)]
pub struct CustomErr {
    pub msg: String,
}

#[tokio::main]
async fn main() -> Result<(), CustomErr> {
    println!("Starting app...");
    let (tx, mut rx): (Sender<bool>, Receiver<bool>) = mpsc::channel(32);
    let tx_mutex = Arc::new(Mutex::new(tx));
    let tx_mutex1 = tx_mutex.clone();

    let client: WsClient = WsClient {
        ws_endpoint: "ws://localhost:3000",
        tx: tx_mutex,
        write: None,
        read: None,
    };

    let ws_client_clone1 = Arc::new(Mutex::new(client));
    let mut thread_handles: Vec<JoinHandle<()>> = vec![];
    
    tx_mutex1.lock().await.send(true).await.unwrap();
    thread_handles.push(tokio::spawn(async move {
        while let Some(_flag) = rx.recv().await {
            if _flag {
                ws_client_clone1.lock().await.init().await;
            }
            let l = ws_client_clone1.as_ref().lock().await;
            let mut r = l.read.as_ref().unwrap().lock().await;
            while let Some(response) = r.next().await {
                match response {
                    Ok(line) => {
                        println!("{}", line);
                    }
                    Err(err) => {
                        println!("Disconnected while receiving messages {}", err);
                        thread::sleep(Duration::from_millis(2000));
                        println!("Reconnecting in event based fn {}", err);
                        tx_mutex1.lock().await.send(true).await.unwrap();
                    }
                }
            }
        }
    }));

    let join_res = futures::future::join_all(thread_handles).await;
    for e in join_res {
        if e.is_err() {
            return Err(CustomErr {
                msg: e.err().unwrap().to_string(),
            });
        }
    }

    Ok(())
}
