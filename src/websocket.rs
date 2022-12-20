use futures_util::{SinkExt, StreamExt};
use std::{io::Error as IoError, net::SocketAddr, sync::mpsc::Sender};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{
    accept_async,
    tungstenite::{Error, Result},
};

async fn accept_connection(peer: SocketAddr, stream: TcpStream, sender: Sender<String>) {
    if let Err(e) = handle_connection(peer, stream, sender).await {
        match e {
            Error::ConnectionClosed | Error::Protocol(_) | Error::Utf8 => (),
            err => println!("Error processing connection: {}", err),
        }
    }
}

async fn handle_connection(
    peer: SocketAddr,
    stream: TcpStream,
    sender: Sender<String>,
) -> Result<()> {
    let mut ws_stream = accept_async(stream).await.expect("Failed to accept");

    println!("New WebSocket connection: {}", peer);

    while let Some(msg) = ws_stream.next().await {
        let msg = msg?;
        if msg.is_text() || msg.is_binary() {
            println!("Message received");
            let msg_str = msg.to_text().unwrap();
            if !&msg_str.eq("<test>") {
                sender.send(msg_str.to_string()).unwrap();
                ws_stream.send(msg).await?;
            }
        }
    }
    Ok(())
}

pub async fn run_server(port: u16, sender: Sender<String>) -> Result<(), IoError> {
    let addr = format!("127.0.0.1:{}", port).to_string();

    // Create the event loop and TCP listener we'll accept connections on.
    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket?;
    println!("Listening on: {}", addr);

    // Let's spawn the handling of each connection in a separate task.
    while let Ok((stream, _)) = listener.accept().await {
        let peer = stream
            .peer_addr()
            .expect("connected streams should have a peer address");
        println!("Peer address: {}", peer);

        tokio::spawn(accept_connection(peer, stream, sender.clone()));
    }
    Ok(())
}
