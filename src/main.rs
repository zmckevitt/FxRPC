/*
    gRPC server to execute system calls.
    Zack McKevitt - 2023
*/

use clap::{crate_version, value_t, App, Arg};
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
                .help("client or server")
                .takes_value(true)
                .possible_values(&["client", "server"]),
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
        "server" => {
            let port = value_t!(matches, "port", u64).unwrap_or_else(|e| e.exit());
            start_rpc_server(port)
        }
        "client" => {
            let _ = remove_file(OUTPUT_FILE);

            let mut csv_file = OpenOptions::new()
                .append(true)
                .create(true)
                .open(OUTPUT_FILE)
                .expect("Cant open output file");
            let row = "thread_id,benchmark,ncores,write_ratio,open_files,duration_total,duration,operations\n";
            let r = csv_file.write(row.as_bytes());
            assert!(r.is_ok());

            bench(1, bench_name.clone(), 0);
            bench(1, bench_name.clone(), 10);
            bench(1, bench_name.clone(), 100);
        }
        _ => panic!("Unknown mode!"),
    }
    Ok(())
}
