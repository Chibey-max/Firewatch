# Firewatch
Firewatch tells a Lightning node operator, within minutes, whether a published security flaw actually affects their node and exactly what to do about it.

It is read-only by design: it holds no credential that can change or spend from a node, it never upgrades or shuts anything down, and it quotes the developers' own fix instead of writing one.

## Quick start

```sh
scripts/demo-up.sh                              # build everything, check the nodes, start the demo
cargo test --workspace                          # 69 tests: engine, connectors, alerts, one per advisory
cargo run -- validate                           # load and list every advisory
cargo run -- check                              # check the live nodes in firewatch.toml once
cargo run -- check --nodes examples/nodes.json  # check a JSON snapshot instead
cargo run -- check --json                       # the report as dashboard JSON
cargo run -- watch                              # re-check, alert on changes, serve the dashboard API
cargo run -- test-alert                         # send a test DM to your phone
```

Add `--include-unverified` to `check` or `watch` to also run advisories not yet verified against an official source.

`watch` listens on localhost only. It refuses any other `listen` address unless you pass `--allow-remote`, because the report says which nodes are vulnerable. `--dashboard <dir>` serves a built dashboard at `/`.

A web page open in your browser can reach localhost too, so `watch` also turns away what a hostile site could send: a request whose `Host` isn't `localhost`, `127.0.0.1` or `[::1]` (DNS rebinding), and a request whose `Origin` is another site (reading the report, or a cross-site `POST /api/check`). With `--allow-remote`, open the dashboard by the machine's IP address; names other than localhost are still refused.

`check` exits with `0` when nothing applies, `2` when something can't be told, `3` when a node is affected, and `1` on error.

## Configuration

Copy [firewatch.example.toml](firewatch.example.toml) to `firewatch.toml` (git-ignored) and add your nodes.

**Core Lightning** is read over `clnrest` with a rune restricted to `getinfo` and `listconfigs`:

```sh
lightning-cli createrune restrictions='[["method=getinfo","method=listconfigs"]]'
```

**LND** is read over REST with `readonly.macaroon`. LND's self-signed `tls.cert` is pinned: Firewatch accepts exactly that certificate and nothing else.

```toml
[[nodes]]
kind = "lnd"
name = "lnd-old"
url = "https://127.0.0.1:8085"         # Polar's REST port for the node
macaroon_file = "/home/you/.polar/networks/2/volumes/lnd/lnd-old/data/chain/bitcoin/regtest/readonly.macaroon"
tls_cert = "/home/you/.polar/networks/2/volumes/lnd/lnd-old/tls.cert"
```

Use absolute paths; `~` isn't expanded.

Firewatch probes every credential before trusting it and refuses one that can do more than read: a rune that can call `listfunds`, or a macaroon that can sign messages.

When a node can't be read, the report says why in plain words: the node isn't running, the request timed out, the TLS certificate isn't the pinned one, or the node rejected the credential. That node is Can't tell, never green, and the other nodes are still checked.

## Build check

A version number is only text a binary prints. LND also reports the commit it was built from, so Firewatch compares it with the commit the official release tag points to, from [releases/lnd.toml](releases/lnd.toml):

| `build_check` | Meaning |
|---|---|
| `match` | The binary is the official release its version claims. Verdicts are normal. |
| `mismatch` | Same version number, but a different commit, or the official commit built with changes (LND's `commit=` says `-dirty` or isn't an official tag). The version can't be trusted, so **every** finding on the node becomes Can't tell. |
| `unknown_release` | The version isn't in the table, for example a release newer than it. Verdicts are normal. |
| `not_checked` | No commit reported (Core Lightning) or no table. |

Refresh the table with `scripts/lnd-releases.sh`; it only replaces the file when the fetch succeeds. `--releases <file>` points at another one. The table is required: Firewatch stops if it's missing, rather than giving an unofficial build normal verdicts.

**Alerts** are NIP-17 encrypted Nostr DMs to the operator's key. Set `recipient` to your npub under `[alerts.nostr]`. Firewatch creates its own key in `secret_key_file` on first run and delivers to your NIP-17 inbox relays (kind 10050) as well as the relays listed. Any NIP-17 client can read them, for example 0xchat, Amethyst or Primal. To read them in a terminal:

```sh
cargo run -p firewatch-alerts --example inbox -- --new          # make a test key
cargo run -p firewatch-alerts --example inbox -- <nsec>         # print DMs sent to it
```

`watch` alerts when a finding turns red or amber, when it turns green again, and when a node has been unreachable for 3 checks in a row. A node restarting for an upgrade does not trigger an alert.

## Dashboard API

`watch` serves the latest report on `http://127.0.0.1:7070`. See [docs/API.md](docs/API.md) for the endpoints and the JSON format.

## Demo

The demo runs on a [Polar](https://lightningpolar.com) regtest network named `Firewatch-demo`: an old Core Lightning (`old-cln`, 26.06.7), a current one (`current-cln`, 26.06.9), and two LND nodes (`alice` on 0.20.0-beta, `lnd-old` on 0.18.5-beta). The step-by-step script is in [docs/DEMO.md](docs/DEMO.md).

Polar's built-in Core Lightning list stops at 25.12. Build Polar-compatible images of the demo versions, then add them in Polar under **Manage Images → Custom Nodes**:

```sh
for v in 26.06.7 26.06.9; do
  docker build -f demo/polar/Dockerfile.cln --build-arg CLN_VERSION=v$v \
    -t firewatch/clightning:$v demo/polar
done
```

During the demo, upgrade the vulnerable node as an operator would, and reset it afterwards for the next run:

```sh
demo/upgrade.sh     # old-cln: 26.06.7 -> 26.06.9, keeps a snapshot of its data
demo/reset.sh       # back to 26.06.7 with the snapshot
```

Core Lightning migrates its database on upgrade and cannot downgrade, which is why `reset.sh` restores a snapshot instead of swapping the image back.

For the build check, `demo/polar/Dockerfile.lnd-unofficial` builds LND 0.20.0-beta from the official tag plus one empty commit: it reports the official version number but not the official commit. Add it in Polar as a custom LND node named `lnd-unofficial`.

```sh
scripts/verify-demo.py                   # every node shows what demo/expected.toml says; every source opens
scripts/verify-demo.py --after-upgrade   # the same after demo/upgrade.sh
scripts/audit-quotes.py                  # every verified quote appears word for word on its source page
```

## Verdicts

| Verdict | When |
|---|---|
| Affected | The version is in the affected range and every condition is confirmed true |
| Can't tell | The version is in range but a condition can't be read over RPC, the version can't be parsed, the node can't be read, or its build isn't the official release |
| Not affected | The version is outside the range, or a condition is confirmed false |

Firewatch never shows green when it is unsure.

## Layout

| Path | What it is |
|---|---|
| `crates/firewatch-core` | Shared types (`Advisory`, `NodeInfo`, `Verdict`, `Report`), version parsing, the matching engine, the build check and change detection. Pure: no network, no credentials. |
| `crates/firewatch-connectors` | Read-only Core Lightning and LND connectors. |
| `crates/firewatch-alerts` | NIP-17 Nostr DM alerts. |
| `crates/firewatch-cli` | The `firewatch` binary: `check`, `watch` (loop and API), `test-alert`, `validate`. |
| `dashboard/` | The web dashboard (Vite, React, Tailwind). |
| `advisories/` | One TOML file per advisory. Only `verified = true` advisories run by default. |
| `docs/sources/` | The evidence for each Lightning Labs advisory: what was copied from where, and the audit. |
| `releases/` | Official release commits for the build check. |
| `scripts/` | `demo-up.sh`, `verify-demo.py`, `audit-quotes.py`, `lnd-releases.sh`. |
| `demo/` | Polar images, the upgrade and reset scripts, and the expected verdicts. |
| `examples/nodes.json` | Node snapshots for working without a live network. |

## Advisory format

```toml
id = "FW-2026-003"
implementation = "cln"                 # cln | lnd | ldk
title = "Vulnerabilities fixed in 26.06.8"
affected = "< 26.06.8"                 # ">= 0.18.0, < 0.21.0"; "||" for alternatives
fixed = "26.06.8"                      # one per release line: "0.20.2, 0.21.1"; omit if no fix yet
severity = "critical"                  # critical | high | medium | low
exploited_in_wild = true
source = "https://github.com/ElementsProject/lightning/releases/tag/v26.06.8"
reviewed_by = "Dave"
verified = true

[[conditions]]                         # every condition must hold for Affected
kind = "version_range"                 # version_range | config_enabled | feature_bit | network | manual

[remediation]
action = "upgrade"                     # upgrade | offline | disable_feature
steps = "We strongly recommend upgrading to this release."   # quoted from the source
```

Other condition kinds:

```toml
[[conditions]]
kind = "config_enabled"                # CLN listconfigs
option = "experimental-splicing"

[[conditions]]
kind = "feature_bit"                   # LND GetInfo; either bit of the even/odd pair
bit = 62

[[conditions]]
kind = "network"                       # mainnet (CLN: bitcoin), testnet, signet, regtest
networks = ["mainnet"]

[[conditions]]
kind = "manual"                        # always Can't tell
check_hint = "What the operator should check by hand, and how."
```

A file that fails to parse or validate stops the load, so a broken advisory is never silently skipped. Adding an advisory means adding its test in `crates/firewatch-core/tests/advisories.rs`.

## Limitations

- Firewatch covers only the advisories in its database; it does not discover new vulnerabilities.
- Conditions that depend on `lnd.conf` options not exposed over RPC return Can't tell, not a verdict.
- The build check covers LND only: Core Lightning doesn't report the commit it was built from.
- It serves self-hosted node operators, not users of custodial or LSP-managed phone wallets.
- The advisory database is maintained by people; a missed advisory is a missed alert.
