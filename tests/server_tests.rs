use libc::{O_CREAT, O_RDWR};
use fxmark_grpc::*;

const PAGE_SIZE: usize = 1024;

#[test]
fn read_test() -> Result<(), Box<dyn std::error::Error>> {

    let mut client = BlockingClient::connect("http://[::1]:8080")?;

    let filename = "read_test.txt";
    let fd = client.grpc_open(filename, O_CREAT | O_RDWR).unwrap();
    assert!(fd != -1, "ReadTest: Open Failed");

    // let page: &mut [u8; PAGE_SIZE] = &mut [0; PAGE_SIZE];
    let mut page: Vec<u8> = vec![0; PAGE_SIZE];
    let result = client.grpc_read(fd, &mut page).unwrap();
    assert!(result != -1, "ReadTest: Read Failed");

    let binding = String::from_utf8(page).unwrap();
    let page_str = binding.trim_matches(char::from(0));
    assert!(page_str == "ReadTest\n", "ReadTest: read request returned the following data: {:?}", page_str); 

    let result = client.grpc_close(fd).unwrap(); 
    assert!(result != -1, "ReadTest: Close Failed");

    Ok(())
}

#[test]
fn write_test() -> Result<(), Box<dyn std::error::Error>> {

    let mut client = BlockingClient::connect("http://[::1]:8080")?;

    let filename = "write_test.txt";
    let fd = client.grpc_open(filename, O_CREAT | O_RDWR).unwrap();
    assert!(fd != -1, "WriteTest: Open Failed");

    let page = "WriteTest".as_bytes();
    let result = client.grpc_write(fd, &page.to_vec()).unwrap();
    
    // Length of test in files/read_test.txt
    assert!(result != -1, "WriteTest: Write Failed");

    let result = client.grpc_close(fd).unwrap();
    assert!(result != -1, "WriteTest: Close Failed");

    let result = client.grpc_remove(filename).unwrap();
    assert!(result != -1, "WriteTest: Remove Failed");

    Ok(())
}

#[test]
fn write_read_test() -> Result<(), Box<dyn std::error::Error>> {

    let mut client = BlockingClient::connect("http://[::1]:8080")?;
    
    let filename = "write_read_test.txt";
    let fd = client.grpc_open(filename, O_CREAT | O_RDWR).unwrap();
    assert!(fd != -1, "WriteReadTest: Open Failed");

    let page = "WriteReadTest".as_bytes();
    let result = client.grpc_write(fd, &page.to_vec()).unwrap();
    assert!(result != -1, "WriteReadTest: Write Failed");

    let mut page: Vec<u8> = vec![0; PAGE_SIZE];
    let result = client.grpc_read(fd, &mut page).unwrap();
    assert!(result != -1, "WriteReadTest: Read Failed");

    let binding = String::from_utf8(page).unwrap();
    let page_str = binding.trim_matches(char::from(0));
    assert!(page_str == "WriteReadTest", 
        "WriteReadTest: read request returned the following data: {:?}",
        page_str);
    
    let result = client.grpc_close(fd).unwrap();
    assert!(result != -1, "WriteReadTest: Close Failed");

    let result = client.grpc_remove(filename).unwrap();
    assert!(result != -1, "WriteReadTest: Remove Failed");

    Ok(())
} 
