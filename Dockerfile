###
##: Set up native compile
###
FROM rust:bookworm AS build-native
RUN apt-get update && apt-get install -y ca-certificates openssl sqlite3
RUN echo $(arch)-unknown-linux-gnu > /tmp/rust-target

###
##: Set up arm64 cross compile
###
FROM --platform=$BUILDPLATFORM rust:bookworm AS build-cross-arm64

ARG CC=aarch64-linux-gnu-gcc
ARG CXX=aarch64-linux-gnu-g++
ARG PKG_CONFIG_SYSROOT_DIR=/usr/lib/aarch64-linux-gnu/
ARG CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc

RUN dpkg --add-architecture arm64
RUN apt-get update
RUN apt-get install -y g++-aarch64-linux-gnu
RUN apt-get install -y libsqlite3-dev:arm64 libssl-dev:arm64

RUN rustup target add aarch64-unknown-linux-gnu
RUN echo aarch64-unknown-linux-gnu > /tmp/rust-target

###
##: Build targets
###
FROM build-native AS build-arm64-on-arm64
FROM build-native AS build-amd64-on-amd64
FROM build-cross-arm64 AS build-arm64-on-amd64

#####

###
##: Build stage
###
FROM build-$TARGETARCH-on-$BUILDARCH as builder

WORKDIR /opt/helipad

COPY . /opt/helipad
RUN cargo build --release --target=$(cat /tmp/rust-target)
RUN cp ./target/$(cat /tmp/rust-target)/release/helipad .

###
##: Bundle stage
###
FROM --platform=$TARGETPLATFORM debian:bookworm-slim AS runner

RUN apt-get update && \
    apt-get install -y ca-certificates openssl sqlite3 && \
    rm -fr /var/lib/apt/lists/*

WORKDIR /opt/helipad

COPY --from=builder /opt/helipad/helipad .
COPY --from=builder /opt/helipad/webroot ./webroot
COPY --from=builder /opt/helipad/helipad.conf .

RUN useradd -u 1000 helipad
RUN mkdir /data && chown -R 1000:1000 /data

USER helipad

EXPOSE 2112/tcp

ENTRYPOINT ["/opt/helipad/helipad"]
