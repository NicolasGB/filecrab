<div align="center">
  <img src="logo.png" alt="logo" />
  <h3>⚡ A blazingly fast file and text sharing service ⚡</h3>

  ![Build](https://img.shields.io/github/actions/workflow/status/NicolasGB/filecrab/ci.yml?branch=main)
  ![Rust](https://img.shields.io/badge/rust-1.70+-blueviolet.svg?logo=rust)
  ![License](https://img.shields.io/badge/license-MIT-blue.svg)
</div>

# filecrab

File and text sharing application, built on top of [MinIO](https://min.io/) and
[SurrealDB](https://surrealdb.com/) and powered by [Rust](https://www.rust-lang.org/). You can host
your own instance, simply need a MinIO bucket and a SurrealDB  instance.

A useful [CLI](filecrab-cli) will allow you to upload files and text to your instance.

## Features

- File sharing.
- File expiration.
- One-time text sharing.
- Files **optionally** encrypted.
- Text **always** encrypted.
- Server-side cleanup of expired files via a command that can be run on the server, e.g. with a cron
  job.
- Memorable words list for IDs, inspired by
  [Magic Wormhole](https://github.com/magic-wormhole/magic-wormhole.rs).

## Security

All data is encrypted through the [age](https://github.com/str4d/rage/tree/main/age) library.
Encryption is done **client side**, in the CLI tool, this allows us to stream files directly to the
storage without the need of reading it in memory on the server. The password is **never** sent to
the server.

## Server

### Configuration

The server can be configured with environment variables, see the [example](.env.example).

### Running

You can run the application with all required services using the following commands:

```sh
# Build the filecrab Docker image.
make build
# Or if you want to build in release mode.
# make build mode=release

# Run the multi-container application.
make up
```

## CLI

### Installation

You can install the CLI with the following command:

```sh
cargo install --path filecrab-cli
```

### Usage

You can upload a file using the following command:

```sh
filecrab-cli upload <PATH>
```

You can download a file using the following command:

```sh
filecrab-cli download <ID>
```

Please refer to the help for more information:

```sh
filecrab-cli --help
```

## License

This project is licensed under the [MIT license](LICENSE).
