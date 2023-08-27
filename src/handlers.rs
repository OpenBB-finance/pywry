use crate::{
	constants,
	structs::{ConsolePrinter, UserEvent},
};
use std::path::PathBuf;

#[cfg(not(target_os = "windows"))]
use crate::utils::get_icon;

use wry::{
	application::{event_loop::EventLoopProxy, window::WindowId},
	webview::WebViewBuilder,
};

pub fn add_handlers<'a>(
	init_view: WebViewBuilder<'a>, proxy: &'a EventLoopProxy<UserEvent>,
	window_id: WindowId, download_path: String, export_image: String, window_icon: &str,
	is_headless: Option<bool>, console: ConsolePrinter,
) -> WebViewBuilder<'a> {
	let _is_export = !export_image.is_empty();
	let is_headless = is_headless.unwrap_or_default();
	#[cfg(target_os = "macos")]
	let maxos_script = constants::MACOS_COPY_PASTE_SCRIPT;
	#[cfg(not(target_os = "macos"))]
	let maxos_script = "";

	// we add a download handler, if export_image is set it takes precedence over download_path
	return init_view
		.with_download_started_handler({
			let _proxy = proxy.clone();
			move |_uri: String, default_path| {
				#[cfg(not(target_os = "macos"))]
				{
					if _uri.starts_with("blob:") {
						let submitted =
							_proxy.send_event(UserEvent::BlobReceived(_uri, window_id)).is_ok();
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
				_ if string.starts_with("#PYWRY_RESULT:") => {
					proxy
						.send_event(UserEvent::STDout(string[14..].to_string()))
						.unwrap_or_default();

					if !is_headless && !console.active {
						proxy.send_event(UserEvent::CloseWindow(window_id)).unwrap_or_default();
					}
				}
				_ if string.starts_with("data:") => {
					proxy.send_event(UserEvent::BlobChunk(Some(string))).unwrap_or_default();
				}
				"#EOF" => {
					proxy.send_event(UserEvent::BlobChunk(None)).unwrap_or_default();
				}
				_ if string.starts_with("#OPEN_FILE:") => {
					proxy
						.send_event(UserEvent::OpenFile(Some(PathBuf::from(&string[11..]))))
						.unwrap_or_default();
				}
				"#DEVTOOLS" => {
					proxy.send_event(UserEvent::DevTools(window_id)).unwrap_or_default();
				}
				_ => {}
			}
		})
		.with_download_completed_handler({
			let proxy = proxy.clone();
			move |_uri, filepath, success| {
				let _filepath = filepath.unwrap_or_default();

				#[cfg(not(target_os = "macos"))]
				proxy
					.send_event(UserEvent::DownloadComplete(
						Some(_filepath),
						success,
						download_path.clone(),
						export_image.clone(),
						window_id,
					))
					.unwrap_or_default();

				#[cfg(target_os = "macos")]
				{
					if success && _is_export {
						proxy.send_event(UserEvent::CloseWindow(window_id)).unwrap_or_default();
					}
				}
			}
		})
		.with_new_window_req_handler({
			let _window_icon = window_icon.to_string();
			#[cfg(not(target_os = "windows"))]
			{
				let proxy = proxy.clone();
				move |uri: String| {
					let submitted = proxy
						.send_event(UserEvent::NewWindow(uri.clone(), get_icon(&_window_icon)))
						.is_ok();
					submitted
				}
			}
			#[cfg(target_os = "windows")]
			{
				move |_uri: String| true
			}
		})
		.with_initialization_script(constants::BLOBINIT_SCRIPT)
		.with_initialization_script(constants::PYWRY_WINDOW_SCRIPT)
		.with_initialization_script(constants::PLOTLY_RENDER_JS)
		.with_initialization_script(maxos_script);
}
