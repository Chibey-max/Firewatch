#!/usr/bin/env bash
# Run `firewatch watch` against simulated Core Lightning nodes, so the dashboard
# can be worked on live without Polar or Docker. Run from anywhere:
#
#   dashboard/dev/sim.sh             # start; Ctrl+C stops everything
#   dashboard/dev/sim.sh upgrade     # in another terminal: old-cln -> v26.06.9
#   dashboard/dev/sim.sh reset       # old-cln back to v26.06.7
#
# Then `npm run dev` in dashboard/ and open http://localhost:5173.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../.." && pwd)"
state="${FIREWATCH_SIM_DIR:-/tmp/firewatch-sim}"

case "${1:-start}" in
  upgrade) echo v26.06.9 > "$state/old-cln.version"; echo "old-cln now reports v26.06.9"; exit ;;
  reset) echo v26.06.7 > "$state/old-cln.version"; echo "old-cln now reports v26.06.7"; exit ;;
  start) ;;
  *) echo "usage: $0 [start|upgrade|reset]" >&2; exit 2 ;;
esac

mkdir -p "$state"
echo v26.06.7 > "$state/old-cln.version"
echo v26.06.9 > "$state/current-cln.version"

pids=()
trap 'kill "${pids[@]}" 2>/dev/null || true' EXIT
python3 -I "$here/fake_cln.py" 18182 "$state/old-cln.version" & pids+=($!)
python3 -I "$here/fake_cln.py" 18183 "$state/current-cln.version" & pids+=($!)

echo "Simulated nodes: old-cln (v26.06.7) on :18182, current-cln (v26.06.9) on :18183"
cd "$repo"
cargo run --quiet -- watch --config "$here/firewatch.sim.toml"
