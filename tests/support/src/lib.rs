include!(concat!(env!("OUT_DIR"), "/bindings_module.rs"));
pub mod fixture;
pub mod scenarios;

pub fn free_port() -> u16 {
    std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}
