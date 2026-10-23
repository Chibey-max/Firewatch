#!/usr/bin/env bash
# Regenerate dashboard/src/fixtures/*.json from the real CLI, so the sample
# data always matches the current report format and advisories. Run after any
# change to report.rs or advisories/.
#
#   dashboard/dev/regen-fixtures.sh
set -euo pipefail

repo="$(cd "$(dirname "$0")/../.." && pwd)"
out="$repo/dashboard/src/fixtures"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
cd "$repo"
cargo build --quiet
fw="$repo/target/debug/firewatch"

# Snapshots: the docs/DEMO.md network (old-cln, current-cln, alice, lnd-old)
# before and after old-cln's upgrade, plus
# examples/nodes.json with everything upgraded.
python3 - "$tmp" <<'PY'
import json, sys
tmp = sys.argv[1]
old = {"name": "old-cln", "implementation": "cln", "version": "v26.06.7", "network": "regtest",
       "config": {"experimental-splicing": False}}
cur = dict(old, name="current-cln", version="v26.06.9")
alice = {"name": "alice", "implementation": "lnd", "version": "0.20.0-beta commit=v0.20.0-beta",
         "network": "regtest"}
lnd_old = dict(alice, name="lnd-old", version="0.18.5-beta commit=v0.18.5-beta")
json.dump([old, cur, alice, lnd_old], open(f"{tmp}/demo.json", "w"))
json.dump([dict(old, version="v26.06.9"), cur, alice, lnd_old], open(f"{tmp}/after.json", "w"))
nodes = json.load(open("examples/nodes.json"))
nodes[0]["version"] = "v26.06.9"
nodes[2]["version"] = "0.21.4-beta commit=v0.21.4-beta"
json.dump(nodes, open(f"{tmp}/green.json", "w"))
PY

# A node nothing answers for, read live so the error text is the connector's own.
cat > "$tmp/down.toml" <<'TOML'
[[nodes]]
kind = "cln"
name = "current-cln"
url = "http://127.0.0.1:8183"
rune = "unused"
TOML

"$fw" check --nodes examples/nodes.json --json --include-unverified > "$out/report.mixed.json" || true
"$fw" check --nodes "$tmp/green.json" --json > "$out/report.green.json" || true
"$fw" check --nodes "$tmp/demo.json" --json > "$out/report.demo.json" || true
"$fw" check --nodes "$tmp/after.json" --json > "$out/report.demo-after.json" || true
"$fw" check --config "$tmp/down.toml" --json > "$tmp/down.out.json" || true

python3 - "$tmp" "$out" <<'PY'
import json, sys
tmp, out = sys.argv[1:]
down = json.load(open(f"{tmp}/down.out.json"))["nodes"][0]
mixed = json.load(open(f"{out}/report.mixed.json"))
offline = dict(down, name="offline-node")
json.dump({"checked_at": mixed["checked_at"], "nodes": [mixed["nodes"][0], offline]},
          open(f"{out}/report.error.json", "w"), indent=2)
json.dump({"checked_at": mixed["checked_at"], "nodes": []},
          open(f"{out}/report.empty.json", "w"), indent=2)
PY
# Match the repo's formatting so CI's prettier --check passes.
(cd "$repo/dashboard" && npx --no-install prettier --write --log-level warn src/fixtures)
echo "Fixtures regenerated in dashboard/src/fixtures/"
