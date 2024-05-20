use rpc::client::Client;
use rpc::rpc::*;
use rpc::server::{RPCHandler, Server};
use rpc::transport::stdtcp::*;
use std::net::{TcpListener, TcpStream};

////////////////////////////// FS RPC Hdrs  //////////////////////////////

#[derive(Debug, Eq, PartialEq, PartialOrd, Clone, Copy)]
#[repr(u8)]
pub(crate) enum DRPC {
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
    println!("Hello from RPC open!");
    Ok(())
}

fn handle_read(hdr: &mut RPCHeader, payload: &mut [u8]) -> Result<(), RPCError> {
    println!("Hello from RPC read!");
    Ok(())
}

fn handle_pread(hdr: &mut RPCHeader, payload: &mut [u8]) -> Result<(), RPCError> {
    println!("Hello from RPC pread!");
    Ok(())
}

fn handle_write(hdr: &mut RPCHeader, payload: &mut [u8]) -> Result<(), RPCError> {
    println!("Hello from RPC write!");
    Ok(())
}

fn handle_pwrite(hdr: &mut RPCHeader, payload: &mut [u8]) -> Result<(), RPCError> {
    println!("Hello from RPC pwrite!");
    Ok(())
}

fn handle_close(hdr: &mut RPCHeader, payload: &mut [u8]) -> Result<(), RPCError> {
    println!("Hello from RPC close!");
    Ok(())
}

fn handle_remove(hdr: &mut RPCHeader, payload: &mut [u8]) -> Result<(), RPCError> {
    println!("Hello from RPC remove!");
    Ok(())
}

fn handle_fsync(hdr: &mut RPCHeader, payload: &mut [u8]) -> Result<(), RPCError> {
    println!("Hello from RPC fsync!");
    Ok(())
}

fn handle_mkdir(hdr: &mut RPCHeader, payload: &mut [u8]) -> Result<(), RPCError> {
    println!("Hello from RPC mkdir!");
    Ok(())
}

fn handle_rmdir(hdr: &mut RPCHeader, payload: &mut [u8]) -> Result<(), RPCError> {
    println!("Hello from RPC rmdir!");
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
}

fn server_from_stream(stream: TcpStream) {
    let mut server = Server::new(Box::new(stream));
    register_rpcs(&mut server);
    // I dont think we need this, client registration in DiNOS
    // is usually for allocation of kernel resources (shmem and dcm)
    // server.add_client(&CLIENT_REGISTRAR);
    loop {
        match server.try_handle() {
            // Socket connection is broken
            Err(_) => break,
            _ => {}
        }
    }
}

pub fn start_drpc_server_tcp(bind_addr: &str, port: u64) {
    println!("Starting DRPC server on port {}", port);
    // TODO: bind to addr/port specified in parameters
    let listener = TcpListener::bind("127.0.0.1:8080").expect("Failed to create TCP transport");

    for stream in listener.incoming() {
        server_from_stream(stream.unwrap());
    }
}

pub fn start_drpc_server_uds() {}

////////////////////////////////// CLIENT //////////////////////////////////

pub trait FxRPC {
    fn rpc_open(&mut self, path: &str, flags: i32, mode: u32) -> Result<(), RPCError>;
    fn rpc_read(&mut self, fd: i32, page: &mut Vec<u8>, size: usize) -> Result<(), RPCError>;
    fn rpc_pread(
        &mut self,
        fd: i32,
        page: &mut Vec<u8>,
        size: usize,
        offset: i64,
    ) -> Result<(), RPCError>;
    fn rpc_write(&mut self, fd: i32, page: &Vec<u8>, size: usize) -> Result<(), RPCError>;
    fn rpc_pwrite(
        &mut self,
        fd: i32,
        page: &Vec<u8>,
        size: usize,
        offset: i64,
    ) -> Result<(), RPCError>;
    fn rpc_close(&mut self, fd: i32) -> Result<(), RPCError>;
    fn rpc_remove(&mut self, path: &str) -> Result<(), RPCError>;
    fn rpc_mkdir(&mut self, path: &str, mode: u32) -> Result<(), RPCError>;
}

impl FxRPC for Client {
    fn rpc_open(&mut self, path: &str, flags: i32, mode: u32) -> Result<(), RPCError> {
        let data_in = [0u8; 32];
        let mut data_out = [0u8; 32];
        self.call(DRPC::Open as RPCType, &[&data_in], &mut [&mut data_out])?;
        Ok(())
    }

    fn rpc_read(&mut self, fd: i32, page: &mut Vec<u8>, size: usize) -> Result<(), RPCError> {
        let data_in = [0u8; 32];
        let mut data_out = [0u8; 32];
        self.call(DRPC::Read as RPCType, &[&data_in], &mut [&mut data_out])?;
        Ok(())
    }

    fn rpc_pread(
        &mut self,
        fd: i32,
        page: &mut Vec<u8>,
        size: usize,
        offset: i64,
    ) -> Result<(), RPCError> {
        let data_in = [0u8; 32];
        let mut data_out = [0u8; 32];
        self.call(DRPC::PRead as RPCType, &[&data_in], &mut [&mut data_out])?;
        Ok(())
    }

    fn rpc_write(&mut self, fd: i32, page: &Vec<u8>, size: usize) -> Result<(), RPCError> {
        let data_in = [0u8; 32];
        let mut data_out = [0u8; 32];
        self.call(DRPC::Write as RPCType, &[&data_in], &mut [&mut data_out])?;
        Ok(())
    }

    fn rpc_pwrite(
        &mut self,
        fd: i32,
        page: &Vec<u8>,
        size: usize,
        offset: i64,
    ) -> Result<(), RPCError> {
        let data_in = [0u8; 32];
        let mut data_out = [0u8; 32];
        self.call(DRPC::PWrite as RPCType, &[&data_in], &mut [&mut data_out])?;
        Ok(())
    }

    fn rpc_close(&mut self, fd: i32) -> Result<(), RPCError> {
        let data_in = [0u8; 32];
        let mut data_out = [0u8; 32];
        self.call(DRPC::Close as RPCType, &[&data_in], &mut [&mut data_out])?;
        Ok(())
    }

    fn rpc_remove(&mut self, path: &str) -> Result<(), RPCError> {
        let data_in = [0u8; 32];
        let mut data_out = [0u8; 32];
        self.call(DRPC::Remove as RPCType, &[&data_in], &mut [&mut data_out])?;
        Ok(())
    }

    fn rpc_mkdir(&mut self, path: &str, mode: u32) -> Result<(), RPCError> {
        let data_in = [0u8; 32];
        let mut data_out = [0u8; 32];
        self.call(DRPC::MkDir as RPCType, &[&data_in], &mut [&mut data_out])?;
        Ok(())
    }
}

// TODO: allow for various transpots/bind locations
pub fn init_client() -> Client {
    // TODO: make parameters for this, maybe wrap this function or
    // leverage the ConnType enum to distinguish tcp/uds?
    let stream = TcpStream::connect("127.0.0.1:8080");
    let mut client = Client::new(Box::new(stream.unwrap()));
    client
}
