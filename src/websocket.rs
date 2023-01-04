use futures_util::{SinkExt, StreamExt};
use std::{io::Error as IoError, net::SocketAddr, sync::mpsc::Sender};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{
    accept_async,
    tungstenite::{Error, Message, Result},
};

async fn accept_connection(peer: SocketAddr, stream: TcpStream, sender: Sender<String>, debug: bool) {
    if let Err(e) = handle_connection(peer, stream, sender, debug).await {
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
    debug: bool,
) -> Result<()> {
    let mut ws_stream = accept_async(stream).await?;

    if debug {
        println!("New WebSocket connection: {}", peer);
    }

    while let Some(msg) = ws_stream.next().await {
        let msg = msg?;
        if msg.is_text() || msg.is_binary() {
            if debug {
                println!("Message received");
            }
            let msg_str = msg.to_text().unwrap_or("<test>");
            if !&msg_str.eq("<test>") {
                let response = match sender.send(msg_str.to_string()) {
                    Err(_) => "ERROR: Could not send the html",
                    Ok(_) => "SUCCESS",
                };
                let new_message = Message::Text(response.to_string());
                ws_stream.send(new_message).await?;
            }
        }
    }
    Ok(())
}

pub async fn run_server(port: u16, sender: Sender<String>, debug: bool) -> Result<(), IoError> {
    let addr = format!("localhost:{}", port).to_string();

    // Create the event loop and TCP listener we'll accept connections on.
    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket?;
    if debug {
        println!("Listening on: {}", addr);
    }

    // Let's spawn the handling of each connection in a separate task.
    while let Ok((stream, _)) = listener.accept().await {
        let peer = stream
            .peer_addr()
            .expect("connected streams should have a peer address");

        if debug {
            println!("Peer address: {}", peer);
        }

        tokio::spawn(accept_connection(peer, stream, sender.clone(), debug));
    }
    Ok(())
}
