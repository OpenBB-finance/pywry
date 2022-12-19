use futures::future;
use futures::prelude::*;
use std::io::Error as IoError;

use std::sync::mpsc::Sender;
use tokio::{
    net::{TcpListener, TcpStream},
    task,
};
use tokio_tungstenite::{accept_async, tungstenite::error::Error};

enum ConnectionError {
    Tungstenite(Error),
    Mspc(String),
}

async fn handle_connection(
    sender: Sender<String>,
    raw_stream: TcpStream,
) -> Result<(), ConnectionError> {
    let ws_stream = match accept_async(raw_stream).await {
        Err(err) => return Err(ConnectionError::Tungstenite(err)),
        Ok(stream) => stream,
    };
    let (_, incoming) = ws_stream.split();

    let mut x = String::new();
    let broadcast_incoming = incoming
        .try_filter(|msg| future::ready(!msg.is_close()))
        .try_for_each(|msg| {
            x = msg.to_text().unwrap_or_default().to_string();
            future::ok(())
        });

    if let Err(error) = broadcast_incoming.await {
        return Err(ConnectionError::Tungstenite(error));
    }
    if !&x.eq("<test>") {
        println!("Sent response");
        if let Err(error) = sender.send(x.clone()) {
            println!("Error sending response");
            return Err(ConnectionError::Mspc(error.0));
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
        // How will we return errors inside task::spawn?
        task::spawn(handle_connection(sender.clone(), stream));
    }

    Ok(())
}
