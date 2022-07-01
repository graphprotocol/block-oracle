FROM rust:1.61 as builder

WORKDIR /app

RUN apt install libc-dev pkg-config libssl-dev

COPY rust-toolchain.toml Cargo.lock Cargo.toml ./
COPY crates/ ./crates/
RUN cargo build --release && cp target/release/block-oracle ./block-oracle && rm -rf target

FROM debian:latest as runtime

RUN apt update --yes
RUN apt install --yes libc-dev pkg-config libssl-dev ca-certificates

COPY --from=builder /app/block-oracle /usr/local/bin
COPY --from=builder /app/crates/oracle/config/staging/config.toml ./config.toml

CMD ["./usr/local/bin/block-oracle"]
