FROM rust:1.61.0-alpine as builder

WORKDIR /app

RUN apk add libc-dev pkgconfig openssl-dev

COPY rust-toolchain.toml Cargo.lock Cargo.toml ./
COPY crates/ ./crates/
RUN cargo build --release && cp target/release/block-oracle ./block-oracle && rm -rf target

FROM ubuntu:latest as runtime

COPY --from=builder /app/block-oracle /usr/local/bin
COPY --from=builder /app/crates/oracle/config/dev/config.toml ./config.toml

CMD ["./usr/local/bin/block-oracle"]
