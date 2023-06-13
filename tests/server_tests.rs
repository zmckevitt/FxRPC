use libc::{O_CREAT, O_RDWR};
use fxmark_grpc::*;

const PAGE_SIZE: usize = 1024;


#[tokio::test]
async fn read_test() -> Result<(), Box<dyn std::error::Error>> {
    let filename = "read_test.txt";
    let fd = grpc_open(filename, O_CREAT | O_RDWR).await?;
    assert!(fd != -1);
    println!("ReadTest: open request returned file descriptor: {}", fd);

    let page: &mut [u8; PAGE_SIZE] = &mut [0; PAGE_SIZE];
    let result = grpc_read(fd, &mut page.to_vec()).await?;
    assert!(result != -1);

    let page_str = String::from_utf8(page.to_vec()).unwrap();
    println!("ReadTest: read request returned the following data: {:?}", page_str); 

    let result = grpc_close(fd).await?; 
    assert!(result != -1);

    Ok(())
}

#[tokio::test]
async fn write_test() -> Result<(), Box<dyn std::error::Error>> {
    let filename = "write_test.txt";
    let fd = grpc_open(filename, O_CREAT | O_RDWR).await?;
    assert!(fd != -1);
    println!("WriteTest: open request returned file descriptor: {}", fd);

    let page = "WriteTest".as_bytes();
    let result = grpc_write(fd, &page.to_vec()).await?;
    
    // Length of test in files/read_test.txt
    assert!(result != -1);

    let result = grpc_close(fd).await?;
    assert!(result != -1);

    let result = grpc_remove(filename).await?;
    assert!(result != -1);

    Ok(())
}

#[tokio::test]
async fn write_read_test() -> Result<(), Box<dyn std::error::Error>> {
    let filename = "write_read_test.txt";
    let fd = grpc_open(filename, O_CREAT | O_RDWR).await?;
    assert!(fd != -1);
    println!("WriteReadTest: open request returned file descriptor: {}", fd);

    let page = "WriteReadTest".as_bytes();
    let result = grpc_write(fd, &page.to_vec()).await?;
    assert!(result != -1);

    let result = grpc_read(fd, &mut page.to_vec()).await?;
    assert!(result != -1);

    let page_str = String::from_utf8(page.to_vec())
        .expect("WriteReadTest: Could not convert read to string");
    assert!(page_str == "WriteReadTest");
    println!("WriteReadTest: read request returned the following data: {:?}",
        page_str);
    
    let result = grpc_close(fd).await?;
    assert!(result != -1);

    let result = grpc_remove(filename).await?;
    assert!(result != -1);

    Ok(())
}
