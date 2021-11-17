FROM rust:latest AS builder

RUN apt-get update
RUN apt-get upgrade -y
RUN apt-get install -y sqlite3
RUN apt-get install -y openssl

WORKDIR /opt
USER root

RUN git clone https://github.com/Podcastindex-org/helipad.git
WORKDIR /opt/helipad
RUN cargo build --release
RUN cp target/release/helipad .

EXPOSE 2112/tcp

ENTRYPOINT ["/opt/helipad/helipad"]