use pyo3::prelude::*;
use wry::{
    application::{
        event::{Event, StartCause, WindowEvent},
        event_loop::{ControlFlow, EventLoop},
        window::WindowBuilder,
    },
    webview::WebViewBuilder,
};

/// A function to show the provided html in a WRY browser
#[pyfunction]
fn show_html(title: String, html_content: String, hide_output: bool) -> PyResult<String> {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title(&title)
        .build(&event_loop)
        .unwrap();
    let _webview = WebViewBuilder::new(window)
        .unwrap()
        .with_html(&html_content)
        .unwrap()
        .build()
        .unwrap();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(StartCause::Init) => {
                if !hide_output {
                    println!("Wry has started!");
                }
            },
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            _ => (),
        }
    });
}

/// A Python module implemented in Rust.
#[pymodule]
fn pywry(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(show_html, m)?)?;
    Ok(())
}
