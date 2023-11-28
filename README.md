# Fxmark gRPC

Distributed fxmark benchmark using gRPC. Project uses gRPC to pass basic file related syscalls from a client to a file server that executes the relevent syscalls and returns the result to the client. Currently, this program supports the following syscalls and file system operations:

- Open
- Read/pRead
- Write/pWrite
- Close
- Remove
- Fsync
- Mkdir
- Rmdir

This project will automatically deploy virtualized Ubuntu images to run the benchmarking suite via qemu, and log the results of the benchmark to the ```run/``` directory.

## System dependencies

First, install Rust with nightly:
```
curl --proto '=https' --tlsv1.3 https://sh.rustup.rs -sSf | sh
source "$HOME/.cargo/env"
rustup default nightly
```
Next, add yourself to the kvm group:
```
sudo adduser [username] kvm
```
You will need to reset your shell for this to take effect:
```
exit
```
To build the Fxmark gRPC program in ```prog/```, you will need the following dependencies:
```
sudo apt install protobuf-compiler -y
```
To run the automated benchmark runner in ```run/```, you will need the python numa package:
```
pip install py-libnuma
```

## Fxmark gRPC Program

The Fxmark gRPC program is located in the ```prog/``` directory.

### Building

This project contains a client/server library for distributed syscalls using gRPC. To build the project, first install the necessary dependencies, then build with the nightly rust toolchain:
```
cargo build --release
```

### Running mixXX Benchmarks

This project makes use of the ```mixXX``` benchmarks from the Fxmark filesystem benchmark suite. Note: the client is currently hardcoded to expect the server to be running on port 8080.

To run a local version of the benchmarks (client and server running locally) with write ratios of 0 and 10, 1 open file, and 10s duration:
```
cargo run -- --mode loc_server --port 8080 
cargo run -- --mode loc_client --wratio 0 10 --openf 1 --duration 10
```

### Testing

To run unit tests for various syscalls and directory operations, first initialize the file system:
```
echo "ReadTest" > /dev/shm/read_test.txt
```
Run the server:
```
cargo run -- --mode loc_server --port 8080
```
Run the tests:
```
cargo test
```

## Benchmarking Fxmark gRPC

### Huge Page Configuration

Install the dependencies:
```bash
sudo apt install -y libhugetlbfs-dev libhugetlbfs-bin
```

Before running any benchmarks, it's necessary to setup up huge pages.
First, you'll want to ensure the default huge page size by running:
```bash
cat /proc/meminfo | grep -i hugepage
```
You want to see:
```
Hugepagesize:       2048 kB
```

Next, you can see what pages are preallocated with:
```bash
numastat -m
```

Generally, 32768.00 MB (or 16384 2 MB pages) per node is more than enough (assuming no more than 24 cores per node).
Run as many of the following commands as you have numa nodes to preallocate the pages:

```bash
echo 16384 | sudo numactl -m 0 tee -a /proc/sys/vm/nr_hugepages_mempolicy
echo 32768 | sudo numactl -m 1 tee -a /proc/sys/vm/nr_hugepages_mempolicy
echo 49152 | sudo numactl -m 2 tee -a /proc/sys/vm/nr_hugepages_mempolicy
echo 65536 | sudo numactl -m 3 tee -a /proc/sys/vm/nr_hugepages_mempolicy
```

For memcached benches on a 4x machine, you need something more like:
```bash
echo 131072 | sudo numactl -m 0 tee -a /proc/sys/vm/nr_hugepages_mempolicy
echo 262144 | sudo numactl -m 1 tee -a /proc/sys/vm/nr_hugepages_mempolicy
echo 393216 | sudo numactl -m 2 tee -a /proc/sys/vm/nr_hugepages_mempolicy
echo 524288 | sudo numactl -m 3 tee -a /proc/sys/vm/nr_hugepages_mempolicy
```

Rerun ```numastat -m``` to verify the pages are preallocated.

Then, you'll need to initiate the hugetlbfs with:
```bash
sudo hugeadm --create-global-mounts
```

### Running the benchmarks

The code to automatically emulate and benchmark the Fxmark gRPC program is located in ```run/```.

To run the benchmarks with a qemu emulation layer (requires preconfigured disk image - see CONFIGURATION.md):
```
cargo run -- --transport <uds or tcp> --image <path to disk image> --wratio <write ratios> --openf <open files> --duration <experiment duration> --csv <optional alternate csv output>
```
For example, to run emulated fxmark (tcp):
```
cargo run -- --transport tcp --image <path to disk image> --wratio 0 --openf 1 --duration 20
```
To run the same benchmark using uds (requires ```prog/``` to be built with ```--release```):
```
cargo run -- --transport uds --wratio 0 --openf 1 --duration 20
```
If running UDS benchmarks on a non-NUMA architecture, specify with the ```--nonuma``` flag:
```
cargo run -- --transport uds --wratio 0 --openf 1 --duration 20 --nonuma
```
Note: the program writes and removes ephemeral disk images to/from ```/tmp```.
