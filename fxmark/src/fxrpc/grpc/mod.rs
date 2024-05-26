pub mod server;
pub use crate::fxrpc::grpc::server::*;

pub mod client;
pub use crate::fxrpc::grpc::client::*;

pub mod syscalls {
    tonic::include_proto!("syscalls");
}

#[derive(Debug, Default)]
pub struct SyscallService {}
