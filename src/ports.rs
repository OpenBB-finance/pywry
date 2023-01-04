use std::net::TcpListener;

pub fn get_available_port() -> Option<u16> {
    (14733..16789).find(|port| port_is_available(*port))
}

fn port_is_available(port: u16) -> bool {
    TcpListener::bind(("localhost", port)).is_ok()
}
