# Rustus

[Tus](https://tus.io/) protocol implementation written in Rust.

## Features

This implementation has several features to make usage as simple as possible.

* Rustus is robust, since it uses asynchronous Rust;
* It can store information about files in databases;
* You can specify directory structure to organize your uploads;
* Highly configurable;

### Supported info storages

* FileSystem
* PostgresSQL
* Mysql
* SQLite
* Redis

### Supported data storages

* FileSystem

## Installation

Since I haven't configured build automation yet, you can build it
from source using `cargo`.

```bash
cargo install --path .
```

Or you can use a docker image.

```bash
docker run --rm -it -p 1081:1081 s3rius/rustus:latest
```

Docker image and binaries will be available soon.

## Architecture

Files and info about them are separated from each other.
In order to modify original file rustus searches for information about
the file in information storage.

However, automatic migration between different information
storages is not supported yet.

## Info storages

The info storage is a database or directory. The main goal is to keep track
of uploads. Rustus stores information about download in json format inside
database.

File storage is a default one. You can customize the directory of an .info files
by providing `--info-dir` parameter.

```bash
rustus --info-dir "./info_dir"
```

If you want to choose different storage you have to specify its type and connection string.

```bash
# Redis info storage
rustus --info-storage redis-info-storage --info-db-dsn "redis://localhost"
# PostgreSQL info storage
rustus --info-storage db-info-storage --info-db-dsn "postgres://rustus:rustus@192.168.1.89:5440/rustus"
# SQLite3 info storage
rustus --info-storage db-info-storage --info-db-dsn "sqlite:////test.sqlite3"
# MySQL
rustus --info-storage db-info-storage --info-db-dsn "mysql://rustus:rustus@192.168.1.89:3306/rustus"
```

## Hooks

Rustus supports several event hooks, such as:
* File hooks;
* HTTP hooks;
* AMQP hooks.

You can combine them, but you have to be careful, since
AMQP hooks won't block uploading.

If you want to check the "Authorization" header value or validate some information,
you have to use webhooks or File hooks.

Hooks have priorities: file hooks are the most important, then goes webhooks and AMQP hooks have the least priority.
If pre-create hook failed, the upload would not start.
Of course, since AMQP is a protocol that doesn't allow you to track responses.
We can't validate anything to stop uploading.


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
* [x] Redis support for info storage;
* [x] Notification interface;
* [x] Notifications via http hooks;
* [x] Notifications via RabbitMQ;
* [X] Executable files notifications;
* [ ] S3 as data storage store support;
* [ ] Rustus helm chart;
* [ ] Cloud native rustus operator.
