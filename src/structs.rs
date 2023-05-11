use serde_json::Value;
#[cfg(not(target_os = "windows"))]
use wry::application::window::Icon;
use wry::application::window::{Theme, WindowId};

use std::{
    convert::TryFrom,
    fs::read_to_string,
    io::{self, Write},
    path::PathBuf,
};

/// A struct for printing logs as JSON messages to the console.
///
/// # Example
/// ```
/// use pywry::structs::ConsolePrinter;
///
/// let console_printer = ConsolePrinter::new(true);
/// console_printer.debug("This is a debug message");
/// console_printer.info("This is an info message");
///
/// // Messages are printed to stdout as json strings
/// // {"debug": "This is a debug message"}
/// // {"info": "This is an info message"}
///
/// // They are then read by the python script and printed to the console
/// // Can also be used with match statements for debug only code blocks
/// // such as:
/// let match_result = match console_printer.active {
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
/// This struct is used to print logs to the console as json strings. The `active` field determines
/// if debug messages should be printed. This is useful for printing debug messages when
/// the `debug` flag is set to `true` in the `WindowManager` struct.
///
#[derive(Copy, Clone)]
pub struct ConsolePrinter {
    pub active: bool,
}

impl ConsolePrinter {
    pub fn new(active: bool) -> Self {
        Self { active }
    }

    pub fn get_json(&self, message: &str, level: &str) -> String {
        match serde_json::from_str(&format!("{{\"{}\": \"{}\"}}", level, message)) {
            Ok(json) => json,
            Err(_) => serde_json::json!({}),
        }
        .to_string()
    }

    pub fn debug(&self, message: &str) {
        if self.active {
            self.stdout_handler(message, "debug");
        }
    }

    pub fn info(&self, message: &str) {
        self.stdout_handler(message, "info");
    }

    pub fn error(&self, message: &str) {
        self.stdout_handler(message, "error");
    }

    pub fn stdout_handler(&self, message: &str, level: &str) {
        let json = self.get_json(message, level);
        std::thread::spawn(move || {
            let stdout = io::stdout();
            let mut handler = stdout.lock();
            handler.write_all(format!("{}\n", json).as_bytes()).unwrap();
            handler.flush().unwrap();
        });
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
    NewPlot(String, WindowId),
    OpenFile(Option<PathBuf>),
    STDout(String),
    #[cfg(not(target_os = "windows"))]
    NewWindow(String, Option<Icon>),
}

pub struct Showable {
    pub content: String,
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

        let html_path = json["html_path"].as_str().unwrap_or_default().to_string();
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
            theme = match json_data["theme"].as_str().unwrap_or_default() {
                "dark" => Theme::Dark,
                "light" => Theme::Light,
                _ => Theme::Light,
            };
            if json_data["layout"].is_object() {
                let raw_width = json_data["layout"]["width"].as_u64().unwrap_or(800);
                let raw_height = json_data["layout"]["height"].as_u64().unwrap_or(600);
                width = Some(u32::try_from(raw_width).unwrap_or(800));
                height = Some(u32::try_from(raw_height).unwrap_or(600));
            }
            data = Some(json_data);
        }

        let mut content = match html_path.is_empty() {
            true => html_str,
            false => read_to_string(html_path).unwrap_or_default(),
        };

        if content.is_empty() {
            content = String::from(
                "<h1 style='color:red'>No html content to show, please provide a html_path or a html_str key</h1>",
            );
        }

        Some(Self {
            content,
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
            content: "".to_string(),
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

pub struct ShowableHeadless {
    pub data: Option<Value>,
    pub export_image: String,
    pub scale: Option<u32>,
}

impl ShowableHeadless {
    pub fn new(raw_json: &str) -> Option<Self> {
        let json: serde_json::Value = match serde_json::from_str(raw_json) {
            Err(_) => return None,
            Ok(item) => item,
        };

        let json_data: Value = json["json_data"].clone();
        let export_image = json["export_image"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let mut data: Option<Value> = None;
        let mut scale: Option<u32> = None;

        if !json_data.is_null() {
            if json_data["layout"].is_object() {
                data = Some(json_data);
                scale = Some(
                    json["json_data"]["scale"]
                        .as_u64()
                        .unwrap_or(2)
                        .try_into()
                        .unwrap(),
                );
            }
        }

        Some(Self {
            data,
            export_image,
            scale,
        })
    }
}

impl Default for ShowableHeadless {
    fn default() -> Self {
        Self {
            data: None,
            export_image: "".to_string(),
            scale: None,
        }
    }
}

pub struct PlotData {
    pub figure: Option<Value>,
    pub format: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub scale: Option<u32>,
}

impl PlotData {
    pub fn new(showable: &ShowableHeadless) -> Self {
        let figure = showable.data.clone();
        let format = showable
            .export_image
            .clone()
            .split('.')
            .last()
            .unwrap_or_default()
            .to_string();
        let mut width = None;
        let mut height = None;
        let scale = showable.scale;

        if !figure.is_none() {
            let raw_width = figure.as_ref().unwrap()["layout"]["width"]
                .as_u64()
                .unwrap_or(800);
            let raw_height = figure.as_ref().unwrap()["layout"]["height"]
                .as_u64()
                .unwrap_or(600);
            width = Some(u32::try_from(raw_width).unwrap_or(800));
            height = Some(u32::try_from(raw_height).unwrap_or(600));
        }

        Self {
            figure,
            format,
            width,
            height,
            scale,
        }
    }

    pub fn to_json(raw_json: &str) -> Value {
        let show = match ShowableHeadless::new(raw_json) {
            Some(showable) => showable,
            None => ShowableHeadless::default(),
        };

        let plot_data = Self::new(&show);
        serde_json::json!({
            "figure": plot_data.figure,
            "format": plot_data.format,
            "width": plot_data.width,
            "height": plot_data.height,
            "scale": plot_data.scale,
        })
    }
}
