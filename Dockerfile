##: Compile amd64 on amd64
FROM rust:bookworm AS build-amd64-on-amd64

RUN apt-get update
RUN apt-get install -y apt-utils sqlite3 openssl

WORKDIR /opt/helipad
COPY . /opt/helipad

RUN cargo build --release
RUN cp ./target/release/helipad .

##: Cross compile arm64 on amd64
FROM --platform=$BUILDPLATFORM rust:bookworm AS build-arm64-on-amd64

RUN dpkg --add-architecture arm64
RUN apt update
RUN apt install -y g++-aarch64-linux-gnu
RUN apt install -y libsqlite3-dev:arm64 libssl-dev:arm64
RUN rustup target add aarch64-unknown-linux-gnu

ARG CC=aarch64-linux-gnu-gcc
ARG CXX=aarch64-linux-gnu-g++
ARG PKG_CONFIG_SYSROOT_DIR=/usr/lib/aarch64-linux-gnu/
ARG CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc

WORKDIR /opt/helipad
COPY . /opt/helipad

RUN uname -a
RUN cargo build --release --target=aarch64-unknown-linux-gnu
RUN cp ./target/aarch64-unknown-linux-gnu/release/helipad .

##: Build selector (based on archs)
FROM build-$TARGETARCH-on-$BUILDARCH as builder

##: Bundle stage
FROM --platform=$TARGETPLATFORM debian:bookworm-slim AS runner

RUN apt-get update && apt-get install -y apt-utils ca-certificates openssl sqlite3 tini

WORKDIR /opt/helipad

COPY --from=builder /opt/helipad/helipad .
COPY --from=builder /opt/helipad/webroot ./webroot
COPY --from=builder /opt/helipad/helipad.conf .

RUN useradd -u 1000 helipad

USER helipad

EXPOSE 2112/tcp

ENTRYPOINT ["/opt/helipad/helipad"]
