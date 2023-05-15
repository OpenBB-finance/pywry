use std::sync::mpsc::Sender;
use tokio::io::{self, AsyncBufReadExt};

pub async fn send_message(sender: Sender<String>, message: String) {
	match serde_json::from_str::<serde_json::Value>(&message.trim()) {
		Ok(_) => {
			sender.send(message).unwrap();
		}
		Err(_) => {
			sender.send(serde_json::json!({ "error": "Invalid JSON" }).to_string()).unwrap();
		}
	}
}

pub async fn run_listener(
	sender: Sender<String>,
) -> Result<(), Box<dyn std::error::Error>> {
	let stdin = io::stdin();
	let mut reader = io::BufReader::new(stdin);
	let rt = tokio::runtime::Runtime::new()?;

	loop {
		let mut line = String::new();

		// Read from stdin asynchronously
		match reader.read_line(&mut line).await.expect("Failed to read from stdin") {
			// No bytes read, so EOF has been reached
			0 => return Ok(()),
			_ => {
				// We spawn a new task to handle the message so that we don't block the
				// main thread.
				rt.spawn(send_message(sender.clone(), line));
			}
		}
	}
}
