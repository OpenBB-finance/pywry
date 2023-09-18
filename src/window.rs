use crate::{
	constants::DEV_TOOLS_HTML,
	events::handle_events,
	handlers::add_handlers,
	pipe::run_listener,
	structs::{ConsolePrinter, Showable, UserEvent},
	utils::{decode_path, get_icon},
};
use mime_guess;

#[cfg(target_os = "windows")]
use simple_home_dir::*;
#[cfg(target_os = "windows")]
use std::{env::temp_dir, path::PathBuf};

use std::{
	collections::HashMap,
	fs::{canonicalize, read},
};

use wry::{
	application::{
		dpi::{LogicalSize, PhysicalPosition},
		event_loop::{ControlFlow, EventLoop, EventLoopProxy, EventLoopWindowTarget},
		window::{Theme, WindowBuilder, WindowId},
	},
	http::{header::CONTENT_TYPE, Response},
	webview::{WebView, WebViewBuilder},
};

#[cfg(wry_event_loop)]
use wry::application::event_loop::EventLoopBuilder;

#[cfg(target_os = "windows")]
use wry::webview::WebContext;

/// Creates a new window and returns the window id and webview
/// # Arguments
/// * `to_show` - The Showable struct that contains the information to show
/// * `event_loop` - The event loop to create the window on
/// * `proxy` - The event loop proxy to send events to
/// * `console` - The ConsolePrinter struct to print log messages to the console
/// # Returns
/// * `Result<(WindowId, WebView), String>` - The window id and webview or an error message
pub fn create_new_window(
	to_show: Showable, event_loop: &&EventLoopWindowTarget<UserEvent>,
	proxy: &EventLoopProxy<UserEvent>, console: ConsolePrinter,
) -> Result<(WindowId, WebView), String> {
	let content = to_show.content.clone().into_bytes();
	let window_icon = to_show.icon.clone();

	let content = match console.active {
		true => {
			let mut dev_tools_html = DEV_TOOLS_HTML.as_bytes().to_vec();
			dev_tools_html.extend(content);
			dev_tools_html
		}
		false => content,
	};

	let json_data = to_show.data.clone().unwrap_or_default();
	console.debug(&format!("json_data: {}", json_data));

	let screen_size = event_loop.available_monitors().nth(0).unwrap().size();
	let window_size = (to_show.width.unwrap_or(800), to_show.height.unwrap_or(600));

	let mut pre_window = WindowBuilder::new()
		.with_title(to_show.title)
		.with_position(PhysicalPosition::new(
			(screen_size.width / 2) - (window_size.0 / 2) + (rand::random::<u32>() % 100),
			(screen_size.height / 2) - (window_size.1 / 2) + (rand::random::<u32>() % 100),
		))
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

	let minimized = !to_show.export_image.is_empty() && !console.active;
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
		Ok(item) => item,
	};

	let protocol =
		webview.with_background_color(background_color).with_hotkeys_zoom(true);

	#[cfg(target_os = "windows")]
	let mut cache_directory = WebContext::new(Some(PathBuf::from(
		home_dir().unwrap_or_else(|| temp_dir()).join(".cache").join("wry"),
	)));
	#[cfg(target_os = "windows")]
	let protocol = protocol.with_web_context(&mut cache_directory);

	let protocol = match to_show.options.url.starts_with("wry://") {
		true => protocol.with_custom_protocol("wry".into(), move |request| {
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
		}),
		false => protocol,
	};

	let export_image = to_show.export_image.clone();
	let _is_export = !export_image.is_empty();
	let download_path = to_show.download_path.clone();

	let init_view = match !to_show.data.is_none() {
		true => {
			let variable_value =
				serde_json::to_string(&to_show.data.unwrap()).unwrap_or_default();
			let initialization_script = match !export_image.is_empty() {
				true => format!(
					"window.json_data = {}; window.export_image = {:?};",
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

	let init_view = add_handlers(
		init_view,
		proxy,
		window_id,
		download_path,
		export_image,
		&window_icon,
		Some(false),
		console,
	);

	let init_view = match to_show.options.init_script.is_some() {
		true => init_view.with_initialization_script(&to_show.options.init_script.unwrap()),
		false => init_view,
	};

	return match init_view.with_devtools(console.active).with_url(&to_show.options.url) {
		Err(error3) => return Err(error3.to_string()),
		Ok(subitem) => match subitem.build() {
			Err(error4) => return Err(error4.to_string()),
			Ok(sub2item) => {
				if !minimized {
					let proxy = proxy.clone();
					proxy.send_event(UserEvent::NewWindowCreated(window_id)).unwrap_or_default();
				}
				Ok((window_id, sub2item))
			}
		},
	};
}

/// Starts Main Runtime Loop and creates a new window when a message is received from Python
/// # Arguments
/// * `console` - The ConsolePrinter struct to print log messages to the console
///
/// # Returns
/// * `Result<(), String>` - An error message or nothing
pub fn start_wry(console: ConsolePrinter) -> Result<(), String> {
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

		handle_events(event, &mut webviews, &proxy, console.clone(), event_loop, false);
	});
}
