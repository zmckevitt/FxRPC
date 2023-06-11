use libc::*;
use syscall::{OpenRequest, OpenResponse,
              ReadRequest, ReadResponse,
              WriteRequest, WriteResponse,
              CloseRequest, CloseResponse,
              syscall_client::SyscallClient};

pub mod syscall {
    tonic::include_proto!("syscalls");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = SyscallClient::connect("http://[::1]:8080").await?;
    let filename = String::from("files/read_test.txt");
    let request = tonic::Request::new(OpenRequest {
        path: filename,
        flags: O_CREAT | O_RDWR,
    });
    let response = client.open(request).await?.into_inner();
    let fd = response.result;
    println!("ReadTest: open request returned file descriptor: {}", fd);

    if fd == -1 {
        panic!("Open failed");
    }

    let request = tonic::Request::new(ReadRequest {
        fd: fd,
    });

    let response = client.read(request).await?.into_inner();
    let result = response.result;
    if result == -1 {
        panic!("pread error");
    } 
    let page = response.page;
    println!("ReadTest: request returned the following data: {:?}", String::from_utf8(page)); 

    Ok(())
}
