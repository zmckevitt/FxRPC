/*
    gRPC server to execute system calls.
    Zack McKevitt - 2023
*/

use tonic::{transport::Server, Request, Response, Status};
use tokio::runtime::Runtime;
use std::thread;
use libc::*;

use syscalls::{OpenRequest, ReadRequest, WriteRequest, CloseRequest, RemoveRequest, FsyncRequest, 
               SyscallResponse, syscall_server::{Syscall, SyscallServer}};

mod fxmark;
use crate::fxmark::run_benchmarks;

// TODO: make sure this doesnt swap! More info:
// https://unix.stackexchange.com/questions/59300/how-to-place-store-a-file-in-memory-on-linux
// Temporary FS path
const PATH: &str = "/dev/shm/";

pub mod syscalls {
    tonic::include_proto!("syscalls");
}

#[derive(Debug, Default)]
pub struct SyscallService {}

// TODO: make S_IRWXU a function parameter
fn libc_open(filename: &str, flags: i32, mode: u32) -> Response<syscalls::SyscallResponse> {
    let file_path = format!("{}{}{}", PATH, filename, char::from(0));
    let fd;
    unsafe {
        fd = open(file_path.as_ptr() as *const i8, flags, mode);
    }
    Response::new(syscalls::SyscallResponse {
        result: fd,
        page: vec![0],
    })
}

fn libc_read(fd: i32, size: usize, offset: i64) -> Response<syscalls::SyscallResponse> {
    let res;
    //let page: &mut [u8; size] = &mut [0; size];
    let page: Vec<u8> = vec![0; size];
    unsafe {
        res = pread(fd, page.as_ptr() as *mut c_void, size, offset);
    }
    Response::new(syscalls::SyscallResponse {
        result: res as i32,
        page: page.to_vec(),
    })
}

// TODO: Error handling
fn libc_write(fd: i32, page: Vec<u8>, len: usize) -> Response<syscalls::SyscallResponse> {
    let res;
    unsafe {
        res = write(fd, page.as_ptr() as *const c_void, len);
        if res != len as isize {
            panic!("Write Failed");
        };
    }
    Response::new(syscalls::SyscallResponse {
        result: res as i32,
        page: vec![0],
    })
}

fn libc_close(fd: i32) -> Response<syscalls::SyscallResponse> {
    let res;
    unsafe {
        res = close(fd);
    }
    Response::new(syscalls::SyscallResponse {
        result: res,
        page: vec![0],
    })
}

fn libc_remove(filename: &str) -> Response<syscalls::SyscallResponse> {
    let file_path = format!("{}{}{}", PATH, filename, char::from(0));
    let fd;
    unsafe {
        fd = remove(file_path.as_ptr() as *const i8);
    }
    Response::new(syscalls::SyscallResponse {
        result: fd,
        page: vec![0],
    })
}

fn libc_fsync(fd: i32) -> Response<syscalls::SyscallResponse> {
    let res;
    unsafe {
        res = fsync(fd);
    }
    Response::new(syscalls::SyscallResponse {
        result: res,
        page: vec![0],
    })
}

// TODO: Do error handling
#[tonic::async_trait]
impl Syscall for SyscallService {
    async fn open(&self, request: Request<OpenRequest>) -> Result<Response<SyscallResponse>, Status> {
        let r = request.into_inner();
        Ok(libc_open(&r.path, r.flags, r.mode))
    }
    async fn read(&self, request: Request<ReadRequest>) -> Result<Response<SyscallResponse>, Status> {
        let r = request.into_inner();
        Ok(libc_read(r.fd, r.size as usize, r.offset))
    }
    async fn write(&self, request: Request<WriteRequest>) -> Result<Response<SyscallResponse>, Status> {
        let r = request.into_inner();
        Ok(libc_write(r.fd, r.page, r.len as usize))
    }
    async fn close(&self, request: Request<CloseRequest>) -> Result<Response<SyscallResponse>, Status> {
        let r = request.into_inner();
        Ok(libc_close(r.fd))
    }
    async fn remove(&self, request: Request<RemoveRequest>) -> Result<Response<SyscallResponse>, Status> {
        let r = request.into_inner();
        Ok(libc_remove(&r.path))
    }
    async fn fsync(&self, request: Request<FsyncRequest>) -> Result<Response<SyscallResponse>, Status> {
        let r = request.into_inner();
        Ok(libc_fsync(r.fd))
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    
    // Spawn server in background
    thread::spawn(|| {
        // Create Syscall server
        let address = "[::1]:8080".parse().unwrap();
        let syscalls_service = SyscallService::default();

        println!("Starting server on port {}", 8080);

        let rt = Runtime::new().expect("Failed to obtain runtime object.");
        let server_future = Server::builder()
            .add_service(SyscallServer::new(syscalls_service))
            .serve(address);
        rt.block_on(server_future)
            .expect("Failed to successfully run the future on RunTime.");
    });

    run_benchmarks();
    // loop {} ;

    Ok(())
}
