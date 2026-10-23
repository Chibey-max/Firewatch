# Draft issue for ekzyis/lightning-vulns

Status: **draft, not filed.** File it only once Dave, Isaac and Patrick agree (task D6-4). On stage, say "proposed upstream", not "accepted".

---

**Title:** Proposal: a machine-readable file per advisory, next to the README

Hi! Thanks for keeping this list. As far as we can tell it's the only place that tracks advisories across LND, Core Lightning, Eclair, LDK and btcd together.

We're building [Firewatch](https://github.com/Chibey-max/Firewatch), a read-only tool that checks a running LND or Core Lightning node against published advisories and tells the operator whether that node is affected. We'd like to read advisories from this list instead of keeping our own copy, but today each entry is prose. A tool can't tell from it which versions are affected, or what condition has to hold.

**Proposal:** alongside each README section, a small TOML file under `advisories/` with the same facts in fields. The README stays the human-readable version; nothing about it changes.

```toml
# advisories/eclair-channel-actor-leak-via-open-channel-race.toml
title = "Eclair: Channel Actor Leak via Open Channel Race"
implementation = "eclair"
affected = "<= 0.14.0"            # comparators joined by commas; || for "or"
fixed = "0.14.1"                  # several lines: "0.20.2, 0.21.1"
disclosed = 2026-09-30
patched = 2026-07-29
references = [
  "https://lnfuzz.org/…",         # the links the README entry already lists
]

# Optional: when the bug only bites under a condition a tool can check.
# [[conditions]]
# kind = "network"                # or "config_enabled", "feature_bit", "manual"
# networks = ["mainnet"]
```

How it maps to what each entry already has:

| README entry | Field |
|---|---|
| `## Implementation: Title` heading | `title`, `implementation` |
| **Disclosure** date | `disclosed` |
| **Patched**: implementation version (date) | `fixed`, `patched` |
| Reference links | `references` |
| Affected versions (often in the description) | `affected` |

Only `affected` is new information. Everything else is already in each entry.

We're happy to send the PR ourselves: the files for the existing entries, plus a small check that every README section has a matching file. If you'd rather use a different format, such as JSON or OSV, or a single file instead of one per entry, we'll follow your lead.

---

## Notes for the team (not part of the issue)

- The example's `affected` range (`<= 0.14.0`) comes from lnfuzz.org's advisory ("Affects Eclair v0.14.0 and earlier"). Check every range against its primary source before the PR, as we did for `advisories/`.
- Our own TOML has more fields (`severity`, `verified`, `remediation`, `reviewed_by`). Those are ours to keep, not part of the proposal.
- Firewatch's version-range syntax (`<`, `<=`, `>=`, `,` for "and", `||` for "or") is what `crates/firewatch-core/src/version.rs` parses, so Firewatch could read these ranges as they are. The files themselves would still need a small importer, because our loader requires fields like `severity` and has no `eclair` implementation.
