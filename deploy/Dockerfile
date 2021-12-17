FROM lukemathwalker/cargo-chef:latest-rust-1.57.0 AS chef
WORKDIR app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release --bin tuser

FROM debian:buster-20211201-slim AS runtime
WORKDIR app
COPY --from=builder /app/target/release/tuser /usr/local/bin/
ENTRYPOINT ["/usr/local/bin/tuser"]
