# Shared helpers for the demo scripts. Sourced, not run.
set -euo pipefail

NETWORK_NAME="${NETWORK_NAME:-Firewatch-demo}"
NODE="${NODE:-old-cln}"
OLD_VERSION="${OLD_VERSION:-26.06.7}"

# Polar keeps networks in ~/.polar/networks/<id>; find ours by name.
NETWORK_ID="${NETWORK_ID:-$(python3 - "$NETWORK_NAME" <<'PY'
import json, os, sys
data = json.load(open(os.path.expanduser("~/.polar/networks/networks.json")))
ids = [n["id"] for n in data["networks"] if n["name"].strip() == sys.argv[1]]
print(ids[0] if ids else "")
PY
)}"
[ -n "$NETWORK_ID" ] || { echo "No Polar network named '$NETWORK_NAME'" >&2; exit 1; }

NETWORK_DIR="$HOME/.polar/networks/$NETWORK_ID"
CONTAINER="polar-n$NETWORK_ID-$NODE"
DATA_DIR="$NETWORK_DIR/volumes/c-lightning/$NODE/lightningd"
SNAPSHOT_DIR="$NETWORK_DIR/firewatch-snapshot/$NODE"

# Recreate one node's container from Polar's compose file with another image.
# Polar regenerates docker-compose.yml on start, so we never edit it.
recreate_with_image() {
  local image="$1" override
  override="$(mktemp --suffix=.yml)"
  printf 'services:\n  %s:\n    image: %s\n' "$NODE" "$image" > "$override"
  local out status=0
  out="$(docker compose -p "polar-network-$NETWORK_ID" \
    -f "$NETWORK_DIR/docker-compose.yml" -f "$override" \
    up -d --no-deps --force-recreate "$NODE" 2>&1)" || status=$?
  rm -f "$override"
  [ "$status" -eq 0 ] || { echo "$out" >&2; return "$status"; }
}

wait_for_version() {
  local want="$1"
  for _ in $(seq 1 60); do
    got="$(docker exec -u "$(id -u)" -e HOME=/home/clightning "$CONTAINER" \
      lightning-cli --network=regtest getinfo 2>/dev/null \
      | python3 -c 'import json,sys; print(json.load(sys.stdin)["version"])' 2>/dev/null || true)"
    if [ "$got" = "v$want" ]; then
      echo "$NODE is running v$want"
      return 0
    fi
    sleep 1
  done
  echo "$NODE did not come up on v$want; see: docker logs $CONTAINER" >&2
  return 1
}
