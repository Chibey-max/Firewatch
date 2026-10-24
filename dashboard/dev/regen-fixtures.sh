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
# API listen addresses as Polar sets them (docs/API.md, "Exposure"): Core
# Lightning's clnrest on all interfaces, LND's unreadable without lnd_conf.
cln_apis = [{"api": "rest", "listen": {"state": "read", "addresses": ["0.0.0.0:3010"]}}]
lnd_apis = [{"api": api, "listen": {"state": "unknown", "why": "lnd_conf isn't set"}}
            for api in ("rest", "grpc")]
old = {"name": "old-cln", "implementation": "cln", "version": "v26.06.7", "network": "regtest",
       "config": {"experimental-splicing": False}, "apis": cln_apis}
cur = dict(old, name="current-cln", version="v26.06.9")
# Official release commits (releases/lnd.toml), so the build check reads "match".
alice = {"name": "alice", "implementation": "lnd", "version": "0.20.0-beta commit=v0.20.0-beta",
         "network": "regtest", "commit_hash": "b9ea7070c20ad2ca8514a47d9b4d560a501f0487",
         "apis": lnd_apis}
lnd_old = dict(alice, name="lnd-old", version="0.18.5-beta commit=v0.18.5-beta",
               commit_hash="4ccf4fc24c750d098cf24566ef4bbc0311c7d476")
# DEMO.md's optional lnd-unofficial: claims 0.20.0 but was built from another commit.
unofficial = dict(alice, name="lnd-unofficial", commit_hash="fc472ef1" + "0" * 32)
json.dump([old, cur, alice, lnd_old, unofficial], open(f"{tmp}/demo.json", "w"))
json.dump([dict(old, version="v26.06.9"), cur, alice, lnd_old, unofficial], open(f"{tmp}/after.json", "w"))
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
