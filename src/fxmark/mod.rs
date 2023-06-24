// Copyright © 2021 VMware, Inc. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! fxmark implementation for nrk.

extern crate alloc;

use std::convert::TryInto;
use std::time::Duration;
use std::thread;
use std::fs::OpenOptions;
use std::io::Write;

use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::num::ParseIntError;
use core::ptr;
use core::str::FromStr;
use core::sync::atomic::{AtomicUsize, Ordering};

use lazy_static::lazy_static;

mod utils;
use utils::topology::ThreadMapping;
use utils::topology::*;

mod mix;
use crate::fxmark::mix::MIX;

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
    fn init(&self, cores: Vec<u64>, open_files: usize);
    fn run(
        &self,
        barrier: &AtomicUsize,
        duration: u64,
        core: usize,
        write_ratio: usize,
    ) -> Vec<usize>;
}

unsafe extern "C" fn fxmark_bencher_trampoline<T>(arg: *mut u8, cores: usize, core_id: usize) -> *mut u8
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
    );
    ptr::null_mut()
}

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
    ) -> MicroBench<'a, T> {
        
        let mapping = ThreadMapping::Sequential;
        let topology = MachineTopology::new();
        let max_cores = topology.cores() / 2;

        let thread_increments = if max_cores > 90 {
            8
        } else if max_cores > 24 {
            4
        } else if max_cores > 16 {
            4
        } else {
            2
        };
       
        let mut threads = Vec::new();
 
        for t in (0..(max_cores+1)).step_by(thread_increments) {
            if t == 0 {
                threads.push(t+1);
            } else {
                threads.push(t);
            }
        }

        threads.sort();

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

    fn fxmark_bencher(&self, 
                      cores: usize, 
                      core_id: usize, 
                      benchmark: &str, 
                      write_ratio: usize, 
                      open_files: usize) {

        let bench_duration_secs = if cfg!(feature = "smoke") { 1 } else { 10 };
        let iops = self.bench.run(
            &POOR_MANS_BARRIER,
            bench_duration_secs,
            core_id,
            write_ratio,
        );

        let mut csv_file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(OUTPUT_FILE)
            .expect("Cant open output file");
        for iteration in 1..(bench_duration_secs + 1) {
            let r = csv_file.write(
                format!(
                    "{},{:?},{},{},{},{},{},{}\n",
                    core_id,
                    benchmark,
                    cores,
                    write_ratio,
                    open_files,
                    bench_duration_secs,
                    iteration,
                    iops[iteration as usize]
                )
                .as_bytes()
            );
            assert!(r.is_ok());
            //let r = csv_file.write("\n".as_bytes());
            //assert!(r.is_ok());
        }
    }
}

pub fn max_open_files() -> usize {
    let topology = MachineTopology::new();
    topology.cores() / 2
}

pub fn bench(open_files: usize, benchmark: String, write_ratio: usize) {
    // info!("thread_id,benchmark,core,write_ratio,open_files,duration_total,duration,operations");

    fn start<
        T: Bench + Default + core::marker::Send + core::marker::Sync + 'static + core::clone::Clone,
    >(
        microbench: Arc<MicroBench<'static, T>>,
        open_files: usize,
        write_ratio: usize,
    ) {

        let thread_mappings = microbench.thread_mappings.clone();
        let threads = microbench.threads.clone();

        for tm in thread_mappings.iter() {
            for ts in threads.iter() {

                let topology = MachineTopology::new();
                utils::disable_dvfs();

                let cpus = topology.allocate(*tm, *ts, false);
                let cores: Vec<u64> = cpus.iter().map(|c| c.cpu).collect();

                println!(
                    "Run Benchmark={} TM={} Cores={}; Write-Ratio={} Open-Files={}",
                    microbench.benchmark, *tm, ts, write_ratio, open_files
                );

                // currently we'll run out of 4 KiB frames
                let mut thandles = Vec::with_capacity(cores.len());
                // Set up barrier
                POOR_MANS_BARRIER.store(cores.len(), Ordering::SeqCst);
                
                let mb = microbench.clone();
                mb.bench.init(cores.clone(), open_files);
                // microbench.cores = cores.len();

                let clen = cores.len();

                unsafe {
                    for core_id in cores {
                        let mb1 = mb.clone();
                        thandles.push(thread::spawn(move || {
                                utils::pin_thread(core_id);
                                let arg = Arc::into_raw(mb1) as *const _ as *mut u8;
                                fxmark_bencher_trampoline::<T>(arg, clen, core_id as usize);
                        }));
                    }
                }

                for thandle in thandles {
                    let _ = thandle.join();
                }
            }
        }
    }


    if benchmark == "mix" {
        let mb = MicroBench::<MIX>::new(
                "mix",
                write_ratio,
                open_files,
            );
        let microbench = Arc::new(
            mb,
        );
        // microbench.bench.init(cores.clone(), open_files);
        start::<MIX>(microbench, open_files, write_ratio);
    }
}
