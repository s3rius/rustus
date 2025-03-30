FROM rust:1.85-bookworm AS builder

RUN apt-get update -y && apt-get install -y libjemalloc-dev=5.3.0-1 \
    build-essential=12.9 \
    ca-certificates=20230311 \
    cmake=3.25.1-1 \
    && apt-get clean -y

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY imgs ./imgs

ENV JEMALLOC_SYS_WITH_MALLOC_CONF="background_thread:true,metadata_thp:auto,tcache:false,dirty_decay_ms:30000,muzzy_decay_ms:30000,abort_conf:true"
ENV CMAKE_POLICY_VERSION_MINIMUM="3.10"

RUN cargo build --release --bin rustus

FROM debian:bookworm-20250317-slim AS base

RUN apt-get update -y && apt-get install -y ca-certificates=20230311 \
    && apt-get clean -y

COPY --from=builder /app/target/release/rustus /usr/local/bin/

ENTRYPOINT ["/usr/local/bin/rustus"]

FROM base AS rootless

RUN useradd --create-home  -u 1000 --user-group rustus
WORKDIR /home/rustus
USER rustus
