FROM rust:1.64.0-bullseye AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY imgs ./imgs
RUN cargo build --release --bin rustus --features=all

FROM debian:bullseye-20211201-slim AS base
COPY --from=builder /app/target/release/rustus /usr/local/bin/
ENTRYPOINT ["/usr/local/bin/rustus"]

FROM base as rootless

RUN useradd --create-home  -u 1000 --user-group rustus
WORKDIR /home/rustus
USER rustus