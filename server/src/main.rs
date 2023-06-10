use tonic::{transport::Server, Request, Response, Status};
use syscalls::{SyscallRequest, SyscallResponse, syscall_server::{Syscall, SyscallServer}};

pub mod syscalls {
    tonic::include_proto!("syscalls");
}

#[derive(Debug, Default)]
pub struct SyscallService {}

#[tonic::async_trait]
impl Syscall for SyscallService {
    async fn call(&self, request: Request<SyscallRequest>) -> Result<Response<SyscallResponse>, Status> {
        let r = request.into_inner();
        match r.call {
            0 => Ok(Response::new(syscalls::SyscallResponse {
                result: 0,
                data: { format!("Open Request Issued.") },
            })),
            1 => Ok(Response::new(syscalls::SyscallResponse { 
                result: 0,
                data: { format!("Read Request Issued.") },
            })),
            2 => Ok(Response::new(syscalls::SyscallResponse { 
                result: 0,
                data: { format!("Write Request Issued.") },
            })),
            3 => Ok(Response::new(syscalls::SyscallResponse {
                result: 0,
                data: { format!("Close Request Issued.") },
            })),
            _ => Err(Status::new(tonic::Code::OutOfRange, "Invalid Syscall"))
        }
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
