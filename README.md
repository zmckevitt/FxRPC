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

## Building

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
cargo build
```

## Running mixXX Benchmarks

This project makes use of the ```mixXX``` benchmarks from the Fxmark filesystem benchmark suite. This benchmark consists of a mixed read/write ratio, e.g. ```mixX10``` represents a write ratio of 10%. To run the ```mixX0 mixX10 mixX100``` benchmarks, build and run the server and client. Note: the client is currently hardcoded to expect the server to be running on port 8080.

To run a local version of the client (connecting to a server on localhost):
```
cargo run -- --mode server --port 8080 
cargo run -- --mode loc_client
```

## Testing

To run unit tests for various syscalls and directory operations, first initialize the file system:
```
echo "ReadTest" > /dev/shm/read_test.txt
```
Run the server:
```
cargo run -- --mode server --port 8080
```
Run the tests:
```
cargo test
```
