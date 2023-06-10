use std::io::stdin;

use syscall::{SyscallRequest, syscall_client::SyscallClient};

pub mod syscall {
    tonic::include_proto!("syscalls");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = SyscallClient::connect("http://[::1]:8080").await?;
    loop {
        let mut id = String::new();
        println!("Please provide syscall id (0-3): ");
        stdin().read_line(&mut id).unwrap();
        let id = match id.trim().to_lowercase().chars().next().unwrap() {
            '0' => 0,
            '1' => 1,
            '2' => 2,
            '3' => 3,
            _ => break,
        };
        let request = tonic::Request::new(SyscallRequest {
            call: id,
        });
        let response = client.call(request).await?.into_inner();
        println!("Result: {} | Data: '{}'", 
            response.result,
            response.data
        );
    }
    Ok(())
}
