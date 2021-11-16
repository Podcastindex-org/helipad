FROM node:12-buster-slim AS builder

RUN apt-get update \
    && apt-get -y upgrade \
    && apt-get -y install --no-install-recommends sqlite3 openssl

WORKDIR /root/helipad
USER root

# git clone https://github.com/Podcastindex-org/helipad.git
# curl https://sh.rustup.rs -sSf | sh -s -- -y
# cd helipad && cargo build --release

COPY helipad/target/release/helipad helipadbin
COPY helipad/*.html .
COPY helipad/*.js .
COPY helipad/*.mp3 .

EXPOSE 2112/tcp

ENTRYPOINT ["/root/helipad/helipadbin","2112"]
