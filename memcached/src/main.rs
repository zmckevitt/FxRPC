/*
    Memcached benchmark runner
    Reto Achermann - 2023
*/

use std::env;
use std::fs::{remove_file, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

use clap::Parser;

mod builder;
use builder::Machine;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[arg(short, long, default_value_t = 1024)]
    memory: usize,

    #[arg(short, long, default_value_t = 100)]
    queries: usize,

    /// Name of the person to greet
    #[arg(short, long, value_name = "FILE")]
    image: Option<PathBuf>,

    /// Name of the person to greet
    #[arg(short, long, value_name = "FILE")]
    csv: Option<PathBuf>,
}

#[cfg(not(feature = "baremetal"))]
fn rackscale_memcached_checkout() -> PathBuf {
    let cwd = std::env::current_dir().unwrap();

    let out_dir_path = cwd.join("sharded-memcached");

    let out_dir = out_dir_path.display().to_string();

    println!("OUT_DIR {:?}", out_dir);

    // clone abd build the benchmark
    if !out_dir_path.is_dir() {
        println!("RMDIR {:?}", out_dir_path);
        Command::new(format!("rm",))
            .args(&["-rf", out_dir.as_str()])
            .status()
            .unwrap();

        println!("MKDIR {:?}", out_dir_path);
        Command::new(format!("mkdir",))
            .args(&["-p", out_dir.as_str()])
            .status()
            .unwrap();

        println!("CLONE {:?}", out_dir_path);
        let url = "https://github.com/achreto/memcached-bench.git";
        Command::new("git")
            .args(&["clone", "--depth=1", url, out_dir.as_str()])
            .output()
            .expect("failed to clone");
    } else {
        Command::new("git")
            .args(&["pull"])
            .current_dir(out_dir_path.as_path())
            .output()
            .expect("failed to pull");
    }

    println!(
        "CHECKOUT a703eedd8032ff1e083e8c5972eacc95738c797b {:?}",
        out_dir
    );

    let res = Command::new("git")
        .args(&["checkout", "a703eedd8032ff1e083e8c5972eacc95738c797b"])
        .current_dir(out_dir_path.as_path())
        .output()
        .expect("git checkout failed");
    if !res.status.success() {
        std::io::stdout().write_all(&res.stdout).unwrap();
        std::io::stderr().write_all(&res.stderr).unwrap();
        panic!("git checkout failed!");
    }

    println!("BUILD {:?}", out_dir_path);
    for (key, value) in env::vars() {
        println!("{}: {}", key, value);
    }

    let build_args = &["-j"];
    // now build the benchmark
    let status = Command::new("make")
        .args(build_args)
        .current_dir(&out_dir_path)
        .output()
        .expect("Can't make app dir");

    if !status.status.success() {
        println!("BUILD FAILED");
        std::io::stdout().write_all(&status.stdout).unwrap();
        std::io::stderr().write_all(&status.stderr).unwrap();
        panic!("BUILD FAILED");
    }

    out_dir_path
}

fn main() {
    let args = Args::parse();

    let outdir = rackscale_memcached_checkout();

    let kv_store = outdir.join("memcached/memcached");
    let loadbalancer = outdir.join("loadbalancer/loadbalancer");

    let csv = if let Some(csv) = args.csv.as_ref() {
        csv.clone()
    } else {
        PathBuf::from("memcached_benchmark_sharded_linux.csv")
    };

    let row = "benchmark,os,nthreads,servers,protocol,mem,queries,time,thpt,notes\n";
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

    let image = if let Some(image) = args.image.as_ref() {
        image.clone()
    } else {
        PathBuf::from("foo")
    };

    let mut num_clients = 1;

    let output = Command::new("python3")
        .arg("run.py")
        .arg("--servers")
        .arg(format!("{}", num_clients))
        .arg("--offset")
        .arg("net")
        .arg("--network-only")
        .output()
        .expect("failed to execute process");

    let mut total_cores = 1;
    while total_cores < max_cores {
        // Round up to get the number of clients
        let new_num_clients = (total_cores + (total_cores_per_node - 1)) / total_cores_per_node;
        let cores_per_client = total_cores / num_clients;
        if num_clients != new_num_clients {
            num_clients = new_num_clients;
            total_cores = total_cores - (total_cores % num_clients);

            // setup networking here!

            let output = Command::new("python3")
                .arg("run.py")
                .arg("--servers")
                .arg(format!("{}", num_clients))
                .arg("net")
                .arg("--network-only")
                .output()
                .expect("failed to execute process");
        }

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
            "\n\nRunning test with {:?} total core(s), {:?} clients (cores_per_client={:?})",
            total_cores, num_clients, cores_per_client
        );

        let output = Command::new("python3")
            .arg("run.py")
            .arg("--cores")
            .arg(format!("{}", cores_per_client))
            .arg("--memory")
            .arg(format!("{}", args.memory))
            .arg("--image")
            .arg(image.clone())
            .arg("--queries")
            .arg(format!("{}", args.queries))
            .arg("--servers")
            .arg(format!("{}", num_clients))
            .arg("--loadbalancer")
            .arg(loadbalancer.display().to_string())
            .arg("--kvstore")
            .arg(kv_store.display().to_string())
            .arg("--out")
            .arg(csv.clone())
            .arg("net")
            .arg("--no-network-setup")
            .output()
            .expect("failed to execute process");

        println!("Status: {}", output.status);
        println!("Stdout: {}", String::from_utf8_lossy(&output.stdout));
        if !output.status.success() {
            println!("Stderr: {}", String::from_utf8_lossy(&output.stderr));
            panic!("failed to execute benchmark!")
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

// run.py  --servers 1 net --network-only
// run.py --cores 1 --memory 1024 --image ubuntu-server-cloudimg-amd64.img --queries 100 --servers 1 --loadbalancer /home/achreto/memcached/sharded-memcached/loadbalancer/loadbalancer --kvstore /home/achreto/memcached/sharded-memcached/memcached/memcached net --no-network-setup

// qemu-system-x86_64 -name memcached0,debug-threads=on -enable-kvm -nographic -machine q35 -numa node,memdev=nmem0,nodeid=0 -cpu host,migratable=no,+invtsc,+tsc,+x2apic,+fsgsbase -smp 1,sockets=1,maxcpus=1 -numa cpu,node-id=0,socket-id=0 -m 9120M -object memory-backend-memfd,id=nmem0,merge=off,dump=on,prealloc=off,size=9120M,host-nodes=0,policy=bind,share=on -device virtio-net,netdev=nd0,mac=56:b4:44:e9:62:d0 -netdev tap,id=nd0,script=no,ifname=tap0 -drive file=qemu_disk_image_0.img,if=virtio

// qemu-system-x86_64 -name memcached0,debug-threads=on -enable-kvm -nographic -machine q35 -numa node,memdev=nmem0,nodeid=0 -cpu host,migratable=no,+invtsc,+tsc,+x2apic,+fsgsbase -smp 1,sockets=1,maxcpus=1 -numa cpu,node-id=0,socket-id=0 -m 9120M -object memory-backend-memfd,id=nmem0,merge=off,dump=on,prealloc=off,size=9120M,host-nodes=0,policy=bind,share=on -device virtio-net,netdev=nd0,mac=56:b4:44:e9:62:d0 -netdev tap,id=nd0,script=no,ifname=tap0 -drive file=qemu_disk_image_0.img,if=virtio: Invalid parameter 'id'
