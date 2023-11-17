// Copyright © 2021 VMware, Inc. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::path::Path;

use hwloc2::{ObjectType, Topology, TopologyObject};
// use lazy_static::lazy_static;

/// Environment variable that points to machine config (for baremetal booting)
const BAREMETAL_MACHINE: &'static str = "BAREMETAL_MACHINE";

/// Different machine types we can run on.
#[derive(Eq, PartialEq, Debug, Clone)]
pub enum Machine {
    /// A bare-metal machine identified by a string.
    /// The name is described in the corresponding TOML file.
    ///
    /// (e.g., Machine::BareMetal("b1542".into()) should have a corresponding b1542.toml file).
    Baremetal(String),
    /// Run on a virtual machine with QEMU (machine parameters determined by current host)
    Qemu,
}

impl Machine {
    pub fn determine() -> Self {
        match std::env::var(BAREMETAL_MACHINE) {
            Ok(name) => {
                if name.is_empty() {
                    panic!("{} enviroment variable empty.", BAREMETAL_MACHINE);
                }
                if !Path::new(&name).exists() {
                    panic!(
                        "'{}.toml' file not found. Check {} enviroment variable.",
                        name, BAREMETAL_MACHINE
                    );
                }
                Machine::Baremetal(name)
            }
            _ => Machine::Qemu,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Machine::Qemu => "qemu",
            Machine::Baremetal(s) => s.as_str(),
        }
    }

    /// Return a set of cores to run benchmark, run fewer total iterations
    /// and instead more with high core counts.
    pub fn _thread_defaults_low_mid_high(&self) -> Vec<usize> {
        if cfg!(feature = "smoke") {
            return vec![1, 4];
        }

        let uniform_threads = self.thread_defaults_uniform();
        let mut threads = Vec::with_capacity(6);

        for low in uniform_threads.iter().take(2) {
            threads.push(*low);
        }

        let mid = uniform_threads.len() / 2;
        if let Some(e) = uniform_threads.get(mid) {
            threads.push(*e);
        }

        for high in uniform_threads.iter().rev().take(3) {
            threads.push(*high);
        }

        threads.sort_unstable();
        threads.dedup();

        threads
    }

    /// Return a set of cores to run benchmark on sampled uniform between
    /// 1..self.max_cores().
    pub fn thread_defaults_uniform(&self) -> Vec<usize> {
        if cfg!(feature = "smoke") {
            return vec![1, 4];
        }

        let max_cores = self.max_cores();
        let nodes = self.max_numa_nodes();

        let mut threads = Vec::with_capacity(12);
        // On larger machines thread increments are bigger than on smaller
        // machines:
        let thread_incremements = if max_cores > 96 {
            16
        } else if max_cores > 24 {
            8
        } else if max_cores > 16 {
            4
        } else {
            2
        };

        for t in (0..(max_cores + 1)).step_by(thread_incremements) {
            if t == 0 {
                // Can't run on 0 threads
                threads.push(t + 1);
            } else {
                threads.push(t);
            }
        }

        threads.push(max_cores / nodes);
        threads.push(max_cores);

        threads.sort_unstable();
        threads.dedup();

        threads
    }

    pub fn max_cores(&self) -> usize {
        if let Machine::Qemu = self {
            let topo = Topology::new().expect("Can't retrieve System topology?");
            topo.objects_with_type(&ObjectType::Core)
                .map_or(1, |cpus| cpus.len())
        } else {
            match self.name() {
                "l0318" => 96,
                "b1542" => 28,
                _ => unreachable!("unknown machine"),
            }
        }
    }

    pub fn max_numa_nodes(&self) -> usize {
        if let Machine::Qemu = self {
            let topo = Topology::new().expect("Can't retrieve System topology?");
            // TODO: Should be ObjectType::NUMANode but this fails in the C library?
            topo.objects_with_type(&ObjectType::Package)
                .map_or(1, |nodes| nodes.len())
        } else {
            match self.name() {
                "l0318" => 4,
                "b1542" => 2,
                _ => unreachable!("unknown machine"),
            }
        }
    }

    pub fn _rackscale_core_affinity(&self, cores_per_vm: Vec<usize>) -> Vec<(usize, Vec<u32>)> {
        let max_cores = self.max_cores();
        let max_numa_nodes = self.max_numa_nodes();

        // Sanity checking
        assert!(max_cores % max_numa_nodes == 0);
        assert!(cores_per_vm.iter().sum::<usize>() <= max_cores);

        // Get cores by NUMA node
        let topo = Topology::new().expect("Can't retrieve system topology");
        let packages = topo
            .objects_with_type(&ObjectType::Package)
            .expect("Failed to get packages");
        assert!(max_numa_nodes == packages.len());
        let mut cpus_by_node = Vec::new();
        for package in packages {
            let mut cores = self.get_cores(package, Vec::new());
            cores.sort();
            cpus_by_node.push(cores);
        }

        // This could maybe be a proper bin packing problem, but we'll just
        // use a naive round-robin of nodes instead
        let mut node_indices = vec![0; max_numa_nodes];
        let mut placement_cores = Vec::new();
        let mut node_index = 0;

        for vm_cores in cores_per_vm {
            // There is room on this node
            let start_index = node_indices[node_index];
            let end_index = start_index + vm_cores;
            if end_index > cpus_by_node[node_index].len() {
                panic!("No room on node for VM??");
            }
            placement_cores.push((
                node_index,
                cpus_by_node[node_index][start_index..end_index].to_vec(),
            ));
            node_indices[node_index] = end_index;
            node_index = (node_index + 1) % max_numa_nodes;
        }

        // Returns an array of cores per vm that the vm should be pinned to.
        return placement_cores;
    }

    /// This mimics the output of corealloc -c max_cores -t interleave
    fn get_cores<'a>(&self, to: &'a TopologyObject, mut cores: Vec<u32>) -> Vec<u32> {
        if to.object_type() == ObjectType::Core {
            // Choose the id of the lowest processing unit
            let mut ids = Vec::new();
            for child in to.children() {
                ids.push(child.os_index());
            }
            cores.push(*ids.iter().min().unwrap());
        } else {
            for child in to.children() {
                cores = self.get_cores(child, cores);
            }
        }
        cores
    }
}
