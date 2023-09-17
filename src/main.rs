#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::missing_errors_doc, clippy::must_use_candidate)]
use std::env;

pub mod constants;
pub mod events;
pub mod handlers;
pub mod headless;
pub mod pipe;
pub mod structs;
pub mod utils;
pub mod window;

pub struct WindowManager {
	pub debug: bool,
}

impl WindowManager {
	pub fn new() -> Self {
		Self { debug: false }
	}

	pub fn start(&self, debug: bool) -> Result<bool, String> {
		let console_printer = structs::ConsolePrinter::new(debug);
		match window::start_wry(console_printer) {
			Err(error) => {
				let error_str = format!("Error starting wry server: {}", error);
				Err(error_str)
			}
			Ok(_) => Ok(true),
		}
	}

	pub fn start_headless(&self, debug: bool) -> Result<bool, String> {
		let console_printer = structs::ConsolePrinter::new(debug);
		match headless::start_headless(console_printer) {
			Err(error) => {
				let error_str = format!("Error starting headless server: {}", error);
				Err(error_str)
			}
			Ok(_) => Ok(true),
		}
	}
}

/// Starts the main runtime loop
pub fn main() -> Result<(), String> {
	let args: Vec<String> = env::args().collect();
	let debug = args.contains(&"--debug".to_string());
	let headless = args.contains(&"--headless".to_string());

	let wm = WindowManager::new();

	match headless {
		true => match wm.start_headless(debug) {
			Err(error) => Err(error),
			Ok(_) => Ok(()),
		},
		false => match wm.start(debug) {
			Err(error) => Err(error),
			Ok(_) => Ok(()),
		},
	}
}
