/*
    Library for gRPC system call server and clients.
    Zack McKevitt - 2023
*/

use libc::*;
use syscalls::{
    syscall_client::SyscallClient,
    syscall_server::{Syscall, SyscallServer},
    CloseRequest, DirRequest, FstatRequest, FstatResponse, FsyncRequest, OpenRequest, ReadRequest,
    RemoveRequest, SyscallResponse, WriteRequest,
};
use tokio::net::{UnixListener, UnixStream};
use tokio::runtime::Builder;
use tokio::runtime::Runtime;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::{transport::Endpoint, transport::Server, transport::Uri, Request, Response, Status};
use tower::service_fn;

use std::os::unix::net::UnixListener as StdUnixListener;
use std::path::Path;

pub use crate::fxrpc::*;

//////////////////////////////////////// CLIENT ////////////////////////////////////////
pub struct BlockingClient {
    client: SyscallClient<tonic::transport::Channel>,
    rt: Option<Runtime>,
}

impl BlockingClient {
    pub fn connect_tcp<D>(dst: D) -> Result<Self, tonic::transport::Error>
    where
        D: TryInto<tonic::transport::Endpoint>,
        D::Error: Into<StdError>,
    {
        let rt = Builder::new_multi_thread().enable_all().build().unwrap();
        let client = rt.block_on(SyscallClient::connect(dst))?;

        Ok(Self {
            client,
            rt: Some(rt),
        })
    }

    pub fn connect_uds() -> Result<Self, tonic::transport::Error> {
        async fn connect_uds_async() -> tonic::transport::Channel {
            Endpoint::try_from("http://[::]:8080")
                .unwrap()
                .connect_with_connector(service_fn(|_: Uri| UnixStream::connect(UDS_PATH)))
                .await
                .unwrap()
        }

        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let channel = rt.block_on(connect_uds_async());
        let client = SyscallClient::new(channel);

        Ok(Self {
            client,
            rt: Some(rt),
        })
    }
}

impl FxRPC for BlockingClient {
    fn rpc_open(
        &mut self,
        path: &str,
        flags: i32,
        mode: u32,
    ) -> Result<i32, Box<dyn std::error::Error>> {
        let request = tonic::Request::new(OpenRequest {
            path: path.to_string(),
            flags: flags,
            mode: mode,
        });
        let response = self
            .rt
            .as_ref()
            .unwrap()
            .block_on(self.client.open(request))?
            .into_inner();
        Ok(response.result)
    }

    fn rpc_read(
        &mut self,
        fd: i32,
        page: &mut Vec<u8>,
        size: usize,
    ) -> Result<i32, Box<dyn std::error::Error>> {
        let request = tonic::Request::new(ReadRequest {
            pread: false,
            fd: fd,
            size: size as u32,
            offset: 0,
        });
        
        let response = self
            .rt
            .as_ref()
            .unwrap()
            .block_on(self.client.read(request))?
            .into_inner();
        *page = response.page;
        Ok(response.result)
    }

    fn rpc_pread(
        &mut self,
        fd: i32,
        page: &mut Vec<u8>,
        size: usize,
        offset: i64,
    ) -> Result<i32, Box<dyn std::error::Error>> {
        let request = tonic::Request::new(ReadRequest {
            pread: true,
            fd: fd,
            size: size as u32,
            offset: offset,
        });
        
        let response = self
            .rt
            .as_ref()
            .unwrap()
            .block_on(self.client.read(request))?
            .into_inner();
        *page = response.page;
        Ok(response.result)
    }

    fn rpc_write(
        &mut self,
        fd: i32,
        page: &Vec<u8>,
        size: usize,
    ) -> Result<i32, Box<dyn std::error::Error>> {
        let request = tonic::Request::new(WriteRequest {
            pwrite: false,
            fd: fd,
            page: page.to_vec(),
            len: size as u32,
            offset: 0,
        });

        let response = self
            .rt
            .as_ref()
            .unwrap()
            .block_on(self.client.write(request))?
            .into_inner();
        Ok(response.result)
    }

    fn rpc_pwrite(
        &mut self,
        fd: i32,
        page: &Vec<u8>,
        size: usize,
        offset: i64,
    ) -> Result<i32, Box<dyn std::error::Error>> {
        let request = tonic::Request::new(WriteRequest {
            pwrite: true,
            fd: fd,
            page: page.to_vec(),
            len: size as u32,
            offset: offset,
        });

        let response = self
            .rt
            .as_ref()
            .unwrap()
            .block_on(self.client.write(request))?
            .into_inner();
        Ok(response.result)
    }

    fn rpc_close(&mut self, fd: i32) -> Result<i32, Box<dyn std::error::Error>> {
        let request = tonic::Request::new(CloseRequest { fd: fd });

        let response = self
            .rt
            .as_ref()
            .unwrap()
            .block_on(self.client.close(request))?
            .into_inner();
        Ok(response.result)
    }

    fn rpc_remove(&mut self, path: &str) -> Result<i32, Box<dyn std::error::Error>> {
        let request = tonic::Request::new(RemoveRequest {
            path: path.to_string(),
        });
        let response = self
            .rt
            .as_ref()
            .unwrap()
            .block_on(self.client.remove(request))?
            .into_inner();
        Ok(response.result)
    }

    fn rpc_mkdir(&mut self, path: &str, mode: u32) -> Result<i32, Box<dyn std::error::Error>> {
        let request = tonic::Request::new(DirRequest {
            path: path.to_string(),
            mode: mode,
        });
        let response = self
            .rt
            .as_ref()
            .unwrap()
            .block_on(self.client.mkdir(request))?
            .into_inner();
        Ok(response.result)
    }

    fn rpc_rmdir(&mut self, path: &str) -> Result<i32, Box<dyn std::error::Error>> {
        let request = tonic::Request::new(DirRequest {
            path: path.to_string(),
            mode: 0,
        });
        let response = self
            .rt
            .as_ref()
            .unwrap()
            .block_on(self.client.rmdir(request))?
            .into_inner();
        Ok(response.result)
    }
}
