/*
    gRPC server to execute system calls.
    Zack McKevitt - 2023
*/

use clap::{crate_version, App, Arg, value_t};

mod fxmark;
use crate::fxmark::run_benchmarks;

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
                .takes_value(true)
        )
        .arg(
            Arg::with_name("duration")
                .short("d")
                .long("duration")
                .required(false)
                .help("Duration for each run")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("type")
                .short("t")
                .long("type")
                .multiple(true)
                .takes_value(true)
                .required(false)
                .possible_values(&[
                    "drbl", "drbh", "dwol", "dwom", "dwal", "mwrl", "mwrm", "mixX0", "mixX1",
                    "mixX5", "mixX10", "mixX20", "mixX40", "mixX60", "mixX80", "mixX100",
                ])
                .help("Benchmark to run.")
        )
        .get_matches_from(args);
    let mode = value_t!(matches, "mode", String).unwrap();

    match mode.as_str() {
        "server" => {
            let port = value_t!(matches, "port", u64).unwrap_or_else(|e| e.exit());
            start_rpc_server(port)
        }, 
        "client" => { 
            let duration = value_t!(matches, "duration", u64).unwrap_or_else(|e| e.exit());
            let versions: Vec<&str> = match matches.values_of("type") {
                Some(iter) => iter.collect(),
                None => unreachable!(),
            };
            run_benchmarks(duration, versions);
        }, 
        _ => panic!("Unknown mode!"),
    }
    Ok(())
}
