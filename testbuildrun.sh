#!/usr/bin/env bash

export HELIPAD_DATABASE_DIR="/data/database.db"

export LND_TLSCERT="/opt/umbrel/lnd/tls.cert"

export LND_ADMINMACAROON="/opt/umbrel/lnd/data/chain/bitcoin/mainnet/admin.macaroon"

export LND_URL="127.0.0.1:10009"


clear && cargo build && ./target/debug/helipad