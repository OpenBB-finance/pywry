use crate::{
    constants::{BLOBINIT_SCRIPT, DEV_TOOLS_HTML},
    pipe::run_listener,
    structs::{DebugPrinter, Showable, UserEvent},
};
use image::ImageFormat;
use mime_guess;
use std::{
    collections::HashMap,
    fs::{canonicalize, read},
    path::PathBuf,
    sync::mpsc::{Receiver, Sender},
};

#[cfg(not(target_os = "macos"))]
use std::fs::{copy, create_dir_all, remove_file};
use urlencoding::decode as urldecode;
use wry::{
    application::{
        dpi::LogicalSize,
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoop, EventLoopProxy, EventLoopWindowTarget},
        window::{Icon, Theme, WindowBuilder, WindowId},
    },
    http::{header::CONTENT_TYPE, Response},
    webview::{WebView, WebViewBuilder},
};

/// Gets the icon from the path
/// # Arguments
/// * `icon` - The path to the icon
/// # Returns
/// * `Option<Icon>` - The icon or None
fn get_icon(icon: &str) -> Option<Icon> {
    let icon_object = match read(icon) {
        Err(_) => None,
        Ok(bytes) => {
            let imagebuffer = match image::load_from_memory_with_format(&bytes, ImageFormat::Png) {
                Err(_) => None,
                Ok(loaded) => {
                    let imagebuffer = loaded.to_rgba8();
                    let (icon_width, icon_height) = imagebuffer.dimensions();
                    let icon_rgba = imagebuffer.into_raw();
                    match Icon::from_rgba(icon_rgba, icon_width, icon_height) {
                        Err(_) => None,
                        Ok(icon) => Some(icon),
                    }
                }
            };
            imagebuffer
        }
    };
    icon_object
}

/// Creates a new window and returns the window id and webview
/// # Arguments
/// * `to_show` - The Showable struct that contains the information to show
/// * `event_loop` - The event loop to create the window on
/// * `proxy` - The event loop proxy to send events to
/// * `debug` - The DebugPrinter struct to print debug messages
/// # Returns
/// * `Result<(WindowId, WebView), String>` - The window id and webview or an error message
fn create_new_window(
    mut to_show: Showable,
    event_loop: &&EventLoopWindowTarget<UserEvent>,
    proxy: &EventLoopProxy<UserEvent>,
    debug: DebugPrinter,
) -> Result<(WindowId, WebView), String> {
    if to_show.html_path.is_empty() && to_show.html_str.is_empty() {
        to_show.html_str = String::from(
            "<h1 style='color:red'>No html content to show, please provide a html_path or a html_str key</h1>",
        );
    }
    let window_icon = to_show.icon.clone();

    let content = match to_show.html_path.is_empty() {
        true => to_show.html_str.as_bytes().to_vec(),
        false => to_show.html_path.as_bytes().to_vec(),
    };

    let content = match debug.active {
        true => {
            let mut dev_tools_html = DEV_TOOLS_HTML.as_bytes().to_vec();
            dev_tools_html.extend(content);
            dev_tools_html
        }
        false => content,
    };

    let mut pre_window = WindowBuilder::new()
        .with_title(to_show.title)
        .with_window_icon(get_icon(&window_icon))
        .with_min_inner_size(LogicalSize::new(800, 450))
        .with_theme(Some(Theme::Dark));

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

    let minimized = !to_show.export_image.is_empty();
    if minimized {
        window.set_visible(false);
        window.set_maximized(false);
    } else {
        window.set_always_on_top(true);
    }

    let window_id = window.id();
    let background_color = match to_show.theme {
        Theme::Light => (255, 255, 255, 255),
        Theme::Dark => (0, 0, 0, 255),
        _ => (255, 255, 255, 255),
    };

    let webview = match WebViewBuilder::new(window) {
        Err(error2) => return Err(error2.to_string()),
        Ok(item) => {
            let protocol = item
                .with_background_color(background_color)
                .with_hotkeys_zoom(true)
                .with_custom_protocol("wry".into(), move |request| {
                    let path = request.uri().path();
                    let clean_path = &path[1..];
                    let content = content.clone();
                    let mut mime = mime_guess::from_path("index.html");

                    let content = if path == "/" {
                        content.into()
                    } else {
                        let file_path = match clean_path.starts_with("file://") {
                            true => {
                                let decoded = urldecode(&clean_path).expect("UTF-8").to_string();
                                let path = PathBuf::from(&decoded);
                                if ":" == &decoded[9..10] {
                                    path.strip_prefix("file://").unwrap().to_path_buf()
                                } else {
                                    let path = PathBuf::from(&decoded[6..]);
                                    path.to_path_buf()
                                }
                            }
                            false => PathBuf::from(clean_path),
                        };

                        let file_path = file_path.to_str().unwrap();

                        mime = mime_guess::from_path(file_path);
                        match read(canonicalize(file_path).unwrap_or_default()) {
                            Err(_) => content.into(),
                            Ok(bytes) => bytes.into(),
                        }
                    };

                    let mimetype = mime
                        .first()
                        .map(|mime| mime.to_string())
                        .unwrap_or_else(|| "text/plain".to_string());

                    Response::builder()
                        .header(CONTENT_TYPE, mimetype)
                        .header("Access-Control-Allow-Origin", "null")
                        .body(content)
                        .map_err(Into::into)
                });
            let export_image = to_show.export_image.clone();
            let _is_export = !export_image.is_empty();
            let download_path = to_show.download_path.clone();

            let init_view = match !to_show.data.is_none() {
                true => {
                    let variable_value =
                        serde_json::to_string(&to_show.data.unwrap()).unwrap_or_default();
                    let initialization_script = match !export_image.is_empty() {
                        true => format!(
                            "window.json_data = {}; window.save_image = true; window.export_image = {:?};",
                            variable_value, export_image
                        ),
                        false => format!(
                            "window.json_data = {}; window.download_path = {:?};",
                            variable_value, download_path
                        ),
                    };

                    protocol.with_initialization_script(&initialization_script)
                }
                false => protocol,
            };

            // we add a download handler, if export_image is set it takes precedence over download_path
            let init_view = init_view
                .with_download_started_handler({
                    let _proxy = proxy.clone();
                    move |_uri: String, default_path| {
                        #[cfg(not(target_os = "macos"))]
                        {
                            if _uri.starts_with("blob:") {
                                let submitted = _proxy
                                    .send_event(UserEvent::BlobReceived(dbg!(_uri), window_id))
                                    .is_ok();
                                return submitted;
                            }
                            let submitted = _proxy
                                .send_event(UserEvent::DownloadStarted(
                                    _uri.clone(),
                                    default_path.display().to_string(),
                                ))
                                .is_ok();

                            return submitted;
                        }

                        #[cfg(target_os = "macos")]
                        {
                            if _is_export {
                                let mut path = PathBuf::from(&export_image);
                                if path.is_dir() {
                                    path.push(default_path.file_name().unwrap());
                                }
                                *default_path = path.clone();
                            } else if !download_path.is_empty() {
                                let mut path = PathBuf::from(&download_path);
                                if path.is_dir() {
                                    path.push(default_path.file_name().unwrap());
                                }
                                *default_path = path.clone();
                            }
                            true
                        }
                    }
                })
                .with_ipc_handler({
                    let proxy = proxy.clone();
                    move |_, string| match string.as_str() {
                        _ if string.starts_with("data:") => {
                            let _ = proxy.send_event(UserEvent::BlobChunk(Some(string)));
                        }
                        "#EOF" => {
                            let _ = proxy.send_event(UserEvent::BlobChunk(None));
                        }
                        _ if string.starts_with("#OPEN:") => {
                            let _ = proxy
                                .send_event(UserEvent::OpenFile(Some(PathBuf::from(&string[6..]))));
                        }
                        "#DEVTOOLS" => {
                            let _ = proxy.send_event(UserEvent::DevTools(window_id));
                        }
                        _ => {}
                    }
                })
                .with_initialization_script(BLOBINIT_SCRIPT);

            let init_view = init_view
                .with_download_completed_handler({
                    let proxy = proxy.clone();
                    move |_uri, filepath, success| {
                        let _filepath = filepath.unwrap_or_default();

                        #[cfg(not(target_os = "macos"))]
                        let _ = proxy.send_event(UserEvent::DownloadComplete(
                            Some(_filepath),
                            success,
                            download_path.clone(),
                            export_image.clone(),
                            window_id,
                        ));

                        #[cfg(target_os = "macos")]
                        {
                            if success && _is_export {
                                let _ = proxy.send_event(UserEvent::CloseWindow(window_id));
                            }
                        }
                    }
                })
                .with_new_window_req_handler({
                    #[cfg(not(target_os = "windows"))]
                    {
                        let proxy = proxy.clone();
                        move |uri: String| {
                            let submitted = proxy
                                .send_event(UserEvent::NewWindow(
                                    uri.clone(),
                                    get_icon(&window_icon),
                                ))
                                .is_ok();
                            submitted
                        }
                    }
                    #[cfg(target_os = "windows")]
                    {
                        move |_uri: String| true
                    }
                });

            match init_view
                .with_devtools(debug.active)
                .with_url("wry://localhost")
            {
                Err(error3) => return Err(error3.to_string()),
                Ok(subitem) => match subitem.build() {
                    Err(error4) => return Err(error4.to_string()),
                    Ok(sub2item) => {
                        if !minimized {
                            let proxy = proxy.clone();
                            let _ = proxy.send_event(UserEvent::NewWindowCreated(window_id));
                        }
                        sub2item
                    }
                },
            }
        }
    };

    Ok((window_id, webview))
}

/// Starts Main Runtime Loop and creates a new window when a message is received from Python
/// # Arguments
/// * `sender` - The sender to send messages from Python to Wry Event Loop
/// * `receiver` - The receiver Wry uses to receive messages from Python
/// * `debug` - The DebugPrinter struct to print debug messages
///
/// # Returns
/// * `Result<(), String>` - An error message or nothing
pub fn start_wry(
    sender: Sender<String>,
    receiver: Receiver<String>,
    debug: DebugPrinter,
) -> Result<(), String> {
    let event_loop: EventLoop<UserEvent> = EventLoop::with_user_event();
    let proxy = event_loop.create_proxy();
    let mut webviews = HashMap::new();

    std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async move { run_listener(sender.clone()).await.unwrap() })
    });

    event_loop.run(move |event, event_loop, control_flow| {
        *control_flow = ControlFlow::Poll;

        let response = receiver.try_recv().unwrap_or_default();

        if !response.is_empty() {
            debug.print("Received response");

            let chart = Showable::new(&response).unwrap_or_default();
            match create_new_window(chart, &event_loop, &proxy, debug) {
                Err(error) => println!("Window Creation Error: {}", error),

                Ok(new_window) => {
                    webviews.insert(new_window.0, new_window.1);
                }
            };
        }

        match event {
            // UserEvent::NewWindowCreated
            Event::UserEvent(UserEvent::NewWindowCreated(window_id)) => {
                debug.print("New Window Created");
                if let Some(webview) = webviews.get_mut(&window_id) {
                    webview.window().set_always_on_top(false);
                }
            }
            // UserEvent::DownloadStarted
            #[cfg(not(target_os = "macos"))]
            Event::UserEvent(UserEvent::DownloadStarted(uri, path)) => {
                if uri.len() < 200 {
                    debug.print(&format!("\nDownload Started: {}", uri));
                }
                debug.print(&format!("\nPath: {}", path));
            }
            // UserEvent::DownloadComplete
            #[cfg(not(target_os = "macos"))]
            Event::UserEvent(UserEvent::DownloadComplete(
                filepath,
                success,
                download_path,
                export_image,
                window_id,
            )) => {
                let is_export = !export_image.is_empty();
                debug.print(&format!("\nDownload Complete: {}", success));

                if let Some(filepath) = filepath {
                    let decoded = urldecode(&filepath.to_str().unwrap())
                        .expect("UTF-8")
                        .to_string();

                    let file_path = match decoded.starts_with("file://") {
                        true => {
                            if ":" == &decoded[9..10] {
                                let path = PathBuf::from(&decoded);
                                path.strip_prefix("file://").unwrap().to_path_buf()
                            } else {
                                let path = PathBuf::from(&decoded[6..]);
                                path.to_path_buf()
                            }
                        }
                        false => PathBuf::from(decoded),
                    };

                    let new_path = match !download_path.is_empty() {
                        true => match !export_image.is_empty() {
                            true => {
                                let path = PathBuf::from(&export_image);
                                path.to_path_buf()
                            }
                            false => {
                                let mut path = PathBuf::from(&download_path);
                                path.push(file_path.file_name().unwrap());
                                path.to_path_buf()
                            }
                        },
                        false => file_path.to_path_buf(),
                    };

                    debug.print(&format!(
                        "\nOriginal Path: {:?}",
                        file_path.to_str().unwrap()
                    ));
                    debug.print(&format!("New Path: {}", new_path.to_str().unwrap()));

                    let dir = new_path.parent().unwrap();
                    match !dir.exists() {
                        true => {
                            debug.print(&format!("\nCreating directory: {}", dir.display()));
                            if let Err(error) = create_dir_all(dir) {
                                println!("Error creating directory: {}", error);
                            }
                        }
                        false => {}
                    }

                    match copy(&file_path, &new_path) {
                        Err(error) => println!("\nError copying file: {}", error),
                        Ok(_) => {
                            if is_export {
                                let _ = proxy.send_event(UserEvent::CloseWindow(window_id));
                            }
                            if let Err(error) = remove_file(&file_path) {
                                println!("Error deleting file: {}", error);
                            }
                        }
                    }
                }
            }
            // UserEvent::CloseWindow
            Event::UserEvent(UserEvent::CloseWindow(window_id)) => {
                debug.print("Closing Window");
                if let Some(_) = webviews.get(&window_id) {
                    debug.print("Closing Webview");
                    webviews.remove(&window_id);
                }
            }
            // UserEvent::BlobChunk
            Event::UserEvent(UserEvent::BlobChunk(_)) => {
                debug.print("Blob Chunk");
            }
            // WindowEvent::CloseRequested
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
                ..
            } => {
                debug.print("Close Requested");
                if let Some(_) = webviews.get(&window_id) {
                    debug.print("Closing Webview");
                    webviews.remove(&window_id);
                }
            }
            // UserEvent::DevTools
            Event::UserEvent(UserEvent::DevTools(window_id)) => {
                debug.print("DevTools");
                if let Some(webview) = webviews.get(&window_id) {
                    debug.print("Opening DevTools");
                    let _ = webview.open_devtools();
                }
            }
            // UserEvent::OpenFile
            Event::UserEvent(UserEvent::OpenFile(filepath)) => {
                if filepath.is_some() {
                    let decoded = urldecode(&filepath.unwrap().to_str().unwrap())
                        .expect("UTF-8")
                        .to_string();
                    let path = PathBuf::from(decoded);
                    if let Err(error) = open::that(&path.to_str().unwrap()) {
                        println!("Error opening file: {}", error);
                    }
                }
            }
            // WindowEvent::NewWindow
            #[cfg(not(target_os = "windows"))]
            Event::UserEvent(UserEvent::NewWindow(uri, window_icon)) => {
                debug.print(&format!("\nNew Window Requested: {}", uri));
                match uri.starts_with("http://") || uri.starts_with("https://") {
                    true => {
                        let pre_window = WindowBuilder::new()
                            .with_title(uri.to_string())
                            .with_window_icon(window_icon)
                            .with_inner_size(LogicalSize::new(1300, 900))
                            .with_resizable(true)
                            .with_theme(Some(Theme::Dark));

                        let window = match pre_window.build(event_loop) {
                            Err(error) => {
                                println!("Window Creation Error: {}", error);
                                return;
                            }
                            Ok(item) => item,
                        };

                        let window_id = window.id();

                        let webview = WebViewBuilder::new(window)
                            .unwrap()
                            .with_url(&uri)
                            .unwrap()
                            .build()
                            .unwrap();

                        webviews.insert(window_id, webview);

                        debug.print("New Window Created");
                    }
                    false => {
                        debug.print(&format!("Invalid URI tried to open in new window: {}", uri));
                    }
                }
            }
            _ => {}
        }
    });
}
