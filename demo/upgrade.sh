#!/usr/bin/env bash
# Upgrade the demo's vulnerable node in place, as the operator would.
#   demo/upgrade.sh              # old-cln -> 26.06.9
#   NODE=x demo/upgrade.sh 26.06.10
#
# The first run snapshots the node's data so demo/reset.sh can undo it:
# Core Lightning migrates its database on upgrade and cannot downgrade.
source "$(dirname "$0")/common.sh"
NEW_VERSION="${1:-26.06.9}"

docker image inspect "firewatch/clightning:$NEW_VERSION" >/dev/null 2>&1 || {
  echo "Missing image firewatch/clightning:$NEW_VERSION; build it (see README)" >&2
  exit 1
}

if [ ! -d "$SNAPSHOT_DIR" ]; then
  echo "Snapshotting $NODE (v$OLD_VERSION) for demo/reset.sh"
  docker stop -t 30 "$CONTAINER" >/dev/null
  mkdir -p "$(dirname "$SNAPSHOT_DIR")"
  cp -a "$DATA_DIR" "$SNAPSHOT_DIR"
fi

echo "Upgrading $NODE to $NEW_VERSION"
recreate_with_image "firewatch/clightning:$NEW_VERSION"
wait_for_version "$NEW_VERSION"
