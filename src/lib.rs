#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use image::ImageFormat;
use pyo3::prelude::*;
use std::fs;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use wry::{
    application::{
        dpi::LogicalSize,
        event::{Event, StartCause, WindowEvent},
        event_loop::{ControlFlow, EventLoop},
        window::{Fullscreen, Icon, WindowBuilder},
    },
    webview::WebViewBuilder,
};

use async_std::channel::unbounded;
use async_std::task;
use std::collections::HashMap;
use test_wry_multi::run_server;
use wry::{
    application::{
        event::{Event, StartCause, WindowEvent},
        event_loop::{ControlFlow, EventLoop, EventLoopProxy, EventLoopWindowTarget},
        window::{Window, WindowBuilder, WindowId},
    },
    webview::{WebView, WebViewBuilder},
};

use std::{env, io::Error as IoError};
async_std::channel::Sender

#[pyclass]
struct SendData(Sender);


#[pymethods]
impl SendData {
    #[new]
    fn new() -> Self {
        sender, receiver = 
        start();
    }
}


// #[pyfunction]
fn start() -> wry::Result<()> {
    enum UserEvents {
        CloseWindow(WindowId),
        NewWindow(),
    }

    let event_loop = EventLoop::<UserEvents>::with_user_event();
    let mut webviews = HashMap::new();
    let proxy = event_loop.create_proxy();

    let (tx, rx) = unbounded();

    task::spawn(run_server(tx));

    task::spawn(
        event_loop.run(move |event, event_loop, control_flow| {
            *control_flow = ControlFlow::Wait;

            let response = rx.try_recv().unwrap_or_default();

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
        });
    );
}

/// A Python module implemented in Rust.
#[pymodule]
fn pywry(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(start, m)?)?;
    Ok(())
}
