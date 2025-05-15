# Website Status Checker

A concurrent website status checker written in Rust. It reads a list of URLs (from a file or command line), checks their HTTP status codes, response times, and logs the results with timestamps. Results are saved to `status_output.txt`.

## Features

- Concurrent checking using threads
- Supports at least 50 websites
- Configurable:
  - Number of worker threads
  - Timeout per request
  - Number of retries
- Collects:
  - HTTP status code
  - Response time
  - UTC timestamp


## Build Instructions


```sh
cargo build --release
```
To run the program make sure you specify number of worker threads (e.g., 8), timeout (e.g., 4 seconds), and retries (e.g., 2) when running cargo run:
```sh
cargo run -- --file sites.txt --timeout 5 --retries 2 --workers 10
```
