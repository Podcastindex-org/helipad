export HELIPAD_DATABASE_DIR="/home/user/database.db"

export LND_TLSCERT="/home/user/tls.cert"

export LND_ADMINMACAROON="/home/user/admin.macaroon"

export LND_URL="mylndnode.example.com:10009"


clear && cargo build && ./target/debug/helipad