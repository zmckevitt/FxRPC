use libc::*;
use syscall::{OpenRequest, ReadRequest, WriteRequest, CloseRequest, RemoveRequest,
              syscall_client::SyscallClient};

const PAGE_SIZE: usize = 1024;

pub mod syscall {
    tonic::include_proto!("syscalls");
}

async fn grpc_open(path: &str, flags: i32) -> Result<i32, Box<dyn std::error::Error>> {
    let mut client = SyscallClient::connect("http://[::1]:8080").await?;
    let request = tonic::Request::new(OpenRequest {
        path: path.to_string(),
        flags: flags,
    });
    let response = client.open(request).await?.into_inner();
    Ok(response.result)
}

async fn grpc_read(fd: i32, page: &mut Vec<u8>) -> Result<i32, Box<dyn std::error::Error>> {
    let mut client = SyscallClient::connect("http://[::1]:8080").await?; 
    let request = tonic::Request::new(ReadRequest {
        fd: fd,
    });

    let response = client.read(request).await?.into_inner();
    *page = response.page;
    Ok(response.result)
}

async fn grpc_write(fd: i32, page: &Vec<u8>) -> Result<i32, Box<dyn std::error::Error>> {
    let mut client = SyscallClient::connect("http://[::1]:8080").await?;
    let request = tonic::Request::new(WriteRequest {
        fd: fd,
        page: page.to_vec(),
    });

    let response = client.write(request).await?.into_inner();
    Ok(response.result)
}

async fn grpc_close(fd: i32) -> Result<i32, Box<dyn std::error::Error>> {
    let mut client = SyscallClient::connect("http://[::1]:8080").await?; 
    let request = tonic::Request::new(CloseRequest {
        fd: fd,
    });

    let response = client.close(request).await?.into_inner();
    Ok(response.result)
}

async fn grpc_remove(path: &str) -> Result<i32, Box<dyn std::error::Error>> {
    let mut client = SyscallClient::connect("http://[::1]:8080").await?;
    let request = tonic::Request::new(RemoveRequest {
        path: path.to_string(),
    });
    let response = client.remove(request).await?.into_inner();
    Ok(response.result)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let filename = "files/read_test.txt";
    let fd = grpc_open(filename, O_CREAT | O_RDWR).await?;
    println!("ReadTest: open request returned file descriptor: {}", fd);

    assert!(fd != -1);

    let page: &mut [u8; PAGE_SIZE] = &mut [0; PAGE_SIZE];
    let result = grpc_read(fd, &mut page.to_vec()).await?;
 
    // Length of test in files/read_test.txt
    assert!(result == 13);

    println!("ReadTest: request returned the following data: {:?}", String::from_utf8(page.to_vec())); 

    let result = grpc_close(fd).await?;    

    assert!(result != -1);

    Ok(())
}
