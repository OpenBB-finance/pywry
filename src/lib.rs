#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use pyo3::prelude::*;
use window::start_wry;
use websocket::send_message;
use std::sync::mpsc;

pub mod websocket;
pub mod window;


#[pyfunction]
fn send_html(py: Python<'_>, html: String) -> PyResult<&PyAny> {
    pyo3_asyncio::async_std::future_into_py(py, async {
        send_message(html).await;
        Ok(Python::with_gil(|py| py.None()))
    })
}

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
    m.add_function(wrap_pyfunction!(send_html, m)?)?;
    Ok(())
}
