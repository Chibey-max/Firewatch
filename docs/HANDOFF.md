# Firewatch: project history and handoff

Last updated: 28 Oct 2026. If you are a person or an AI assistant picking this work up, read this first, then `README.md`, `docs/PITCH.md`, `docs/DEMO.md` and `docs/API.md`. The task lists are the PDFs in `docs/`: `Firewatch-Project-Brief.pdf`, `Firewatch-Task-List.pdf` and `Firewatch-Offline-Languages-Tasks.pdf` (the newest).

## The project

Firewatch is a read-only Rust tool that checks LND and Core Lightning nodes against verified security advisories. Each node gets one of three verdicts: **Affected** (red), **Can't tell** (amber) or **Not affected** (green). It sends encrypted Nostr alerts (NIP-17 DMs) and serves a dashboard.

- **Event:** Btrust Bitcoin Build-a-thon, DevFest Kaduna. Build day is 29 Oct 2026; the demo is 31 Oct 2026. The organizers confirmed twice (22 and 24 Oct) that building beforehand is allowed.
- **Team:**

  | Who | Role | GitHub |
  |---|---|---|
  | Dave | Rust 1: matching engine, advisories, release, pitch | `Chibey-max` (repo owner) |
  | Isaac | Rust 2: connectors, security hardening | `CHEF-SAVY` |
  | Patrick | Frontend (dashboard) | `Phantomcall` |

- **Repo:** https://github.com/Chibey-max/Firewatch
- **Install:**
  ```sh
  cargo install firewatch
  ```
  or the shell installer on GitHub Releases (v0.1.1 once tagged).

## Rules for anyone working on it

1. **Never add AI attribution** to commits, pushes or PRs: no `Co-Authored-By: Claude`, no "Generated with Claude Code".
2. **Never handle tokens.** Dave runs `cargo login` himself. Don't paste crates.io tokens, macaroons, runes or Nostr secret keys anywhere.
3. **Quote remediation text word for word** from the developers' official notes. Never reword it. `scripts/audit-quotes.py` checks this.
4. **Only verified advisories go in the demo.** FW-2026-005 is unverified. FW-2026-004 was deleted.
5. **Use the right GitHub account.** `gh` must be logged in as **Chibey-max**, not Devcyprian.
6. **Get an explicit yes before each merge, tag or crates.io publish.** A crates.io publish can't be undone.
7. **Make every change through a PR.** `main` is protected: each PR needs 1 approval plus the CI checks "Rust (fmt, clippy, tests)" and "Dashboard (lint, format, tests, build)". Dave can't approve his own PRs, so Isaac or Patrick approve them.

## Code map

| Crate | What it does |
|---|---|
| `crates/firewatch-core` | The engine: advisories, version ranges, the release table and build check, reports, changes between checks, exposure |
| `crates/firewatch-connectors` | Core Lightning over clnrest with a rune. LND over REST with a macaroon and a pinned `tls.cert`. |
| `crates/firewatch-alerts` | NIP-17 Nostr DMs (nostr-sdk 0.45). `lang.rs` holds the alert text in each language. |
| `crates/firewatch-cli` | The `firewatch` binary: `check`, `watch`, `init`, `test-alert`, `validate`. `state.rs` remembers the last check across restarts. |
| `dashboard/` | Patrick's web UI. The `watch` daemon serves it. |
| `advisories/*.toml` | The advisories, each linked to its source in `docs/sources/` |
| `releases/lnd.toml` | Official LND release commits, used by the build check |
| `demo/`, `scripts/` | Polar demo setup, `upgrade.sh`, `reset.sh`, `verify-demo.py`, `audit-quotes.py`, `package-data.sh` |

Key behaviour:

- **Read-only credentials only.** Before using a credential, Firewatch probes it: `listfunds` on Core Lightning and `signmessage` on LND. If the probe succeeds, the credential can do more than read, so Firewatch refuses it.
- **The narrow Core Lightning rune (PR #26):**
  ```
  [["method=getinfo","method=listconfigs"],["method/listconfigs","pnameconfig=clnrest-host","pnameconfig=clnrest-port","pnameconfig=grpc-host","pnameconfig=grpc-port"]]
  ```
- **No plain `http://` to another machine.** Firewatch refuses it unless the node sets `allow_plain_http = true` (#27 and #29).
- **The dashboard API serves this machine only,** with DNS-rebinding and CORS guards.
- **Build check:** Firewatch compares each LND node's commit with `releases/lnd.toml` (`match`, `mismatch`, `unknown_release`, `not_checked`). Core Lightning nodes get `unreleased` or `modified`.
- **Data is built into the binary:** the advisories, the release table and `dashboard/dist`, by `crates/firewatch-cli/build.rs`.
- **Config lookup order:** `--config`, then `./firewatch.toml`, then `~/.config/firewatch/firewatch.toml`.
- **Alerts:**
  - Each check sends one grouped DM per node.
  - An "unreachable" alert goes out only after 3 failed checks in a row.
  - After a restart, the first check sends one "While Firewatch was off (since HH:MM)" catch-up, and only if something changed. The state is kept in `state.json` next to the config, mode 0600, written atomically.
- **Alert languages:**
  - `language = "ha" | "yo" | "ig" | "pcm"` under `[alerts]` sets the language.
  - All four non-English languages are **unreviewed drafts**. They are refused unless `allow_draft_language = true`, and every alert is then marked as a draft.
  - The developers' fix text stays in English.
- **Alerts that can't be sent wait** in an outbox (`crates/firewatch-cli/src/outbox.rs`), one findings message and one reachability message per node, latest state wins. Every check tries to send them, oldest first, stopping at the first failure. A late message ends with "Detected 14:02, sent 14:40 (no connection until then)." Red then fixed before anything got through becomes one message. The outbox is saved in `state.json` too.

## Releases

- **Versions:** v0.1.0 is out on GitHub Releases and crates.io, as all four crates. v0.1.1 (A1–A4, draft languages) is bumped in PR "Release v0.1.1"; it ships once the tag is pushed and crates.io is published. rc.2 is yanked on crates.io.
- **How a release works:** cargo-dist 0.32 builds when a `v<version>` tag is pushed. The tag must exactly match `version` in `Cargo.toml`, so bump the version in a PR first.
- **Trap:** cargo-dist drops `working-directory` from the steps in `.github/build-setup.yml`, so those steps `cd dashboard` themselves.
- **Moving or deleting a pushed tag:** Dave runs those git commands himself.
- **Publishing to crates.io** (only after Dave says yes):
  ```sh
  scripts/package-data.sh
  cargo publish --workspace
  ```
  The token is scoped. A yank-only token can't publish.

## Polar demo network

The network is `Firewatch-demo` (Polar network 2).

| Node | Version | REST port |
|---|---|---|
| old-cln | Core Lightning 26.06.7 | 8182 |
| current-cln | Core Lightning 26.06.9 | 8183 |
| alice | LND 0.20.0 | 8081 |
| lnd-old | LND 0.18.5 | 8085 |
| lnd-unofficial | custom LND build | 8086 |

- **Custom images:** `firewatch/clightning:26.06.7`, `firewatch/clightning:26.06.9` and `firewatch/lnd:0.20.0-unofficial`, built from `demo/polar/`.
- **Expected results:** `demo/expected.toml`. Check them with `scripts/verify-demo.py`.
- **Local config:** `firewatch.toml` is git-ignored and uses the narrow runes. The old wide runes are blacklisted.
- **After a laptop reboot,** bitcoind (`polar-n2-backend1`) has no restart policy. Run `docker start polar-n2-backend1`. See `docs/DEMO.md`.

## Phone alerts

- **App:** use Nostur on iPhone. Damus doesn't show NIP-17 DMs.
- **Dave's Nostur npub:** `npub1d06e93lahjrmxspq0gsal3d2elp8alctye2chj9ef5kyemclwrkqrdaxzw`
- **Firewatch's sender npub:** `npub18t9qd5qczhg3533vqzhmzwlwnnpnnuhplje6fvyx9ty3mer85gdqc78aas`. Its secret key stays local and is never committed.
- **Draft test alerts (28 Oct):** all four arrived. Yoruba and Igbo special letters display correctly. Still to check: whether the Hausa ƙ in "saƙonni" displays on the phone or shows as a plain k.

## What has been done (in order)

1. Read the brief and fixed its contradictions in the docs.
2. Built the backend: the engine, connectors, Nostr alerts, `watch`, the demo scripts and the build check.
3. Set up the Polar demo network, including the custom images and the lnd-old and lnd-unofficial nodes.
4. Reviewed and merged the team's PRs: narrow rune (#26), plain-http refusal (#27/#29), Docker ports on this machine only (#28), `init` hang fix (#22), and others.
5. Made it installable with one command: built-in data, `firewatch init`, cargo-dist releases and crates.io.
6. Wrote the README getting-started guide and the phone setup guide.
7. Wrote the pitch (`docs/PITCH.md`) and the demo runbook (`docs/DEMO.md`).
8. Released v0.1.0 (PR #30) and published it to crates.io.
9. **PR #31, merged 28 Oct**, for tasks A4, B6, B1, B2, A8 and C1 in the offline and languages task list:
   - Remember state across restarts.
   - Alerts in Hausa, Yoruba, Igbo and Pidgin, as drafts.
   - Reviewer sheets in `docs/strings/alerts-*.csv`, with instructions in `docs/strings/README.md`.
   - A power-cut version of the "Fix it" pitch beat.
   - All of it tested live on Polar. 153 tests pass.

## What is left

| Item | Owner | Notes |
|---|---|---|
| Check the Hausa ƙ in Nostur | Dave | Zoom in on "saƙonni" in the test alert. |
| B0/G1: native-speaker reviewers per language | Dave | Unknown whether any agreed. Only reviewed languages can be switched on. |
| B7: put the reviewed text in | Dave, once the reviewed CSVs return | Follow `docs/strings/README.md`. In `lang.rs`, set `reviewed: true` and fill in `reviewer`. |
| B8: language-related dashboard work | Patrick, with the reviewers | |
| A1–A3: outbox, "Detected … sent …" line, delivery status in `/api/health` | Dave (was Isaac's) | Built on branch `dave/a1-outbox`, tested against a local relay. Needs a review and merge. Tell Isaac so the work isn't done twice. |
| A5: outage runs with the Wi-Fi off | Dave and Isaac | The power-cut run is done. The Wi-Fi-off runs can start once A1 merges: Wi-Fi off, `demo/upgrade.sh`, Wi-Fi on, time until the phone buzzes. Three times; write the times in `docs/PITCH.md`. |
| A6: dashboard shows waiting alerts | Patrick | Reads `alerts_pending` and `oldest_pending_since` from `/api/health` (see `docs/API.md`). |
| G2: tag v0.1.1 (28 Oct, 18:00) | Dave | Version-bump PR, then approval, then Dave pushes the tag, then a crates.io publish once Dave says yes. |
| Docker lockdown from #28 | Dave | Needs sudo on Dave's machine. |
| Rehearsals and a backup demo video | Team | Time the pitch. Fill in the time blank in the C1 beat. |
| Ask the organizers for the judging criteria; line up real Kaduna node operators | Dave | |

## How to check everything still works

```sh
cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace                  # 171 tests
python3 scripts/verify-demo.py          # needs the Polar network running
python3 scripts/audit-quotes.py
```
