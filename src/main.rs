/*
    gRPC server to execute system calls.
    Zack McKevitt - 2023
*/

use clap::{crate_version, value_t, App, Arg};
use std::sync::{Arc, Mutex};
use std::fs::{remove_file, OpenOptions};
use std::io::Write;

mod fxmark;
use crate::fxmark::{bench, OUTPUT_FILE};

use fxmark_grpc::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args();
    let matches = App::new("Fxmark gRPC benchmark")
        .version(crate_version!())
        .author("Jon Gjengset, Gerd Zellweger, Zack McKevitt")
        .about("Distributed version of the Fxmark benchmark using gRPC")
        .arg(
            Arg::with_name("mode")
                .long("mode")
                .required(true)
                .help("loc_client, emu_client, loc_server, or emu_server")
                .takes_value(true)
                .possible_values(&["loc_client", "emu_client", "loc_server", "emu_server"]),
        )
        .arg(
            Arg::with_name("port")
                .long("port")
                .required(false)
                .help("Port to bind server")
                .takes_value(true),
        )
        .get_matches_from(args);

    let mode = value_t!(matches, "mode", String).unwrap();
    let bench_name = String::from("mix");

    match mode.as_str() {
        "loc_server" | "emu_server" => {
            let port = value_t!(matches, "port", u64).unwrap_or_else(|e| e.exit());
            let bind_addr = if mode == "loc_server" {
                "[::1]"
            } else {
                "172.31.0.1"
            };
            
            start_rpc_server(bind_addr, port)
        }
        "loc_client" | "emu_client" => {

            let host_addr = if mode == "loc_client" { 
                "http://[::1]:8080"
            } else {
                "http://172.31.0.1:8080" 
            };

            let client = Arc::new(Mutex::new(
                BlockingClient::connect(host_addr).unwrap(),
            ));
           
            let log_mode = Arc::new(if mode == "loc_client" {
                LogMode::CSV
            }
            else {
                LogMode::STDOUT
            });

            let _ = remove_file(OUTPUT_FILE);

            let mut csv_file = OpenOptions::new()
                .append(true)
                .create(true)
                .open(OUTPUT_FILE)
                .expect("Cant open output file");
            let row = "thread_id,benchmark,ncores,write_ratio,open_files,duration_total,duration,operations\n";

            match *log_mode {
                LogMode::CSV => {
                    let r = csv_file.write(row.as_bytes());
                    assert!(r.is_ok());
                }
                LogMode::STDOUT => {
                    print!("{}", row);
                }
            }
 
            bench(1, bench_name.clone(), 0, client.clone(), log_mode.clone());
            bench(1, bench_name.clone(), 10, client.clone(), log_mode.clone());
            bench(1, bench_name.clone(), 100, client.clone(), log_mode.clone());
        }
        _ => panic!("Unknown mode!"),
    }
    Ok(())
}
