use crate::websocket::run_server;
use async_std::task;
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender};
use wry::{
    application::{
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget},
        window::{WindowBuilder, WindowId},
    },
    webview::{WebView, WebViewBuilder},
};

fn create_new_window(
    title: String,
    html: String,
    event_loop: &EventLoopWindowTarget<()>,
) -> (WindowId, WebView) {
    let window = WindowBuilder::new()
        .with_title(title)
        .build(event_loop)
        .unwrap();
    let window_id = window.id();

    let webview = WebViewBuilder::new(window)
        .unwrap()
        .with_html(html)
        .unwrap()
        .build()
        .unwrap();
    (window_id, webview)
}

pub fn start_wry(port: u16, sender: Sender<String>, receiver: Receiver<String>) -> Result<(), ()> {
    let event_loop = EventLoop::new();
    let mut webviews = HashMap::new();

    task::spawn(run_server(port, sender));

    event_loop.run(move |event, event_loop, control_flow| {
        *control_flow = ControlFlow::Poll;

        let response = receiver.try_recv().unwrap_or_default();

        if !response.is_empty() {
            println!("Received: {}", response);
            let new_window = create_new_window(
                format!("Window {}", webviews.len() + 1),
                response,
                &event_loop,
            );
            webviews.insert(new_window.0, new_window.1);
        }

        if let Event::WindowEvent {
            event, window_id, ..
        } = event
        {
            if event == WindowEvent::CloseRequested {
                webviews.remove(&window_id);
                if webviews.is_empty() {
                    *control_flow = ControlFlow::Exit
                }
            }
        }
    });
}
