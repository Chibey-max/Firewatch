# Firewatch

[![CI](https://github.com/Chibey-max/Firewatch/actions/workflows/ci.yml/badge.svg)](https://github.com/Chibey-max/Firewatch/actions/workflows/ci.yml)

Firewatch tells a Lightning node operator, within minutes, whether a published security flaw actually affects their node and exactly what to do about it.

It is read-only by design: it holds no credential that can change or spend from a node, it never upgrades or shuts anything down, and it quotes the developers' own fix instead of writing one.

## Getting started

Six steps, about ten minutes. Firewatch only ever reads your node: nothing in this guide gives it a credential that can spend or change settings.

### What you need

- **Linux or macOS**, on Intel or ARM (a Raspberry Pi works). Run Firewatch on the node's machine, or on one that can reach the node's REST API.
- **The node's REST API turned on:**
  - **LND:** on by default, at `https://127.0.0.1:8080`.
  - **Core Lightning:** `clnrest` is off until it has a port. Add `clnrest-port=3010` to the node's config file (usually `~/.lightning/config`) and restart the node. It serves HTTPS with a self-signed certificate by default; Firewatch pins that certificate, and `firewatch init` sets it up.
- **Umbrel or Start9:** SSH into the box first (for example `ssh umbrel@umbrel.local`) and run every command below there.
- **For phone alerts (optional):** a Nostr app that shows NIP-17 private messages. On iPhone use **Nostur**; on Android, **Amethyst** or **0xchat**. Damus doesn't show these messages. Step 2 walks through it.

### 1. Install

**Option A: the installer.** No Rust or Node.js needed.

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/Chibey-max/Firewatch/releases/latest/download/firewatch-installer.sh | sh
```

It downloads one `firewatch` binary into `~/.local/bin`, with the advisories, the LND release table and the dashboard built in. If your shell then says `firewatch: command not found`, open a new terminal, or run `source ~/.local/bin/env`.

This always installs the newest release. Every release also has plain archives and SHA-256 checksums on the [releases page](https://github.com/Chibey-max/Firewatch/releases).

**Option B: from crates.io.** For people who already use Rust. It compiles for a few minutes.

1. Install Rust with [rustup](https://rustup.rs), if you haven't: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
2. Make sure there's a C compiler: `sudo apt install build-essential` on Ubuntu or Debian, `xcode-select --install` on macOS.
3. Install:
   ```sh
   cargo install firewatch --locked
   ```
   `--locked` builds with the exact dependency versions we tested.

Either way, check it worked:

```sh
firewatch --version      # firewatch 0.1.0
firewatch validate       # lists the built-in advisories
```

### 2. Set up your phone for alerts (optional)

Firewatch sends alerts as **encrypted Nostr direct messages** (NIP-17). Nostr has no accounts or passwords: your identity is a pair of keys that your phone app creates for you.

| Key | Looks like | What it's for |
|---|---|---|
| **Public key** (npub) | `npub1…` | Your address. Give it to Firewatch: alerts are encrypted to it. Safe to share. |
| **Private key** (nsec) | `nsec1…` | Opens your messages and proves you're you. **Never share it**, not with Firewatch, a teammate or a website. |

**a. Install an app that shows NIP-17 messages**

| Phone | App | |
|---|---|---|
| iPhone | **Nostur** (App Store) | Tested with Firewatch. |
| Android | **Amethyst** (Play Store) or **0xchat** | |

Damus doesn't show these messages, so alerts never appear there. Check you have the right app: it talks about Nostr, and it never asks for a phone number or password.

**b. Create your keys**

Open the app and choose **create a new account** (the wording varies by app). It generates your keys on the phone. A fresh account just for alerts is a good idea: it keeps node alerts separate from anything social.

**c. Back up the private key**

The app shows or lets you export your `nsec1…` key, usually under your profile, account or keys settings. Save it in a password manager. If you lose the phone without it, you lose the account, and you'd set up a new one and run `firewatch init --force` with the new npub.

**d. Copy your public key**

Open your profile. Your `npub1…` is shown there, usually with a copy button or a share or QR option. Copy the whole string.

**e. Give it to Firewatch**

Paste the npub when `firewatch init` asks (step 3). To change it later, edit `recipient` in `~/.config/firewatch/firewatch.toml`.

**f. Accept the first message**

After `firewatch test-alert` (step 4), the message arrives under **message requests**, because the app doesn't know the sender yet:

1. Open it and tap **Accept message request**.
2. Follow the sender. That's Firewatch's own key, which it created on first run in `~/.config/firewatch/firewatch.nsec`, and `test-alert` prints it.
3. Optionally rename the contact to "Firewatch", so alerts are easy to spot.

From then on, alerts land in your normal inbox.

**Relays.** Messages travel through Nostr relays. Firewatch sends to `relay.damus.io`, `nos.lol` and `relay.primal.net`, and also to any DM relays your app has published for your key. Most apps use some of these by default. If `test-alert` succeeds but nothing arrives, add `wss://relay.damus.io` and `wss://nos.lol` in the app's relay settings and reopen your messages.

**One recipient per config.** Firewatch sends to one npub. For a team, share one alerts account between the phones that need it, or run one Firewatch config per person.

### 3. Set up with `firewatch init`

```sh
firewatch init
```

It looks for your node in the usual places: LND in `~/.lnd` (or `$LND_DIR`, or Umbrel's folder), Core Lightning in `~/.lightning`, and Polar networks. For each node it finds, it asks whether to watch it, then tests the connection:

- **LND:** answer yes. Nothing else to do: LND already has a `readonly.macaroon`, and `init` uses it and LND's `tls.cert`. Confirm the REST address if yours isn't `https://127.0.0.1:8080`.
- **Core Lightning:** Firewatch needs a rune that can only read. Creating a rune is a change to the node, which Firewatch never makes, so `init` prints the command:
  ```sh
  lightning-cli createrune restrictions='[["method=getinfo","method=listconfigs"],["method/listconfigs","pnameconfig=clnrest-host","pnameconfig=clnrest-port","pnameconfig=grpc-host","pnameconfig=grpc-port"]]'
  ```
  Run it in another terminal and paste the `rune` value it prints. `init` won't show the rune back on screen.
- **A node it didn't find,** or one on another machine: answer yes to "Add a node by hand?" and give the address and credential paths.

Then paste your npub, or press Enter to skip alerts.

`init` writes `~/.config/firewatch/firewatch.toml`, readable only by you, since it holds runes. Run `firewatch init --force` to start over, or edit the file: [firewatch.example.toml](firewatch.example.toml) shows every option.

### 4. Check it works

```sh
firewatch check
```

Each node gets one of three answers: **Affected**, **Can't tell** (with what to check by hand), or **Not affected** (with the reason). It exits with `0` when nothing applies, `2` when something can't be told, `3` when a node is affected, and `1` on error.

If you gave an npub:

```sh
firewatch test-alert
```

The first message lands in your app's **message requests**. Tap **Accept message request**, and follow the sender so later alerts go straight to your inbox (step 2f).

### 5. Keep it running

```sh
firewatch watch
```

It checks every 30 seconds, sends one alert per node when something turns red or amber, and another when it's fixed. Open the dashboard at **http://127.0.0.1:7070**. Alerts only come while `watch` runs, so on a node that's always on, run it as a background service.

**After a power cut or a restart, Firewatch catches up.** After every check it saves what it knows to `state.json`, next to the config, readable only by you. When `watch` starts again, it compares the first check with that, and each node that changed while it was off gets one message: *"While Firewatch was off (since 14:02): old-cln is no longer affected by FW-2026-003"*. Nodes that didn't change get nothing. If the file is missing or damaged, Firewatch starts fresh and says why.

**Alerts in your language.** Set `language` under `[alerts]` to `ha` (Hausa), `yo` (Yoruba), `ig` (Igbo) or `pcm` (Nigerian Pidgin). The developers' fixes and Firewatch's reasons stay in English, word for word. A language is offered once a native speaker has reviewed it; [docs/strings/](docs/strings/) shows where each one stands and how to review it.

**Linux (systemd):**

```sh
mkdir -p ~/.config/systemd/user
cat > ~/.config/systemd/user/firewatch.service <<'UNIT'
[Unit]
Description=Firewatch: security alerts for Lightning nodes
After=network-online.target

[Service]
ExecStart=%h/.local/bin/firewatch watch
Restart=on-failure

[Install]
WantedBy=default.target
UNIT
systemctl --user enable --now firewatch
loginctl enable-linger "$USER"      # keep it running after you log out
```

If you installed with `cargo install`, use `%h/.cargo/bin/firewatch` in `ExecStart`. Read its log with `journalctl --user -u firewatch -f`.

The dashboard only answers on the machine itself, because the report says which nodes are vulnerable. To see it from your laptop when Firewatch runs on the node, tunnel the port: `ssh -L 7070:127.0.0.1:7070 you@your-node`, then open http://127.0.0.1:7070.

### 6. Update and uninstall

Advisories are built into each release, so **update Firewatch to get new ones**: run the installer again, or `cargo install firewatch --locked --force`. Your config stays.

To uninstall: `rm ~/.local/bin/firewatch` (installer) or `cargo uninstall firewatch`. Then remove the config with `rm -r ~/.config/firewatch`, and the service, if you made one, with `systemctl --user disable --now firewatch` and `rm ~/.config/systemd/user/firewatch.service`.

### Troubleshooting

| You see | Do this |
|---|---|
| `firewatch: command not found` | Open a new terminal, or `source ~/.local/bin/env` (installer) or `source ~/.cargo/env` (cargo). |
| `no config at …` | Run `firewatch init`. |
| `Can't connect to …; is the node running?` | Start the node. For Core Lightning, check `clnrest-port` is set. Check the port in the config matches the node's REST port. |
| `... is plain http:// to another machine` | Use `https://` with `tls_cert` (see [Configuration](#configuration)), or set `allow_plain_http = true` on the node if the connection is already private. |
| `TLS certificate not trusted` | `tls_cert` must point at the node's own certificate: LND's `tls.cert`, or Core Lightning's `server.pem` in its network folder (e.g. `~/.lightning/bitcoin/server.pem`). |
| `Credential rejected by the node` | The rune or macaroon belongs to another node. Make a new one on this node and run `firewatch init --force`. |
| `Credential is not read-only` | Firewatch refuses credentials that can do more than read. Use LND's `readonly.macaroon`, or a rune made with the restrictions above. |
| No message on the phone | Use Nostur, Amethyst or 0xchat (not Damus), look in message requests, and make sure this machine has internet. `firewatch test-alert` prints which relays took the message. |

## Working on Firewatch

```sh
scripts/demo-up.sh                              # build everything, check the nodes, start the demo
cargo test --workspace                          # 153 tests: engine, connectors, alerts, one per advisory
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

**Core Lightning** is read over `clnrest` with a rune restricted to `getinfo`, and to `listconfigs` for the four options Firewatch reads (where clnrest and cln-grpc listen):

```sh
lightning-cli createrune restrictions='[["method=getinfo","method=listconfigs"],["method/listconfigs","pnameconfig=clnrest-host","pnameconfig=clnrest-port","pnameconfig=grpc-host","pnameconfig=grpc-port"]]'
```

Asked for every option, `listconfigs` on Core Lightning before 26.06.9 returns secrets: bitcoind's RPC password, a `--recover` seed, the `wallet` database address and `tor-service-password` ([26.06.9 release notes](https://github.com/ElementsProject/lightning/releases/tag/v26.06.9)). This rune can't ask for those. A rune made with the older `[["method=getinfo","method=listconfigs"]]` still works, but `check` and `watch` warn about it: make the new one, then retire the old one with `lightning-cli blacklistrune start=<its unique_id>`.

**LND** is read over REST with `readonly.macaroon`. LND's self-signed `tls.cert` is pinned: Firewatch accepts exactly that certificate and nothing else.

```toml
[[nodes]]
kind = "lnd"
name = "lnd-old"
url = "https://127.0.0.1:8085"         # Polar's REST port for the node
macaroon_file = "/home/you/.polar/networks/2/volumes/lnd/lnd-old/data/chain/bitcoin/regtest/readonly.macaroon"
tls_cert = "/home/you/.polar/networks/2/volumes/lnd/lnd-old/tls.cert"
```

Use absolute paths; `~` isn't expanded. Add `lnd_conf = "/path/to/lnd.conf"` to also check where LND's APIs listen (see [Exposure](#exposure)): LND doesn't report it over RPC.

Firewatch probes every credential before trusting it and refuses one that can do more than read: a rune that can call `listfunds`, or a macaroon that can sign messages.

**A node on another machine** must be read over `https://`: the rune or macaroon goes in a header with every request, and over plain `http://` anyone on the network path can copy it. Firewatch refuses a plain `http://` url unless its host is `localhost` or a loopback address, before sending anything. For Core Lightning, run clnrest over HTTPS (its default) and set `tls_cert` to its `server.pem`; Firewatch pins it, so the self-signed certificate works. If the connection is already private (a Docker container on this machine, a VPN), set `allow_plain_http = true` on that node; `check` and `watch` then warn that the credential is sent unencrypted.

When a node can't be read, the report says why in plain words: the node isn't running, the request timed out, the TLS certificate isn't the pinned one, the url points at the wrong port or scheme, or the node rejected the credential. [docs/API.md](docs/API.md) lists every message. That node is Can't tell, never green, and the other nodes are still checked.

## Exposure

Each node also gets an `exposure` list: whether its REST and gRPC APIs listen beyond this machine. A leaked rune or macaroon only works for someone who can reach the API. In August 2026 a BTCPay Server flaw let attackers obtain LND macaroons, and funds were stolen; the [BTCPay Server 2.4.2 advisory](https://blog.btcpayserver.org/security-advisory-btcpay-server-2-4-2/) says: *"Do not leave an affected LND deployment accessible from the internet."*

- **Core Lightning:** `clnrest-host`/`clnrest-port` and `grpc-host`/`grpc-port`, read with `listconfigs`, one option per call, through the same restricted rune.
- **LND:** `restlisten` and `rpclisten` from `lnd.conf`, when `lnd_conf` is set, read the way LND reads them: quoted values, `tcp://` and `unix://` forms, a bare port meaning localhost, and LND's default ports (8080, 10009). Flags on LND's command line override the file and can't be seen, so an option the file doesn't set is `unknown`, not LND's localhost default. Point `lnd_conf` at the file LND actually loads (`lnd.conf` in its `lnddir`, or its `--configfile`): Firewatch can't tell which file LND read.

An API on `0.0.0.0`, `::` or a bare `:port` is `exposed`, with a hand check for the port. It's not an advisory verdict, so it never changes a node's red, amber or green. On Polar, Core Lightning nodes show `exposed` (Polar binds their APIs to `0.0.0.0` inside Docker), and LND nodes show `unknown` (Polar sets their listen addresses as command-line flags).

## Build check

A version number is only text a binary prints. LND also reports the commit it was built from, so Firewatch compares it with the commit the official release tag points to, from [releases/lnd.toml](releases/lnd.toml). Core Lightning reports no commit, only what `git describe` adds to its version when it isn't built as a release:

| `build_check` | Meaning |
|---|---|
| `match` | The binary is the official release its version claims. Verdicts are normal. |
| `mismatch` | Same version number, but a different commit, or the official commit built with changes (LND's `commit=` says `-dirty` or isn't an official tag). The version can't be trusted, so **every** finding on the node becomes Can't tell. |
| `unreleased` | Core Lightning built from a commit after a release tag (`v26.06.7-12-gabc1234`). It may hold changes or fixes the release doesn't, so **every** finding becomes Can't tell. |
| `modified` | Core Lightning built from a release tag with local changes (`v26.06.7-modded`). A clean source build often gets `-modded` too, when the build only regenerates files ([ElementsProject/lightning#7152](https://github.com/ElementsProject/lightning/issues/7152)), so red findings stay red, and Not affected becomes Can't tell: never green when unsure. |
| `unknown_release` | The version isn't in the table, for example a release newer than it. Verdicts are normal. |
| `not_checked` | Core Lightning released as tagged (`v26.06.9`), or LND without a commit or table. |

Refresh the table with `scripts/lnd-releases.sh`; it only replaces the file when the fetch succeeds. `--releases <file>` points at another one. The table is required: Firewatch stops if it's missing, rather than giving an unofficial build normal verdicts.

**Alerts** are NIP-17 encrypted Nostr DMs to the operator's key. Set `recipient` to your npub under `[alerts.nostr]`. Firewatch creates its own key in `secret_key_file` on first run and delivers to your NIP-17 inbox relays (kind 10050) as well as the relays listed. Any NIP-17 client can read them, for example 0xchat, Amethyst or Primal. To read them in a terminal:

```sh
cargo run -p firewatch-alerts --example inbox -- --new          # make a test key
cargo run -p firewatch-alerts --example inbox -- <nsec>         # print DMs sent to it
```

`watch` alerts when a finding turns red or amber, when it turns green again, and when a node has been unreachable for 3 checks in a row. A node restarting for an upgrade does not trigger an alert.

## Dashboard API

`watch` serves the latest report on `http://127.0.0.1:7070`. See [docs/API.md](docs/API.md) for the endpoints and the JSON format.

## Dashboard

`dashboard/` is the web page the operator watches: every node in one list, worst first, and the selected node's action, the developers' quoted fixes with links, what to check by hand and what a condition ruled out. Light by default, with a dark theme for dim rooms; it animates every change of verdict, and a lost or recovered connection arrives as a toast.

```sh
cd dashboard && npm install && npm run dev      # http://localhost:5173, reads the watch API
```

For the demo, `scripts/demo-up.sh` builds it and serves it from `watch` at `http://127.0.0.1:7070` (by hand: `npm run build`, then `watch --dashboard dashboard/dist`). Without a node network, `dashboard/dev/sim.sh` runs `watch` against simulated Core Lightning nodes, and `?fixture=demo` shows recorded sample data. See [dashboard/README.md](dashboard/README.md).

## Demo

The demo runs on a [Polar](https://lightningpolar.com) regtest network named `Firewatch-demo`: an old Core Lightning (`old-cln`, 26.06.7), a current one (`current-cln`, 26.06.9), and three LND nodes (`alice` on 0.20.0-beta, `lnd-old` on 0.18.5-beta, and `lnd-unofficial`, a 0.20.0-beta built from an unofficial commit). What to do is in [docs/DEMO.md](docs/DEMO.md); what to say is in [docs/PITCH.md](docs/PITCH.md).

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
| `dashboard/` | The operator's web dashboard (React, TypeScript, Vite, Tailwind), its fixtures, the live sim and the backup-video recorder. |
| `advisories/` | One TOML file per advisory. Only `verified = true` advisories run by default. |
| `docs/sources/` | The evidence for each Lightning Labs advisory: what was copied from where, and the audit. |
| `releases/` | Official release commits for the build check. |
| `scripts/` | `demo-up.sh`, `verify-demo.py`, `audit-quotes.py`, `lnd-releases.sh`. |
| `demo/` | Polar images, the upgrade and reset scripts, and the expected verdicts. |
| `examples/nodes.json` | Node snapshots for working without a live network. |
| `.github/workflows/ci.yml` | CI: Rust fmt, clippy and tests, and the dashboard's lint, format check, tests and build. |

## Docs

| File | What it is |
|---|---|
| [docs/PITCH.md](docs/PITCH.md) | The 3-minute pitch, deck outline and judge Q&A. |
| [docs/DEMO.md](docs/DEMO.md) | The demo runbook: setup, each beat, recovery. |
| [docs/API.md](docs/API.md) | The dashboard API and JSON contract. |
| [docs/sources/](docs/sources/) | Evidence for each Lightning Labs advisory, with the quote audit. |
| [docs/upstream-issue.md](docs/upstream-issue.md) | Draft proposal for the community advisory list (not filed yet). |
| [docs/event-rules.md](docs/event-rules.md) | What the organizers confirmed. |
| `docs/Firewatch-Project-Brief.pdf`, `docs/Firewatch-Task-List.pdf` | The original plan (Oct 20–22). The code has moved past parts of it; this README and the docs above are current. |

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
kind = "config_enabled"                # CLN: Can't tell for now; the rune reads only the API options
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
- For Core Lightning, the build check only sees what the version string admits (`-<n>-g<hash>`, `-modded`): it reports no commit, so a build that claims a clean tag can't be verified.
- It serves self-hosted node operators, not users of custodial or LSP-managed phone wallets.
- The advisory database is maintained by people; a missed advisory is a missed alert.
