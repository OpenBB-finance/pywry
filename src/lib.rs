#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use pyo3::prelude::*;
use window::start_wry;
use std::sync::mpsc;

pub mod websocket;
pub mod window;


#[pyfunction]
fn start() -> PyResult<()> {
    let (sender, receiver) = mpsc::channel();
    start_wry(sender, receiver);
    Ok(())
}

/// A Python module implemented in Rust.
#[pymodule]
fn pywry(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(start, m)?)?;
    Ok(())
}
