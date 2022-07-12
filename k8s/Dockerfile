# https://www.lpalmieri.com/posts/fast-rust-docker-builds/

FROM lukemathwalker/cargo-chef:latest AS cargo-chef-planner
WORKDIR /app

COPY . .
# First, we use `cargo-chef` to compile a recipe of our Rust dependencies.
RUN cargo chef prepare --recipe-path recipe.json


FROM lukemathwalker/cargo-chef:latest as builder
WORKDIR /app

RUN apt-get install -y && \
	apt-get install -y libc-dev pkg-config libssl-dev

# Compile all Rust dependencies from `cargo-chef`. As long as
# `cargo-chef-recipe.json` doesn't change, this will be cached.
COPY --from=cargo-chef-planner /app/recipe.json cargo-chef-recipe.json
RUN cargo chef cook --release --recipe-path cargo-chef-recipe.json

# Finally, compile our binary in release mode.
COPY . .
RUN cargo build --release --bin block-oracle


FROM debian:latest as runtime
WORKDIR /app

RUN apt-get update -y && \
	apt-get install -y libc-dev pkg-config libssl-dev ca-certificates

COPY --from=builder /app/target/release/block-oracle /usr/local/bin
COPY --from=builder /app/crates/oracle/config/staging/config.toml ./config.toml

CMD ./usr/local/bin/block-oracle
