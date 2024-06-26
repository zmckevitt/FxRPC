# FxRPC

This project contains a distributed filesystem benchmark where clients forward system calls over RPCs to a centralized file server. Currently, FxRPC supports gRPC and Dinos-RPC libraries.

## System dependencies

### Building Fxmark
The following dependencies are required to build FxRPC locally. For running emulated benchmarks, see the requirements below. Note that one must build Fxmark locally to run emulated UDS benchmarks.

This project uses submodules, so initialize them first:
```
git submodule update --init
```
Rust (using nightly) can be installed as follows:
```
curl --proto '=https' --tlsv1.3 https://sh.rustup.rs -sSf | sh
source "$HOME/.cargo/env"
rustup default nightly
```
To build the FxRPC program in ```fxmark/```, you will need the following dependencies for gRPC:
```
sudo apt install protobuf-compiler -y
```
Lastly, make sure to build with the release flag:
```
cd fxmark
cargo build --release
```


### Emulation Mode

To run emulated FxRPC, you will need the python numa package:
```
pip install py-libnuma
```
To run with QEMU emulation, you must add yourself to the kvm group (you will need to reset your shell for this to take effect):
```
sudo adduser [username] kvm
```
Emulation mode requires a disk image configured to auto-login as root and start the FxRPC program. This image can be created automatically:
```
sudo apt install cloud-image-utils
python3 tools/create_disk_image.py
```
This will automatically create a new ubuntu disk image, ```ubuntu-server-cloudimg-amd64.img```, based off of the precompiled Fxmark binary in ```run/bin```. For information on configuring this disk image manually, see ```CONFIGURATION.md```.

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

## Running Benchmarks

### Local Fxmark instance

This project makes use of the ```mixXX``` benchmarks for varying read/write ratios. The crate expects the following options when running natively:
```
cargo run -- 
--mode <"client", "server">
--rpc <"drpc", "grpc">
--transport <"tcplocal", "tcpremote", "uds">
--port <optional, defaults to 8080>
--wratio <space separated list of write ratios>
--openf <number of open files>
--duration <benchmark duration in seconds>
-o <output file>
```
Where ```mode``` specifies client/server modality, ```rpc``` distinguishes between gRPC and Dinos-RPC libraries, and ```transport``` specifies which transport protocol/bind address to use: ```tcplocal``` establishes a tcp connection on localhost, ```tcpremote``` establishes a pseudo-remote tcp connection using bridge interfaces (used for emulation mode), and ```uds``` uses Unix Domain Sockets.

Additionally, the client can specify the benchmark parameters: ```wratio``` sets the ratio of writes and can take multiple values (defaults to 50%), ```openf``` specifies the number of open files (defaults to 1), and ```duration``` specifies the duration of the benchmark in seconds (defaults to 10).

For example, a local FxRPC benchmark using Dinos-RPC, 0% and 10% write ratios, 1 open file, for 10 seconds, can be run with the following commands:
```
cargo run -- --mode=server --transport=tcplocal --rpc=drpc
cargo run -- --mode=client --transport=tcplocal --rpc=drpc --wratio 0 10 --openf 1 --duration 10
```

If no output file is specified, benchmark data will be written to ```fxrpc_bench.csv```.

### Running Emulated benchmarks

The code to automatically emulate and benchmark the FxRPC program is located in ```run/```.

To run the benchmarks with a qemu emulation layer (requires preconfigured disk image - see CONFIGURATION.md):
```
cargo run -- --transport <uds or tcp> --rpc <grpc or drpc> --image <path to disk image> --wratio <write ratios> --openf <open files> --duration <experiment duration> --csv <optional alternate csv output>
```
For example, to run emulated fxmark (tcp) over grpc:
```
cargo run -- --transport tcp --rpc grpc --image <path to disk image> --wratio 0 --openf 1 --duration 20
```
To run the same benchmark using uds (requires ```fxmark/``` to be built locally with ```--release``` flag):
```
cargo run -- --transport uds --rpc <drpc or grpc> --wratio 0 --openf 1 --duration 20
```
If running benchmarks on a non-NUMA architecture, specify with the ```--nonuma``` flag, for example:
```
cargo run -- --transport uds --rpc <drpc or grpc> --wratio 0 --openf 1 --duration 20 --nonuma
```
Note: the program writes and removes ephemeral disk images to/from ```/tmp```.
