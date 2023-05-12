#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::missing_errors_doc, clippy::must_use_candidate)]

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use std::sync::mpsc;

pub mod constants;
pub mod events;
pub mod handlers;
pub mod headless;
pub mod pipe;
pub mod structs;
pub mod utils;
pub mod window;

#[pyclass]
struct WindowManager {
	#[pyo3(get, set)]
	debug: bool,
}

#[pymethods]
impl WindowManager {
	#[new]
	fn new() -> Self {
		Self { debug: false }
	}

	fn start(&self, debug: bool) -> PyResult<()> {
		let (sender, receiver) = mpsc::channel();
		let console_printer = structs::ConsolePrinter::new(debug);
		match window::start_wry(sender, receiver, console_printer) {
			Err(error) => {
				let error_str = format!("Error starting wry server: {}", error);
				Err(PyValueError::new_err(error_str))
			}
			Ok(_) => Ok(()),
		}
	}

	fn start_headless(&self, debug: bool) -> PyResult<()> {
		let (sender, receiver) = mpsc::channel();
		let console_printer = structs::ConsolePrinter::new(debug);
		match headless::start_headless(sender, receiver, console_printer) {
			Err(error) => {
				let error_str = format!("Error starting headless server: {}", error);
				Err(PyValueError::new_err(error_str))
			}
			Ok(_) => Ok(()),
		}
	}
}

/// # PyWry Web Viewer
/// Easily create HTML webviewers in python utilizing the [wry](https://github.com/tauri-apps/wry) library.
#[pymodule]
fn pywry(_py: Python, m: &PyModule) -> PyResult<()> {
	m.add("__version__", env!("CARGO_PKG_VERSION"))?;
	m.add_class::<WindowManager>()?;
	Ok(())
}
