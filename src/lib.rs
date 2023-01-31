#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::missing_errors_doc, clippy::must_use_candidate)]

use ports::get_available_port;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use std::sync::mpsc;
use window::start_wry;

pub mod ports;
pub mod structs;
pub mod websocket;
pub mod window;

#[pyclass]
struct WindowManager {
    port: u16,
}

#[pymethods]
impl WindowManager {
    #[new]
    fn new() -> Self {
        let target_port = get_available_port().map_or(0, |port| port);
        Self { port: target_port }
    }

    fn start(&self, debug: bool, port: Option<u16>) -> PyResult<()> {
        let port = port.unwrap_or(self.port);

        let (sender, receiver) = mpsc::channel();
        match start_wry(port, sender, receiver, debug) {
            Err(error) => {
                let error_str = format!("Error starting wry server: {}", error);
                Err(PyValueError::new_err(error_str))
            }
            Ok(_) => Ok(()),
        }
    }

    const fn get_port(&self) -> u16 {
        self.port
    }
}

/// A Python module implemented in Rust.
#[pymodule]
fn pywry(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<WindowManager>()?;
    Ok(())
}
