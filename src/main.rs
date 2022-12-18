use std::{thread, time};
use window::start_wry;
use std::sync::mpsc;

pub mod ports;
pub mod websocket;
pub mod window;

fn main() {
    let (sender, receiver) = mpsc::channel();
    start_wry(8500, sender.clone(), receiver).unwrap();
    printl
    thread::sleep(time::Duration::from_secs(5));
    sender.send("<h1>The first HTML test</h1>".to_string()).unwrap();
    sender.send("<h1>The second HTML test</h1>".to_string()).unwrap();
    sender.send("<h1>The third HTML test</h1>".to_string()).unwrap();
    sender.send("<h1>The fourth HTML test</h1>".to_string()).unwrap();
}
