/*
    gRPC server to execute system calls.
    Zack McKevitt - 2023
*/

use std::thread;

mod fxmark;
use crate::fxmark::run_benchmarks;

use fxmark_grpc::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    
    // Spawn server in background
    thread::spawn(|| {
        start_rpc_server("8080");
    });

    // run_benchmarks();
    loop {} ;

    Ok(())
}
