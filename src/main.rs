/*
    gRPC server to execute system calls.
    Zack McKevitt - 2023
*/

use tonic::{transport::Server, Request, Response, Status};
use syscalls::{OpenRequest, OpenResponse, ReadRequest, ReadResponse, 
               WriteRequest, WriteResponse, CloseRequest, CloseResponse, 
               RemoveRequest, RemoveResponse, syscall_server::{Syscall, SyscallServer}};
use libc::*;
use nrfs::*;

// Need to make sure this is consistent with what client expects
const PAGE_SIZE: usize = 1024;
const PATH: &str = "./files/";

pub mod syscalls {
    tonic::include_proto!("syscalls");
}

#[derive(Debug, Default)]
pub struct SyscallService {}

fn serve_open(filename: &str, flags: i32) -> Response<syscalls::OpenResponse> {
    let file_path = format!("{}{}", PATH, filename);
    let fd;
    unsafe {
        fd = open(file_path.as_ptr() as *const i8, flags);
    }
    Response::new(syscalls::OpenResponse {
        result: fd,
    })
}

fn serve_read(fd: i32) -> Response<syscalls::ReadResponse> {
    let res;
    let page: &mut [u8; PAGE_SIZE] = &mut [0; PAGE_SIZE];
    unsafe {
        res = pread(fd, page.as_ptr() as *mut c_void, PAGE_SIZE, 0);
        // if res != PAGE_SIZE as isize {
        //     panic!("pread() failed");
        // }
    }
    Response::new(syscalls::ReadResponse {
        result: res as i32,
        page: page.to_vec(),
    })
}

// TODO: Error handling
fn serve_write(fd: i32, page: Vec<u8>) -> Response<syscalls::WriteResponse> {
    let res;
    unsafe {
        let len = page.len();
        res = write(fd, page.as_ptr() as *const c_void, len);
        if res != len as isize {
            panic!("Write Failed");
        };
    }
    Response::new(syscalls::WriteResponse {
        result: res as i32,
    })
}

fn serve_close(fd: i32) -> Response<syscalls::CloseResponse> {
    let res;
    unsafe {
        res = close(fd);
    }
    Response::new(syscalls::CloseResponse {
        result: res,
    })
}

fn serve_remove(filename: &str) -> Response<syscalls::RemoveResponse> {
    let file_path = format!("{}{}", PATH, filename);
    let fd;
    unsafe {
        fd = remove(file_path.as_ptr() as *const i8);
    }
    Response::new(syscalls::RemoveResponse {
        result: fd,
    })
}

// TODO: Do error handling
#[tonic::async_trait]
impl Syscall for SyscallService {
    async fn open(&self, request: Request<OpenRequest>) -> Result<Response<OpenResponse>, Status> {
        let r = request.into_inner();
        Ok(serve_open(&r.path, r.flags))
    }
    async fn read(&self, request: Request<ReadRequest>) -> Result<Response<ReadResponse>, Status> {
        let r = request.into_inner();
        Ok(serve_read(r.fd))
    }
    async fn write(&self, request: Request<WriteRequest>) -> Result<Response<WriteResponse>, Status> {
        let r = request.into_inner();
        Ok(serve_write(r.fd, r.page))
    }
    async fn close(&self, request: Request<CloseRequest>) -> Result<Response<CloseResponse>, Status> {
        let r = request.into_inner();
        Ok(serve_close(r.fd))
    }
    async fn remove(&self, request: Request<RemoveRequest>) -> Result<Response<RemoveResponse>, Status> {
        let r = request.into_inner();
        Ok(serve_remove(&r.path))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let address = "[::1]:8080".parse().unwrap();
    let syscalls_service = SyscallService::default();

    println!("Starting server on port {}", 8080);

    Server::builder().add_service(SyscallServer::new(syscalls_service))
        .serve(address)
        .await?;
    Ok(())
}
