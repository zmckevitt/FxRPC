use log::debug;
use rpc::client::Client;
use rpc::rpc::*;
use rpc::transport::stdtcp::*;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

use abomonation::{decode, encode};

use crate::fxrpc::drpc::*;
use crate::fxrpc::FxRPC;
use crate::fxrpc::PAGE_SIZE;

////////////////////////////////// CLIENT //////////////////////////////////

fn decode_response(payload: &mut [u8]) -> (i32, usize, Vec<u8>) {
    match unsafe { decode::<Response>(payload) } {
        Some((req, _)) => (req.result, req.size, req.page.clone()),
        None => panic!("Cannot decode response!"),
    }
}

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
        let mut data_out = [0u8; std::mem::size_of::<Response>()];

        match self.call(DRPC::Open as RPCType, &[&bytes], &mut [&mut data_out]) {
            Ok(_) => {
                let (result, size, page) = decode_response(&mut data_out);
                debug!(
                    "Received - result: {:?}, size: {:?}, page: {:?}",
                    result, size, page
                );
                Ok(result)
            }
            Err(_) => Err(Box::from("Open RPC failed")),
        }
    }

    fn rpc_read(
        &mut self,
        fd: i32,
        page: &mut Vec<u8>,
        size: usize,
    ) -> Result<i32, Box<dyn std::error::Error>> {
        let request = ReadReq {
            fd: fd,
            size: size,
            offset: 0,
        };

        let mut bytes = Vec::new();
        unsafe { encode(&request, &mut bytes) }.expect("Failed to encode open request");

        // probably a bit conservative
        let mut data_out = [0u8; 2 * PAGE_SIZE];

        match self.call(DRPC::Read as RPCType, &[&bytes], &mut [&mut data_out]) {
            Ok(_) => {
                let (result, size, ret_page) = decode_response(&mut data_out);
                debug!(
                    "Received - result: {:?}, size: {:?}, page: {:?}",
                    result, size, ret_page
                );
                *page = ret_page;

                Ok(result)
            }
            Err(_) => Err(Box::from("Read RPC failed")),
        }
    }

    fn rpc_pread(
        &mut self,
        fd: i32,
        page: &mut Vec<u8>,
        size: usize,
        offset: i64,
    ) -> Result<i32, Box<dyn std::error::Error>> {
        let request = ReadReq {
            fd: fd,
            size: size,
            offset: offset,
        };

        let mut bytes = Vec::new();
        unsafe { encode(&request, &mut bytes) }.expect("Failed to encode open request");

        // probably a bit conservative
        let mut data_out = [0u8; 2 * PAGE_SIZE];

        match self.call(DRPC::PRead as RPCType, &[&bytes], &mut [&mut data_out]) {
            Ok(_) => {
                let (result, size, ret_page) = decode_response(&mut data_out);
                debug!(
                    "Received - result: {:?}, size: {:?}, page: {:?}",
                    result, size, ret_page
                );
                *page = ret_page;

                Ok(result)
            }
            Err(_) => Err(Box::from("PRead RPC failed")),
        }
    }

    fn rpc_write(
        &mut self,
        fd: i32,
        page: &Vec<u8>,
        size: usize,
    ) -> Result<i32, Box<dyn std::error::Error>> {
        let request = WriteReq {
            fd: fd,
            page: page.to_vec(),
            size: size,
            offset: 0,
        };

        let mut bytes = Vec::new();
        unsafe { encode(&request, &mut bytes) }.expect("Failed to encode open request");
        let mut data_out = [0u8; std::mem::size_of::<Response>()];

        match self.call(DRPC::Write as RPCType, &[&bytes], &mut [&mut data_out]) {
            Ok(_) => {
                let (result, size, page) = decode_response(&mut data_out);
                debug!(
                    "Received - result: {:?}, size: {:?}, page: {:?}",
                    result, size, page
                );

                Ok(result)
            }
            Err(_) => Err(Box::from("Write RPC failed")),
        }
    }

    fn rpc_pwrite(
        &mut self,
        fd: i32,
        page: &Vec<u8>,
        size: usize,
        offset: i64,
    ) -> Result<i32, Box<dyn std::error::Error>> {
        let request = WriteReq {
            fd: fd,
            page: page.to_vec(),
            size: size,
            offset: offset,
        };

        let mut bytes = Vec::new();
        unsafe { encode(&request, &mut bytes) }.expect("Failed to encode open request");
        let mut data_out = [0u8; std::mem::size_of::<Response>()];

        match self.call(DRPC::PWrite as RPCType, &[&bytes], &mut [&mut data_out]) {
            Ok(_) => {
                let (result, size, page) = decode_response(&mut data_out);
                debug!(
                    "Received - result: {:?}, size: {:?}, page: {:?}",
                    result, size, page
                );

                Ok(result)
            }
            Err(_) => Err(Box::from("PWrite RPC failed")),
        }
    }

    fn rpc_close(&mut self, fd: i32) -> Result<i32, Box<dyn std::error::Error>> {
        let request = CloseReq { fd: fd };

        let mut bytes = Vec::new();
        unsafe { encode(&request, &mut bytes) }.expect("Failed to encode open request");
        let mut data_out = [0u8; std::mem::size_of::<Response>()];

        match self.call(DRPC::Close as RPCType, &[&bytes], &mut [&mut data_out]) {
            Ok(_) => {
                let (result, size, page) = decode_response(&mut data_out);
                debug!(
                    "Received - result: {:?}, size: {:?}, page: {:?}",
                    result, size, page
                );

                Ok(result)
            }
            Err(_) => Err(Box::from("Close RPC failed")),
        }
    }

    fn rpc_remove(&mut self, path: &str) -> Result<i32, Box<dyn std::error::Error>> {
        let request = RemoveReq {
            path: path.as_bytes().to_vec(),
        };

        let mut bytes = Vec::new();
        unsafe { encode(&request, &mut bytes) }.expect("Failed to encode open request");
        let mut data_out = [0u8; std::mem::size_of::<Response>()];

        match self.call(DRPC::Remove as RPCType, &[&bytes], &mut [&mut data_out]) {
            Ok(_) => {
                let (result, size, page) = decode_response(&mut data_out);
                debug!(
                    "Received - result: {:?}, size: {:?}, page: {:?}",
                    result, size, page
                );

                Ok(result)
            }
            Err(_) => Err(Box::from("Remove RPC failed")),
        }
    }

    fn rpc_mkdir(&mut self, path: &str, mode: u32) -> Result<i32, Box<dyn std::error::Error>> {
        let request = MkdirReq {
            path: path.as_bytes().to_vec(),
            mode: mode,
        };

        let mut bytes = Vec::new();
        unsafe { encode(&request, &mut bytes) }.expect("Failed to encode open request");
        let mut data_out = [0u8; std::mem::size_of::<Response>()];

        match self.call(DRPC::MkDir as RPCType, &[&bytes], &mut [&mut data_out]) {
            Ok(_) => {
                let (result, size, page) = decode_response(&mut data_out);
                debug!(
                    "Received - result: {:?}, size: {:?}, page: {:?}",
                    result, size, page
                );

                Ok(result)
            }
            Err(_) => Err(Box::from("Mkdir RPC failed")),
        }
    }

    fn rpc_rmdir(&mut self, path: &str) -> Result<i32, Box<dyn std::error::Error>> {
        let request = RemoveReq {
            path: path.as_bytes().to_vec(),
        };

        let mut bytes = Vec::new();
        unsafe { encode(&request, &mut bytes) }.expect("Failed to encode open request");
        let mut data_out = [0u8; std::mem::size_of::<Response>()];

        match self.call(DRPC::RmDir as RPCType, &[&bytes], &mut [&mut data_out]) {
            Ok(_) => {
                let (result, size, page) = decode_response(&mut data_out);
                debug!(
                    "Received - result: {:?}, size: {:?}, page: {:?}",
                    result, size, page
                );

                Ok(result)
            }
            Err(_) => Err(Box::from("Rmdir RPC failed")),
        }
    }
}

// TODO: allow for various transpots/bind locations
pub fn init_client_drpc(bind_addr: &str) -> Client {
    // TODO: make parameters for this, maybe wrap this function or
    // leverage the ConnType enum to distinguish tcp/uds?
    let stream = TcpStream::connect(bind_addr).unwrap();
    let transport = StdTCP {
        stream: Arc::new(Mutex::new(stream)),
    };
    let client = Client::new(Box::new(transport));
    client
}
