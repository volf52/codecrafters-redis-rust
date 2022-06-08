mod redis_protocol;
// mod types;

use crate::redis_protocol::process_command;
#[allow(unused_imports)]
use std::env;
#[allow(unused_imports)]
use std::fs;
use std::net::SocketAddr;
use std::str;
use tokio::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() -> io::Result<()> {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    let mut listener = TcpListener::bind("127.0.0.1:6379").await?;

    loop {
        match listener.accept().await {
            Ok((socket, addr)) => {
                tokio::spawn(async move {
                    handle_socket(socket, addr).await;
                });
            }
            Err(err) => println!("error accepting client: {:?}", err),
        }
    }
}

async fn handle_socket(mut socket: TcpStream, addr: SocketAddr) {
    println!("accepted client: {:?}", addr);
    let mut buffer = [0; 1024];
    let t = "+PONG\r\n".as_bytes();

    loop {
        println!("Reading data");
        match socket.read(&mut buffer[..]).await {
            Err(_) | Ok(0) => break,
            Ok(n) => {
                let s = str::from_utf8(&buffer[..n]).expect("couldn't convert as utf8");

                let ans = process_command(s).as_bytes();

                let _ = socket.write(ans).await;
            }
        }
    }

    println!("socket closed {:?}", socket);
}
