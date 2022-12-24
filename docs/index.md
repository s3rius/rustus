---
title: "Welcome page"
description: Rustus docs
---

<div align="left">
    <img src="https://raw.githubusercontent.com/s3rius/rustus/master/imgs/logo_horizontal.svg" alt="logo" width="500">
    <div>
        <p></p>
        <img alt="Docker Image Size (latest by date)" src="https://img.shields.io/docker/image-size/s3rius/rustus?sort=date&style=for-the-badge">
        <img alt="Docker Image Version (latest semver)" src="https://img.shields.io/docker/v/s3rius/rustus?style=for-the-badge">
        <img alt="GitHub" src="https://img.shields.io/github/license/s3rius/rustus?style=for-the-badge">
    </div>
</div>

Rustus is a [TUS](https://tus.io) protocol implementation that helps you handle file uploads.

This project has many features that make it easy to integrate with your application.


## Installation

You can install rustus in four different ways.

### From source

To build it from source rust must be installed. We don't rely on nightly features,
you can use last stable release.

```bash
git clone https://github.com/s3rius/rustus.git
cd rustus
cargo install --path . --features=all
```

Also, you can speedup build by disabling some features.

Available features:

* `amqp_notifier` - adds `AMQP` protocol support for notifying about upload status;
* `db_info_storage` - adds support for storing information about upload in different databases (`Postgres`, `MySQL`, `SQLite`);
* `redis_info_storage` - adds support for storing information about upload in `Redis` database;
* `all` - enables all rustus features.

All precompiled binaries have all features enabled.

### With cargo

If you have cargo installed, it might be easier to
install it directly from crates.io.

```bash
cargo install rustus --features=all
```

### Binaries

All precompiled binaries available on Github releases page.
You can download binaries from [here](https://github.com/s3rius/rustus/releases), unpack it and run.

```bash
./rustus
```

Make sure you download right version for your CPU architecture and OS.

### Using Docker

One of the most simple ways to run rustus is `Docker`.

Rustus has two containers for each version.
1. Debian based image
2. Alpine based image

Alpine based images are more lightweight than Debian

To run Rustus with Docker you just need to run this command

```bash
docker run --rm -p "1081:1081" -d s3rius/rustus --log-level "DEBUG"
```

More information about Rustus docker images you can find on [Docker hub page](https://hub.docker.com/r/s3rius/rustus/).
