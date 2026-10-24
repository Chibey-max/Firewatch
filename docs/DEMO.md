# Demo runbook

Three minutes on a Polar regtest network. Nodes go red on verified advisories, one finding goes amber because its condition can't be read over RPC, another stays green because a condition rules it out, a node that isn't the official build goes amber, and the red Core Lightning node turns green after an upgrade, with a phone receiving the alerts.

What to **say** at each beat is in [PITCH.md](PITCH.md). This page is what to **do**.

Every advisory shown is verified against the developers' own release notes or advisory page. No unverified advisory is used, so the daemon runs without `--include-unverified`.

## Before you go on stage

1. **Polar:** open the `Firewatch-demo` network and click **Start**. It has five nodes: `old-cln` (Core Lightning 26.06.7), `current-cln` (26.06.9), `alice` (LND 0.20.0-beta), `lnd-old` (LND 0.18.5-beta) and `lnd-unofficial` (LND 0.20.0-beta built from an unofficial commit; see below to add it).
2. **Reset the vulnerable node**, in case the last rehearsal left it upgraded:
   ```sh
   demo/reset.sh
   ```
3. **Phone:** a Nostr app that shows NIP-17 private messages, logged in with the key whose npub is `recipient` in `firewatch.toml`.
   - **iPhone: Nostur.** Tested and working. Damus does *not* show these messages.
   - **Android:** Amethyst or 0xchat.

   Send a test alert and confirm it arrives:
   ```sh
   cargo run --release -- test-alert
   ```
   The first message lands in **message requests**: tap **Accept message request** and follow the sender (`npub18t9qd5qczhg3533vqzhmzwlwnnpnnuhplje6fvyx9ty3mer85gdqc78aas`), so stage alerts arrive in the main inbox.
4. **Interval:** set `interval_secs = 5` under `[watch]` in `firewatch.toml` so changes show within 5 seconds.
5. **Check every verdict** against the expected table, and that every source link opens:
   ```sh
   scripts/verify-demo.py
   ```
   It must end with "All demo checks pass."
6. **Screen:** a terminal in `~/Projects/Hackathon/Firewatch`, and a browser ready for `http://127.0.0.1:7070`. The phone goes on the projector.

Do not start the daemon yet: starting it is what makes the phone buzz.

## On stage

The beats match [PITCH.md](PITCH.md). Alerts are one DM per node per check, so starting the daemon sends four: `old-cln`, `alice`, `lnd-old` and `lnd-unofficial`.

| # | Time | Beat | Do |
|---|---|---|---|
| 1 | 0:00 | Hook | Slide 1. |
| 2 | 0:20 | Problem | Slide 2. |
| 3 | 0:40 | It finds the problem | **Isaac:** run `scripts/demo-up.sh`. It builds, checks every node and opens the dashboard. **Patrick:** `old-cln` is red, *exploited in the wild*; hold up the phone when it buzzes. |
| 4 | 1:05 | It quotes, never invents | **Patrick:** open `old-cln`; point at *Developers say* and the source link. |
| 5 | 1:20 | Not every alarm is real | **Patrick:** FW-2026-009 on `alice`: green, "it only applies on mainnet, and this node runs on regtest". |
| 6 | 1:35 | Honest when unsure | **Patrick:** FW-2026-013 on `lnd-old`: amber, with its hand check. |
| 7 | 1:50 | The version can't lie | **Patrick:** `lnd-unofficial`: every finding amber, "its build (fc472ef) is not the official v0.20.0-beta release (b9ea707)". |
| 8 | 2:05 | Fix it | **Isaac:** run `demo/upgrade.sh`. Within one check `old-cln` turns green, and the phone gets one "✅ old-cln is no longer affected by FW-2026-003, FW-2026-006". |
| 9 | 2:30 | Why trust it | Slides 5–6. |

## Adding `lnd-unofficial`

```sh
docker build -f demo/polar/Dockerfile.lnd-unofficial -t firewatch/lnd:0.20.0-unofficial demo/polar
```

In Polar: **Manage Images → Custom Nodes → Add**, implementation LND, image `firewatch/lnd:0.20.0-unofficial`, then drag it onto the network as `lnd-unofficial`. Rename it **while the network is stopped**, or Polar keeps running it under its first name. Add it to `firewatch.toml` like `lnd-old`, with its own REST port, `readonly.macaroon` and `tls.cert`.

## After the demo

```sh
demo/reset.sh            # old-cln back to 26.06.7 with its original data
```


## If something breaks

| Problem | Fix |
|---|---|
| No phone buzz | Check the `watch` log for `alert sent (N relays)`. If it says `ALERT FAILED`, the venue is blocking the relays: switch to the mobile hotspot. The dashboard still shows everything. |
| `demo-up.sh` stops at "Checking every node" | It prints which node is down and why. Start the network in Polar, wait for every node to go green, and run it again. |
| `old-cln` doesn't come back after `upgrade.sh` | `docker logs polar-n<id>-old-cln`, then `demo/reset.sh` and play the backup video. |
| Polar network won't start | Play the backup video. |
| A node card says "Could not read this node" | The node is stopped or restarting. Start it in Polar; the card recovers on the next check. If asked, it's also the honesty point: an unreadable node is never green. |

## Likely questions about the data

- **"Why do the nodes have an exposure note?"** Polar binds the Core Lightning nodes' APIs to `0.0.0.0` inside Docker, so they show `exposed`; it passes LND's listen addresses as command-line flags, which can't be read, so LND nodes show `unknown`. The exposure check never changes a node's colour: it's a note to check the port isn't reachable from the internet, not an advisory verdict. Don't feature it on stage.
- **"Why is lnd-old amber on FW-2026-013?"** The bug only affects nodes where an app uses LND's `HtlcModifier` RPC to intercept invoice payments, such as Lightning Terminal running tapd. RPC can't tell which apps are connected, so Firewatch lists the hand check instead of guessing.
- **"What if the version string lies?"** For LND, Firewatch compares the commit the binary was built from with the commit of the official release tag, and checks that the build describes itself as that tag, so a patched build of the right commit is caught too. On a mismatch every finding becomes Can't tell. Core Lightning doesn't report its commit, so for CLN Firewatch reads what the version admits: a build from a later commit (`-12-gabc1234`) is Can't tell, and a `-modded` build is never shown green.
- **"Why is alice red?"** LND 0.20.0-beta is affected by two Lightning Labs advisories, both version-only, both fixed in 0.20.1-beta. It's real: Polar's newest LND image is 0.20.0.
- **"Why isn't the DNS-seed bug red?"** The DNS peer bootstrapper runs by default on mainnet only, and Firewatch reads the network over RPC. On mainnet the finding would be amber, because `nobootstrap` lives in `lnd.conf`, which RPC doesn't expose.
- **"What about the LND cooperative-close reorg bug in the news?"** It isn't on Lightning Labs' advisory page, so it stays unverified (FW-2026-005) and doesn't run. Firewatch only alerts on what the developers themselves published.
