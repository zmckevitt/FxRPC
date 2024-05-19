use rpc::server::{RPCHandler, Server};
use rpc::rpc::*;
use rpc::transport::stdtcp::*;
use std::net::{TcpListener, TcpStream};

////////////////////////////// DiNOS RPC Implementation //////////////////////////////

#[derive(Debug, Eq, PartialEq, PartialOrd, Clone, Copy)]
#[repr(u8)]
pub(crate) enum DinosRpc {
    /// Create a file
    Create = 0,
    /// Open a file
    Open = 1,
    /// Read from a file
    Read = 2,
    /// Read from a file from the given offset
    PRead = 3,
    /// Write to a file
    Write = 4,
    /// Write to a file
    PWrite = 5,
    /// Close an opened file.
    Close = 6,
    /// Get the information related to the file.
    GetInfo = 7,
    /// Remove the file
    Remove = 8,
    /// Write to a file without going into NR.
    WriteDirect = 9,
    /// Rename a file.
    FileRename = 10,
    /// Create a directory.
    MkDir = 11,
}

////////////////////////////////// SERVER //////////////////////////////////

fn handle_open(hdr: &mut RPCHeader, payload: &mut [u8]) -> Result<(), RPCError> {
    Ok(())
}

fn handle_read(hdr: &mut RPCHeader, payload: &mut [u8]) -> Result<(), RPCError> {
    Ok(())
}

fn handle_pread(hdr: &mut RPCHeader, payload: &mut [u8]) -> Result<(), RPCError> {
    Ok(())
}

fn handle_write(hdr: &mut RPCHeader, payload: &mut [u8]) -> Result<(), RPCError> {
    Ok(())
}

fn handle_pwrite(hdr: &mut RPCHeader, payload: &mut [u8]) -> Result<(), RPCError> {
    Ok(())
}

fn handle_close(hdr: &mut RPCHeader, payload: &mut [u8]) -> Result<(), RPCError> {
    Ok(())
}

fn handle_remove(hdr: &mut RPCHeader, payload: &mut [u8]) -> Result<(), RPCError> {
    Ok(())
}

fn handle_fsync(hdr: &mut RPCHeader, payload: &mut [u8]) -> Result<(), RPCError> {
    Ok(())
}

fn handle_mkdir(hdr: &mut RPCHeader, payload: &mut [u8]) -> Result<(), RPCError> {
    Ok(())
}

fn handle_rmdir(hdr: &mut RPCHeader, payload: &mut [u8]) -> Result<(), RPCError> {
    Ok(())
}

const OPEN_HANDLER:   RPCHandler = handle_open;
const READ_HANDLER:   RPCHandler = handle_read;
const PREAD_HANDLER:  RPCHandler = handle_pread;
const WRITE_HANDLER:  RPCHandler = handle_write;
const PWRITE_HANDLER: RPCHandler = handle_pwrite;
const CLOSE_HANDLER:  RPCHandler = handle_close;
const REMOVE_HANDLER: RPCHandler = handle_remove;
const FSYNC_HANDLER:  RPCHandler = handle_fsync;
const MKDIR_HANDLER:  RPCHandler = handle_mkdir;
const RMDIR_HANDLER:  RPCHandler = handle_rmdir;

fn register_rpcs(server: &mut Server) {
    server.register(DinosRpc::Open as RPCType, &OPEN_HANDLER).unwrap();
    server.register(DinosRpc::Read as RPCType, &READ_HANDLER).unwrap();
    server.register(DinosRpc::PRead as RPCType, &PREAD_HANDLER).unwrap();
    server.register(DinosRpc::Write as RPCType, &WRITE_HANDLER).unwrap();
    server.register(DinosRpc::PWrite as RPCType, &PWRITE_HANDLER).unwrap();
    server.register(DinosRpc::Close as RPCType, &CLOSE_HANDLER).unwrap();
    server.register(DinosRpc::Remove as RPCType, &REMOVE_HANDLER).unwrap();
    server.register(DinosRpc::MkDir as RPCType, &MKDIR_HANDLER).unwrap();
}

fn server_from_stream(stream: TcpStream) {
    let mut server = Server::new(Box::new(stream));
    register_rpcs(&mut server);
    // server_accept
    server.run_server();
}

pub fn start_drpc_server_tcp(bind_addr: &str, port: u64) {
    // TODO: bind to addr/port specified in parameters
    let listener = TcpListener::bind("127.0.0.1:8080")
        .expect("Failed to create TCP transport");

    for stream in listener.incoming() {
        server_from_stream(stream.unwrap());
    }

}

pub fn start_drpc_server_uds() {}
