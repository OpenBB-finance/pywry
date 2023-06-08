use tokio::io::{self, AsyncBufReadExt};
use wry::application::event_loop::EventLoopProxy;

use crate::structs::UserEvent;

pub async fn send_message(message: String, proxy: &EventLoopProxy<UserEvent>) {
	match serde_json::from_str::<serde_json::Value>(&message.trim()) {
		Ok(_) => {
			proxy.send_event(UserEvent::NewMessageReceived(message)).unwrap_or_default();
		}
		Err(_) => {
			proxy
				.send_event(UserEvent::NewMessageReceived(
					serde_json::json!({ "error": "Invalid JSON" }).to_string(),
				))
				.unwrap_or_default();
		}
	}
}

pub async fn run_listener(
	proxy: &EventLoopProxy<UserEvent>,
) -> Result<(), Box<dyn std::error::Error>> {
	let stdin = io::stdin();
	let mut reader = io::BufReader::new(stdin);

	loop {
		let mut line = String::new();

		// Read from stdin asynchronously
		match reader.read_line(&mut line).await.expect("Failed to read from stdin") {
			// No bytes read, so EOF has been reached
			0 => return Ok(()),
			_ => send_message(line, proxy).await,
		}
	}
}
