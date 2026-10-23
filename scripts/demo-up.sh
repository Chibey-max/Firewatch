#!/usr/bin/env bash
# Start the demo with one command: checks Docker and every node, builds the
# release binary and the dashboard, then runs `firewatch watch` serving both.
#
#   scripts/demo-up.sh                  # uses firewatch.toml
#   CONFIG=other.toml scripts/demo-up.sh
#   NO_OPEN=1 scripts/demo-up.sh       # don't open a browser
#
# Polar is a GUI: start the Firewatch-demo network by hand first.
set -euo pipefail
cd "$(dirname "$0")/.."

CONFIG="${CONFIG:-firewatch.toml}"
die() { echo "demo-up: $*" >&2; exit 1; }

docker info >/dev/null 2>&1 || die "Docker isn't running. Start it, then Polar."
[ -f "$CONFIG" ] || die "No $CONFIG. Copy firewatch.example.toml and fill in the nodes."

echo "==> Building Firewatch (release)"
cargo build --release --quiet

echo "==> Building the dashboard"
( cd dashboard
  [ -d node_modules ] || npm ci --silent
  npm run --silent build >/dev/null )

echo "==> Checking every node in $CONFIG"
# check exits 0, 2 or 3 with a report; anything else is an error, already
# printed above (a bad config, a missing release table, ...).
status=0
report="$(./target/release/firewatch check --config "$CONFIG" --json)" || status=$?
case "$status" in
  0|2|3) ;;
  *) die "firewatch check failed (exit $status). Fix the error above, then run this again." ;;
esac
python3 -I - "$report" <<'PY' || die "Fix the nodes above (start the network in Polar?), then run this again."
import json, sys
report = json.loads(sys.argv[1])
down = 0
for node in report["nodes"]:
    if node["error"]:
        down += 1
        print(f"   DOWN  {node['name']}: {node['error']}")
    else:
        print(f"   ok    {node['name']} ({node['version']}): {node['status']}")
    if node["name"] == "old-cln" and node["version"] not in (None, "v26.06.7"):
        print("   note  old-cln isn't on v26.06.7: run demo/reset.sh for the red -> green beat")
sys.exit(1 if down else 0)
PY

listen="$(python3 -I -c 'import sys, tomllib; print(tomllib.load(open(sys.argv[1], "rb")).get("watch", {}).get("listen", "127.0.0.1:7070"))' "$CONFIG")"
url="http://$listen"

# Open the dashboard once the first check has finished. `exec` keeps this
# shell's PID, so $$ is the watch process: stop waiting if it exits.
( until curl -sf "$url/api/report" >/dev/null; do
    kill -0 $$ 2>/dev/null || exit 0
    sleep 0.5
  done
  echo "==> Dashboard: $url"
  [ -n "${NO_OPEN:-}" ] || { command -v xdg-open >/dev/null && xdg-open "$url" >/dev/null 2>&1; } || true ) &

echo "==> Starting firewatch watch (Ctrl+C to stop)"
exec ./target/release/firewatch watch --config "$CONFIG" --dashboard dashboard/dist
