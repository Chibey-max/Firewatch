#!/usr/bin/env bash
# Copies the data the firewatch binary embeds into crates/firewatch-cli/data/,
# so the crate published to crates.io carries it (a crate can't reach outside
# its own folder). Builds the dashboard first. Run before `cargo publish`.
set -euo pipefail
cd "$(dirname "$0")/.."

( cd dashboard && { [ -d node_modules ] || npm ci --silent; } && npm run --silent build >/dev/null )

data=crates/firewatch-cli/data
rm -rf "$data"
mkdir -p "$data/releases"
cp -R advisories "$data/advisories"
cp releases/lnd.toml "$data/releases/lnd.toml"
cp -R dashboard/dist "$data/dashboard"
echo "packaged $(ls "$data/advisories" | wc -l) advisories, the release table and the dashboard into $data"
