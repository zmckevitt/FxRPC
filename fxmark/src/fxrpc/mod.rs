pub mod drpc;
pub mod grpc;
use crate::fxrpc::drpc::*;
use crate::fxrpc::grpc::*;

pub use crate::fxmark::PAGE_SIZE;

type StdError = Box<dyn std::error::Error + Send + Sync + 'static>;
type Result<T, E = StdError> = ::std::result::Result<T, E>;

// File system path
pub const FS_PATH: &str = "/dev/shm/";
pub const UDS_PATH: &str = "/dev/shm/uds";

#[derive(Clone, Copy, PartialEq)]
#[repr(C)]
pub enum LogMode {
    CSV,
    STDOUT,
}

#[derive(Clone, Copy, PartialEq)]
#[repr(C)]
pub enum ConnType {
    TcpLocal,
    TcpRemote,
    UDS,
}

impl std::fmt::Display for ConnType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ConnType::TcpLocal => write!(f, "tcplocal"),
            ConnType::TcpRemote => write!(f, "tcpremote"),
            ConnType::UDS => write!(f, "uds"),
        }
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub enum RPCType {
    DRPC,
    GRPC,
}

impl std::fmt::Display for RPCType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            RPCType::DRPC => write!(f, "drpc"),
            RPCType::GRPC => write!(f, "grpc"),
        }
    }
}

#[derive(Clone)]
#[repr(C)]
pub struct ClientParams {
    pub cid: usize,
    pub nclients: usize,
    pub ccores: usize,
    pub log_mode: LogMode,
    pub conn_type: ConnType,
    pub rpc_type: RPCType,
}

pub trait FxRPC {
    fn rpc_open(
        &mut self,
        path: &str,
        flags: i32,
        mode: u32,
    ) -> Result<i32, Box<dyn std::error::Error>>;
    fn rpc_read(
        &mut self,
        fd: i32,
        page: &mut Vec<u8>,
        size: usize,
    ) -> Result<i32, Box<dyn std::error::Error>>;
    fn rpc_pread(
        &mut self,
        fd: i32,
        page: &mut Vec<u8>,
        size: usize,
        offset: i64,
    ) -> Result<i32, Box<dyn std::error::Error>>;
    fn rpc_write(
        &mut self,
        fd: i32,
        page: &Vec<u8>,
        size: usize,
    ) -> Result<i32, Box<dyn std::error::Error>>;
    fn rpc_pwrite(
        &mut self,
        fd: i32,
        page: &Vec<u8>,
        size: usize,
        offset: i64,
    ) -> Result<i32, Box<dyn std::error::Error>>;
    fn rpc_close(&mut self, fd: i32) -> Result<i32, Box<dyn std::error::Error>>;
    fn rpc_remove(&mut self, path: &str) -> Result<i32, Box<dyn std::error::Error>>;
    fn rpc_mkdir(&mut self, path: &str, mode: u32) -> Result<i32, Box<dyn std::error::Error>>;
    fn rpc_rmdir(&mut self, path: &str) -> Result<i32, Box<dyn std::error::Error>>;
}

pub fn init_client(conn_type: ConnType, rpc_type: RPCType) -> Box<dyn FxRPC> {
    match rpc_type {
        RPCType::GRPC => match conn_type {
            ConnType::TcpLocal => {
                Box::new(BlockingClient::connect_tcp("http://[::1]:8080").unwrap())
            }
            ConnType::TcpRemote => {
                Box::new(BlockingClient::connect_tcp("http://172.31.0.1:8080").unwrap())
            }
            ConnType::UDS => Box::new(BlockingClient::connect_uds().unwrap()),
        },
        RPCType::DRPC => match conn_type {
            ConnType::TcpLocal => Box::new(init_client_drpc()),
            ConnType::TcpRemote => Box::new(init_client_drpc()),
            ConnType::UDS => Box::new(init_client_drpc()),
        },
    }
}

pub fn run_server(conn_type: ConnType, rpc_type: RPCType, port: u16) {
    println!("Starting {} {} server", rpc_type, conn_type);
    match rpc_type {
        RPCType::GRPC => match conn_type {
            ConnType::TcpLocal => start_rpc_server_tcp("[::1]", port),
            ConnType::TcpRemote => start_rpc_server_tcp("172.31.0.1", port),
            ConnType::UDS => start_rpc_server_uds(UDS_PATH).unwrap(),
        },
        RPCType::DRPC => match conn_type {
            ConnType::TcpLocal => start_drpc_server_tcp("127.0.0.1", port),
            ConnType::TcpRemote => start_drpc_server_tcp("172.31.0.1", port),
            ConnType::UDS => start_drpc_server_uds(),
        },
    };
}
