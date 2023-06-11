# Fxmark gRPC (WiP)

Distributed fxmark benchmark using gRPC. Project uses gRPC to pass basic file related syscalls from a client to a file server that executes the relevent syscalls and returns the result to the client. Currently, this program supports the following syscalls:

- Open
- Read
- Write
- Close
- Remove

## Building and Testing

The project currently contains a classic client/server implementation with a basic skeleton client built in the ```client/``` directory. However, the ```server/tests/``` directory contains various mini clients to act as unit tests to evaluate the server's functionality. To run these tests, first start the server: ```cd server && cargo run``` and then run the tests: ```cd server && cargo test```. To run the client and server separately, first start the server: ```cd server && cargo run```, and then run the client ```cd client && cargo run```.
