#!/usr/bin/env bash
# Run medicinecabinet locally with throwaway secrets and a scratch state dir.
# Login token: dev-token-medicinecabinet-0000 (printed). State: ${MC_DEV_DIR:-/tmp/medicinecabinet-dev}.
set -euo pipefail
dir="${MC_DEV_DIR:-/tmp/medicinecabinet-dev}"
mkdir -p "$dir"
key_file="$dir/secret.key"
[ -f "$key_file" ] || openssl rand -hex 32 > "$key_file"
export MEDICINECABINET_TOKEN=dev-token-medicinecabinet-0000
export MEDICINECABINET_SECRET_KEY="$(cat "$key_file")"
export MEDICINECABINET_STATE_DIR="$dir"
export MEDICINECABINET_LISTEN="${MEDICINECABINET_LISTEN:-127.0.0.1:8099}"
echo "medicinecabinet dev: http://$MEDICINECABINET_LISTEN  login token: dev-token-medicinecabinet-0000"
cd "$(dirname "$0")/.."
exec cargo run -q -- "$@"
