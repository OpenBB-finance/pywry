#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
// #![allow()]

use ports::get_available_port;
use pyo3::prelude::*;
use std::sync::mpsc;
use window::start_wry;

pub mod ports;
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
        let target_port = match get_available_port() {
            None => 0,
            Some(port) => port,
        };
        Self { port: target_port }
    }

    fn start(&self) -> PyResult<()> {
        let (sender, receiver) = mpsc::channel();
        start_wry(self.port, sender, receiver).unwrap();
        Ok(())
    }

    // TODO: find a better way to do this
    fn get_port(&self) -> u16 {
        self.port
    }
}

/// A Python module implemented in Rust.
#[pymodule]
fn pywry(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<WindowManager>()?;
    Ok(())
}
