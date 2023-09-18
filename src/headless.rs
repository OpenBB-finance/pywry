use crate::{
	constants,
	events::handle_events,
	handlers::add_handlers,
	pipe::run_listener,
	structs::{ConsolePrinter, ShowableHeadless, UserEvent},
	utils::decode_path,
};
use std::{
	collections::HashMap,
	fs::{canonicalize, read},
};

use wry::{
	application::{
		dpi::LogicalSize,
		event_loop::{ControlFlow, EventLoop, EventLoopProxy, EventLoopWindowTarget},
		window::{WindowBuilder, WindowId},
	},
	http::{header::CONTENT_TYPE, Response},
	webview::{WebView, WebViewBuilder},
};

#[cfg(wry_event_loop)]
use wry::application::event_loop::EventLoopBuilder;

/// Creates a new headless window and returns the window id and webview
/// # Arguments
/// * `to_show` - The Showable struct that contains the information to show
/// * `event_loop` - The event loop to create the window on
/// * `proxy` - The event loop proxy to send events to
/// * `console` - The ConsolePrinter struct to print log messages to the console
/// # Returns
/// * `Result<(WindowId, WebView), String>` - The window id and webview or an error message
fn create_new_window_headless(
	to_show: ShowableHeadless, event_loop: &&EventLoopWindowTarget<UserEvent>,
	proxy: &EventLoopProxy<UserEvent>, console: ConsolePrinter,
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
		Ok(item) => item,
	};

	let protocol = webview.with_hotkeys_zoom(true).with_custom_protocol(
		"wry".into(),
		move |request| {
			let path = request.uri().path();
			let clean_path = &path[1..];
			let content = content.clone();
			let mut mime = mime_guess::from_path("index.html");

			let content = if path == "/" {
				content.into()
			} else {
				let decoded = decode_path(clean_path);
				mime = mime_guess::from_path(decoded.clone());
				match read(canonicalize(decoded).unwrap_or_default()) {
					Err(_) => content.into(),
					Ok(bytes) => bytes.into(),
				}
			};

			let mimetype = mime
				.first()
				.map(|mime| mime.to_string())
				.unwrap_or_else(|| "text/plain".to_string());

			#[cfg(target_os = "windows")]
			let headers = "https://wry.localhost".to_string();
			#[cfg(not(target_os = "windows"))]
			let headers = "wry://localhost".to_string();

			Response::builder()
				.header(CONTENT_TYPE, mimetype)
				.header("Access-Control-Allow-Origin", headers)
				.header("Accept-Encoding", "gzip, compress, br, deflate")
				.body(content)
				.map_err(Into::into)
		},
	);
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

	let init_view = add_handlers(
		init_view,
		proxy,
		window_id,
		"".to_string(),
		to_show.export_image,
		"".to_string().as_str(),
		Some(true),
		console,
	);

	return match init_view.with_devtools(console.active).with_url("wry://localhost") {
		Err(error3) => return Err(error3.to_string()),
		Ok(subitem) => match subitem.build() {
			Err(error4) => return Err(error4.to_string()),
			Ok(sub2item) => Ok((window_id, sub2item)),
		},
	};
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
/// * `console` - The ConsolePrinter struct to print log messages to the console
///
/// # Returns
/// * `Result<(), String>` - An error message or nothing
pub fn start_headless(console: ConsolePrinter) -> Result<(), String> {
	#[cfg(wry_event_loop)]
	let event_loop: EventLoop<UserEvent> =
		EventLoopBuilder::<UserEvent>::with_user_event().build();
	#[cfg(not(wry_event_loop))]
	let event_loop: EventLoop<UserEvent> = EventLoop::with_user_event();

	let proxy = event_loop.create_proxy();
	let mut webviews = HashMap::new();
	let mut listener_spawned = false;

	event_loop.run(move |event, event_loop, control_flow| {
		*control_flow = ControlFlow::Wait;

		if !listener_spawned {
			console.debug("Starting listener thread");

			let chart = ShowableHeadless::new("").unwrap_or_default();
			match create_new_window_headless(chart, &event_loop, &proxy, console) {
				Err(error) => console.error(&format!("Window Creation Error: {}", error)),

				Ok(new_window) => {
					webviews.insert(new_window.0, new_window.1);
				}
			};

			let proxy = proxy.clone();
			std::thread::spawn(move || {
				tokio::runtime::Builder::new_current_thread()
					.enable_all()
					.build()
					.unwrap()
					.block_on(async {
						match run_listener(&proxy).await {
							Ok(_) => (),
							Err(error) => {
								console.debug(&format!("Error: {}", error));
							}
						}
					});
			});

			listener_spawned = true;
		}

		handle_events(event, &mut webviews, &proxy, console.clone(), event_loop, true);
	});
}
