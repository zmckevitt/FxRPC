/*
    Library for gRPC system call clients.
    Zack McKevitt - 2023
*/

use syscalls::{OpenRequest, ReadRequest, WriteRequest, CloseRequest, RemoveRequest,
              syscall_client::SyscallClient};

pub mod syscalls {
    tonic::include_proto!("syscalls");
}

pub async fn grpc_open(path: &str, flags: i32) -> Result<i32, Box<dyn std::error::Error>> {
    let mut client = SyscallClient::connect("http://[::1]:8080").await?;
    let request = tonic::Request::new(OpenRequest {
        path: path.to_string(),
        flags: flags,
    });
    let response = client.open(request).await?.into_inner();
    Ok(response.result)
}

pub async fn grpc_read(fd: i32, page: &mut Vec<u8>) -> Result<i32, Box<dyn std::error::Error>> {
    let mut client = SyscallClient::connect("http://[::1]:8080").await?;
    let request = tonic::Request::new(ReadRequest {
        fd: fd,
    });

    let response = client.read(request).await?.into_inner();
    *page = response.page;
    Ok(response.result)
}

pub async fn grpc_write(fd: i32, page: &Vec<u8>) -> Result<i32, Box<dyn std::error::Error>> {
    let mut client = SyscallClient::connect("http://[::1]:8080").await?;
    let request = tonic::Request::new(WriteRequest {
        fd: fd,
        page: page.to_vec(),
    });

    let response = client.write(request).await?.into_inner();
    Ok(response.result)
}

pub async fn grpc_close(fd: i32) -> Result<i32, Box<dyn std::error::Error>> {
    let mut client = SyscallClient::connect("http://[::1]:8080").await?;
    let request = tonic::Request::new(CloseRequest {
        fd: fd,
    });

    let response = client.close(request).await?.into_inner();
    Ok(response.result)
}

pub async fn grpc_remove(path: &str) -> Result<i32, Box<dyn std::error::Error>> {
    let mut client = SyscallClient::connect("http://[::1]:8080").await?;
    let request = tonic::Request::new(RemoveRequest {
        path: path.to_string(),
    });
    let response = client.remove(request).await?.into_inner();
    Ok(response.result)
}
