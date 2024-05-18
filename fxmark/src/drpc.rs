use rpc::server::{RPCHandler, Server};
use rpc::rpc::{RPCError, RPCHeader, RPCType};
use rpc::transport::stdtcp::*;
use std::net::{TcpListener, TcpStream};

#[derive(Debug, Eq, PartialEq, PartialOrd, Clone, Copy)]
#[repr(u8)]
pub(crate) enum KernelRpc {
    /// Create a file
    Create = 0,
    /// Open a file
    Open = 1,
    /// Read from a file
    Read = 2,
    /// Read from a file from the given offset
    ReadAt = 3,
    /// Write to a file
    Write = 4,
    /// Write to a file
    WriteAt = 5,
    /// Close an opened file.
    Close = 6,
    /// Get the information related to the file.
    GetInfo = 7,
    /// Delete the file
    Delete = 8,
    /// Write to a file without going into NR.
    WriteDirect = 9,
    /// Rename a file.
    FileRename = 10,
    /// Create a directory.
    MkDir = 11,

    /// Log (print) message of a process.
    Log = 12,
    /// Allocate physical memory for a process.
    AllocatePhysical = 13,
    /// Release physical memory from a process.
    ReleasePhysical = 14,
    /// Allocate a core for a process
    RequestCore = 15,
    /// Release a core from a process.
    ReleaseCore = 16,

    /// Get the hardware threads for the rack
    GetHardwareThreads = 17,

    /// send process logs reference to client
    GetShmemStructure = 18,

    /// request shmem frames
    GetShmemFrames = 19,
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

pub(crate) const OPEN_HANDLER:   RPCHandler = handle_open;
pub(crate) const READ_HANDLER:   RPCHandler = handle_read;
pub(crate) const PREAD_HANDLER:  RPCHandler = handle_pread;
pub(crate) const WRITE_HANDLER:  RPCHandler = handle_write;
pub(crate) const PWRITE_HANDLER: RPCHandler = handle_pwrite;
pub(crate) const CLOSE_HANDLER:  RPCHandler = handle_close;
pub(crate) const REMOVE_HANDLER: RPCHandler = handle_remove;
pub(crate) const FSYNC_HANDLER:  RPCHandler = handle_fsync;
pub(crate) const MKDIR_HANDLER:  RPCHandler = handle_mkdir;
pub(crate) const RMDIR_HANDLER:  RPCHandler = handle_rmdir;

fn register_rpcs(server: &mut Server) {
    server.register(KernelRpc::Open as RPCType, &OPEN_HANDLER).unwrap();
    server.register(KernelRpc::Read as RPCType, &READ_HANDLER).unwrap();
    server.register(KernelRpc::ReadAt as RPCType, &PREAD_HANDLER).unwrap();
    server.register(KernelRpc::Write as RPCType, &WRITE_HANDLER).unwrap();
    server.register(KernelRpc::WriteAt as RPCType, &PWRITE_HANDLER).unwrap();
    server.register(KernelRpc::Close as RPCType, &CLOSE_HANDLER).unwrap();
    server.register(KernelRpc::Delete as RPCType, &REMOVE_HANDLER).unwrap();
    server.register(KernelRpc::MkDir as RPCType, &MKDIR_HANDLER).unwrap();
}

fn server_from_stream(stream: TcpStream) {
    let mut server = Server::new(Box::new(stream));
    register_rpcs(&mut server);
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
