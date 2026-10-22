# Firewatch brief: updates to paste

Each block says where it goes in the Google Doc. Before pasting, check that the quoted release-note wording matches the official pages exactly.

---

## 1. The problem → table: add this row at the top

| Date | Software | What happened |
|---|---|---|
| Oct 7, 2026 | Core Lightning | 26.06.9 shipped more security fixes (channel reestablishment, splicing, HTLC handling during shutdown); developers "strongly recommend upgrading" ([release notes](https://github.com/ElementsProject/lightning/releases/tag/v26.06.9)) |

In the Sep 22 row, change the link to the official page: [release notes](https://github.com/ElementsProject/lightning/releases/tag/v26.06.8)

---

## 2. Advisory database → replace the whole TOML example with this

```toml
id = "FW-2026-003"
implementation = "cln"
title = "Vulnerabilities fixed in 26.06.8"
affected = "< 26.06.8"
fixed = "26.06.8"
severity = "critical"            # critical | high | medium | low
exploited_in_wild = true
source = "https://github.com/ElementsProject/lightning/releases/tag/v26.06.8"
reviewed_by = "Dave"

# every condition must hold for 'Affected'
# a condition we cannot read over RPC makes the verdict 'Can't tell'
[[conditions]]
kind = "version_range"           # version_range | config_enabled | feature_bit | manual

[remediation]
action = "upgrade"               # upgrade | offline | disable_feature
steps = "We strongly recommend upgrading to this release."   # quoted from the source, never written by us
```

---

## 3. Advisory database → add this right after the TOML example

**Condition kinds**

- **`version_range`**: the node's version falls inside `affected`. Ranges use comparison syntax, joined by commas for "and": `< 26.06.8`, `>= 0.18.0, < 0.21.0`. Versions are normalized first: the leading `v` is dropped, `-beta`/`rcN` is read as a pre-release, and Umbrel/Start9 suffixes are removed.
- **`config_enabled`**: a named option is on. Read from CLN's `listconfigs`. Field: `option = "experimental-splicing"`.
- **`feature_bit`**: a feature bit is set. Read from LND's `GetInfo`. Field: `bit = <n>`.
- **`manual`**: depends on something no RPC exposes, such as peer behavior or how channels have been closed. It always returns **Can't tell**. Field: `check_hint = "What the operator should check by hand, and how"`.

---

## 4. Advisory database → seed list table: replace the FW-2026-003 and FW-2026-005 rows, and add FW-2026-006

| ID | Software | Affected | Fixed | Trigger condition | Action | Source status |
|---|---|---|---|---|---|---|
| FW-2026-003 | Core Lightning | < 26.06.8 | 26.06.8 | Version only | Upgrade; attacks reported Oct 2 | **Verified:** [v26.06.8 release notes](https://github.com/ElementsProject/lightning/releases/tag/v26.06.8) |
| FW-2026-005 | LND | before 0.21.0 | 0.21.0 | `manual`: cooperative-close + one-block-reorg attack using a revoked commitment; depends on peer behavior, which RPC can't read | Upgrade | News only ([CryptoSlate](https://cryptoslate.com/old-lightning-nodes-may-be-exposed-to-a-full-channel-wipeout-after-lnd-fix-lands-later-than-disclosed/)); find the primary disclosure |
| FW-2026-006 | Core Lightning | < 26.06.9 (confirm range) | 26.06.9 | Version only | Upgrade | **Verified:** [v26.06.9 release notes](https://github.com/ElementsProject/lightning/releases/tag/v26.06.9) |

---

## 5. Advisory database → add this below the seed list

**Note on CLN ranges.** The CLN release notes don't list affected versions for each bug. Our ranges are "everything below the fixed release." The v26.06.7 notes say "Users pinned to v26.06.6 or earlier were never affected," but that line refers to a Docker image that briefly shipped without the fixes, not to the vulnerabilities themselves. Dave to confirm before Q&A.

---

## 6. Verdict logic → three-verdicts table: replace the "Can't tell" row

| Verdict | When | What the operator sees |
|---|---|---|
| Can't tell | Version in range, but a condition can't be read over RPC, or the condition is `manual` | Amber: which setting to check by hand, and how (from `check_hint`) |

---

## 7. Demo script → replace steps 1, 3, 4 and 5 and the pre-check

1. **Setup (on screen before you start):** a Polar regtest network with three nodes: an old Core Lightning (26.06.7), a current Core Lightning (26.06.9), and an LND node older than 0.21.0. Firewatch dashboard open, a phone with a Nostr client on the projector.

3. **Scan (30 s):** run `firewatch check`. The old CLN node turns red for FW-2026-003 and FW-2026-006. The phone buzzes with the encrypted alert and the quoted fix.

4. **Honesty moment (30 s):** the LND node shows amber for FW-2026-005, because whether it's exposed depends on peer behavior that no RPC can show. The dashboard tells the operator what to check by hand. "We never show green when we are not sure." *Fallback: if FW-2026-005 has no primary source by Oct 28, drop this step and explain Can't tell out loud. Never demo an unverified advisory.*

5. **Fix (45 s):** upgrade the old node to 26.06.9 in Polar. Firewatch re-checks automatically, and the node turns green on both advisories, with the reason shown.

**Pre-check before Oct 29:** confirm Polar can run CLN 26.06.7 and upgrade that node to 26.06.9, and can run an LND version older than 0.21.0. If it can't, build those versions from source in Docker. Either way, record a backup video of the full run.

---

## 8. Team table → replace the Rust 3 "Done means" cell

Nostr DM alerts, re-check every N minutes, CLI `firewatch check`

## 9. Must-have / Stretch line: no change needed

Telegram is already listed under Stretch. The edit in section 8 removes the contradiction.

---

## 10. Prep before Oct 29 → replace the second checklist item

- ☐ ~~FW-003 official source~~ (done: v26.06.8 release notes). Find the primary disclosure for FW-005. Find an official statement for FW-004 or drop it. Confirm the FW-006 range.

**Add:**

- ☐ Dave: draft the advisory format spec and shared types (`Advisory`, `NodeInfo`, `Verdict`, dashboard JSON) so the hour-0 meeting takes 15 minutes

---

## 11. Sources → add

- [Core Lightning v26.06.8 release notes](https://github.com/ElementsProject/lightning/releases/tag/v26.06.8)
- [Core Lightning v26.06.9 release notes](https://github.com/ElementsProject/lightning/releases/tag/v26.06.9)
- [CryptoSlate: LND cooperative-close flaw](https://cryptoslate.com/old-lightning-nodes-may-be-exposed-to-a-full-channel-wipeout-after-lnd-fix-lands-later-than-disclosed/)
