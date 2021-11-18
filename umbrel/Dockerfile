##: Build stage
FROM rust:latest AS builder

USER root

RUN apt-get update
RUN apt-get upgrade -y
RUN apt-get install -y apt-utils
RUN apt-get install -y sqlite3
RUN apt-get install -y openssl

WORKDIR /opt

RUN git clone https://github.com/Podcastindex-org/helipad.git
WORKDIR /opt/helipad
RUN cargo build --release
RUN cp target/release/helipad .


##: Bundle stage
FROM debian:buster-slim AS runner

USER root

RUN apt-get update
RUN apt-get upgrade -y
RUN apt-get install -y apt-utils
RUN apt-get install -y sqlite3
RUN apt-get install -y openssl

RUN useradd -ms /bin/bash heliuser
RUN mkdir /opt/helipad

WORKDIR /opt/helipad
COPY --from=builder /opt/helipad/target/release/helipad .
COPY --from=builder /opt/helipad/*.html .
COPY --from=builder /opt/helipad/*.js .
COPY --from=builder /opt/helipad/*.mp3 .

RUN chown -R heliuser /opt

ENV HELIPAD_RUNAS_USER="heliuser"

EXPOSE 2112/tcp

ENTRYPOINT ["/opt/helipad/helipad"]

