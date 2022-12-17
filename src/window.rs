use crate::websocket::run_server;
use async_std::task;
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender};
use wry::{
    application::{
        dpi::LogicalSize,
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget},
        window::{WindowBuilder, WindowId},
    },
    webview::{WebView, WebViewBuilder},
};

fn create_new_window(
    html: String,
    title: String,
    figure: serde_json::Value,
    event_loop: &EventLoopWindowTarget<()>,
) -> (WindowId, WebView) {

    if !figure.is_null() {

        let title: String = "OpenBB - ".to_string()
            + &figure["layout"]["title"]["text"]
                .as_str()
                .unwrap_or("Plots");

        let width: u32 = figure["layout"]["width"].as_u64().unwrap_or(800) as u32;
        let height: u32 = figure["layout"]["height"].as_u64().unwrap_or(600) as u32;

        let html = std::fs::read_to_string(html).unwrap().replace("\"{{figure_json}}\"", &figure.to_string());

        let window = WindowBuilder::new()
            .with_inner_size(LogicalSize::new(width + 80, height + 80))
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

    } else {

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
}

pub fn start_wry(port: u16, sender: Sender<String>, receiver: Receiver<String>) -> Result<(), ()> {
    let event_loop = EventLoop::new();
    let mut webviews = HashMap::new();

    task::spawn(run_server(port, sender));

    event_loop.run(move |event, event_loop, control_flow| {
        *control_flow = ControlFlow::Poll;

        let response = receiver.try_recv().unwrap_or_default();

        if !response.is_empty() {
            println!("Received response");
            let json: serde_json::Value = serde_json::from_str(&response).unwrap_or_default();

            let html: String = json["html"].as_str().unwrap_or("").to_string();
            let title: String = json["title"].as_str().unwrap_or("").to_string();
            let figure: serde_json::Value = json["plotly"].clone();

            let new_window = create_new_window(html, title, figure, &event_loop);
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
