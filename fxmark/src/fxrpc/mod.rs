pub mod grpc;
pub use crate::fxrpc::grpc::*;
pub mod drpc;
pub use crate::fxrpc::drpc::*;

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

#[allow(non_camel_case_types)]
#[derive(Clone, Copy)]
pub enum ConnType {
    TCP_LOCAL,
    TCP_REMOTE,
    UDS,
}

#[derive(Clone)]
pub struct ClientParams {
    pub cid: usize,
    pub nclients: usize,
    pub ccores: usize,
    pub log_mode: LogMode,
    pub conn_type: ConnType,
}

pub(crate) trait FxRPC {
    fn rpc_open(&mut self, path: &str, flags: i32, mode: u32) -> Result<i32, Box<dyn std::error::Error>>;
    fn rpc_read(&mut self, fd: i32, page: &mut Vec<u8>, size: usize) -> Result<i32, Box<dyn std::error::Error>>;
    fn rpc_pread(
        &mut self,
        fd: i32,
        page: &mut Vec<u8>,
        size: usize,
        offset: i64,
    ) -> Result<i32, Box<dyn std::error::Error>>;
    fn rpc_write(&mut self, fd: i32, page: &Vec<u8>, size: usize) -> Result<i32, Box<dyn std::error::Error>>;
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
