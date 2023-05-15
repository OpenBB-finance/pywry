use image::ImageFormat;
use std::{fs::read, path::PathBuf};

use wry::application::window::Icon;

use urlencoding::decode as urldecode;

pub fn decode_path(path: &str) -> PathBuf {
	let decoded = urldecode(&path).expect("UTF-8").to_string();
	let file_path = match decoded.starts_with("file://") {
		true => {
			let decoded = urldecode(&path).expect("UTF-8").to_string();
			let path = PathBuf::from(&decoded);
			if ":" == &decoded[9..10] {
				path.strip_prefix("file://").unwrap().to_path_buf()
			} else {
				let path = PathBuf::from(&decoded[6..]);
				path.to_path_buf()
			}
		}
		false => PathBuf::from(path),
	};

	file_path
}

/// Gets the icon from the path
/// # Arguments
/// * `icon` - The path to the icon
/// # Returns
/// * `Option<Icon>` - The icon or None
pub fn get_icon(icon: &str) -> Option<Icon> {
	let icon_object = match read(icon) {
		Err(_) => None,
		Ok(bytes) => {
			let imagebuffer =
				match image::load_from_memory_with_format(&bytes, ImageFormat::Png) {
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
