# Rustus

[Tus](https://tus.io/) protocol implementation written in Rust.

## Features

This implementation has several features to make usage as simple as possible.

* Rustus is robust, since it uses asynchronous Rust;
* It can store information about files in databases;
* You can specify directory structure to organize your uploads;
* Highly configurable;

## Installation

Since I haven't configured build automation yet, you can build it
from source using `cargo`.

```bash
cargo install --path .
```

Or you can build a docker image.

```bash
docker build --tag="rustus:latest" --cache-from=s3rius/tuser:latest -f deploy/Dockerfile .
```

Docker image and binaries will be available soon.

## Architecture

Files and info about them are separated from each other.
In order to modify original file rustus searches for information about
the file in information storage.

However, automatic migration between different information
storages is not supported.


## Configuration

You can configure rustus via command line or environment variables.
All options are listed in `rustus --help`.

### Roadmap

* [x] Data storage interface;
* [x] Info storage interface;
* [x] Core TUS protocol;
* [x] Extensions interface;
* [x] Creation extension;
* [x] Creation-defer-length extension;
* [x] Creation-with-upload extension;
* [x] Termination extension;
* [x] Route to get uploaded files;
* [x] Database support for info storage;
* [ ] S3 as data storage store support;
* [ ] Notification interface;
* [ ] Notifications via http hooks;
* [ ] Notifications via RabbitMQ;
* [ ] Rustus helm chart;
* [ ] Cloud native rustus operator.
