use rpc::rpc::*;
use rpc::server::{RPCHandler, Server};
use rpc::transport::stdtcp::*;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

use crate::fxrpc::drpc::fileops::*;

////////////////////////////////// SERVER //////////////////////////////////

fn construct_ret(hdr: &mut RPCHeader, payload: &mut [u8]) {
    hdr.msg_id = 0;
    hdr.msg_type = 0;
    hdr.msg_len = 0 as MsgLen;
}

fn handle_open(hdr: &mut RPCHeader, payload: &mut [u8]) -> Result<(), RPCError> {
    let msg_id: MsgId = hdr.msg_id;
    let msg_type: RPCType = hdr.msg_type;
    let msg_len: usize = hdr.msg_len as usize;
    let path_str = std::str::from_utf8(&payload[0..msg_len]);
    println!(
        "Request with ID {:?}, type {:?}, len {:?}",
        msg_id, msg_type, msg_len
    );
    println!("Payload: {:?}", path_str);
    construct_ret(hdr, payload);
    Ok(())
}

fn handle_read(hdr: &mut RPCHeader, payload: &mut [u8]) -> Result<(), RPCError> {
    let msg_id: MsgId = hdr.msg_id;
    let msg_type: RPCType = hdr.msg_type;
    let msg_len: MsgLen = hdr.msg_len;
    println!(
        "Request with ID {:?}, type {:?}, len {:?}",
        msg_id, msg_type, msg_len
    );
    construct_ret(hdr, payload);
    Ok(())
}

fn handle_pread(hdr: &mut RPCHeader, payload: &mut [u8]) -> Result<(), RPCError> {
    let msg_id: MsgId = hdr.msg_id;
    let msg_type: RPCType = hdr.msg_type;
    let msg_len: MsgLen = hdr.msg_len;
    println!(
        "Request with ID {:?}, type {:?}, len {:?}",
        msg_id, msg_type, msg_len
    );
    construct_ret(hdr, payload);
    Ok(())
}

fn handle_write(hdr: &mut RPCHeader, payload: &mut [u8]) -> Result<(), RPCError> {
    let msg_id: MsgId = hdr.msg_id;
    let msg_type: RPCType = hdr.msg_type;
    let msg_len: MsgLen = hdr.msg_len;
    println!(
        "Request with ID {:?}, type {:?}, len {:?}",
        msg_id, msg_type, msg_len
    );
    construct_ret(hdr, payload);
    Ok(())
}

fn handle_pwrite(hdr: &mut RPCHeader, payload: &mut [u8]) -> Result<(), RPCError> {
    let msg_id: MsgId = hdr.msg_id;
    let msg_type: RPCType = hdr.msg_type;
    let msg_len: MsgLen = hdr.msg_len;
    println!(
        "Request with ID {:?}, type {:?}, len {:?}",
        msg_id, msg_type, msg_len
    );
    construct_ret(hdr, payload);
    Ok(())
}

fn handle_close(hdr: &mut RPCHeader, payload: &mut [u8]) -> Result<(), RPCError> {
    let msg_id: MsgId = hdr.msg_id;
    let msg_type: RPCType = hdr.msg_type;
    let msg_len: MsgLen = hdr.msg_len;
    println!(
        "Request with ID {:?}, type {:?}, len {:?}",
        msg_id, msg_type, msg_len
    );
    construct_ret(hdr, payload);
    Ok(())
}

fn handle_remove(hdr: &mut RPCHeader, payload: &mut [u8]) -> Result<(), RPCError> {
    let msg_id: MsgId = hdr.msg_id;
    let msg_type: RPCType = hdr.msg_type;
    let msg_len: MsgLen = hdr.msg_len;
    println!(
        "Request with ID {:?}, type {:?}, len {:?}",
        msg_id, msg_type, msg_len
    );
    construct_ret(hdr, payload);
    Ok(())
}

fn handle_fsync(hdr: &mut RPCHeader, payload: &mut [u8]) -> Result<(), RPCError> {
    let msg_id: MsgId = hdr.msg_id;
    let msg_type: RPCType = hdr.msg_type;
    let msg_len: MsgLen = hdr.msg_len;
    println!(
        "Request with ID {:?}, type {:?}, len {:?}",
        msg_id, msg_type, msg_len
    );
    construct_ret(hdr, payload);
    Ok(())
}

fn handle_mkdir(hdr: &mut RPCHeader, payload: &mut [u8]) -> Result<(), RPCError> {
    let msg_id: MsgId = hdr.msg_id;
    let msg_type: RPCType = hdr.msg_type;
    let msg_len: MsgLen = hdr.msg_len;
    println!(
        "Request with ID {:?}, type {:?}, len {:?}",
        msg_id, msg_type, msg_len
    );
    construct_ret(hdr, payload);
    Ok(())
}

fn handle_rmdir(hdr: &mut RPCHeader, payload: &mut [u8]) -> Result<(), RPCError> {
    let msg_id: MsgId = hdr.msg_id;
    let msg_type: RPCType = hdr.msg_type;
    let msg_len: MsgLen = hdr.msg_len;
    println!(
        "Request with ID {:?}, type {:?}, len {:?}",
        msg_id, msg_type, msg_len
    );
    construct_ret(hdr, payload);
    Ok(())
}

const OPEN_HANDLER: RPCHandler = handle_open;
const READ_HANDLER: RPCHandler = handle_read;
const PREAD_HANDLER: RPCHandler = handle_pread;
const WRITE_HANDLER: RPCHandler = handle_write;
const PWRITE_HANDLER: RPCHandler = handle_pwrite;
const CLOSE_HANDLER: RPCHandler = handle_close;
const REMOVE_HANDLER: RPCHandler = handle_remove;
const FSYNC_HANDLER: RPCHandler = handle_fsync;
const MKDIR_HANDLER: RPCHandler = handle_mkdir;
const RMDIR_HANDLER: RPCHandler = handle_rmdir;

fn register_rpcs(server: &mut Server) {
    server
        .register(DRPC::Open as RPCType, &OPEN_HANDLER)
        .unwrap();
    server
        .register(DRPC::Read as RPCType, &READ_HANDLER)
        .unwrap();
    server
        .register(DRPC::PRead as RPCType, &PREAD_HANDLER)
        .unwrap();
    server
        .register(DRPC::Write as RPCType, &WRITE_HANDLER)
        .unwrap();
    server
        .register(DRPC::PWrite as RPCType, &PWRITE_HANDLER)
        .unwrap();
    server
        .register(DRPC::Close as RPCType, &CLOSE_HANDLER)
        .unwrap();
    server
        .register(DRPC::Remove as RPCType, &REMOVE_HANDLER)
        .unwrap();
    server
        .register(DRPC::MkDir as RPCType, &MKDIR_HANDLER)
        .unwrap();
    server
        .register(DRPC::RmDir as RPCType, &RMDIR_HANDLER)
        .unwrap();
}

fn server_from_stream(stream: TcpStream) {
    let transport = StdTCP {
        stream: Arc::new(Mutex::new(stream)),
    };
    let mut server = Server::new(Box::new(transport));
    register_rpcs(&mut server);
    // I dont think we need this, client registration in DiNOS
    // is usually for allocation of kernel resources (shmem and dcm)
    // server.add_client(&CLIENT_REGISTRAR);
    server.run_server();
}

pub fn start_drpc_server_tcp(bind_addr: &str, port: u64) {
    println!("Starting DRPC server on port {}", port);
    // TODO: bind to addr/port specified in parameters
    let listener = TcpListener::bind("127.0.0.1:8080").expect("Failed to create TCP transport");

    for stream in listener.incoming() {
        std::thread::spawn(move || server_from_stream(stream.unwrap()));
    }
}

pub fn start_drpc_server_uds() {}
