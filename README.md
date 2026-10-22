# Firewatch
Firewatch tells a Lightning node operator, within minutes, whether a published security flaw actually affects their node and exactly what to do about it.

It is read-only by design: it holds no credential that can change or spend from a node, it never upgrades or shuts anything down, and it quotes the developers' own fix instead of writing one.

## Quick start

```sh
cargo test --workspace                          # engine, connectors, one test per advisory
cargo run -- validate                           # load and list every advisory
cargo run -- check                              # check the live nodes in firewatch.toml
cargo run -- check --nodes examples/nodes.json  # check a JSON snapshot instead
cargo run -- check --json                       # the report as dashboard JSON
cargo run -- check --include-unverified
```

Copy `firewatch.example.toml` to `firewatch.toml` (git-ignored) and add your nodes. Core Lightning is read over `clnrest` with a rune restricted to `getinfo` and `listconfigs`; Firewatch refuses any rune that can do more:

```sh
lightning-cli createrune restrictions='[["method=getinfo","method=listconfigs"]]'
```

### Demo network (Polar)

Polar's built-in Core Lightning list stops at 25.12. Build Polar-compatible images of the demo versions, then add them in Polar under **Manage Images → Custom Nodes**:

```sh
for v in 26.06.7 26.06.9; do
  docker build -f demo/polar/Dockerfile.cln --build-arg CLN_VERSION=v$v \
    -t firewatch/clightning:$v demo/polar
done
```

`check` exits with `0` when nothing applies, `2` when something can't be told, `3` when a node is affected, and `1` on error.

## Verdicts

| Verdict | When |
|---|---|
| Affected | The version is in the affected range and every condition is confirmed true |
| Can't tell | The version is in range but a condition can't be read over RPC, or the version can't be parsed |
| Not affected | The version is outside the range, or a condition is confirmed false |

Firewatch never shows green when it is unsure.

## Layout

| Path | What it is |
|---|---|
| `crates/firewatch-core` | Shared types (`Advisory`, `NodeInfo`, `Verdict`, `Report`), version parsing and the matching engine. Pure: no network, no credentials. |
| `crates/firewatch-cli` | The `firewatch` binary. |
| `advisories/` | One TOML file per advisory. Only `verified = true` advisories run by default. |
| `examples/nodes.json` | Stub node snapshots, used until the connectors exist. |

Connectors produce a `NodeInfo`; alerts and the dashboard consume a `Report`. Neither needs to touch the engine.

## Advisory format

```toml
id = "FW-2026-003"
implementation = "cln"                 # cln | lnd | ldk
title = "Vulnerabilities fixed in 26.06.8"
affected = "< 26.06.8"                 # comparators joined by commas: ">= 0.18.0, < 0.21.0"
fixed = "26.06.8"                      # omit when no fix exists yet
severity = "critical"                  # critical | high | medium | low
exploited_in_wild = true
source = "https://github.com/ElementsProject/lightning/releases/tag/v26.06.8"
reviewed_by = "Dave"
verified = true

[[conditions]]                         # every condition must hold for Affected
kind = "version_range"                 # version_range | config_enabled | feature_bit | manual

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
kind = "manual"                        # always Can't tell
check_hint = "What the operator should check by hand, and how."
```

A file that fails to parse or validate stops the load, so a broken advisory is never silently skipped. Adding an advisory means adding its test in `crates/firewatch-core/tests/advisories.rs`.

## Limitations

- Firewatch covers only the advisories in its database; it does not discover new vulnerabilities.
- Conditions that depend on `lnd.conf` options not exposed over RPC return Can't tell, not a verdict.
- It serves self-hosted node operators, not users of custodial or LSP-managed phone wallets.
- The advisory database is maintained by people; a missed advisory is a missed alert.
