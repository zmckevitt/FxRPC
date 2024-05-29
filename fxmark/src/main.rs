/*
    gRPC server to execute system calls.
    Zack McKevitt - 2023
*/
use clap::{crate_version, value_t, App, Arg};
use std::fs::{remove_file, OpenOptions};
use std::io::Write;

#[macro_use]
extern crate abomonation;

mod fxmark;
use crate::fxmark::bench;
use crate::fxmark::utils::topology::MachineTopology;

pub mod fxrpc;
use crate::fxrpc::ConnType;
use crate::fxrpc::RPCType;
use crate::fxrpc::*;

const DEFAULT_OUTFILE: &str = "fxrpc_bench.csv";

fn parseargs(args: std::env::Args) -> clap::ArgMatches<'static> {
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
                .possible_values(&["client", "server", "loc_client_drpc"]),
        )
        .arg(
            Arg::with_name("rpc")
                .long("rpc")
                .required(true)
                .help("Dinos RPC (drpc) or gRPC (grpc)")
                .takes_value(true)
                .possible_values(&["drpc", "grpc"]),
        )
        .arg(
            Arg::with_name("transport")
                .long("transport")
                .required(true)
                .help("TCP Local (tcplocal) TCP Remote (tcpremote) UDS (uds)")
                .takes_value(true)
                .possible_values(&["tcplocal", "tcpremote", "uds"]),
        )
        .arg(
            Arg::with_name("port")
                .long("port")
                .short("p")
                .required(false)
                .help("Port")
                .default_value("8080")
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
        .arg(
            Arg::with_name("outfile")
                .short("o")
                .required(false)
                .help("Optional output file name")
                .takes_value(true)
                .default_value(DEFAULT_OUTFILE),
        )
        .get_matches_from(args);
    matches
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args();
    let matches = parseargs(args);

    let mode = value_t!(matches, "mode", String).unwrap();
    let port = value_t!(matches, "port", u16).unwrap();
    let conn_type: ConnType = {
        match value_t!(matches, "transport", String).unwrap().as_str() {
            "tcplocal" => ConnType::TcpLocal,
            "tcpremote" => ConnType::TcpRemote,
            "uds" => ConnType::UDS,
            &_ => panic!("Unknown ConnType!"),
        }
    };
    let rpc_type: RPCType = match value_t!(matches, "rpc", String).unwrap().as_str() {
        "grpc" => RPCType::GRPC,
        "drpc" => RPCType::DRPC,
        &_ => panic!("Unknown RPCType!"),
    };
    let bench_name = String::from("mix");
    let outfile = value_t!(matches, "outfile", String).unwrap();

    match mode.as_str() {
        "server" => {
            run_server(conn_type, rpc_type, port);
        }
        "client" => {
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

            let cid = if conn_type != ConnType::TcpLocal {
                value_t!(matches, "cid", usize).unwrap_or_else(|e| e.exit())
            } else {
                0
            };

            let nclients = if conn_type != ConnType::TcpLocal {
                value_t!(matches, "nclients", usize).unwrap_or_else(|e| e.exit())
            } else {
                1
            };

            let ccores = if conn_type == ConnType::TcpLocal {
                let topology = MachineTopology::new();
                let max_cores = topology.cores() / 2;
                max_cores
            } else {
                value_t!(matches, "ccores", usize).unwrap_or_else(|e| e.exit())
            };

            let log_mode = if conn_type == ConnType::TcpLocal {
                LogMode::CSV
            } else {
                LogMode::STDOUT
            };

            let client_params = ClientParams {
                cid: cid,
                nclients: nclients,
                ccores: ccores,
                log_mode: log_mode,
                conn_type: conn_type,
                rpc_type: rpc_type,
            };

            let row = "thread_id,benchmark,ncores,write_ratio,open_files,duration_total,duration,operations,client_id,client_cores,nclients,rpctype\n";
            match log_mode {
                LogMode::CSV => {
                    let _ = remove_file(outfile.clone());
                    let mut csv_file = OpenOptions::new()
                        .append(true)
                        .create(true)
                        .open(outfile.clone())
                        .expect("Cant open output file");
                    let r = csv_file.write(row.as_bytes());
                    assert!(r.is_ok());
                }
                LogMode::STDOUT => {
                    if conn_type != ConnType::UDS {
                        print!("{}", row);
                    }
                }
            }

            for of in openfs {
                for wr in &wratios {
                    bench(
                        bench_name.clone(),
                        of,
                        *wr,
                        duration,
                        &client_params,
                        &outfile,
                    );
                }
            }
        }
        _ => panic!("Unknown mode!"),
    }
    Ok(())
}
