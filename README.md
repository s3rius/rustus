<div align="center">
    <img src="./imgs/logo_horizontal.svg" alt="logo" width="500">
    <div>
        <p></p>
        <img alt="Docker Image Size (latest by date)" src="https://img.shields.io/docker/image-size/s3rius/rustus?sort=date&style=for-the-badge">
        <img alt="Docker Image Version (latest semver)" src="https://img.shields.io/docker/v/s3rius/rustus?style=for-the-badge">
        <img alt="GitHub" src="https://img.shields.io/github/license/s3rius/rustus?style=for-the-badge">
    </div>
    <p><a href="https://tus.io/">TUS</a> protocol implementation written in Rust.</p>
</div>

## Features

This implementation has several features to make usage as simple as possible.

* Rustus is robust, since it uses asynchronous Rust;
* It can store information about uploads in databases;
* You can specify directory structure to organize your uploads;
* It has a lot of hooks options, and hooks can be combined.
* Highly configurable;

Please check out [Documentation](https://s3rius.github.io/rustus/) for more information about configuration and deploy.

## Installation

You can install rustus by 4 different ways.

### From source

To build it from source rust must be installed.
Preferred version is 1.59.0.

```bash
git clone https://github.com/s3rius/rustus.git
cd rustus
cargo install --path . --features=all,metrics
```
Also you can speedup build by disabling some features.

Available features:

* `amqp_notifier` - adds amqp protocol support for notifying about upload status;
* `db_info_storage` - adds support for storing information about upload in different databases (Postgres, MySQL, SQLite);
* `http_notifier` - adds support for notifying about upload status via http protocol;
* `redis_info_storage` - adds support for storing information about upload in redis database;
* `hashers` - adds support for checksum verification;
* `metrics` - adds rustus specific metrics to prometheus endpoint;
* `all` - enables all rustus features except `metrics`.

All precompiled binaries have all features enabled.

### With cargo

If you have cargo installed maybe it would be easier to
install it directly from crates.io.

```bash
cargo install rustus --features=all
```

### Binaries

All precompiled binaries available on github releases page.
You can download binaries from [here](https://github.com/s3rius/rustus/releases), unpack it and run.

```bash
./rustus
```

Make sure that you download version for your cpu and os.

### Using docker

One of the most simple ways to run rustus is docker.

Rustus has two containers for each version.
1. debian based image
2. alpine based image

Alpine based images are more lightweight than debian

To run rustus you just need to run this command

```bash
docker run --rm -p "1081:1081" -d s3rius/rustus --log-level "DEBUG"
```