#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use image::ImageFormat;
use pyo3::prelude::*;
use wry::{
    application::{
        dpi::LogicalSize,
        event::{Event, StartCause, WindowEvent},
        event_loop::{ControlFlow, EventLoop},
        window::{Fullscreen, Icon, WindowBuilder},
    },
    webview::WebViewBuilder,
};

/// A function to show the provided html in a WRY browser
#[pyfunction]
fn show_html(
    html_content: String,
    hide_output: Option<bool>,
    title: Option<String>,
    transparent: Option<bool>,
    fullscreen: Option<bool>,
    width: Option<u32>,
    height: Option<u32>,
) -> PyResult<String> {
    let title = title.unwrap_or("PyWry - Star Us!".to_string());
    let hide_output = hide_output.unwrap_or(false);
    let transparent = transparent.unwrap_or(false);
    let fullscreen = fullscreen.unwrap_or(false);
    let width = width.unwrap_or(800);
    let height = height.unwrap_or(600);

    let bytes: Vec<u8> = include_bytes!("../assets/icon2.png").to_vec();
    let imagebuffer = image::load_from_memory_with_format(&bytes, ImageFormat::Png)
        .unwrap()
        .into_rgba8();
    let (icon_width, icon_height) = imagebuffer.dimensions();
    let icon_rgba = imagebuffer.into_raw();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_decorations(!transparent)
        .with_transparent(transparent)
        .with_fullscreen(if fullscreen {
            Some(Fullscreen::Borderless(None))
        } else {
            None
        })
        .with_inner_size(LogicalSize::new(width, height))
        .with_title(title)
        // and then in the window initialization
        .with_window_icon(Some(
            Icon::from_rgba(icon_rgba, icon_width, icon_height).unwrap(),
        ))
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
            }
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
