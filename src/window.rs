use crate::{structs::Showable, websocket::run_server};
use std::{
    collections::HashMap,
    sync::mpsc::{Receiver, Sender},
};
use tokio::{runtime::Runtime, task};
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
    to_show: Showable,
    event_loop: &EventLoopWindowTarget<()>,
) -> Result<(WindowId, WebView), String> {
    let mut pre_window = WindowBuilder::new()
        .with_title(to_show.title)
        .with_window_icon(to_show.icon);

    if to_show.height.is_some() && to_show.width.is_some() {
        pre_window = pre_window.with_inner_size(LogicalSize::new(
            to_show.width.unwrap_or(800) + 80,
            to_show.height.unwrap_or(600) + 80,
        ));
    }

    let window = match pre_window.build(event_loop) {
        Err(error) => return Err(error.to_string()),
        Ok(item) => item,
    };
    let window_id = window.id();
    let webview = match WebViewBuilder::new(window) {
        Err(error2) => return Err(error2.to_string()),
        Ok(item) => match item.with_html(to_show.html) {
            Err(error3) => return Err(error3.to_string()),
            Ok(subitem) => match subitem.build() {
                Err(error4) => return Err(error4.to_string()),
                Ok(sub2item) => sub2item,
            },
        },
    };
    Ok((window_id, webview))
}

pub fn start_wry(
    port: u16,
    sender: Sender<String>,
    receiver: Receiver<String>,
) -> Result<(), String> {
    let event_loop = EventLoop::new();
    let mut webviews = HashMap::new();
    let rt = match Runtime::new() {
        Err(_) => return Err("Could not start a runtime".to_string()),
        Ok(item) => item,
    };

    rt.block_on(async { task::spawn(run_server(port, sender)) });

    event_loop.run(move |event, event_loop, control_flow| {
        *control_flow = ControlFlow::Poll;

        let response = receiver.try_recv().unwrap_or_default();

        if !response.is_empty() {
            println!("Received response");
            let chart = Showable::new(&response).unwrap_or_default();
            match create_new_window(chart, &event_loop) {
                Err(error) => println!("Window Creation Error: {}", error),
                Ok(new_window) => {
                    webviews.insert(new_window.0, new_window.1);
                }
            };
        }

        if let Event::WindowEvent {
            event, window_id, ..
        } = event
        {
            if event == WindowEvent::CloseRequested {
                webviews.remove(&window_id);
            }
        }
    });
}
