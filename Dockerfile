FROM rust:1.82-bookworm AS builder

RUN apt-get update -y && apt-get install -y libjemalloc-dev \
    build-essential \
    ca-certificates \
    cmake \
    && apt-get clean -y

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY imgs ./imgs

ENV JEMALLOC_SYS_WITH_MALLOC_CONF="background_thread:true,metadata_thp:auto,tcache:false,dirty_decay_ms:30000,muzzy_decay_ms:30000,abort_conf:true"

RUN cargo build --release --bin rustus

FROM debian:bookworm-20241111-slim AS base

RUN apt-get update -y && apt-get install -y ca-certificates \
    && apt-get clean -y

COPY --from=builder /app/target/release/rustus /usr/local/bin/

ENTRYPOINT ["/usr/local/bin/rustus"]

FROM base AS rootless

RUN useradd --create-home  -u 1000 --user-group rustus
WORKDIR /home/rustus
USER rustus
