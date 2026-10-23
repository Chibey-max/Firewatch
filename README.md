# Firewatch

[![CI](https://github.com/Chibey-max/Firewatch/actions/workflows/ci.yml/badge.svg)](https://github.com/Chibey-max/Firewatch/actions/workflows/ci.yml)

Firewatch tells a Lightning node operator, within minutes, whether a published security flaw actually affects their node and exactly what to do about it.

It is read-only by design: it holds no credential that can change or spend from a node, it never upgrades or shuts anything down, and it quotes the developers' own fix instead of writing one.

## Quick start

```sh
cargo test --workspace                          # engine, connectors, alerts, one test per advisory
cargo run -- validate                           # load and list every advisory
cargo run -- check                              # check the live nodes in firewatch.toml once
cargo run -- check --nodes examples/nodes.json  # check a JSON snapshot instead
cargo run -- check --json                       # the report as dashboard JSON
cargo run -- watch                              # re-check, alert on changes, serve the dashboard API
cargo run -- test-alert                         # send a test DM to your phone
```

Add `--include-unverified` to `check` or `watch` to also run advisories not yet verified against an official source.

`check` exits with `0` when nothing applies, `2` when something can't be told, `3` when a node is affected, and `1` on error.

## Configuration

Copy [firewatch.example.toml](firewatch.example.toml) to `firewatch.toml` (git-ignored) and add your nodes.

**Core Lightning** is read over `clnrest` with a rune restricted to `getinfo` and `listconfigs`:

```sh
lightning-cli createrune restrictions='[["method=getinfo","method=listconfigs"]]'
```

**LND** is read over REST with `readonly.macaroon`. LND's self-signed `tls.cert` is pinned: Firewatch accepts exactly that certificate and nothing else.

Firewatch probes every credential before trusting it and refuses one that can do more than read: a rune that can call `listfunds`, or a macaroon that can sign messages.

**Alerts** are NIP-17 encrypted Nostr DMs to the operator's key. Set `recipient` to your npub under `[alerts.nostr]`. Firewatch creates its own key in `secret_key_file` on first run and delivers to your NIP-17 inbox relays (kind 10050) as well as the relays listed. Any NIP-17 client can read them, for example 0xchat, Amethyst or Primal. To read them in a terminal:

```sh
cargo run -p firewatch-alerts --example inbox -- --new          # make a test key
cargo run -p firewatch-alerts --example inbox -- <nsec>         # print DMs sent to it
```

`watch` alerts when a finding turns red or amber, when it turns green again, and when a node has been unreachable for 3 checks in a row. A node restarting for an upgrade does not trigger an alert.

## Dashboard API

`watch` serves the latest report on `http://127.0.0.1:7070`. See [docs/API.md](docs/API.md) for the endpoints and the JSON format.

## Dashboard

`dashboard/` is the web page the operator watches: every node in one list, worst first, and the selected node's action, the developers' quoted fixes with links, what to check by hand and what a condition ruled out. Dark by default, with a light theme for projectors.

```sh
cd dashboard && npm install && npm run dev      # http://localhost:5173, reads the watch API
```

For the demo, build it once (`npm run build`) and set `dashboard_dir = "dashboard/dist"` under `[watch]`: `watch` then serves it at `http://127.0.0.1:7070`. Without a node network, `dashboard/dev/sim.sh` runs `watch` against simulated Core Lightning nodes, and `?fixture=demo` shows recorded sample data. See [dashboard/README.md](dashboard/README.md).

## Demo

The demo runs on a [Polar](https://lightningpolar.com) regtest network named `Firewatch-demo`. It has an old Core Lightning (`old-cln`, 26.06.7), a current one (`current-cln`, 26.06.9) and an LND node (`alice`).

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

## Verdicts

| Verdict | When |
|---|---|
| Affected | The version is in the affected range and every condition is confirmed true |
| Can't tell | The version is in range but a condition can't be read over RPC, the version can't be parsed, or the node can't be read |
| Not affected | The version is outside the range, or a condition is confirmed false |

Firewatch never shows green when it is unsure.

## Layout

| Path | What it is |
|---|---|
| `crates/firewatch-core` | Shared types (`Advisory`, `NodeInfo`, `Verdict`, `Report`), version parsing, the matching engine and change detection. Pure: no network, no credentials. |
| `crates/firewatch-connectors` | Read-only Core Lightning and LND connectors. |
| `crates/firewatch-alerts` | NIP-17 Nostr DM alerts. |
| `crates/firewatch-cli` | The `firewatch` binary: `check`, `watch` (loop and API), `test-alert`, `validate`. |
| `advisories/` | One TOML file per advisory. Only `verified = true` advisories run by default. |
| `demo/` | Polar images, the upgrade and reset scripts, and the [demo runbook](docs/DEMO.md). |
| `examples/nodes.json` | Node snapshots for working without a live network. |
| `dashboard/` | The operator's web dashboard (React, TypeScript, Vite), its fixtures, the live sim and the backup-video recorder. |
| `.github/workflows/ci.yml` | CI: Rust fmt, clippy and tests, and the dashboard's lint, format check and build. |

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
- It serves self-hosted node operators, not users of custodial or LSP-managed phone wallets.
- The advisory database is maintained by people; a missed advisory is a missed alert.
