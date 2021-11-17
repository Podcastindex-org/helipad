##: Build stage
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


##: Bundle stage
FROM rust:latest AS runner
RUN apt-get update && apt-get upgrade -y && apt-get install -y sqlite3 && apt-get install -y openssl

RUN useradd -ms /bin/bash heliuser

WORKDIR /opt
COPY --from=builder /opt/helipad .

RUN chown -R heliuser /opt

ENV HELIPAD_RUNAS_USER="heliuser"

EXPOSE 2112/tcp

ENTRYPOINT ["/opt/helipad/helipad"]