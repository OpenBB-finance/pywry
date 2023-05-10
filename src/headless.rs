use crate::{
    constants,
    pipe::run_listener,
    structs::{ConsolePrinter, PlotData, ShowableHeadless, UserEvent},
};
use std::{
    collections::HashMap,
    fs::{canonicalize, read},
    io::{self, Write},
    path::PathBuf,
    sync::mpsc::{Receiver, Sender},
};

#[cfg(not(target_os = "macos"))]
use urlencoding::decode as urldecode;
use wry::{
    application::{
        dpi::LogicalSize,
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoop, EventLoopProxy, EventLoopWindowTarget},
        window::{WindowBuilder, WindowId},
    },
    http::{header::CONTENT_TYPE, Response},
    webview::{WebView, WebViewBuilder},
};

/// Creates a new headless window and returns the window id and webview
/// # Arguments
/// * `to_show` - The Showable struct that contains the information to show
/// * `event_loop` - The event loop to create the window on
/// * `proxy` - The event loop proxy to send events to
/// * `console` - The ConsolePrinter struct to print log messages to the console
/// # Returns
/// * `Result<(WindowId, WebView), String>` - The window id and webview or an error message
fn create_new_window_headless(
    to_show: ShowableHeadless,
    event_loop: &&EventLoopWindowTarget<UserEvent>,
    proxy: &EventLoopProxy<UserEvent>,
    console: ConsolePrinter,
) -> Result<(WindowId, WebView), String> {
    let content = constants::HEADLESS_HTML.as_bytes().to_vec();

    let content = match console.active {
        true => {
            let mut dev_tools_html = constants::DEV_TOOLS_HTML.as_bytes().to_vec();
            dev_tools_html.extend(content);
            dev_tools_html
        }
        false => content,
    };

    let pre_window = WindowBuilder::new()
        .with_decorations(false)
        .with_min_inner_size(LogicalSize::new(800, 450))
        .with_visible(console.active)
        .with_inner_size(LogicalSize::new(800, 600));

    let window = match pre_window.build(event_loop) {
        Err(error) => return Err(error.to_string()),
        Ok(item) => item,
    };

    let window_id = window.id();

    let webview = match WebViewBuilder::new(window) {
        Err(error2) => return Err(error2.to_string()),
        Ok(item) => {
            let protocol =
                item.with_hotkeys_zoom(true)
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
                                    let decoded =
                                        urldecode(&clean_path).expect("UTF-8").to_string();
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

            let init_view = match !to_show.data.is_none() {
                true => {
                    let variable_value =
                        serde_json::to_string(&to_show.data.unwrap()).unwrap_or_default();
                    let initialization_script = format!(
                        "window.json_data = {}; window.export_image = {:?};",
                        variable_value, export_image
                    );

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
                    }
                })
                .with_ipc_handler({
                    let proxy = proxy.clone();
                    move |_, string| match string.as_str() {
                        _ if string.starts_with("#SEND_IMAGE:") => {
                            let data_url = string.replace("#SEND_IMAGE:", "").to_string();
                            proxy
                                .send_event(UserEvent::STDout(data_url))
                                .unwrap_or_default();
                        }
                        _ if string.starts_with("data:") => {
                            proxy
                                .send_event(UserEvent::BlobChunk(Some(string)))
                                .unwrap_or_default();
                        }
                        "#EOF" => {
                            proxy
                                .send_event(UserEvent::BlobChunk(None))
                                .unwrap_or_default();
                        }
                        _ if string.starts_with("#OPEN_FILE:") => {
                            proxy
                                .send_event(UserEvent::OpenFile(Some(PathBuf::from(&string[11..]))))
                                .unwrap_or_default();
                        }
                        "#DEVTOOLS" => {
                            proxy
                                .send_event(UserEvent::DevTools(window_id))
                                .unwrap_or_default();
                        }
                        _ => {}
                    }
                })
                .with_initialization_script(constants::BLOBINIT_SCRIPT)
                .with_initialization_script(constants::PYWRY_WINDOW_SCRIPT)
                .with_initialization_script(constants::PLOTLY_RENDER_JS);

            match init_view
                .with_devtools(console.active)
                .with_url("wry://localhost")
            {
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

/// Starts Main Runtime Loop and creates a new headless window on `WindowManager.start_headless()`
///
/// # Description
/// This function creates a new event loop and event loop proxy. We then create a new headless window and
/// wait for data to be sent from Python. Once we receive data we send it to the webview to render the plot.
///
/// We return a dictionary with the key `result` and base64 string encoded image.
///
/// # Arguments
/// * `sender` - The sender to send messages from Python to Wry Event Loop
/// * `receiver` - The receiver Wry uses to receive messages from Python
/// * `console` - The ConsolePrinter struct to print log messages to the console
///
/// # Returns
/// * `Result<(), String>` - An error message or nothing
pub fn start_headless(
    sender: Sender<String>,
    receiver: Receiver<String>,
    console: ConsolePrinter,
) -> Result<(), String> {
    let event_loop: EventLoop<UserEvent> = EventLoop::with_user_event();
    let proxy = event_loop.create_proxy();
    let mut webviews = HashMap::new();
    let mut init_headless = false;

    std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async move { run_listener(sender.clone()).await.unwrap() })
    });

    event_loop.run(move |event, event_loop, control_flow| {
        *control_flow = ControlFlow::Poll;
        if !init_headless {
            console.debug("Received response");

            let chart = ShowableHeadless::new("").unwrap_or_default();
            match create_new_window_headless(chart, &event_loop, &proxy, console) {
                Err(error) => console.error(&format!("Window Creation Error: {}", error)),

                Ok(new_window) => {
                    webviews.insert(new_window.0, new_window.1);
                }
            };

            init_headless = true;
        }

        let response = receiver.try_recv().unwrap_or_default();

        if !response.is_empty() && init_headless {
            let window_id = webviews.iter_mut().next().unwrap().0;
            proxy
                .send_event(UserEvent::NewPlot(response, *window_id))
                .unwrap_or_default();
        }

        match event {
            // UserEvent::STDout
            Event::UserEvent(UserEvent::STDout(data_url)) => {
                std::thread::spawn(move || {
                    let stdout: io::Stdout = io::stdout();
                    let mut handler = stdout.lock();
                    let response = serde_json::json!({ "result": data_url }).to_string();
                    handler
                        .write_all(format!("{}\n", response).as_bytes())
                        .unwrap();
                    handler.flush().unwrap();
                });
            }
            Event::UserEvent(UserEvent::NewPlot(data, _windowid)) => {
                let _proxy = proxy.clone();
                let plot_data = PlotData::to_json(&data).to_string();

                webviews
                    .iter_mut()
                    .next()
                    .unwrap()
                    .1
                    .evaluate_script(&format!("plotly_render({});", plot_data))
                    .unwrap();
            }
            // UserEvent::CloseWindow
            Event::UserEvent(UserEvent::CloseWindow(window_id)) => {
                console.debug("Closing Window");
                match webviews.get(&window_id) {
                    Some(_) => {
                        console.debug("Closing Webview");
                        webviews.remove(&window_id);
                    }
                    None => console.debug("Webview not found"),
                }
            }
            // UserEvent::BlobChunk
            Event::UserEvent(UserEvent::BlobChunk(_)) => {
                console.debug("Blob Chunk");
            }
            // WindowEvent::CloseRequested
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
                ..
            } => {
                console.debug("Close Requested");
                match webviews.get(&window_id) {
                    Some(_) => {
                        console.debug("Closing Webview");
                        webviews.remove(&window_id);
                    }
                    None => console.debug("Webview not found"),
                }
            }
            // UserEvent::DevTools
            Event::UserEvent(UserEvent::DevTools(window_id)) => {
                console.debug("DevTools");
                match webviews.get(&window_id) {
                    Some(webview) => {
                        console.debug("Opening DevTools");
                        webview.open_devtools();
                    }
                    None => console.debug("Webview not found"),
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
                        console.error(&format!("Error opening file: {}", error));
                    }
                }
            }
            _ => {}
        }
    });
}
