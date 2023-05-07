use serde_json::Value;
#[cfg(not(target_os = "windows"))]
use wry::application::window::Icon;
use wry::application::window::{Theme, WindowId};

use std::{fs::read_to_string, path::PathBuf};

/// A struct for printing debug messages
///
/// # Example
/// ```
/// use pywry::structs::DebugPrinter;
///
/// let debug_printer = DebugPrinter::new(true);
/// debug_printer.print("This is a debug message");
///
/// // Can also be used with match statements for debug only code
/// let match_result = match debug_printer.active {
///    true => "Debug is active",
///   false => "Debug is not active",
/// };
/// println!("{}", match_result);
/// ```
/// # Output
/// ```text
/// This is a debug message
/// Debug is active
/// ```
/// # Notes
/// This struct is used to print debug messages to the console.
/// The `active` field determines if debug messages should be printed. This is useful for
/// printing debug messages when the `debug` flag is set to `true` in the `WindowManager` struct.
/// The `print` method sends a debug message to the console. The `active` field is checked
/// before printing the message.
///
#[derive(Copy, Clone)]
pub struct DebugPrinter {
    pub active: bool,
}

impl DebugPrinter {
    pub fn new(active: bool) -> Self {
        Self { active }
    }

    pub fn print(&self, message: &str) {
        if self.active {
            println!("{}", message);
        }
    }
}

pub enum UserEvent {
    #[cfg(not(target_os = "macos"))]
    DownloadStarted(String, String),
    #[cfg(not(target_os = "macos"))]
    DownloadComplete(Option<PathBuf>, bool, String, String, WindowId),
    #[cfg(not(target_os = "macos"))]
    BlobReceived(String, WindowId),
    BlobChunk(Option<String>),
    CloseWindow(WindowId),
    DevTools(WindowId),
    NewWindowCreated(WindowId),
    OpenFile(Option<PathBuf>),
    #[cfg(not(target_os = "windows"))]
    NewWindow(String, Option<Icon>),
}

pub struct Showable {
    pub html_path: String,
    pub html_str: String,
    pub title: String,
    pub height: Option<u32>,
    pub width: Option<u32>,
    pub icon: String,
    pub data: Option<Value>,
    pub download_path: String,
    pub export_image: String,
    pub theme: Theme,
}

impl Showable {
    pub fn new(raw_json: &str) -> Option<Self> {
        let json: serde_json::Value = match serde_json::from_str(raw_json) {
            Err(_) => return None,
            Ok(item) => item,
        };

        let mut html_path = json["html_path"].as_str().unwrap_or_default().to_string();
        let html_str = json["html_str"].as_str().unwrap_or_default().to_string();
        let json_data: Value = json["json_data"].clone();
        let icon = json["icon"].as_str().unwrap_or_default().to_string();
        let title = json["title"].as_str().unwrap_or_default().to_string();
        let mut height: Option<u32> = json["height"].as_u64().and_then(|x| u32::try_from(x).ok());
        let mut width: Option<u32> = json["width"].as_u64().and_then(|x| u32::try_from(x).ok());
        let mut data: Option<Value> = None;
        let export_image = json["export_image"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let download_path = json["download_path"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let mut theme = Theme::Light;

        if !json_data.is_null() {
            if json_data["layout"].is_object() {
                let raw_width = json_data["layout"]["width"].as_u64().unwrap_or(800);
                let raw_height = json_data["layout"]["height"].as_u64().unwrap_or(600);
                width = Some(u32::try_from(raw_width).unwrap_or(800));
                height = Some(u32::try_from(raw_height).unwrap_or(600));
                theme = Theme::Dark;
            }
            data = Some(json_data);
        }

        if !html_path.is_empty() {
            html_path = read_to_string(html_path).unwrap_or_default();
        }

        Some(Self {
            html_path,
            html_str,
            title,
            height,
            width,
            icon,
            data,
            download_path,
            export_image,
            theme,
        })
    }
}

impl Default for Showable {
    fn default() -> Self {
        Self {
            html_path: "".to_string(),
            html_str: "<h1 style='color:red'>There was an error displaying the HTML</h1>"
                .to_string(),
            title: "Error Creating Showable Object".to_string(),
            height: None,
            width: None,
            icon: "".to_string(),
            data: None,
            download_path: "".to_string(),
            export_image: "".to_string(),
            theme: Theme::Light,
        }
    }
}
