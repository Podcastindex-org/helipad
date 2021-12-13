##: Build stage
FROM rust:latest AS builder

USER root

RUN apt-get update
RUN apt-get upgrade -y
RUN apt-get install -y apt-utils
RUN apt-get install -y sqlite3
RUN apt-get install -y openssl

WORKDIR /opt
ARG GIT_REPO=https://github.com/Podcastindex-org/helipad.git
ARG GIT_BRANCH=main
RUN git clone -b ${GIT_BRANCH} ${GIT_REPO}
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

RUN chown -R 1000:1000 /opt
RUN mkdir /data && chown -R 1000:1000 /data

USER 1000
RUN mkdir /opt/helipad

WORKDIR /opt/helipad
COPY --from=builder /opt/helipad/target/release/helipad .
COPY --from=builder /opt/helipad/*.html .
COPY --from=builder /opt/helipad/*.js .
COPY --from=builder /opt/helipad/*.mp3 .
COPY --from=builder /opt/helipad/*.ico .

EXPOSE 2112/tcp

ENTRYPOINT ["/opt/helipad/helipad"]
