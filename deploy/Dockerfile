FROM rustlang/rust:buster AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY imgs ./imgs

ENV JEMALLOC_SYS_WITH_MALLOC_CONF="background_thread:true,metadata_thp:always,tcache:false,dirty_decay_ms:0,muzzy_decay_ms:0,abort_conf:true"
RUN cargo build --release --bin rustus

FROM debian:bullseye-20231120-slim AS base

COPY --from=builder /app/target/release/rustus /usr/local/bin/
RUN apt update && apt install -y libssl-dev ca-certificates libjemalloc-dev && apt clean

VOLUME [ "/data" ]

ENTRYPOINT ["/usr/local/bin/rustus"]

FROM base as rootless

RUN useradd --create-home  -u 1000 --user-group rustus
WORKDIR /home/rustus
USER rustus
