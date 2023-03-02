use image::ImageFormat;
use serde_json::Value;
use std::fs::{read, read_to_string};
use wry::application::window::Icon;

pub struct Showable {
    pub html_path: String,
    pub html_str: String,
    pub title: String,
    pub height: Option<u32>,
    pub width: Option<u32>,
    pub icon: Option<Icon>,
    pub figure: Option<Value>,
    pub data: Option<Value>,
    pub download_path: String,
    pub export_image: String,
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
        let mut figure: Option<Value> = None;
        let mut data: Option<Value> = None;
        let export_image = json["export_image"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let download_path = json["download_path"]
            .as_str()
            .unwrap_or_default()
            .to_string();

        if !json_data.is_null() && json_data["layout"].is_object() {
            let raw_width = json_data["layout"]["width"].as_u64().unwrap_or(800);
            let raw_height = json_data["layout"]["height"].as_u64().unwrap_or(600);
            width = Some(u32::try_from(raw_width).unwrap_or(800));
            height = Some(u32::try_from(raw_height).unwrap_or(600));
            figure = Some(json_data);
        } else if !json_data.is_null() {
            data = Some(json_data);
        }

        if !html_path.is_empty() {
            html_path = read_to_string(html_path).unwrap_or_default();
        }

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

        Some(Self {
            html_path,
            html_str,
            title,
            height,
            width,
            icon: icon_object,
            figure,
            data,
            download_path,
            export_image,
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
            icon: None,
            figure: None,
            data: None,
            download_path: "".to_string(),
            export_image: "".to_string(),
        }
    }
}
