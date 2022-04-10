FROM lukemathwalker/cargo-chef:latest-rust-1.57.0 AS chef
WORKDIR app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --features=all,metrics --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release --bin rustus --features=all,metrics

FROM debian:bullseye-20211201-slim AS base
COPY --from=builder /app/target/release/rustus /usr/local/bin/
ENTRYPOINT ["/usr/local/bin/rustus"]

FROM base as rootless

RUN useradd --create-home  -u 1000 --user-group rustus
WORKDIR /home/rustus
USER rustus