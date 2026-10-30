#!/usr/bin/env bash
# Run medicine-cabinet locally with throwaway secrets and a scratch state dir.
# Login token: dev-token-medicine-cabinet-0000 (printed). State: ${MC_DEV_DIR:-/tmp/medicine-cabinet-dev}.
set -euo pipefail
dir="${MC_DEV_DIR:-/tmp/medicine-cabinet-dev}"
mkdir -p "$dir"
key_file="$dir/secret.key"
[ -f "$key_file" ] || openssl rand -hex 32 > "$key_file"
export MEDICINE_CABINET_TOKEN=dev-token-medicine-cabinet-0000
export MEDICINE_CABINET_SECRET_KEY="$(cat "$key_file")"
export MEDICINE_CABINET_STATE_DIR="$dir"
export MEDICINE_CABINET_TIMEZONE="${MEDICINE_CABINET_TIMEZONE:-Europe/Brussels}"
export MEDICINE_CABINET_LISTEN="${MEDICINE_CABINET_LISTEN:-127.0.0.1:8099}"
echo "medicine-cabinet dev: http://$MEDICINE_CABINET_LISTEN  login token: dev-token-medicine-cabinet-0000"
cd "$(dirname "$0")/.."
exec cargo run -q -- "$@"
