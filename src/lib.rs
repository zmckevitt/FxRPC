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
use tokio::runtime::Builder;
use tokio::runtime::Runtime;
use tonic::{transport::Server, Request, Response, Status};

type StdError = Box<dyn std::error::Error + Send + Sync + 'static>;
type Result<T, E = StdError> = ::std::result::Result<T, E>;

// File system path
pub const PATH: &str = "/dev/shm/";

// pub const PAGE_SIZE: usize = 1024;

pub enum LogMode {
    CSV,
    STDOUT,
}

pub mod syscalls {
    tonic::include_proto!("syscalls");
}

//////////////////////////////////////// CLIENT ////////////////////////////////////////

pub struct BlockingClient {
    client: SyscallClient<tonic::transport::Channel>,
    rt: Runtime,
}

#[derive(Clone)]
pub struct ClientWrapper(pub *mut BlockingClient);

unsafe impl Send for ClientWrapper {}
impl Copy for ClientWrapper {}

impl BlockingClient {
    pub fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
    where
        D: TryInto<tonic::transport::Endpoint>,
        D::Error: Into<StdError>,
    {
        let rt = Builder::new_multi_thread().enable_all().build().unwrap();
        let client = rt.block_on(SyscallClient::connect(dst))?;

        Ok(Self { client, rt })
    }

    pub fn grpc_open(
        &mut self,
        path: &str,
        flags: i32,
        mode: u32,
    ) -> Result<i32, Box<dyn std::error::Error>> {
        let request = tonic::Request::new(OpenRequest {
            path: path.to_string(),
            flags: flags,
            mode: mode,
        });
        let response = self.rt.block_on(self.client.open(request))?.into_inner();
        Ok(response.result)
    }

    fn grpc_read_base(
        &mut self,
        pread: bool,
        fd: i32,
        page: &mut Vec<u8>,
        size: usize,
        offset: i64,
    ) -> Result<i32, Box<dyn std::error::Error>> {
        let request = tonic::Request::new(ReadRequest {
            pread: pread,
            fd: fd,
            size: size as u32,
            offset: offset,
        });

        let response = self.rt.block_on(self.client.read(request))?.into_inner();
        *page = response.page;
        Ok(response.result)
    }

    pub fn grpc_read(
        &mut self,
        fd: i32,
        page: &mut Vec<u8>,
        size: usize,
    ) -> Result<i32, Box<dyn std::error::Error>> {
        self.grpc_read_base(false, fd, page, size, 0)
    }

    pub fn grpc_pread(
        &mut self,
        fd: i32,
        page: &mut Vec<u8>,
        size: usize,
        offset: i64,
    ) -> Result<i32, Box<dyn std::error::Error>> {
        self.grpc_read_base(true, fd, page, size, offset)
    }

    fn grpc_write_base(
        &mut self,
        pwrite: bool,
        fd: i32,
        page: &Vec<u8>,
        len: usize,
        offset: i64,
    ) -> Result<i32, Box<dyn std::error::Error>> {
        let request = tonic::Request::new(WriteRequest {
            pwrite: pwrite,
            fd: fd,
            page: page.to_vec(),
            len: len as u32,
            offset: offset,
        });

        let response = self.rt.block_on(self.client.write(request))?.into_inner();
        Ok(response.result)
    }

    pub fn grpc_write(
        &mut self,
        fd: i32,
        page: &Vec<u8>,
        size: usize,
    ) -> Result<i32, Box<dyn std::error::Error>> {
        self.grpc_write_base(false, fd, page, size, 0)
    }

    pub fn grpc_pwrite(
        &mut self,
        fd: i32,
        page: &Vec<u8>,
        size: usize,
        offset: i64,
    ) -> Result<i32, Box<dyn std::error::Error>> {
        self.grpc_write_base(true, fd, page, size, offset)
    }

    pub fn grpc_close(&mut self, fd: i32) -> Result<i32, Box<dyn std::error::Error>> {
        let request = tonic::Request::new(CloseRequest { fd: fd });

        let response = self.rt.block_on(self.client.close(request))?.into_inner();
        Ok(response.result)
    }

    pub fn grpc_remove(&mut self, path: &str) -> Result<i32, Box<dyn std::error::Error>> {
        let request = tonic::Request::new(RemoveRequest {
            path: path.to_string(),
        });
        let response = self.rt.block_on(self.client.remove(request))?.into_inner();
        Ok(response.result)
    }

    pub fn grpc_fsync(&mut self, fd: i32) -> Result<i32, Box<dyn std::error::Error>> {
        let request = tonic::Request::new(FsyncRequest { fd: fd });

        let response = self.rt.block_on(self.client.fsync(request))?.into_inner();
        Ok(response.result)
    }

    pub fn grpc_mkdir(&mut self, path: &str, mode: u32) -> Result<i32, Box<dyn std::error::Error>> {
        let request = tonic::Request::new(DirRequest {
            path: path.to_string(),
            mode: mode,
        });
        let response = self.rt.block_on(self.client.mkdir(request))?.into_inner();
        Ok(response.result)
    }

    pub fn grpc_rmdir(&mut self, path: &str) -> Result<i32, Box<dyn std::error::Error>> {
        let request = tonic::Request::new(DirRequest {
            path: path.to_string(),
            mode: 0,
        });
        let response = self.rt.block_on(self.client.rmdir(request))?.into_inner();
        Ok(response.result)
    }

    pub fn grpc_fstat_size(&mut self, fd: i32) -> Result<i64, Box<dyn std::error::Error>> {
        let request = tonic::Request::new(FstatRequest { fd: fd });

        let response = self.rt.block_on(self.client.fstat(request))?.into_inner();
        Ok(response.size)
    }
}

//////////////////////////////////////// SERVER ////////////////////////////////////////

#[derive(Debug, Default)]
pub struct SyscallService {}

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

fn libc_mkdir(dirname: &str, mode: u32) -> Response<syscalls::SyscallResponse> {
    let dir_path = format!("{}{}{}", PATH, dirname, char::from(0));
    let res;
    unsafe {
        res = mkdir(dir_path.as_ptr() as *const i8, mode);
    }
    Response::new(syscalls::SyscallResponse {
        result: res,
        page: vec![0],
    })
}

fn libc_rmdir(dirname: &str) -> Response<syscalls::SyscallResponse> {
    let dir_path = format!("{}{}{}", PATH, dirname, char::from(0));
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

pub fn start_rpc_server(bind_addr: &str, port: u64) {
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
