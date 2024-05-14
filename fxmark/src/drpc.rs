use rpc::server::Server;
use rpc::transport::stdtcp::*;
use std::net::TcpListener;

pub fn start_drpc_server_tcp(bind_addr: &str, port: u64) {
    // TODO: bind to addr/port specified in parameters
    let transport = Box::new(
        TcpListener::bind("127.0.0.1:8080")
        .expect("Failed to create TCP transport"),
    );

    let server = Server::new(transport);
}

pub fn start_drpc_server_uds() {

}
