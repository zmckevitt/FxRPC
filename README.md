# Fxmark gRPC (WiP)

Distributed fxmark benchmark using gRPC. Project uses gRPC to pass basic file related syscalls from a client to a file server that executes the relevent syscalls and returns the result to the client. Currently, this program supports the following syscalls and file system operations:

- Open
- Read/pRead
- Write/pWrite
- Close
- Remove
- Fsync
- Mkdir
- Rmdir

## Building and Testing

This project contains a client/server library. To run the server: ```cargo run -- --mode server --port <port>``` and client: ```cargo run -- --mode client --duration <duration (seconds)> --type <benchmark>```. To run unit tests for various syscalls and directory operations, first run the server, and then run ```cargo test```.
