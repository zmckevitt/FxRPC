// use clap::{crate_version, value_t, App, Arg};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Barrier};
use std::thread;

mod utils;
use utils::topology::ThreadMapping;
use utils::topology::*;

mod bench;
use bench::{
    drbh::DRBH, drbl::DRBL, dwal::DWAL, dwol::DWOL, dwom::DWOM, mix::MIX, mwrl::MWRL, mwrm::MWRM,
};

pub trait Bench {
    fn init(&self, cores: Vec<u64>, open_file: usize);
    fn run(&self, b: Arc<Barrier>, duration: u64, core: u64, write_ratio: usize) -> Vec<usize>;
}

struct BenchMark<T>
where
    T: Bench + Default + std::marker::Send + std::marker::Sync + 'static + std::clone::Clone,
{
    /// Thread assignments.
    thread_mappings: Vec<ThreadMapping>,
    /// Threads-ids to execute the benchmark.
    threads: Vec<usize>,
    /// Benchmark to run.
    bench: T,
}

impl<T> BenchMark<T>
where
    T: Bench + Default + std::marker::Send + std::marker::Sync + 'static + std::clone::Clone,
{
    pub fn new() -> BenchMark<T> {
        BenchMark {
            thread_mappings: Vec::new(),
            threads: Vec::new(),
            bench: Default::default(),
        }
    }

    pub fn thread_defaults(&mut self) -> &mut Self {
        let topology = MachineTopology::new();
        let max_cores = topology.cores() / 2; // Disable hyperthreads

        // On larger machines thread increments are bigger than on
        // smaller machines:
        let thread_incremements = if max_cores > 90 {
            8
        } else if max_cores > 24 {
            4
        } else if max_cores > 16 {
            4
        } else {
            2
        };

        for t in (0..(max_cores + 1)).step_by(thread_incremements) {
            if t == 0 {
                // Can't run on 0 threads
                self.threads(t + 1);
            } else {
                self.threads(t);
            }
        }

        /* Go in increments of one around "interesting" socket boundaries
        let sockets = topology.sockets();
        let cores_on_s0 = topology.cpus_on_socket(sockets[0]);
        let cores_per_socket = cores_on_s0.len();
        for i in 0..sockets.len() {
            let multiplier = i + 1;
            fn try_add(to_add: usize, max_cores: usize, cur_threads: &mut Vec<usize>) {
                if !cur_threads.contains(&to_add) && to_add <= max_cores {
                    cur_threads.push(to_add);
                }
            }

            let core_socket_boundary = multiplier * cores_per_socket;
            try_add(core_socket_boundary - 1, max_cores, &mut self.threads);
            try_add(core_socket_boundary, max_cores, &mut self.threads);
            try_add(core_socket_boundary + 1, max_cores, &mut self.threads);
        }*/

        self.threads.sort();
        self
    }

    /// Run benchmark with `t` threads.
    pub fn threads(&mut self, t: usize) -> &mut Self {
        self.threads.push(t);
        self
    }

    /// Run benchmark with given thread <-> machine mapping.
    pub fn thread_mapping(&mut self, tm: ThreadMapping) -> &mut Self {
        self.thread_mappings.push(tm);
        self
    }

    pub fn report_bench(
        &self,
        name: &str,
        duration: u64,
        file_name: &str,
        ts: usize,
        write_ratio: usize,
        open_files: usize,
        result: &HashMap<u64, Vec<usize>>,
    ) {
        // Append parsed results to a CSV file
        let write_headers = !Path::new(file_name).exists();
        let mut csv_file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(file_name)
            .expect("Can't open file");
        if write_headers {
            let row =
                "git_rev,thread_id,benchmark,ncores,write_ratio,open_files,duration_total,duration,operations\n";
            let r = csv_file.write(row.as_bytes());
            assert!(r.is_ok());
        }

        let mut result: Vec<(u64, Vec<usize>)> =
            result.iter().map(|(k, v)| (*k, v.clone())).collect();
        result.sort_by(|a, b| a.0.cmp(&b.0));
        for (core, iops) in result.iter() {
            let mut time = 1;
            for ops in iops {
                // TODO: need to fix this
                // let r = csv_file.write(format!("{},", std::env!("GIT_HASH")).as_bytes());
                // assert!(r.is_ok());
                let r = csv_file.write(
                    format!(
                        "{},{:?},{},{},{},{},{},{}",
                        *core, name, ts, write_ratio, open_files, duration, time, ops
                    )
                    .as_bytes(),
                );
                assert!(r.is_ok());
                let r = csv_file.write("\n".as_bytes());
                assert!(r.is_ok());
                time += 1;
            }
        }
    }

    /// Start the benchmark
    pub fn start(
        &mut self,
        duration: u64,
        name: &str,
        file_name: &str,
        write_ratio: usize,
        open_files: usize,
    ) {
        let topology = MachineTopology::new();
        utils::disable_dvfs();

        for tm in self.thread_mappings.iter() {
            for ts in self.threads.iter() {
                let cpus = topology.allocate(*tm, *ts, false);
                let cores: Vec<u64> = cpus.iter().map(|c| c.cpu).collect();
                let mut result: HashMap<u64, Vec<usize>> = HashMap::with_capacity(*ts);
                println!(
                    "Run Benchmark={} TM={} Cores={}; Write-Ratio={} Open-Files={}",
                    name, *tm, ts, write_ratio, open_files
                );

                // Need a barrier to synchronize starting of threads
                let barrier = Arc::new(Barrier::new(*ts));
                let mut children = Vec::new();
                self.bench.init(cores.clone(), open_files);

                for core in cores {
                    let b = barrier.clone();
                    let bench = self.bench.clone();

                    children.push(thread::spawn(move || {
                        utils::pin_thread(core);
                        (core, bench.run(b, duration, core, write_ratio))
                    }));
                }

                // Wait for the thread to finish. Returns a result.
                for child in children {
                    let (core, iops) = child.join().unwrap();
                    result.insert(core, iops);
                }
                self.report_bench(
                    name,
                    duration,
                    file_name,
                    *ts,
                    write_ratio,
                    open_files,
                    &result,
                );
            }
        }
    }
}

pub fn run_benchmarks(duration: u64, versions: Vec<&str>) {
    // Example to run the benchmark for 10 seconds, type drbl and drbh
    // `cargo bench --bench fxmark -- --duration 10 --type drbl drbh`
    /*
    let args = std::env::args().filter(|e| e != "--bench");
    let matches = App::new("Fxmark file-system benchmark")
        .version(crate_version!())
        .author("Jon Gjengset <jon@thesquareplanet.com>, Gerd Zellweger <mail@gerdzellweger.com>")
        .about("Benchmark file-systems using different levels of read-write contention")
        .arg(
            Arg::with_name("duration")
                .short("d")
                .long("duration")
                .required(true)
                .help("Duration for each run")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("type")
                .short("t")
                .long("type")
                .multiple(true)
                .takes_value(true)
                .required(true)
                .possible_values(&[
                    "drbl", "drbh", "dwol", "dwom", "dwal", "mwrl", "mwrm", "mixX0", "mixX1",
                    "mixX5", "mixX10", "mixX20", "mixX40", "mixX60", "mixX80", "mixX100",
                ])
                .help("Benchmark to run."),
        )
        .get_matches_from(args);

    let duration = value_t!(matches, "duration", u64).unwrap_or_else(|e| e.exit());
    let versions: Vec<&str> = match matches.values_of("type") {
        Some(iter) => iter.collect(),
        None => unreachable!(),
    };
    */
    let file_name = "fxmark_grpc_benchmark.csv";
    //let _ret = std::fs::remove_file(file_name);
    let thread_mapping = ThreadMapping::Sequential;
    let default_write_ratio = 0;
    let default_open_files = 0;

    // Read a block in a private file
    if versions.contains(&"drbl") {
        BenchMark::<DRBL>::new()
            .thread_defaults()
            .thread_mapping(thread_mapping)
            .start(
                duration,
                "drbl",
                file_name,
                default_write_ratio,
                default_open_files,
            );
    }

    // Read a shared block in a shared file
    if versions.contains(&"drbh") {
        BenchMark::<DRBH>::new()
            .thread_defaults()
            .thread_mapping(thread_mapping)
            .start(
                duration,
                "drbh",
                file_name,
                default_write_ratio,
                default_open_files,
            );
    }

    // Overwrite a block in a private file
    if versions.contains(&"dwol") {
        BenchMark::<DWOL>::new()
            .thread_defaults()
            .thread_mapping(thread_mapping)
            .start(
                duration,
                "dwol",
                file_name,
                default_write_ratio,
                default_open_files,
            );
    }

    // Overwrite a private block in a shared file
    if versions.contains(&"dwom") {
        BenchMark::<DWOM>::new()
            .thread_defaults()
            .thread_mapping(thread_mapping)
            .start(
                duration,
                "dwom",
                file_name,
                default_write_ratio,
                default_open_files,
            );
    }

    // Append a block in a private file
    if versions.contains(&"dwal") {
        BenchMark::<DWAL>::new()
            .thread_defaults()
            .thread_mapping(thread_mapping)
            .start(
                duration,
                "dwal",
                file_name,
                default_write_ratio,
                default_open_files,
            );
    }

    // Rename a private file in a private directory
    if versions.contains(&"mwrl") {
        BenchMark::<MWRL>::new()
            .thread_defaults()
            .thread_mapping(thread_mapping)
            .start(
                duration,
                "mwrl",
                file_name,
                default_write_ratio,
                default_open_files,
            );
    }

    // Move a private file to a shared directory
    if versions.contains(&"mwrm") {
        BenchMark::<MWRM>::new()
            .thread_defaults()
            .thread_mapping(thread_mapping)
            .start(
                duration,
                "mwrm",
                file_name,
                default_write_ratio,
                default_open_files,
            );
    }

    if versions.contains(&"mixX0")
        || versions.contains(&"mixX1")
        || versions.contains(&"mixX5")
        || versions.contains(&"mixX10")
        || versions.contains(&"mixX20")
        || versions.contains(&"mixX40")
        || versions.contains(&"mixX60")
        || versions.contains(&"mixX80")
        || versions.contains(&"mixX100")
    {
        fn open_files_default() -> Vec<usize> {
            let topology = MachineTopology::new();
            let sockets = topology.sockets();
            let mut cpus = topology.cpus_on_socket(sockets[0]);
            cpus.sort_by_key(|c| c.core);
            cpus.dedup_by(|a, b| a.core == b.core);
            let max_cpus_s0 = cpus.len();

            let mut open_files = Vec::new();
            for of in (0..(max_cpus_s0 + 1)).step_by(4) {
                if of == 0 {
                    open_files.push(1);
                } else {
                    open_files.push(of);
                }
            }

            if *open_files.last().unwrap() != max_cpus_s0 {
                open_files.push(max_cpus_s0);
            }

            open_files.clone()
        }

        use core::num::ParseIntError;
        use core::str::FromStr;
        struct Args {
            write_ratio: usize,
        }

        impl FromStr for Args {
            type Err = ParseIntError;

            fn from_str(input: &str) -> Result<Self, Self::Err> {
                let coords: Vec<&str> = input.split('X').collect();
                let write_ratio = coords[1].parse::<usize>()?;
                Ok(Args { write_ratio })
            }
        }

        let open_files = open_files_default();
        let write_ratio = Args::from_str(versions[0]).unwrap().write_ratio;

        for open_file in &open_files {
            BenchMark::<MIX>::new()
                .thread_defaults()
                .thread_mapping(thread_mapping)
                .start(duration, "mix", file_name, write_ratio, *open_file);
        }
    }
}