/*
    Fxmark gRPC benchmark runner
    Zack McKevitt - 2023
*/

use std::fs::{remove_file, OpenOptions};
use std::io::Write;
use std::process::Command;

use clap::{crate_version, value_t, App, Arg};

mod builder;
use builder::Machine;

fn main() {
    let args = std::env::args();
    let matches = App::new("Fxmark gRPC Runner")
        .version(crate_version!())
        .about("Runner for Fxmark gRPC benchmarks")
        .arg(
            Arg::with_name("transport")
                .long("transport")
                .required(true)
                .help("tcp or uds")
                .takes_value(true)
                .possible_values(&["tcp", "uds"]),
        )
        .arg(
            Arg::with_name("rpc")
                .long("rpc")
                .required(true)
                .help("grpc or drpc")
                .takes_value(true)
                .possible_values(&["grpc", "drpc"]),
        )
        .arg(
            Arg::with_name("image")
                .long("image")
                .required(false)
                .help("Path to disk image")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("wratio")
                .long("wratio")
                .required(true)
                .help("Write ratio for mix benchmarks")
                .multiple(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("openf")
                .long("openf")
                .required(true)
                .help("Number of open files for mix benchmarks")
                .multiple(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("duration")
                .long("duration")
                .required(true)
                .help("Duration of benchmarks")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("csv")
                .long("csv")
                .required(false)
                .default_value("")
                .help("Path to csv file")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("nonuma")
                .long("nonuma")
                .required(false)
                .help("Do not pin cores to NUMA node")
                .takes_value(false),
        )
        .get_matches_from(args);

    let transport = value_t!(matches, "transport", String).unwrap_or_else(|e| e.exit());
    let rpc = value_t!(matches, "rpc", String).unwrap_or_else(|e| e.exit());
    let wratios: Vec<&str> = matches.values_of("wratio").unwrap().collect();
    let openfs: Vec<&str> = matches.values_of("openf").unwrap().collect();
    let duration = value_t!(matches, "duration", String).unwrap_or_else(|e| e.exit());
    let csv = value_t!(matches, "csv", String).unwrap_or_else(|e| e.exit());

    let csv = if csv == "" {
        format!("fxrpc_{}_{}_benchmarks.csv", transport, rpc)
    } else {
        csv
    };

    let wr_joined = wratios.join(" ");
    let of_joined = openfs.join(" ");

    fn mem_fn(num_cores: usize) -> usize {
        // Memory must also be divisible by number of clients, which could be 1, 2, 3, or 4
        2048 * (((num_cores + 3 - 1) / 3) * 3)
    }

    let row = "thread_id,benchmark,ncores,write_ratio,open_files,duration_total,duration,operations,client_id,client_cores,nclients,rpctype\n";
    let _ = remove_file(csv.clone());
    let mut csv_file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(csv.clone())
        .expect("Cant open output file");
    let r = csv_file.write(row.as_bytes());
    assert!(r.is_ok());

    // Find max cores, max numa, and max cores per node
    let machine = Machine::determine();
    let max_cores = machine.max_cores();
    let max_numa = machine.max_numa_nodes();
    let total_cores_per_node = core::cmp::max(1, max_cores / max_numa);

    let mut num_clients = 1;

    let mut total_cores = 1;
    while total_cores < max_cores {
        // Round up to get the number of clients
        let new_num_clients = (total_cores + (total_cores_per_node - 1)) / total_cores_per_node;

        if num_clients != new_num_clients {
            num_clients = new_num_clients;
            total_cores = total_cores - (total_cores % num_clients);
        }

        let cores_per_client = total_cores / num_clients;

        let scores = format!("{}", num_clients + 1);

        // We want controller to have it's own socket, so if it's not a 1 socket machine, break
        // when there's equal number of clients to numa nodes.
        if total_cores + num_clients + 1 > machine.max_cores()
            || num_clients == machine.max_numa_nodes()
                && cores_per_client + num_clients + 1 > total_cores_per_node
            || num_clients == max_numa && max_numa > 1
        {
            break;
        }

        eprintln!(
            "\tRunning test with {:?} total core(s), {:?} clients (cores_per_client={:?})",
            total_cores, num_clients, cores_per_client
        );

        // Use python runner to perform emulation
        if transport == "tcp" {
            let image = value_t!(matches, "image", String).unwrap_or_else(|e| e.exit());
            let output = Command::new("python3")
                .arg("run.py")
                .arg("--transport")
                .arg("tcp")
                .arg("--rpc")
                .arg(rpc.clone())
                .arg("--image")
                .arg(image.clone())
                .arg("--scores")
                .arg(scores.clone())
                .arg("--clients")
                .arg(format!("{}", num_clients))
                .arg("--ccores")
                .arg(format!("{}", cores_per_client))
                .arg("--wratio")
                .arg(wr_joined.clone())
                .arg("--openf")
                .arg(of_joined.clone())
                .arg("--duration")
                .arg(duration.clone())
                .arg("--csv")
                .arg(csv.clone())
                .arg("--memory")
                .arg(format!("{}", mem_fn(total_cores) / (num_clients + 1)))
                .output()
                .expect("failed to execute process");

            println!("Status: {}", output.status);
            println!("Stdout: {}", String::from_utf8_lossy(&output.stdout));
            println!("Stderr: {}", String::from_utf8_lossy(&output.stderr));
        }
        // Unix Domain Socket
        else {
            let nonuma = if matches.is_present("nonuma") {
                "--nonuma"
            } else {
                "--numa"
            };
            let output = Command::new("python3")
                .arg("run.py")
                .arg("--transport")
                .arg("uds")
                .arg("--rpc")
                .arg(rpc.clone())
                .arg("--scores")
                .arg(scores.clone())
                .arg("--clients")
                .arg(format!("{}", num_clients))
                .arg("--ccores")
                .arg(format!("{}", cores_per_client))
                .arg("--wratio")
                .arg(wr_joined.clone())
                .arg("--openf")
                .arg(of_joined.clone())
                .arg("--duration")
                .arg(duration.clone())
                .arg("--csv")
                .arg(csv.clone())
                .arg(nonuma)
                .output()
                .expect("failed to execute process");
            println!("Status: {}", output.status);
            println!("Stdout: {}", String::from_utf8_lossy(&output.stdout));
            println!("Stderr: {}", String::from_utf8_lossy(&output.stderr));
        }

        if total_cores == 1 {
            total_cores = 0;
        }
        if num_clients == 3 {
            total_cores += 3;
        } else {
            total_cores += 4;
        }
    }
}
