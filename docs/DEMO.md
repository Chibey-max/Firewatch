# Demo runbook

Three minutes on a Polar regtest network. Two nodes go red, a finding stays green because a condition rules it out, a node goes amber because it can't be read, and the red Core Lightning node turns green after an upgrade, with a phone receiving each alert.

Every advisory shown is verified against the developers' own release notes or advisory page. No unverified advisory is used, so the daemon runs without `--include-unverified`.

## Before you go on stage

1. **Polar:** open the `Firewatch-demo` network and click **Start**. It has `old-cln` (Core Lightning 26.06.7), `current-cln` (26.06.9) and `alice` (LND 0.20.0-beta).
2. **Reset the vulnerable node**, in case the last rehearsal left it upgraded:
   ```sh
   demo/reset.sh
   ```
3. **Phone:** a NIP-17 client (0xchat, Amethyst or Primal) logged in with the key whose npub is `recipient` in `firewatch.toml`. Send a test alert and confirm it arrives:
   ```sh
   cargo run --release -- test-alert
   ```
4. **Build once** so nothing compiles on stage: `cargo build --release`.
5. **Screen:** the dashboard in one window and a terminal in `~/Projects/Hackathon/Firewatch` in another. The phone goes on the projector.
6. **Interval:** set `interval_secs = 5` under `[watch]` in `firewatch.toml` so changes show within 5 seconds.

Do not start `watch` yet: starting it is what makes the phone buzz.

## On stage

| # | Time | Say | Do |
|---|---|---|---|
| 1 | 15 s | "On October 2, Core Lightning warned that attackers were targeting unpatched nodes. Would you have known your node was one of them?" | |
| 2 | 30 s | "Firewatch reads each node with a credential that can't spend or change anything." | Run `./target/release/firewatch watch`. The dashboard shows `old-cln` red (FW-2026-003, FW-2026-006) and `alice` red (FW-2026-007, FW-2026-008). The phone buzzes with encrypted alerts quoting the developers' fix. |
| 3 | 30 s | "A version number alone gives false alarms. This LND bug only bites on mainnet, so on this regtest node Firewatch says not affected, and shows why." | Point at FW-2026-009 on `alice`: green, with "it only applies on mainnet, and this node runs on regtest". |
| 4 | 20 s | "And when Firewatch can't confirm, it never shows green." | In Polar, stop `current-cln`. Within one check it turns amber: "Could not read this node". |
| 5 | 45 s | "The operator upgrades, by hand. Firewatch never touches the node." | Run `demo/upgrade.sh`. Within one check `old-cln` turns green, and the phone gets "✅ old-cln is no longer affected". |
| 6 | 30 s | "Read-only credentials, fixes quoted from the developers, private alerts, and an open advisory format anyone can contribute to." | |

## After the demo

```sh
demo/reset.sh            # old-cln back to 26.06.7 with its original data
```

Start `current-cln` again in Polar.

## If something breaks

| Problem | Fix |
|---|---|
| No phone buzz | Check the `watch` log for `alert sent (N relays)`. If it says `ALERT FAILED`, the venue is blocking the relays: switch to the mobile hotspot. The dashboard still shows everything. |
| `old-cln` doesn't come back after `upgrade.sh` | `docker logs polar-n<id>-old-cln`, then `demo/reset.sh` and play the backup video. |
| Polar network won't start | Play the backup video. |

## Likely questions about the data

- **"Why is alice red?"** LND 0.20.0-beta is affected by two Lightning Labs advisories, both version-only, both fixed in 0.20.1-beta. It's real: Polar's newest LND image is 0.20.0.
- **"Why isn't the DNS-seed bug red?"** The DNS peer bootstrapper runs by default on mainnet only, and Firewatch reads the network over RPC. On mainnet the finding would be amber, because `nobootstrap` lives in `lnd.conf`, which RPC doesn't expose.
- **"What about the LND cooperative-close reorg bug in the news?"** It isn't on Lightning Labs' advisory page, so it stays unverified (FW-2026-005) and doesn't run. Firewatch only alerts on what the developers themselves published.
