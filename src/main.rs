mod resp;

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
use tokio_util::codec::BytesCodec;
// use tokio_util::codec::Decoder;
use tokio_util::codec::Framed;

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

async fn handle_socket(socket: TcpStream, addr: SocketAddr) {
    println!("accepted client: {:?}", addr);
    let mut _buffer = [0; 1024];
    // let pong = "+PONG\r\n".as_bytes();

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
                        RespFrame::Null.to_bytes()
                    }
                    Ok(frame) => {
                        println!("{:?}", frame);

                        frame.process_commands().to_bytes()
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
