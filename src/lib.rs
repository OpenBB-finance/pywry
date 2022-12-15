#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use pyo3::prelude::*;
use wry::{
    application::{
        event::{Event, StartCause, WindowEvent},
        event_loop::{ControlFlow, EventLoop},
    },
};

use async_std::channel::unbounded;
use async_std::task;
use std::collections::HashMap;

use async_std::channel::{Sender, Receiver};
use server::run_server;
use window::{create_new_window, UserEvents};

pub mod server;
pub mod window;

#[pyclass]
struct SendData {
    sender: Sender<String>
}


#[pymethods]
impl SendData {
    #[new]
    fn new() -> Self {
        let (sender, receiver) = unbounded();
        start(sender, receiver);
        Self { sender }
    }
}


// #[pyfunction]
fn start(sender: Sender<String>, receiver: Receiver<String>) -> Result<(), ()> {
    let event_loop = EventLoop::<UserEvents>::with_user_event();
    let mut webviews = HashMap::new();
    let proxy = event_loop.create_proxy();

    task::spawn(run_server(sender));

    task::spawn(
        event_loop.run(move |event, event_loop, control_flow| {
            *control_flow = ControlFlow::Wait;

            let response = receiver.try_recv().unwrap_or_default();

            if !response.is_empty() {
                println!("Received: {}", response);
                let new_window = create_new_window(
                    format!("Window {}", webviews.len() + 1),
                    response,
                    &event_loop,
                    proxy.clone(),
                );
                webviews.insert(new_window.0, new_window.1);
            }

            match event {
                Event::NewEvents(StartCause::Init) => println!("Wry application started!"),
                Event::WindowEvent {
                    event, window_id, ..
                } => match event {
                    WindowEvent::CloseRequested => {
                        webviews.remove(&window_id);
                        if webviews.is_empty() {
                            *control_flow = ControlFlow::Exit
                        }
                    }
                    _ => (),
                },
                Event::UserEvent(UserEvents::CloseWindow(id)) => {
                    webviews.remove(&id);
                    if webviews.is_empty() {
                        *control_flow = ControlFlow::Exit
                    }
                }
                _ => (),
            }
        }),
    );

    Ok(())
}

/// A Python module implemented in Rust.
#[pymodule]
fn pywry(_py: Python, m: &PyModule) -> PyResult<()> {
    // m.add_function(wrap_pyfunction!(start, m)?)?;
    Ok(())
}
