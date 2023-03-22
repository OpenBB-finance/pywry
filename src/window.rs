use crate::{constants::BLOBINIT_SCRIPT, structs::Showable, websocket::run_server};
use mime_guess;
use std::{
    collections::HashMap,
    fs::{canonicalize, copy, create_dir_all, read, remove_file},
    path::PathBuf,
    sync::mpsc::{Receiver, Sender},
};
use tokio::{runtime::Runtime, task};
use urlencoding::decode as urldecode;
use wry::{
    application::{
        dpi::LogicalSize,
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoop, EventLoopProxy, EventLoopWindowTarget},
        window::{Theme, WindowBuilder, WindowId},
    },
    http::{header::CONTENT_TYPE, Response},
    webview::{WebView, WebViewBuilder},
};

enum UserEvent {
    DownloadStarted(String, String),
    DownloadComplete(Option<PathBuf>, bool, String, String),
    BlobReceived(String, WindowId),
    BlobChunk(Option<String>),
}

fn create_new_window(
    mut to_show: Showable,
    event_loop: &&EventLoopWindowTarget<UserEvent>,
    proxy: &EventLoopProxy<UserEvent>,
    debug: bool,
) -> Result<(WindowId, WebView), String> {
    if to_show.html_path.is_empty() && to_show.html_str.is_empty() {
        to_show.html_str = String::from(
            "<h1 style='color:red'>No html content to show, please provide a html_path or a html_str key</h1>",
        );
    }

    let content = if to_show.html_path.is_empty() {
        to_show.html_str.as_bytes().to_vec()
    } else {
        to_show.html_path.as_bytes().to_vec()
    };
    let mut pre_window = WindowBuilder::new()
        .with_title(to_show.title)
        .with_window_icon(to_show.icon)
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
        window.set_visible(to_show.export_image.is_empty());
    } else {
        window.set_focus();
    }

    let window_id = window.id();

    let webview = match WebViewBuilder::new(window) {
        Err(error2) => return Err(error2.to_string()),
        Ok(item) => {
            let protocol = item
                .with_background_color((0, 0, 0, 255))
                .with_hotkeys_zoom(true)
                .with_custom_protocol("wry".into(), move |request| {
                    let path = request.uri().path();
                    let clean_path = &path[1..];
                    let content = content.clone();
                    let mut mime = mime_guess::from_path("index.html");

                    let content = if path == "/" {
                        content.into()
                    } else {
                        let file_path = if clean_path.starts_with("file://") {
                            let decoded = urldecode(&clean_path).expect("UTF-8").to_string();
                            let path = PathBuf::from(&decoded);
                            if ":" == &decoded[9..10] {
                                path.strip_prefix("file://").unwrap().to_path_buf()
                            } else {
                                let path = PathBuf::from(&decoded[6..]);
                                path.to_path_buf()
                            }
                        } else {
                            PathBuf::from(clean_path)
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
            let download_path = to_show.download_path.clone();

            let init_view = if !to_show.data.is_none() || !to_show.figure.is_none() {
                let variable_name = if !to_show.data.is_none() {
                    "json_data"
                } else {
                    "plotly_figure"
                };

                let variable_value = if !to_show.data.is_none() {
                    serde_json::to_string(&to_show.data.unwrap()).unwrap_or_default()
                } else {
                    serde_json::to_string(&to_show.figure.unwrap()).unwrap_or_default()
                };

                let initialization_script = if !export_image.is_empty() {
                    format!(
                        "window.{} = {}; window.save_image = true; window.export_image = '{}';",
                        variable_name, variable_value, export_image
                    )
                } else {
                    format!("window.{} = {};", variable_name, variable_value)
                };

                protocol.with_initialization_script(&initialization_script)
            } else {
                protocol
            };

            // we add a download handler, if export_image is set it takes precedence over download_path
            let init_view = init_view
                .with_download_started_handler({
                    let proxy = proxy.clone();
                    move |uri: String, default_path| {
                        let proxy = proxy.clone();
                        #[cfg(not(target_os = "macos"))]
                        {
                            if uri.starts_with("blob:") {
                                let submitted = proxy
                                    .send_event(UserEvent::BlobReceived(dbg!(uri), window_id))
                                    .is_ok();
                                return submitted;
                            }
                            let submitted = proxy
                                .send_event(UserEvent::DownloadStarted(
                                    uri.clone(),
                                    default_path.display().to_string(),
                                ))
                                .is_ok();

                            return submitted;
                        }

                        #[cfg(target_os = "macos")]
                        {
                            if !export_image.is_empty() {
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
                            println!("Saving to {:?}", default_path.display());
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
                        _ => {}
                    }
                })
                .with_initialization_script(BLOBINIT_SCRIPT);

            let init_view = init_view.with_download_completed_handler({
                let proxy = proxy.clone();
                move |_uri, filepath, success| {
                    let filepath = filepath.unwrap_or_default();
                    #[cfg(not(target_os = "macos"))]
                    let _ = proxy.send_event(UserEvent::DownloadComplete(
                        Some(filepath),
                        success,
                        download_path.clone(),
                        export_image.clone(),
                    ));
                    #[cfg(target_os = "macos")]
                    if success {
                        println!("Filed saved {:?}", filepath.display());
                    }
                }
            });

            match init_view.with_devtools(debug).with_url("wry://localhost") {
                Err(error3) => return Err(error3.to_string()),
                Ok(subitem) => match subitem.build() {
                    Err(error4) => return Err(error4.to_string()),
                    Ok(sub2item) => sub2item,
                },
            }
        }
    };

    Ok((window_id, webview))
}

pub fn start_wry(
    port: u16,
    sender: Sender<String>,
    receiver: Receiver<String>,
    debug: bool,
) -> Result<(), String> {
    let event_loop: EventLoop<UserEvent> = EventLoop::with_user_event();
    let proxy = event_loop.create_proxy();
    let mut webviews = HashMap::new();
    let rt = match Runtime::new() {
        Err(_) => return Err("Could not start a runtime".to_string()),
        Ok(item) => item,
    };

    rt.block_on(async { task::spawn(run_server(port, sender, debug)) });

    event_loop.run(move |event, event_loop, control_flow| {
        *control_flow = ControlFlow::Poll;

        let response = receiver.try_recv().unwrap_or_default();

        if !response.is_empty() {
            if debug {
                println!("Received response");
            }
            let chart = Showable::new(&response).unwrap_or_default();
            match create_new_window(chart, &event_loop, &proxy, debug) {
                Err(error) => println!("Window Creation Error: {}", error),
                Ok(new_window) => {
                    webviews.insert(new_window.0, new_window.1);
                }
            };
        }

        match event {
            // UserEvent::DownloadStarted
            Event::UserEvent(UserEvent::DownloadStarted(uri, path)) => {
                if debug {
                    println!("Download Started: {}", uri);
                    println!("Path: {}", path);
                }
            }
            // UserEvent::DownloadComplete
            Event::UserEvent(UserEvent::DownloadComplete(
                filepath,
                success,
                download_path,
                export_image,
            )) => {
                if debug {
                    println!("Download Complete: {}", success);
                }
                if let Some(filepath) = filepath {
                    let decoded = urldecode(&filepath.to_str().unwrap())
                        .expect("UTF-8")
                        .to_string();
                    let file_path = if decoded.starts_with("file://") {
                        if ":" == &decoded[9..10] {
                            let path = PathBuf::from(decoded);
                            path.strip_prefix("file://").unwrap().to_path_buf()
                        } else {
                            let path = PathBuf::from(&decoded[6..]);
                            path.to_path_buf()
                        }
                    } else {
                        PathBuf::from(decoded)
                    };
                    let new_path = if !download_path.is_empty() {
                        if !export_image.is_empty() {
                            let path = PathBuf::from(&export_image);
                            path.to_path_buf()
                        } else {
                            let mut path = PathBuf::from(&download_path);
                            path.push(file_path.file_name().unwrap());
                            path.to_path_buf()
                        }
                    } else {
                        file_path.to_path_buf()
                    };

                    if debug {
                        println!("Original Path: {}", file_path.to_str().unwrap());
                        println!("New Path: {}", new_path.to_str().unwrap());
                    }
                    let dir = new_path.parent().unwrap();
                    if !dir.exists() {
                        if debug {
                            println!("Creating directory: {}", dir.display());
                        }
                        if let Err(error) = create_dir_all(dir) {
                            println!("Error creating directory: {}", error);
                        }
                    }
                    if let Err(error) = copy(&file_path, &new_path) {
                        println!("Error copying file: {}", error);
                    } else {
                        println!("File saved to: {}", new_path.display());
                        if let Err(error) = remove_file(&file_path) {
                            println!("Error deleting file: {}", error);
                        }
                    }
                }
            }
            // UserEvent::BlobChunk
            Event::UserEvent(UserEvent::BlobChunk(chunk)) => {
                if debug {
                    println!("Blob Chunk: {:?}", chunk);
                }
            }
            // WindowEvent::CloseRequested
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
                ..
            } => {
                if debug {
                    println!("Close Requested");
                }
                if let Some(_) = webviews.get(&window_id) {
                    if debug {
                        println!("Closing Webview");
                    }
                    webviews.remove(&window_id);
                }
            }
            // WindowEvent::Destroyed
            Event::WindowEvent {
                event: WindowEvent::Destroyed,
                window_id,
                ..
            } => {
                if debug {
                    println!("Window Destroyed");
                }
                webviews.remove(&window_id);
            }
            _ => {}
        }
    });
}
