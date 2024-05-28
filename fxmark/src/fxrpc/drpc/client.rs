use rpc::client::Client;
use rpc::rpc::*;
use rpc::transport::stdtcp::*;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

use abomonation::{decode, encode};

use crate::fxrpc::drpc::*;
use crate::fxrpc::FxRPC;

////////////////////////////////// CLIENT //////////////////////////////////

// TODO: ERROR HANDLING

impl FxRPC for Client {
    fn rpc_open(
        &mut self,
        path: &str,
        flags: i32,
        mode: u32,
    ) -> Result<i32, Box<dyn std::error::Error>> {
        let request = OpenReq {
            path: path.as_bytes().to_vec(),
            flags: flags,
            mode: mode,
        };

        let mut bytes = Vec::new();
        unsafe { encode(&request, &mut bytes) }.expect("Failed to encode open request");
        let mut data_out = [0u8; 1];

        self.call(DRPC::Open as RPCType, &[&bytes], &mut [&mut data_out]);
        Ok(0)
    }

    fn rpc_read(
        &mut self,
        fd: i32,
        page: &mut Vec<u8>,
        size: usize,
    ) -> Result<i32, Box<dyn std::error::Error>> {
        let data_in = [0u8; 32];
        let mut data_out = [0u8; 32];
        self.call(DRPC::Read as RPCType, &[&data_in], &mut [&mut data_out]);
        Ok(0)
    }

    fn rpc_pread(
        &mut self,
        fd: i32,
        page: &mut Vec<u8>,
        size: usize,
        offset: i64,
    ) -> Result<i32, Box<dyn std::error::Error>> {
        let data_in = [0u8; 32];
        let mut data_out = [0u8; 32];
        self.call(DRPC::PRead as RPCType, &[&data_in], &mut [&mut data_out]);
        Ok(0)
    }

    fn rpc_write(
        &mut self,
        fd: i32,
        page: &Vec<u8>,
        size: usize,
    ) -> Result<i32, Box<dyn std::error::Error>> {
        let data_in = [0u8; 32];
        let mut data_out = [0u8; 32];
        self.call(DRPC::Write as RPCType, &[&data_in], &mut [&mut data_out]);
        Ok(0)
    }

    fn rpc_pwrite(
        &mut self,
        fd: i32,
        page: &Vec<u8>,
        size: usize,
        offset: i64,
    ) -> Result<i32, Box<dyn std::error::Error>> {
        let data_in = [0u8; 32];
        let mut data_out = [0u8; 32];
        self.call(DRPC::PWrite as RPCType, &[&data_in], &mut [&mut data_out]);
        Ok(0)
    }

    fn rpc_close(&mut self, fd: i32) -> Result<i32, Box<dyn std::error::Error>> {
        let data_in = [0u8; 32];
        let mut data_out = [0u8; 32];
        self.call(DRPC::Close as RPCType, &[&data_in], &mut [&mut data_out]);
        Ok(0)
    }

    fn rpc_remove(&mut self, path: &str) -> Result<i32, Box<dyn std::error::Error>> {
        let data_in = [0u8; 32];
        let mut data_out = [0u8; 32];
        self.call(DRPC::Remove as RPCType, &[&data_in], &mut [&mut data_out]);
        Ok(0)
    }

    fn rpc_mkdir(&mut self, path: &str, mode: u32) -> Result<i32, Box<dyn std::error::Error>> {
        let data_in = [0u8; 32];
        let mut data_out = [0u8; 32];
        self.call(DRPC::MkDir as RPCType, &[&data_in], &mut [&mut data_out]);
        Ok(0)
    }

    fn rpc_rmdir(&mut self, path: &str) -> Result<i32, Box<dyn std::error::Error>> {
        let data_in = [0u8; 32];
        let mut data_out = [0u8; 32];
        self.call(DRPC::RmDir as RPCType, &[&data_in], &mut [&mut data_out]);
        Ok(0)
    }
}

// TODO: allow for various transpots/bind locations
pub fn init_client_drpc() -> Client {
    // TODO: make parameters for this, maybe wrap this function or
    // leverage the ConnType enum to distinguish tcp/uds?
    let stream = TcpStream::connect("127.0.0.1:8080").unwrap();
    let transport = StdTCP {
        stream: Arc::new(Mutex::new(stream)),
    };
    let mut client = Client::new(Box::new(transport));
    client
}
