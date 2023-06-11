use libc::*;
use syscall::{OpenRequest, ReadRequest, WriteRequest, CloseRequest, RemoveRequest,
              syscall_client::SyscallClient};

pub mod syscall {
    tonic::include_proto!("syscalls");
}

#[tokio::test]
async fn read_test() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = SyscallClient::connect("http://[::1]:8080").await?;
    // let filename = String::from("files/read_test.txt");
    let filename = "files/read_test.txt";
    let request = tonic::Request::new(OpenRequest {
        path: filename.to_string(),
        flags: O_CREAT | O_RDWR,
    });
    let response = client.open(request).await?.into_inner();
    let fd = response.result;
    println!("ReadTest: open request returned file descriptor: {}", fd);

    assert!(fd != -1);

    let request = tonic::Request::new(ReadRequest {
        fd: fd,
    });

    let response = client.read(request).await?.into_inner();
    let result = response.result;
    
    // Length of test in files/read_test.txt
    assert!(result == 13);

    let page = response.page;
    println!("ReadTest: request returned the following data: {:?}", String::from_utf8(page)); 

    let request = tonic::Request::new(CloseRequest {
        fd: fd,
    });

    let response = client.close(request).await?.into_inner();
    let result = response.result;

    assert!(result != -1);

    Ok(())
}

#[tokio::test]
async fn write_test() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = SyscallClient::connect("http://[::1]:8080").await?;
    //let filename = String::from("files/write_test.txt");
    let filename = "files/write_test.txt";
    let request = tonic::Request::new(OpenRequest {
        path: filename.to_string(),
        flags: O_CREAT | O_RDWR,
    });
    let response = client.open(request).await?.into_inner();
    let fd = response.result;
    println!("WriteTest: open request returned file descriptor: {}", fd);

    assert!(fd != -1);

    let page = "write test".as_bytes();
    let request = tonic::Request::new(WriteRequest {
        fd: fd,
        page: page.to_vec(),
    });

    let response = client.write(request).await?.into_inner();
    let result = response.result;
    
    // Length of test in files/read_test.txt
    assert!(result != -1);

    let request = tonic::Request::new(CloseRequest {
        fd: fd,
    });

    let response = client.close(request).await?.into_inner();
    let result = response.result;

    assert!(result != -1);

    let request = tonic::Request::new(RemoveRequest {
        path: filename.to_string(),
    });

    let response = client.remove(request).await?.into_inner();
    let result = response.result;

    assert!(result != -1);

    Ok(())
}
