mod resp;
mod store;

use resp::frame::RespFrame;

use std::convert::TryFrom;
#[allow(unused_imports)]
use std::env;
#[allow(unused_imports)]
use std::fs;
use std::net::SocketAddr;
use tokio::io;
use tokio::io::AsyncWriteExt;
// use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::stream::StreamExt;
use tokio::sync::{broadcast, mpsc};
use tokio_util::codec::BytesCodec;
use tokio_util::codec::Framed;

#[tokio::main]
async fn main() -> io::Result<()> {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    let (tx, rx) = mpsc::channel(32);
    let (resp_tx, _) = broadcast::channel(32);

    tokio::select! {
        _ = server_loop(tx, resp_tx.clone()) => {}
        _ = store_handler(rx, resp_tx) => {}
    };

    Ok(())
}

async fn server_loop(
    tx: mpsc::Sender<store::StoreCommand>,
    resp_tx: broadcast::Sender<store::StoreResponse>,
) -> io::Result<()> {
    let mut listener = TcpListener::bind("127.0.0.1:6379").await?;

    loop {
        match listener.accept().await {
            Ok((socket, addr)) => {
                let tx2 = tx.clone();
                let rx = resp_tx.subscribe();

                tokio::spawn(async move {
                    handle_socket(socket, addr, tx2, rx).await;
                });
            }
            Err(err) => println!("error accepting client: {:?}", err),
        }
    }
}

async fn store_handler(
    rx: mpsc::Receiver<store::StoreCommand>,
    tx: broadcast::Sender<store::StoreResponse>,
) {
    let mut store = store::KVStore::new();

    store.run_loop(rx, tx).await;
}

async fn handle_socket(
    socket: TcpStream,
    addr: SocketAddr,
    mut kv_sender: mpsc::Sender<store::StoreCommand>,
    mut kv_receiver: broadcast::Receiver<store::StoreResponse>,
) {
    println!("accepted client: {:?}", addr);

    // let mut transport = RespParser::default().framed(socket);
    let mut transport = Framed::new(socket, BytesCodec::new());

    // 'ev_loop: loop {
    while let Some(msg) = transport.next().await {
        match msg {
            Err(e) => eprintln!("Error receiving msg: {:?}", e),
            Ok(b) => {
                let bytes_to_send = match RespFrame::try_from(b) {
                    Err(e) => {
                        eprintln!("{:?}", e);
                        RespFrame::Error(e.to_string()).to_bytes()
                    }
                    Ok(frame) => {
                        println!("{:?}", frame);

                        frame
                            .process_commands(&mut kv_sender, &mut kv_receiver)
                            .await
                            .to_bytes()
                    }
                };

                let x = transport.get_mut();
                if let Err(e) = x.write_all(&bytes_to_send).await {
                    println!("Error sending pong response to {}: {:?}", addr, e);
                };
            }
        }
    }
    // }

    println!("Client closed {:?}", addr);
}
