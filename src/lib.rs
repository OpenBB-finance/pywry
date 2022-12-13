use wry::{
application::{
  event::{Event, StartCause, WindowEvent},
  event_loop::{ControlFlow, EventLoop},
  window::WindowBuilder,
},
webview::WebViewBuilder,
};
use pyo3::prelude::*;

/// Formats the sum of two numbers as string.
#[pyfunction]
fn sum_as_string(a: usize, b: usize) -> PyResult<String> {
    Ok((a + b).to_string())
}

#[pyfunction]
fn show_html(file_path: String) -> PyResult<String> {
  let event_loop = EventLoop::new();
  let window = WindowBuilder::new()
    .with_title("Juan Step from the MOON")
    .build(&event_loop)
    .unwrap();
  let _webview = WebViewBuilder::new(window).unwrap()
    .with_html(&file_path)
    .unwrap()
    .build()
    .unwrap();

  event_loop.run(move |event, _, control_flow| {
    *control_flow = ControlFlow::Wait;

    match event {
      Event::NewEvents(StartCause::Init) => println!("Wry has started!"),
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
fn plotly_wry(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(sum_as_string, m)?)?;
    m.add_function(wrap_pyfunction!(show_html, m)?)?;
    Ok(())
}
