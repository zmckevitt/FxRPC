pub mod drpc;
pub mod grpc;
pub use crate::fxrpc::drpc::*;
pub use crate::fxrpc::grpc::*;

pub use crate::fxmark::PAGE_SIZE;

type StdError = Box<dyn std::error::Error + Send + Sync + 'static>;
type Result<T, E = StdError> = ::std::result::Result<T, E>;

// File system path
pub const FS_PATH: &str = "/dev/shm/";
pub const UDS_PATH: &str = "/dev/shm/uds";

#[derive(Clone, Copy, PartialEq)]
pub enum LogMode {
    CSV,
    STDOUT,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ConnType {
    TcpLocal,
    TcpRemote,
    UDS,
}

#[derive(Clone, Copy)]
pub enum RPCType {
    GRPC,
    DRPC,
}

#[derive(Clone)]
pub struct ClientParams {
    pub cid: usize,
    pub nclients: usize,
    pub ccores: usize,
    pub log_mode: LogMode,
    pub conn_type: ConnType,
    pub rpc_type: RPCType,
}

pub(crate) trait FxRPC {
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

pub fn run_server(conn_type: ConnType, rpc_type: RPCType) {
    match rpc_type {
        RPCType::GRPC => match conn_type {
            ConnType::TcpLocal => start_rpc_server_tcp("[::1]", 8080),
            ConnType::TcpRemote => start_rpc_server_tcp("172.31.0.1", 8080),
            ConnType::UDS => start_rpc_server_uds(UDS_PATH).unwrap(),
        },
        RPCType::DRPC => match conn_type {
            ConnType::TcpLocal => start_drpc_server_tcp("127.0.0.1", 8080),
            ConnType::TcpRemote => start_drpc_server_tcp("172.31.0.1", 8080),
            ConnType::UDS => start_drpc_server_uds(),
        },
    };
}
