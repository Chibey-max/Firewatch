#!/bin/sh
# Mirrors polarlightning/clightning's entrypoint, using setpriv instead of gosu.
set -e

# give bitcoind a second to boot
sleep 1

# run as the host user's uid/gid so Polar can read the files on the host
USERID=${USERID:-1000}
GROUPID=${GROUPID:-1000}
if ! id clightning > /dev/null 2>&1; then
  echo "adding user clightning ($USERID:$GROUPID)"
  groupadd -f -g "$GROUPID" clightning
  useradd -r -u "$USERID" -g "$GROUPID" clightning
  mkdir -p /home/clightning
  chown -R "$USERID:$GROUPID" /home/clightning
fi

if [ "$(echo "$1" | cut -c1)" = "-" ]; then
  set -- lightningd "$@"
fi

if [ "$1" = "lightningd" ] || [ "$1" = "lightning-cli" ]; then
  echo "Running as clightning user: $*"
  export HOME=/home/clightning
  exec setpriv --reuid="$USERID" --regid="$GROUPID" --init-groups "$@"
fi

exec "$@"
