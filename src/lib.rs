/*
    Library for gRPC system call clients.
    Zack McKevitt - 2023
*/

use syscalls::{OpenRequest, ReadRequest, WriteRequest, CloseRequest, RemoveRequest, FsyncRequest,
              syscall_client::SyscallClient};
use tokio::runtime::Builder;

type StdError = Box<dyn std::error::Error + Send + Sync + 'static>;
type Result<T, E = StdError> = ::std::result::Result<T, E>;

pub mod syscalls {
    tonic::include_proto!("syscalls");
}

// pub struct BlockingClient {
//     client: SyscallClient<tonic::transport::Channel>,
//     rt: Runtime,
// }
// 
// impl BlockingClient {
//     
//     pub fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
//     where
//         D: TryInto<tonic::transport::Endpoint>,
//         D::Error: Into<StdError>,
//     {
//         let rt = Builder::new_multi_thread().enable_all().build().unwrap();
//         let client = rt.block_on(SyscallClient::connect(dst))?;
// 
//         Ok(Self { client, rt })
//     }
    
pub fn grpc_open(path: &str, flags: i32, mode: u32) -> Result<i32, Box<dyn std::error::Error>> {
    let rt = Builder::new_multi_thread().enable_all().build().unwrap();
    let mut client = rt.block_on(SyscallClient::connect("http://[::1]:8080"))?;
    let request = tonic::Request::new(OpenRequest {
        path: path.to_string(),
        flags: flags,
        mode: mode,
    });
    let response = rt.block_on(client.open(request))?.into_inner();
    Ok(response.result)
}

pub fn grpc_read_base(pread: bool, fd: i32, page: &mut Vec<u8>, size: usize, offset: i64) 
    -> Result<i32, Box<dyn std::error::Error>> {

    let rt = Builder::new_multi_thread().enable_all().build().unwrap();
    let mut client = rt.block_on(SyscallClient::connect("http://[::1]:8080"))?;
    let request = tonic::Request::new(ReadRequest {
        pread: pread,
        fd: fd,
        size: size as u32,
        offset: offset,
    });

    let response = rt.block_on(client.read(request))?.into_inner();
    *page = response.page;
    Ok(response.result)
}

pub fn grpc_read(fd: i32, page: &mut Vec<u8>, size: usize) 
    -> Result<i32, Box<dyn std::error::Error>> {
    grpc_read_base(false, fd, page, size, 0)
}

pub fn grpc_pread(fd: i32, page: &mut Vec<u8>, size: usize, offset: i64) 
    -> Result<i32, Box<dyn std::error::Error>> {
    grpc_read_base(true, fd, page, size, offset)
}

pub fn grpc_write_base(pwrite: bool, fd: i32, page: &Vec<u8>, len: usize, offset: i64) 
    -> Result<i32, Box<dyn std::error::Error>> {

    let rt = Builder::new_multi_thread().enable_all().build().unwrap();
    let mut client = rt.block_on(SyscallClient::connect("http://[::1]:8080"))?;
    let request = tonic::Request::new(WriteRequest {
        pwrite: pwrite,
        fd: fd,
        page: page.to_vec(),
        len: len as u32,
        offset: offset,
    });

    let response = rt.block_on(client.write(request))?.into_inner();
    Ok(response.result)
}

pub fn grpc_write(fd: i32, page: &Vec<u8>, size: usize)
    -> Result<i32, Box<dyn std::error::Error>> {
    grpc_write_base(false, fd, page, size, 0)
}

pub fn grpc_pwrite(fd: i32, page: &Vec<u8>, size: usize, offset: i64)
    -> Result<i32, Box<dyn std::error::Error>> {
    grpc_write_base(true, fd, page, size, offset)
}

pub fn grpc_close(fd: i32) -> Result<i32, Box<dyn std::error::Error>> {
    let rt = Builder::new_multi_thread().enable_all().build().unwrap();
    let mut client = rt.block_on(SyscallClient::connect("http://[::1]:8080"))?;
    let request = tonic::Request::new(CloseRequest {
        fd: fd,
    });

    let response = rt.block_on(client.close(request))?.into_inner();
    Ok(response.result)
}

pub fn grpc_remove(path: &str) -> Result<i32, Box<dyn std::error::Error>> {
    let rt = Builder::new_multi_thread().enable_all().build().unwrap();
    let mut client = rt.block_on(SyscallClient::connect("http://[::1]:8080"))?;
    let request = tonic::Request::new(RemoveRequest {
        path: path.to_string(),
    });
    let response = rt.block_on(client.remove(request))?.into_inner();
    Ok(response.result)
}

pub fn grpc_fsync(fd: i32) -> Result<i32, Box<dyn std::error::Error>> {
    let rt = Builder::new_multi_thread().enable_all().build().unwrap();
    let mut client = rt.block_on(SyscallClient::connect("http://[::1]:8080"))?;
    let request = tonic::Request::new(FsyncRequest {
        fd: fd,
    });

    let response = rt.block_on(client.fsync(request))?.into_inner();
    Ok(response.result)
}
// }
