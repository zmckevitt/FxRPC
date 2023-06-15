/*
    Library for gRPC system call clients.
    Zack McKevitt - 2023
*/

use syscalls::{OpenRequest, ReadRequest, WriteRequest, CloseRequest, RemoveRequest,
              syscall_client::SyscallClient};
use tokio::runtime::{Builder, Runtime};

type StdError = Box<dyn std::error::Error + Send + Sync + 'static>;
type Result<T, E = StdError> = ::std::result::Result<T, E>;

pub mod syscalls {
    tonic::include_proto!("syscalls");
}

pub struct BlockingClient {
    client: SyscallClient<tonic::transport::Channel>,
    rt: Runtime,
}

impl BlockingClient {
    
    pub fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
    where
        D: TryInto<tonic::transport::Endpoint>,
        D::Error: Into<StdError>,
    {
        let rt = Builder::new_multi_thread().enable_all().build().unwrap();
        let client = rt.block_on(SyscallClient::connect(dst))?;

        Ok(Self { client, rt })
    }

    pub fn grpc_open(&mut self, path: &str, flags: i32) -> Result<i32, Box<dyn std::error::Error>> {
        let request = tonic::Request::new(OpenRequest {
            path: path.to_string(),
            flags: flags,
        });
        let response = self.rt.block_on(self.client.open(request))?.into_inner();
        Ok(response.result)
    }
    
    pub fn grpc_read(&mut self, fd: i32, page: &mut Vec<u8>) -> Result<i32, Box<dyn std::error::Error>> {
        let request = tonic::Request::new(ReadRequest {
            fd: fd,
        });
    
        let response = self.rt.block_on(self.client.read(request))?.into_inner();
        *page = response.page;
        Ok(response.result)
    }
    
    pub fn grpc_write(&mut self, fd: i32, page: &Vec<u8>) -> Result<i32, Box<dyn std::error::Error>> {
        let request = tonic::Request::new(WriteRequest {
            fd: fd,
            page: page.to_vec(),
        });
    
        let response = self.rt.block_on(self.client.write(request))?.into_inner();
        Ok(response.result)
    }
    
    pub fn grpc_close(&mut self, fd: i32) -> Result<i32, Box<dyn std::error::Error>> {
        let request = tonic::Request::new(CloseRequest {
            fd: fd,
        });
    
        let response = self.rt.block_on(self.client.close(request))?.into_inner();
        Ok(response.result)
    }
    
    pub fn grpc_remove(&mut self, path: &str) -> Result<i32, Box<dyn std::error::Error>> {
        let request = tonic::Request::new(RemoveRequest {
            path: path.to_string(),
        });
        let response = self.rt.block_on(self.client.remove(request))?.into_inner();
        Ok(response.result)
    }
}
