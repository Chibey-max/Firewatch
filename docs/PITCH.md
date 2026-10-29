# Pitch

Three minutes, then questions. **Dave** speaks. **Isaac** drives the terminal and Polar. **Patrick** drives the dashboard and holds up the phone. The operational steps behind each beat are in [DEMO.md](DEMO.md).

## The one-liner

> **Dependabot for Lightning nodes, built to be trusted.**
> Firewatch tells a node operator, within seconds, whether a published security bug affects *their* running node, quotes the developers' own fix, and says so honestly when it can't be sure.

Say it in the first 20 seconds and again at the end.

## Script (target 2:50)

| Time | Beat | Dave says | On screen / who |
|---|---|---|---|
| 0:00–0:20 | **Hook** | "In its August release, Core Lightning wrote that 'increasingly capable AI models are being used to identify potential vulnerabilities in open-source code, significantly increasing the volume and pace of security reports.' In two months every major Lightning implementation shipped security fixes, and on October 2nd Core Lightning warned that attackers were going after nodes that hadn't upgraded. Would you have known your node was one of them?" | Slide 1 |
| 0:20–0:40 | **Problem** | "Bugs get fixed in days. But operators hear late, from X, if at all. And 'am I affected?' is hard: some bugs need a feature switched on, some only hit mainnet. Firewatch closes that gap. Think of it as Dependabot for Lightning nodes." | Slide 2 |
| 0:40–1:05 | **Live: it finds the problem** | "This is a real Lightning network running real node software. Firewatch reads each node with a credential that can't spend or change anything, and checks it against advisories copied word for word from the developers." | **Isaac:** runs `scripts/demo-up.sh`. **Patrick:** dashboard up; `old-cln` red with *exploited in the wild*. Phone buzzes: hold it up. |
| 1:05–1:20 | **It quotes, never invents** | "Red, two advisories. This is the developers' own sentence, and here's their link. We never write the fix ourselves." | **Patrick:** opens `old-cln`, points at *Developers say* and the source link. |
| 1:20–1:35 | **Not every alarm is real** | "A version number alone gives false alarms. This LND bug only bites on mainnet. This node runs regtest, so Firewatch says *not affected*, and tells you why." | **Patrick:** FW-2026-009 on `alice`, green, with its reason. |
| 1:35–1:50 | **Honest when unsure** | "This one only matters if an app intercepts your payments, which no RPC reveals. So instead of guessing, Firewatch goes amber and tells you exactly what to check. We never show green when we're not sure." | **Patrick:** FW-2026-013 on `lnd-old`, amber, with its hand check. |
| 1:50–2:05 | **The version can't lie** | "This node claims to be the official LND 0.20.0. But the commit it was built from isn't the official one, so Firewatch refuses to call anything on it safe." | **Patrick:** `lnd-unofficial`, all amber, *build `<hash>` is not the official v0.20.0-beta (b9ea707)*, where `<hash>` is your image's own commit ([DEMO.md](DEMO.md#building-the-network)). |
| 2:05–2:30 | **Fix it** | "The operator upgrades, by hand. Firewatch never touches the node." | **Isaac:** runs `demo/upgrade.sh`. Card turns green; phone gets ✅ *no longer affected*. Hold it up. |
| 2:30–2:50 | **Why trust it, why here** | "Read-only by design. Only the developers' own sources, checked word for word. Private alerts, end-to-end encrypted. And an open advisory format, proposed upstream, that anyone can add to with a pull request. Kaduna's bootcamps are training new node operators right now. None of them should find out about an exploit from X after losing funds. Firewatch: Dependabot for Lightning nodes." | Slides 5–6, with the QR code to the repo |

### The power-cut version of "Fix it" (C1)

Use this in place of the 2:05–2:30 row once it's been rehearsed. Same 25 seconds, and it shows Firewatch surviving the power cuts operators here deal with.

| Time | Beat | Dave says | On screen / who |
|---|---|---|---|
| 2:05–2:30 | **Power went; you still find out** | "Here, power goes. Watch: Firewatch is off, and while it's off, the operator upgrades. Power's back, Firewatch starts, and it remembers what it knew, so your phone still finds out what changed." | **Isaac:** stops `watch` (Ctrl+C, standing in for the power cut), runs `demo/upgrade.sh`, then starts `watch` again. Phone gets one message: *While Firewatch was off (since 14:02): ✅ old-cln is no longer affected by FW-2026-003, FW-2026-006*. Hold it up. |

- **Measured:** the catch-up is sent on the first check after `watch` starts. On 28 Oct the message was ready the second the first check finished; with Nostr on, add the few seconds Firewatch takes to connect to the relays. Time it in rehearsal and write the number here: ____ s.
- **"Data went": A5 done (29 Oct, Isaac, with PR #34).** Laptop Wi-Fi switched off, `demo/upgrade.sh` run while offline, Wi-Fi switched back on, an Android phone running Amethyst on mobile data. Three runs: the waiting alert was sent **25 s, 12 s and 13 s** after the Wi-Fi was switched on (13 s, 5 s and 6 s after the laptop was back online; times from the system log). Each run: **one** message on the phone, ending *"Detected 01:16:15, sent 01:16:34 (no connection until then)."* The phone buzzed within ~2 s of sending in two runs, and about a minute later in one. Firewatch sent to damus, nos.lol and primal, but Amethyst reads private messages from its own DM inbox relays (here auth.nostr1.com, relay.0xchat.com, nos.lol), so only nos.lol reached the phone directly. With the phone's relays found, a test alert arrived instantly; Firewatch now finds them from the Amethyst share link (PR #35). On stage, allow up to half a minute after the Wi-Fi comes back for the send; if the phone is still quiet after that, use the fallback line: "It's queued; it goes the moment the connection's back."
- **Fallback line** if the phone is slow: "It's on its way: it's sent the moment Firewatch is back up."

Rules for the script:
- Every claim matches what's on screen. Don't name an advisory that isn't in the demo.
- If the phone doesn't buzz, keep going; the dashboard is the demo. Say "the alert's on its way" once, then move on.
- Never present a snapshot or video as live. If you switch to the backup video, say so.

## Deck (6 slides, for Patrick)

The current `docs/firewatch-pitch.pdf` predates the build check, `lnd-unofficial` and the AI hook. Rebuild it to this outline and export it over the same file.

1. **Hook.** The quote from Core Lightning's 26.06.7 release notes (above), attributed, and "Oct 2: attackers target unpatched Core Lightning nodes."
2. **The gap.** "Bugs get fixed in days. Operators hear late." (No "nodes get patched in months": we have no source for it.) Then the three gaps: operators hear late; "am I affected?" is hard; the right action changes.
3. **Firewatch.** The one-liner, and the three verdicts as three coloured cards: Affected, Can't tell, Not affected. Caption: "Never green when unsure."
4. **Live demo.** Title only, so Patrick can switch to the dashboard.
5. **Why trust it.** Four short lines: read-only credentials it can't misuse; only developers' sources, audited word for word; build check (the version can't lie); private, encrypted alerts.
6. **What's next and who.** The open advisory format proposed upstream; the repo QR code; "Built for the node operators Kaduna is training"; Dave, Isaac, Patrick.

Keep every slide under 15 words of body text. The deck should run in under 60 seconds without the demo.

## Numbers you can say

All true as of Oct 24. Re-check before stage:
- **11** verified advisories across Core Lightning, LND and LDK, each quoting its source word for word (`scripts/audit-quotes.py`).
- **121** automated tests; CI on every pull request.
- **5** live nodes in the demo, covering every verdict.
- Reads Core Lightning and LND; checks LND builds against **93** official releases.

## Judge Q&A

Answer in under 20 seconds, then stop. If you don't know, say so and offer to follow up.

| Question | Owner | Answer |
|---|---|---|
| Isn't this just Lightning Labs' advisory page? | Dave | That page lists LND advisories. We also cover Core Lightning and LDK, and we check *your* node's version, network, config and build, then tell you whether it applies and what the developers said to do. |
| Why not just watch GitHub releases? | Dave | A release says an update exists. We say whether your node is affected, and why. Half the demo was a scary-looking bug that correctly doesn't apply. |
| How do you know your data is right? | Dave | Every advisory comes from the implementation's own release notes or advisory page. A script checks every quote word for word against the live page, a named person reviews each entry, and unverified entries don't run. |
| What if Firewatch itself is hacked? | Isaac | It only holds read-only credentials: a Core Lightning rune limited to two read calls, and LND's read-only macaroon. It tests every credential before using it and refuses one that can do more. It can't spend, close channels or change settings. |
| What if the version string lies? | Isaac | For LND we compare the commit the binary was built from with the official release tag; a mismatch makes every finding Can't tell. You saw it on `lnd-unofficial`. Core Lightning doesn't report its commit, so there we read what its version string admits, like a later commit or local changes. |
| Why amber instead of a best guess? | Isaac | A wrong green is worse than no answer: it tells someone with a vulnerable node they're safe. Amber says exactly what to check by hand. |
| Why Nostr for alerts, not Telegram? | Isaac | "Your node is vulnerable" on a third-party server tells others which node to attack. Nostr DMs are end-to-end encrypted to the operator's key, and the gift wrap hides who sent them and when. |
| How fast do new advisories get in? | Dave | Today it's people: we add a file and a test when the developers publish. That's the honest limit: a missed advisory is a missed alert. That's why we've proposed a machine-readable format to the community advisory list. |
| Does cargo-audit already cover LDK? | Isaac | For LDK inside Rust apps, partly. cargo-audit can't see running LND or Core Lightning nodes, and those are our core. |
| Who maintains it after the hackathon? | Dave | It's open source: one file per advisory, a test per file, pull requests welcome. We've drafted a proposal to the community list so the data has a shared home. A maintained advisory database is the kind of ongoing work Btrust's grants fund. |
| Who is this for? | Dave | Self-hosted node operators: home nodes, Umbrel and Start9 users, small routing nodes. Kaduna's bootcamps are producing exactly these people. It's not for users of custodial phone wallets. |
| How does someone install it? | Isaac | One command: the installer from our GitHub releases puts a single binary on the machine, with the advisories and dashboard built in. Then `firewatch init` finds the node, sets up a read-only connection and asks for your phone's Nostr key. For LND that's it; Core Lightning needs one copy-pasted rune. |
| Does it work offline? | Isaac | Checking and the dashboard work fully offline. Only the phone alerts need internet to reach a Nostr relay. |
| Why does the UI look like this? | Patrick | One glance per node: icon, word and colour, never colour alone. The fix is quoted with its source, and hand checks only appear where they're needed. It's built to be readable from the back of the room. |
| Why Kaduna? | Dave | New operators here learn from bootcamps, not developer Discords. The warning has to reach their node, not a feed they don't follow. |

## Before stage

- [ ] Script read aloud with a stopwatch: under 2:50.
- [ ] Every claim checked against the screen in a full run.
- [ ] Each person answers their Q&A rows in under 20 seconds without notes.
- [ ] Printed copy of this page for each person.
