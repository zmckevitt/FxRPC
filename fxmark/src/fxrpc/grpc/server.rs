/*
    Library for gRPC system call server and clients.
    Zack McKevitt - 2023
*/

use libc::*;
use syscalls::{
    syscall_client::SyscallClient,
    syscall_server::{Syscall, SyscallServer},
    CloseRequest, DirRequest, FstatRequest, FstatResponse, FsyncRequest, OpenRequest, ReadRequest,
    RemoveRequest, SyscallResponse, WriteRequest,
};
use tokio::net::{UnixListener, UnixStream};
use tokio::runtime::Builder;
use tokio::runtime::Runtime;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::{transport::Endpoint, transport::Server, transport::Uri, Request, Response, Status};
use tower::service_fn;

use std::os::unix::net::UnixListener as StdUnixListener;
use std::path::Path;

pub use crate::fxrpc::*;

//////////////////////////////////////// SERVER ////////////////////////////////////////

fn libc_open(filename: &str, flags: i32, mode: u32) -> Response<syscalls::SyscallResponse> {
    let file_path = format!("{}{}{}", FS_PATH, filename, char::from(0));
    let fd;
    unsafe {
        fd = open(file_path.as_ptr() as *const i8, flags, mode);
    }
    Response::new(syscalls::SyscallResponse {
        result: fd,
        page: vec![0],
    })
}

fn libc_read(fd: i32, size: usize) -> Response<syscalls::SyscallResponse> {
    let res;
    //let page: &mut [u8; size] = &mut [0; size];
    let page: Vec<u8> = vec![0; size];
    unsafe {
        res = read(fd, page.as_ptr() as *mut c_void, size);
    }
    Response::new(syscalls::SyscallResponse {
        result: res as i32,
        page: page.to_vec(),
    })
}

fn libc_pread(fd: i32, size: usize, offset: i64) -> Response<syscalls::SyscallResponse> {
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

fn libc_write(fd: i32, page: Vec<u8>, len: usize) -> Response<syscalls::SyscallResponse> {
    let res;
    unsafe {
        res = write(fd, page.as_ptr() as *const c_void, len);
    }
    Response::new(syscalls::SyscallResponse {
        result: res as i32,
        page: vec![0],
    })
}

fn libc_pwrite(
    fd: i32,
    page: Vec<u8>,
    len: usize,
    offset: i64,
) -> Response<syscalls::SyscallResponse> {
    let res;
    unsafe {
        res = pwrite(fd, page.as_ptr() as *const c_void, len, offset);
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
    let file_path = format!("{}{}{}", FS_PATH, filename, char::from(0));
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

fn libc_mkdir(dirname: &str, mode: u32) -> Response<syscalls::SyscallResponse> {
    let dir_path = format!("{}{}{}", FS_PATH, dirname, char::from(0));
    let res;
    unsafe {
        res = mkdir(dir_path.as_ptr() as *const i8, mode.try_into().unwrap());
    }
    Response::new(syscalls::SyscallResponse {
        result: res,
        page: vec![0],
    })
}

fn libc_rmdir(dirname: &str) -> Response<syscalls::SyscallResponse> {
    let dir_path = format!("{}{}{}", FS_PATH, dirname, char::from(0));
    let res;
    unsafe {
        res = rmdir(dir_path.as_ptr() as *const i8);
    }
    Response::new(syscalls::SyscallResponse {
        result: res,
        page: vec![0],
    })
}

// Currently only supporting fstat file size
// Not yet clear how to conver MaybeUninit<stat> to Vec<u8>
// Mix only needs file size anyways
fn libc_fstat_size(fd: i32) -> Response<syscalls::FstatResponse> {
    let res;
    let fsize;
    let mut info = std::mem::MaybeUninit::uninit();
    unsafe {
        res = fstat(fd, info.as_mut_ptr());
        fsize = info.assume_init().st_size;
    }
    Response::new(syscalls::FstatResponse {
        result: res,
        size: fsize,
    })
}

// TODO: Do error handling
#[tonic::async_trait]
impl Syscall for SyscallService {
    async fn open(
        &self,
        request: Request<OpenRequest>,
    ) -> Result<Response<SyscallResponse>, Status> {
        let r = request.into_inner();
        Ok(libc_open(&r.path, r.flags, r.mode))
    }
    async fn read(
        &self,
        request: Request<ReadRequest>,
    ) -> Result<Response<SyscallResponse>, Status> {
        let r = request.into_inner();
        match r.pread {
            true => Ok(libc_pread(r.fd, r.size as usize, r.offset)),
            false => Ok(libc_read(r.fd, r.size as usize)),
        }
    }
    async fn write(
        &self,
        request: Request<WriteRequest>,
    ) -> Result<Response<SyscallResponse>, Status> {
        let r = request.into_inner();
        match r.pwrite {
            true => Ok(libc_pwrite(r.fd, r.page, r.len as usize, r.offset)),
            false => Ok(libc_write(r.fd, r.page, r.len as usize)),
        }
    }
    async fn close(
        &self,
        request: Request<CloseRequest>,
    ) -> Result<Response<SyscallResponse>, Status> {
        let r = request.into_inner();
        Ok(libc_close(r.fd))
    }
    async fn remove(
        &self,
        request: Request<RemoveRequest>,
    ) -> Result<Response<SyscallResponse>, Status> {
        let r = request.into_inner();
        Ok(libc_remove(&r.path))
    }
    async fn fsync(
        &self,
        request: Request<FsyncRequest>,
    ) -> Result<Response<SyscallResponse>, Status> {
        let r = request.into_inner();
        Ok(libc_fsync(r.fd))
    }
    async fn mkdir(
        &self,
        request: Request<DirRequest>,
    ) -> Result<Response<SyscallResponse>, Status> {
        let r = request.into_inner();
        Ok(libc_mkdir(&r.path, r.mode))
    }
    async fn rmdir(
        &self,
        request: Request<DirRequest>,
    ) -> Result<Response<SyscallResponse>, Status> {
        let r = request.into_inner();
        Ok(libc_rmdir(&r.path))
    }
    async fn fstat(
        &self,
        request: Request<FstatRequest>,
    ) -> Result<Response<FstatResponse>, Status> {
        let r = request.into_inner();
        Ok(libc_fstat_size(r.fd))
    }
}

pub fn start_rpc_server_tcp(bind_addr: &str, port: u64) {
    // Create Syscall server
    let address = format!("{}:{}", bind_addr, port).parse().unwrap();
    let syscalls_service = SyscallService::default();

    println!("Starting server on port {}", port);

    let rt = Runtime::new().expect("Failed to obtain runtime object.");
    let server_future = Server::builder()
        .add_service(SyscallServer::new(syscalls_service))
        .serve(address);
    rt.block_on(server_future)
        .expect("Failed to successfully run the future on RunTime.");
}

#[tokio::main]
pub async fn start_rpc_server_uds(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting server on UDS path: {}", path);

    // Remove existing UDS dir
    let _ = std::fs::remove_dir_all(Path::new(path).parent().unwrap());

    // Create dir for UDS
    let _ = std::fs::create_dir_all(Path::new(path).parent().unwrap());

    let syscalls_service = SyscallService::default();

    // Create standard, blocking UDS
    let std_uds = StdUnixListener::bind(path).unwrap();

    // Create tokio UDS
    let uds = UnixListener::from_std(std_uds).unwrap();
    let uds_stream = UnixListenerStream::new(uds);
    Server::builder()
        .add_service(SyscallServer::new(syscalls_service))
        .serve_with_incoming(uds_stream)
        .await?;
    Ok(())
}
