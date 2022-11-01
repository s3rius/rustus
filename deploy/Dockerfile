FROM rust:1.64.0-bullseye AS builder

RUN apt-get update -yqq && apt-get install -yqq cmake g++

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY imgs ./imgs
RUN RUSTFLAGS="-C target-cpu=native" cargo build --release --bin rustus --features=all

FROM debian:bullseye-20211201-slim AS base

RUN apt-get update -yqq \
    && apt-get install -yqq libatomic1 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/rustus /usr/local/bin/

ENTRYPOINT ["/usr/local/bin/rustus"]

FROM base as rootless

RUN useradd --create-home  -u 1000 --user-group rustus
WORKDIR /home/rustus
USER rustus