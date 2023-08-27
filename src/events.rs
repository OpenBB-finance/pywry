use crate::structs::{ConsolePrinter, PlotData, Showable, UserEvent};
use crate::window::create_new_window;

#[cfg(not(target_os = "macos"))]
use crate::utils::decode_path;

use std::{
	collections::HashMap,
	io::{self, Write},
	path::PathBuf,
};

#[cfg(not(target_os = "macos"))]
use std::fs::{copy, create_dir_all, remove_file};
use urlencoding::decode as urldecode;

use wry::{
	application::{
		event::{Event, WindowEvent},
		event_loop::{EventLoopProxy, EventLoopWindowTarget},
		window::WindowId,
	},
	webview::WebView,
};

#[cfg(not(target_os = "windows"))]
use wry::{
	application::{
		dpi::LogicalSize,
		window::{Theme, WindowBuilder},
	},
	webview::WebViewBuilder,
};

pub fn handle_events(
	event: Event<UserEvent>, webviews: &mut HashMap<WindowId, WebView>,
	_proxy: &EventLoopProxy<UserEvent>, console: ConsolePrinter,
	_event_loop: &EventLoopWindowTarget<UserEvent>, headless: bool,
) {
	match event {
		// UserEvent::NewMessageReceived
		Event::UserEvent(UserEvent::NewMessageReceived(message)) => {
			console.debug("Received message from Python");
			match headless {
				true => {
					let window_id = webviews.iter_mut().next().unwrap().0;
					_proxy
						.send_event(UserEvent::NewPlot(message, *window_id))
						.unwrap_or_default();
				}
				false => {
					let chart = Showable::new(&message).unwrap_or_default();
					match create_new_window(chart, &_event_loop, &_proxy, console) {
						Err(error) => console.error(&format!("Error creating window: {}", error)),
						Ok(new_window) => {
							webviews.insert(new_window.0, new_window.1);
						}
					};
				}
			}
		}
		// UserEvent::STDout
		Event::UserEvent(UserEvent::STDout(result)) => {
			std::thread::spawn(move || {
				let stdout: io::Stdout = io::stdout();
				let mut handler = stdout.lock();
				let decoded = urldecode(&result).unwrap_or_default();
				handler
					.write_all(
						format!("{}\n", serde_json::json!({ "result": decoded })).as_bytes(),
					)
					.unwrap();
				handler.flush().unwrap();
			});
		}
		// UserEvent::NewPlot
		Event::UserEvent(UserEvent::NewPlot(data, _windowid)) => {
			let plot_data = PlotData::to_json(&data).to_string();

			webviews
				.iter_mut()
				.next()
				.unwrap()
				.1
				.evaluate_script(&format!("plotly_render({});", plot_data))
				.unwrap();
		}
		// UserEvent::NewWindowCreated
		Event::UserEvent(UserEvent::NewWindowCreated(window_id)) => {
			console.debug("New Window Created");
			match webviews.get_mut(&window_id) {
				Some(webview) => {
					webview.window().set_always_on_top(false);
				}
				None => {}
			}
		}
		// UserEvent::DownloadStarted
		#[cfg(not(target_os = "macos"))]
		Event::UserEvent(UserEvent::DownloadStarted(uri, path)) => {
			if uri.len() < 200 {
				console.debug(&format!("Download Started: {}", uri));
			}
			console.debug(&format!("Path: {}", path));
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
			console.debug(&format!("Download Complete: {}", success));

			let decoded = decode_path(&filepath.unwrap().to_str().unwrap());

			let new_path = match !download_path.is_empty() {
				true => match !export_image.is_empty() {
					true => {
						let path = PathBuf::from(&export_image);
						path.to_path_buf()
					}
					false => {
						let mut path = PathBuf::from(&download_path);
						path.push(decoded.file_name().unwrap());
						path.to_path_buf()
					}
				},

				false => decoded.to_path_buf(),
			};

			console.debug(&format!("Original Path: {:?}", decoded));
			console.debug(&format!("New Path: {:?}", new_path));

			let dir = new_path.parent().unwrap();
			match !dir.exists() {
				true => {
					console.debug(&format!("Creating directory: {:?}", dir));
					if let Err(error) = create_dir_all(dir) {
						console.error(&format!("Error creating directory: {}", error));
					}
				}
				false => {}
			}

			match copy(&decoded, &new_path) {
				Err(error) => console.error(&format!("Error copying file: {}", error)),
				Ok(_) => {
					if is_export {
						_proxy.send_event(UserEvent::CloseWindow(window_id)).unwrap_or_default();
					}
					if let Err(error) = remove_file(&decoded) {
						console.error(&format!("Error deleting file: {}", error));
					}
				}
			}
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
		Event::WindowEvent { event: WindowEvent::CloseRequested, window_id, .. } => {
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
				let decoded =
					urldecode(&filepath.unwrap().to_str().unwrap()).expect("UTF-8").to_string();
				let path = PathBuf::from(decoded);
				if let Err(error) = open::that(&path.to_str().unwrap()) {
					console.error(&format!("Error opening file: {}", error));
				}
			}
		}
		// WindowEvent::NewWindow
		#[cfg(not(target_os = "windows"))]
		Event::UserEvent(UserEvent::NewWindow(uri, window_icon)) => {
			console.debug(&format!("New Window Requested: {}", uri));
			match (uri.starts_with("http://") || uri.starts_with("https://"))
				&& !uri.starts_with("https://ogs.google.com")
			{
				true => {
					let pre_window = WindowBuilder::new()
						.with_title(uri.to_string())
						.with_window_icon(window_icon)
						.with_inner_size(LogicalSize::new(1300, 900))
						.with_resizable(true)
						.with_theme(Some(Theme::Dark));

					let window = match pre_window.build(_event_loop) {
						Err(error) => {
							console.error(&format!("Window Creation Error: {}", error));
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

					console.debug("New Window Created");
				}
				false => {
					console.debug(&format!("Invalid URI tried to open in new window: {}", uri));
				}
			}
		}
		_ => {}
	}
}
