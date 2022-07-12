# https://www.lpalmieri.com/posts/fast-rust-docker-builds/

ARG CARGO_CHEF_VERSION=0.1.37

FROM rust:alpine AS cargo-chef
WORKDIR /app
ARG CARGO_CHEF_VERSION

RUN apk add --no-cache musl-dev && \
	cargo install cargo-chef --locked --version $CARGO_CHEF_VERSION
COPY . .

# First, we use `cargo-chef` to compile a recipe of our Rust dependencies.
RUN cargo chef prepare --recipe-path recipe.json


FROM rust:alpine as builder
WORKDIR /app
ARG CARGO_CHEF_VERSION

RUN apk add --no-cache pkgconfig openssl-dev musl-dev && \
	cargo install cargo-chef --locked --version $CARGO_CHEF_VERSION

# Compile all Rust dependencies from `cargo-chef`. As long as
# `cargo-chef-recipe.json` doesn't change, this will be cached.
COPY --from=cargo-chef /app/recipe.json cargo-chef-recipe.json
RUN cargo chef cook --release --recipe-path cargo-chef-recipe.json

# Finally, compile our binary in release mode.
COPY . .
RUN cargo build --release --bin block-oracle


FROM alpine:latest as runtime
WORKDIR /app

RUN apk add --no-cache openssl-dev
COPY --from=builder /app/target/release/block-oracle /usr/local/bin
COPY --from=builder /app/crates/oracle/config/staging/config.toml ./config.toml

CMD ["./usr/local/bin/block-oracle"]