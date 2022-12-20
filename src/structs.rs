use image::ImageFormat;
use serde_json::Value;
use std::fs::{read, read_to_string};
use wry::application::window::Icon;

pub struct Showable {
    pub html: String,
    pub title: String,
    pub height: Option<u32>,
    pub width: Option<u32>,
    pub icon: Option<Icon>,
}

impl Showable {
    pub fn new(raw_json: &str) -> Option<Self> {
        let json: serde_json::Value = match serde_json::from_str(raw_json) {
            Err(_) => return None,
            Ok(item) => item,
        };

        let mut html = json["html"].as_str().unwrap_or_default().to_string();
        let figure: Value = json["plotly"].clone();
        let icon = json["icon"].as_str().unwrap_or_default();
        let title;
        let mut height: Option<u32> = None;
        let mut width: Option<u32> = None;

        if !figure.is_null() {
            title = "OpenBB - ".to_string()
                + figure["layout"]["title"]["text"]
                    .as_str()
                    .unwrap_or("Plots");
            let raw_width = figure["layout"]["width"].as_u64().unwrap_or(800);
            let raw_height = figure["layout"]["height"].as_u64().unwrap_or(600);
            width = Some(u32::try_from(raw_width).unwrap_or(800));
            height = Some(u32::try_from(raw_height).unwrap_or(600));
            html = read_to_string(html)
                .unwrap_or_default()
                .replace("\"{{figure_json}}\"", &figure.to_string());
        } else {
            title = json["title"].as_str().unwrap_or_default().to_string();
        }
        let icon_object: Option<Icon> = match read(icon) {
            Err(_) => None,
            Ok(bytes) => match image::load_from_memory_with_format(&bytes, ImageFormat::Png) {
                Err(_) => None,
                Ok(loaded) => {
                    let imagebuffer = loaded.into_rgb8();
                    let (icon_width, icon_height) = imagebuffer.dimensions();
                    let icon_rgba = imagebuffer.into_raw();
                    match Icon::from_rgba(icon_rgba, icon_width, icon_height) {
                        Err(_) => None,
                        Ok(icon) => Some(icon),
                    }
                }
            },
        };

        Some(Self {
            html,
            title,
            height,
            width,
            icon: icon_object,
        })
    }
}
