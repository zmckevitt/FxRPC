use fxmark_grpc::*;
use libc::{O_CREAT, O_RDWR, S_IRWXU};

const PAGE_SIZE: usize = 1024;

fn read_test_base(pread: bool) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = BlockingClient::connect_tcp("http://[::1]:8080")?;

    let test = if pread { "pReadTest" } else { "ReadTest" };

    let filename = "read_test.txt";
    let fd = client
        .grpc_open_tcp(filename, O_CREAT | O_RDWR, S_IRWXU)
        .unwrap();
    assert!(fd != -1, "{}: Open Failed", test);

    // let page: &mut [u8; PAGE_SIZE] = &mut [0; PAGE_SIZE];
    let mut page: Vec<u8> = vec![0; PAGE_SIZE];
    let result = if pread {
        client.grpc_pread_tcp(fd, &mut page, PAGE_SIZE, 0).unwrap()
    } else {
        client.grpc_read_tcp(fd, &mut page, PAGE_SIZE).unwrap()
    };
    assert!(result != -1, "{}: Read Failed", test);

    let binding = String::from_utf8(page).unwrap();
    let page_str = binding.trim_matches(char::from(0));
    assert!(
        page_str == "ReadTest\n",
        "{}: read request returned the following data: {:?}",
        test,
        page_str
    );

    let result = client.grpc_fsync_tcp(fd).unwrap();
    assert!(result != -1, "{}: Fsync Failed", test);

    let result = client.grpc_close_tcp(fd).unwrap();
    assert!(result != -1, "{}: Close Failed", test);

    Ok(())
}

#[test]
fn read_test() -> Result<(), Box<dyn std::error::Error>> {
    read_test_base(false)
}

#[test]
fn pread_test() -> Result<(), Box<dyn std::error::Error>> {
    read_test_base(true)
}

fn write_test_base(pwrite: bool) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = BlockingClient::connect_tcp("http://[::1]:8080")?;

    let test = if pwrite { "pWriteTest" } else { "WriteTest" };

    let filename = format!("{}{}", test, ".txt");
    let fd = client
        .grpc_open_tcp(&filename, O_CREAT | O_RDWR, S_IRWXU)
        .unwrap();
    assert!(fd != -1, "{}: Open Failed", test);

    let page = "WriteTest".as_bytes();
    let result = if pwrite {
        client
            .grpc_pwrite_tcp(fd, &page.to_vec(), page.len(), 0)
            .unwrap()
    } else {
        client.grpc_write_tcp(fd, &page.to_vec(), page.len()).unwrap()
    };

    // Length of test in files/read_test.txt
    assert!(result != -1, "{}: Write Failed", test);

    let result = client.grpc_fsync_tcp(fd).unwrap();
    assert!(result != -1, "{}: Fsync Failed", test);

    let result = client.grpc_close_tcp(fd).unwrap();
    assert!(result != -1, "{}: Close Failed", test);

    let result = client.grpc_remove_tcp(&filename).unwrap();
    assert!(result != -1, "{}: Remove Failed", test);

    Ok(())
}

#[test]
fn write_test() -> Result<(), Box<dyn std::error::Error>> {
    write_test_base(false)
}

#[test]
fn pwrite_test() -> Result<(), Box<dyn std::error::Error>> {
    write_test_base(true)
}

#[test]
fn write_read_test() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = BlockingClient::connect_tcp("http://[::1]:8080")?;

    let filename = "write_read_test.txt";
    let fd = client
        .grpc_open_tcp(filename, O_CREAT | O_RDWR, S_IRWXU)
        .unwrap();
    assert!(fd != -1, "WriteReadTest: Open Failed");

    let page = "WriteReadTest".as_bytes();
    let result = client.grpc_write_tcp(fd, &page.to_vec(), page.len()).unwrap();
    assert!(result != -1, "WriteReadTest: Write Failed");

    let mut page: Vec<u8> = vec![0; PAGE_SIZE];
    let result = client.grpc_pread_tcp(fd, &mut page, PAGE_SIZE, 0).unwrap();
    assert!(result != -1, "WriteReadTest: Read Failed");

    let binding = String::from_utf8(page).unwrap();
    let page_str = binding.trim_matches(char::from(0));
    assert!(
        page_str == "WriteReadTest",
        "WriteReadTest: read request returned the following data: {:?}",
        page_str
    );

    let result = client.grpc_fsync_tcp(fd).unwrap();
    assert!(result != -1, "WriteReadTest: Fsync Failed");

    let result = client.grpc_close_tcp(fd).unwrap();
    assert!(result != -1, "WriteReadTest: Close Failed");

    let result = client.grpc_remove_tcp(filename).unwrap();
    assert!(result != -1, "WriteReadTest: Remove Failed");

    Ok(())
}

#[test]
fn dir_test() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = BlockingClient::connect_tcp("http://[::1]:8080")?;

    let dirname = "dirTest";
    let res = client.grpc_mkdir_tcp(dirname, S_IRWXU).unwrap();
    assert!(res != 1, "DirTest: Mkdir Failed");

    let res = client.grpc_rmdir_tcp(dirname).unwrap();
    assert!(res != -1, "DirTest: Rmdir Failed");

    Ok(())
}
