// Copyright © 2021 VMware, Inc. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! fxmark implementation for nrk.

extern crate alloc;

use std::convert::TryInto;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::num::ParseIntError;
use core::ptr;
use core::str::FromStr;
use core::sync::atomic::{AtomicUsize, Ordering};

use lazy_static::lazy_static;

pub mod utils;
use utils::topology::ThreadMapping;
use utils::topology::*;

mod mix;
use crate::fxmark::mix::MIX;

use fxmark_grpc::{BlockingClient, LogMode, ClientParams};

const PAGE_SIZE: usize = 1008;

static POOR_MANS_BARRIER: AtomicUsize = AtomicUsize::new(0);

pub const OUTPUT_FILE: &str = "fxmark_grpc_benchmark.csv";

lazy_static! {
    pub static ref MAX_OPEN_FILES: AtomicUsize = AtomicUsize::new(max_open_files());
}

pub fn _calculate_throughput(ops: u64, time: Duration) -> usize {
    let nano_duration = time.as_nanos();
    let nano_per_operation = nano_duration / ops as u128;
    (Duration::from_secs(1).as_nanos() / nano_per_operation)
        .try_into()
        .unwrap()
}

/// This struct is used for passing the core and benchmark type from
/// the command-line/integration tests.
#[derive(Debug, PartialEq)]
pub struct ARGs {
    pub cores: usize,
    pub open_files: usize,
    pub benchmark: String,
    pub write_ratio: usize,
}

/// Both command line and integration tests pass CORExBENCH(ex: 10xdhrl). Convert
/// the string to the struct which can be used in the benchmarks.
impl FromStr for ARGs {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let coords: Vec<&str> = s.split('X').collect();

        let x_fromstr = coords[0].parse::<usize>()?;
        let open_files = coords[1].parse::<usize>()?;
        let benchmark = coords[2].to_string();
        let write_ratio = coords[3].parse::<usize>()?;
        Ok(ARGs {
            cores: x_fromstr,
            open_files,
            benchmark,
            write_ratio,
        })
    }
}

pub trait Bench {
    fn init(&self, cores: Vec<u64>, open_files: usize, client: &mut Arc<Mutex<BlockingClient>>);
    fn run(
        &self,
        barrier: &AtomicUsize,
        duration: u64,
        core: usize,
        write_ratio: usize,
        client: &mut Arc<Mutex<BlockingClient>>,
    ) -> Vec<usize>;
}

unsafe extern "C" fn fxmark_bencher_trampoline<T>(
    arg: *mut u8,
    cores: usize,
    core_id: usize,
    duration: u64,
    client: &mut Arc<Mutex<BlockingClient>>,
    client_params: ClientParams
) -> *mut u8
where
    T: Bench + Default + core::marker::Send + core::marker::Sync + 'static + core::clone::Clone,
{
    let bench: Arc<MicroBench<T>> = Arc::from_raw(arg as *const MicroBench<_>);
    bench.fxmark_bencher(
        cores,
        core_id,
        bench.benchmark,
        bench.write_ratio,
        bench.open_files,
        duration,
        client,
        client_params,
    );
    ptr::null_mut()
}

#[derive(Clone)]
struct MicroBench<'a, T>
where
    T: Bench + Default + core::marker::Send + core::marker::Sync + 'static + core::clone::Clone,
{
    thread_mappings: Vec<ThreadMapping>,
    threads: Vec<usize>,
    benchmark: &'a str,
    write_ratio: usize,
    open_files: usize,
    bench: T,
}

impl<'a, T> MicroBench<'a, T>
where
    T: Bench + Default + core::marker::Send + core::marker::Sync + 'static + core::clone::Clone,
{
    pub fn new(
        benchmark: &'static str,
        write_ratio: usize,
        open_files: usize,
        client_params: &ClientParams,
    ) -> MicroBench<'a, T> {

        let mapping = ThreadMapping::Sequential;
        let max_cores = (*client_params).ccores;

        let mut threads = Vec::new();

        threads.push(max_cores);

        let mut thread_mapping = Vec::new();
        thread_mapping.push(mapping);

        MicroBench {
            thread_mappings: thread_mapping,
            threads: threads,
            benchmark,
            write_ratio,
            open_files,
            bench: Default::default(),
        }
    }

    fn fxmark_bencher(
        &self,
        cores: usize,
        core_id: usize,
        benchmark: &str,
        write_ratio: usize,
        open_files: usize,
        duration: u64,
        client: &mut Arc<Mutex<BlockingClient>>,
        client_params: ClientParams,
    ) {
        // let bench_duration_secs = if cfg!(feature = "smoke") { 1 } else { 10 };
        let bench_duration_secs = duration;
        let iops = self.bench.run(
            &POOR_MANS_BARRIER,
            bench_duration_secs,
            core_id,
            write_ratio,
            client,
        );

        let mut csv_file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(OUTPUT_FILE)
            .expect("Cant open output file");
        for iteration in 1..(bench_duration_secs + 1) {
            let row = format!(
                "{},{:?},{},{},{},{},{},{},{},{},{}\n",
                core_id + (client_params.ccores * client_params.cid),
                benchmark,
                cores * client_params.nclients,
                write_ratio,
                open_files,
                bench_duration_secs,
                iteration,
                iops[iteration as usize],
                client_params.cid,
                client_params.ccores,
                client_params.nclients,
            );

            match client_params.log_mode {
                LogMode::CSV => {
                    let r = csv_file.write(row.as_bytes());
                    assert!(r.is_ok());
                }
                LogMode::STDOUT => {
                    print!("{}", row);
                }
            }
        }
    }
}

pub fn max_open_files() -> usize {
    let topology = MachineTopology::new();
    topology.cores()
}

pub fn bench(
    benchmark: String,
    open_files: usize,
    write_ratio: usize,
    duration: u64,
    client_params: &ClientParams,
    client: Arc<Mutex<BlockingClient>>,
) {
    fn start<
        T: Bench + Default + core::marker::Send + core::marker::Sync + 'static + core::clone::Clone,
    >(
        microbench: MicroBench<'static, T>,
        open_files: usize,
        write_ratio: usize,
        duration: u64,
        mut client: Arc<Mutex<BlockingClient>>,
        client_params: &ClientParams,
    ) {
        let thread_mappings = microbench.thread_mappings.clone();
        let threads = microbench.threads.clone();

        for tm in thread_mappings.iter() {
            for ts in threads.iter() {
                let topology = MachineTopology::new();
                utils::disable_dvfs();

                let cpus = topology.allocate(*tm, *ts, false);
                let cores: Vec<u64> = cpus.iter().map(|c| c.cpu).collect();
                let clen = cores.len();

                if matches!(client_params.log_mode, LogMode::CSV) {
                    println!(
                        "Run Benchmark={} TM={} Cores={}; Write-Ratio={} Open-Files={}",
                        microbench.benchmark, *tm, ts, write_ratio, open_files
                    );
                }

                // currently we'll run out of 4 KiB frames
                let mut thandles = Vec::with_capacity(clen);
                // Set up barrier
                POOR_MANS_BARRIER.store(clen, Ordering::SeqCst);

                for core_id in cores.clone() {
                    let mb = Arc::new(microbench.clone());
                    mb.bench.init(cores.clone(), open_files, &mut client);

                    let mut client1 = client.clone();
                    let bench_duration = duration.clone();
                    let params = (*client_params).clone();
                    thandles.push(thread::spawn(move || {
                        utils::pin_thread(core_id);
                        let arg = Arc::into_raw(mb) as *const _ as *mut u8;
                        unsafe {
                            fxmark_bencher_trampoline::<T>(
                                arg,
                                clen,
                                core_id as usize,
                                bench_duration,
                                &mut client1,
                                params,
                            );
                        }
                    }));
                }

                for thandle in thandles {
                    let _ = thandle.join();
                }
            }
        }
    }

    if benchmark == "mix" {
        let mb = MicroBench::<MIX>::new("mix", write_ratio, open_files, client_params);
        start::<MIX>(mb, open_files, write_ratio, duration, client, client_params);
    }
}
