/*
    gRPC server to execute system calls.
    Zack McKevitt - 2023
*/

use clap::{crate_version, value_t, App, Arg};
use std::fs::{remove_file, OpenOptions};
use std::io::Write;
use std::sync::{Arc, Mutex};

mod fxmark;
use crate::fxmark::{bench, OUTPUT_FILE};

use crate::fxmark::utils::topology::MachineTopology;

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
                .help("loc_client, emu_client, loc_server, emu_server, or uds_server")
                .takes_value(true)
                .possible_values(&["loc_client", "emu_client", "loc_server", "emu_server", "uds_server"]),
        )
        .arg(
            Arg::with_name("port")
                .long("port")
                .required(false)
                .help("Port to bind server")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("wratio")
                .long("wratio")
                .required(false)
                .help("Write ratio for mix benchmarks")
                .multiple(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("openf")
                .long("openf")
                .required(false)
                .help("Number of open files for mix benchmarks")
                .multiple(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("duration")
                .long("duration")
                .required(false)
                .help("Duration for benchmark")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("cid")
                .long("cid")
                .required(false)
                .help("Client ID")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("nclients")
                .long("nclients")
                .required(false)
                .help("Number of clients")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("ccores")
                .long("ccores")
                .required(false)
                .help("Cores per client")
                .takes_value(true),
        )
        .get_matches_from(args);

    let mode = value_t!(matches, "mode", String).unwrap();
    let bench_name = String::from("mix");

    match mode.as_str() {
        "uds_server" => {
            let path = "/tmp/tonic/test";

            // Must unwrap as function is asynchronous
            start_rpc_server_uds(path).unwrap()
        }
        "loc_server" | "emu_server" => {
            let port = value_t!(matches, "port", u64).unwrap_or_else(|e| e.exit());
            let bind_addr = if mode == "loc_server" {
                "[::1]"
            } else {
                "172.31.0.1"
            };

            start_rpc_server_tcp(bind_addr, port)
        }
        "loc_client" | "emu_client" => {
            let wratios: Vec<&str> = matches.values_of("wratio").unwrap().collect();
            let wratios: Vec<usize> = wratios
                .into_iter()
                .map(|x| x.parse::<usize>().unwrap())
                .collect();
            let openfs: Vec<&str> = matches.values_of("openf").unwrap().collect();
            let openfs: Vec<usize> = openfs
                .into_iter()
                .map(|x| x.parse::<usize>().unwrap())
                .collect();

            let duration = value_t!(matches, "duration", u64).unwrap_or_else(|e| e.exit());

            let cid = if mode == "emu_client" {
                value_t!(matches, "cid", usize).unwrap_or_else(|e| e.exit())
            } else {
                0
            };

            let nclients = if mode == "emu_client" {
                value_t!(matches, "nclients", usize).unwrap_or_else(|e| e.exit())
            } else {
                1
            };

            let ccores = if mode == "loc_client" {
                let topology = MachineTopology::new();
                let max_cores = topology.cores() / 2;
                max_cores
            } else {
                value_t!(matches, "ccores", usize).unwrap_or_else(|e| e.exit())
            };

            let log_mode = if mode == "loc_client" {
                LogMode::CSV
            } else {
                LogMode::STDOUT
            };

            let client_params = ClientParams {
                cid: cid,
                nclients: nclients,
                ccores: ccores,
                log_mode: log_mode,
            };

            let row = "thread_id,benchmark,ncores,write_ratio,open_files,duration_total,duration,operations,client_id,client_cores,nclients\n";
            match log_mode {
                LogMode::CSV => {
                    let _ = remove_file(OUTPUT_FILE);
                    let mut csv_file = OpenOptions::new()
                        .append(true)
                        .create(true)
                        .open(OUTPUT_FILE)
                        .expect("Cant open output file");
                    let r = csv_file.write(row.as_bytes());
                    assert!(r.is_ok());
                }
                LogMode::STDOUT => {
                    print!("{}", row);
                }
            }

            let host_addr = if mode == "loc_client" {
                "http://[::1]:8080"
            } else {
                "http://172.31.0.1:8080"
            };

            let client = Arc::new(Mutex::new(BlockingClient::connect(host_addr).unwrap()));
            for of in openfs {
                for wr in &wratios {
                    bench(
                        bench_name.clone(),
                        of,
                        *wr,
                        duration,
                        &client_params,
                        client.clone(),
                    );
                }
            }
        }
        _ => panic!("Unknown mode!"),
    }
    Ok(())
}
