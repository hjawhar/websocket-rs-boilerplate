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
    let (tx1, rx1): (Sender<bool>, Receiver<bool>) = mpsc::channel(32);
    let tx1_mutex = Arc::new(Mutex::new(tx1));
    let tx1_clone1: Arc<Mutex<Sender<bool>>> = tx1_mutex.clone();
    let tx1_clone2: Arc<Mutex<Sender<bool>>> = tx1_mutex.clone();

    let rx1_mutex: Arc<Mutex<Receiver<bool>>> = Arc::new(Mutex::new(rx1));
    let rx1_clone1: Arc<Mutex<Receiver<bool>>> = rx1_mutex.clone();

    let (tx2, rx2): (Sender<bool>, Receiver<bool>) = mpsc::channel(32);
    let tx2_mutex = Arc::new(Mutex::new(tx2));
    let tx2_clone1: Arc<Mutex<Sender<bool>>> = tx2_mutex.clone();
    let rx2_mutex: Arc<Mutex<Receiver<bool>>> = Arc::new(Mutex::new(rx2));
    let rx2_clone1: Arc<Mutex<Receiver<bool>>> = rx2_mutex.clone();

    let mut client: WsClient = WsClient {
        ws_endpoint: "ws://localhost:3000",
        tx: tx1_clone1,
        write: None,
        read: None,
    };
    client.init().await;

    let ws_client_clone1: Arc<WsClient> = Arc::new(client);
    let ws_client_clone2 = ws_client_clone1.clone();
    let mut ws_client_clone3 = ws_client_clone1.clone();
    let mut thread_handles: Vec<JoinHandle<()>> = vec![];

    thread_handles.push(tokio::spawn(async move {
        while let Some(response) = rx2_clone1.lock().await.recv().await {}
    }));

    thread_handles.push(tokio::spawn(async move {
        while let Some(response) = rx1_clone1.lock().await.recv().await {
            let mut r = ws_client_clone2.read.as_ref().unwrap().lock().await;
            if response {
                println!("Refreshing websockets: {}", response);
            }
            while let Some(response) = r.next().await {
                match response {
                    Ok(line) => {
                        println!("{}", line);
                    }
                    Err(err) => {
                        println!("Disconnected while receiving messages {}", err);
                        thread::sleep(Duration::from_millis(2000));
                        println!("Reconnecting in event based fn {}", err);
                        tx2_clone1.lock().await.send(true).await.unwrap();
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
