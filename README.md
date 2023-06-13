# Fxmark gRPC (WiP)

Distributed fxmark benchmark using gRPC. Project uses gRPC to pass basic file related syscalls from a client to a file server that executes the relevent syscalls and returns the result to the client. Currently, this program supports the following syscalls:

- Open
- Read
- Write
- Close
- Remove

## Building and Testing

The project currently contains a server implementation with a client library. To run the unit tests, first start the server: ```cargo run``` and then start the unit tests: ```cargo test```.
