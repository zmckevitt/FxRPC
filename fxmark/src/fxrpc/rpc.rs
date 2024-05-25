use rpc::rpc::*;

pub(crate) trait FxRPC {
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
