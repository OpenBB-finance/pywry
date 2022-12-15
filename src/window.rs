use wry::application::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};
use wry::{
    application::{
        event_loop::{EventLoopProxy, EventLoopWindowTarget},
        window::{Window, WindowBuilder, WindowId},
    },
    webview::{WebView, WebViewBuilder},
};

use crate::websocket::run_server;
use async_std::task;
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender};

enum UserEvents {
    CloseWindow(WindowId),
    NewWindow(),
}

fn create_new_window(
    title: String,
    html: String,
    event_loop: &EventLoopWindowTarget<UserEvents>,
    proxy: EventLoopProxy<UserEvents>,
) -> (WindowId, WebView) {
    let window = WindowBuilder::new()
        .with_title(title)
        .build(event_loop)
        .unwrap();
    let window_id = window.id();
    let handler = move |window: &Window, req: String| match req.as_str() {
        "new-window" => {
            let _ = proxy.send_event(UserEvents::NewWindow());
        }
        "close" => {
            let _ = proxy.send_event(UserEvents::CloseWindow(window.id()));
        }
        _ if req.starts_with("change-title") => {
            let title = req.replace("change-title:", "");
            window.set_title(title.as_str());
        }
        _ => {}
    };

    let webview = WebViewBuilder::new(window)
        .unwrap()
        .with_html(html)
        .unwrap()
        .with_ipc_handler(handler)
        .build()
        .unwrap();
    (window_id, webview)
}

pub fn start_wry(port: u16, sender: Sender<String>, receiver: Receiver<String>) -> Result<(), ()> {
    let event_loop = EventLoop::<UserEvents>::with_user_event();
    let mut webviews = HashMap::new();
    let proxy = event_loop.create_proxy();

    task::spawn(run_server(port, sender));

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
