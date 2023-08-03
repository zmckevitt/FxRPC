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

## Fxmark gRPC Program

The Fxmark gRPC program is located in the ```prog/``` directory.

### Building

This project contains a client/server library for distributed syscalls using gRPC. To build the project, first install the necessary dependencies.
Rust:
```
curl --proto '=https' --tlsv1.3 https://sh.rustup.rs -sSf | sh
source "$HOME/.cargo/env"
```
gRPC:
```
sudo apt install protobuf-compiler
```
And then build with the nightly rust toolchain:
```
rustup default nightly
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

The code to automatically emulate and benchmark the Fxmark gRPC program is located in ```run/```.

To run this, first install necessary python libraries:

```
pip install py-libnuma
```

To run the benchmarks with a qemu emulation layer (requires preconfigured disk image - see CONFIGURATION.md):
```
cargo run -- --image <path to disk image> --scores <cores for server> --wratio <write ratios> --openf <open files> --duration <experiment duration> --csv <optional alternate csv output>
```
Normally, for benchmarks, we run with configuration:
```
cargo run -- --image <path to disk image> --scores 4 --wratio 0 --openf 1 --duration 20
```
Note: the program writes and removes ephemeral disk images to/from ```/tmp```.
