use std::net::TcpListener;

pub fn get_available_port() -> Option<u16> {
    (14733..16789).find(|port| port_is_available(*port))
}

fn is_linux() -> bool {
    println!("target_os: {}", cfg!(target_os));
    cfg!(target_os = "linux")
}

fn port_is_available_linux(port: u16) -> bool {
    let port_str = port.to_string();
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("lsof -i :{}", port_str))
        .output()
        .expect("failed to execute process");
    let output_str = String::from_utf8(output.stdout).unwrap();
    output_str.is_empty()
}

fn port_is_available(port: u16) -> bool {
    if is_linux() {
        return port_is_available_linux(port);
    }
    TcpListener::bind(("localhost", port)).is_ok()
    && TcpListener::bind(("0.0.0.0", port)).is_ok()
    && TcpListener::bind(("127.0.0.1", port)).is_ok()
}
