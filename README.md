<div align="center">
    <img src="https://raw.githubusercontent.com/s3rius/rustus/master/imgs/logo_horizontal.svg" alt="logo" width="500">
    <div>
        <p></p>
        <a href="https://hub.docker.com/r/s3rius/rustus/"><img alt="Docker Image Size (latest by date)" src="https://img.shields.io/docker/image-size/s3rius/rustus?sort=date&style=for-the-badge"></a>
        <a href="https://hub.docker.com/r/s3rius/rustus/"><img alt="Docker Image Version (latest semver)" src="https://img.shields.io/docker/v/s3rius/rustus?style=for-the-badge"></a>
        <a href="https://github.com/s3rius/rustus/blob/master/LICENSE"><img alt="GitHub" src="https://img.shields.io/github/license/s3rius/rustus?style=for-the-badge"></a>
    </div>
    <p>Production-ready <a href="https://tus.io/">TUS</a> protocol implementation written in Rust.</p>
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

To build it from source rust must be installed. We don't rely on nightly features,
you can use last stable release.

```bash
git clone https://github.com/s3rius/rustus.git
cd rustus
cargo install --path .
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

To run rustus you just need to run this command

```bash
docker run --rm -p "1081:1081" -d "ghcr.io/s3rius/rustus" --log-level "DEBUG"
```
