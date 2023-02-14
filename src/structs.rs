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
        let plotly: Value = json["plotly"].clone();
        let icon = json["icon"].as_str().unwrap_or_default().to_string();
        let title = json["title"].as_str().unwrap_or_default().to_string();
        let mut height: Option<u32> = json["height"].as_u64().and_then(|x| u32::try_from(x).ok());
        let mut width: Option<u32> = json["width"].as_u64().and_then(|x| u32::try_from(x).ok());
        let mut figure: Option<Value> = None;
        let export_image = json["export_image"].as_str().unwrap_or_default().to_string();

        if !plotly.is_null() {
            let raw_width = plotly["layout"]["width"].as_u64().unwrap_or(800);
            let raw_height = plotly["layout"]["height"].as_u64().unwrap_or(600);
            width = Some(u32::try_from(raw_width).unwrap_or(800));
            height = Some(u32::try_from(raw_height).unwrap_or(600));
            html_path = read_to_string(html_path).unwrap_or_default();
            figure = Some(plotly);
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
            export_image: "".to_string(),
        }
    }
}
