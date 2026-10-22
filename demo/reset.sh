#!/usr/bin/env bash
# Put the demo's node back on the vulnerable version, with its old data,
# so the demo can be run again.
source "$(dirname "$0")/common.sh"

[ -d "$SNAPSHOT_DIR" ] || {
  echo "No snapshot for $NODE yet; it has not been upgraded by demo/upgrade.sh" >&2
  exit 1
}

echo "Restoring $NODE to v$OLD_VERSION"
docker stop -t 30 "$CONTAINER" >/dev/null 2>&1 || true
rm -rf "$DATA_DIR"
cp -a "$SNAPSHOT_DIR" "$DATA_DIR"
recreate_with_image "firewatch/clightning:$OLD_VERSION"
wait_for_version "$OLD_VERSION"
