# Dashboard API

`firewatch watch` serves this API, by default on `http://127.0.0.1:7070`. All responses are JSON, and CORS is open, so a dev server on another port (for example Vite on `:5173`) can call it directly.

| Method | Path | Returns |
|---|---|---|
| `GET` | `/api/report` | The latest report. `503 {"error": "..."}` until the first check finishes (a few seconds after start). |
| `POST` | `/api/check` | Runs a check now and returns the new report. Use it for a "Re-check" button. |
| `GET` | `/api/advisories` | Every advisory being checked, in the advisory file format. |
| `GET` | `/api/health` | `{"ok": true, "nodes": 3, "advisories": 6, "alerts": "nostr" \| "off"}` |

The daemon re-checks every `interval_secs` (10 in the demo config), so polling `/api/report` every 2 to 5 seconds is enough.

## Running it without nodes

To build the UI before Polar is set up, serve a fixed report:

```sh
cargo run -- check --nodes examples/nodes.json --json --include-unverified > report.json
```

## Report

```json
{
  "checked_at": 1792702526,
  "nodes": [
    {
      "name": "old-cln",
      "implementation": "cln",
      "version": "v26.06.7",
      "network": "regtest",
      "status": "affected",
      "error": null,
      "findings": [
        {
          "advisory_id": "FW-2026-003",
          "title": "Vulnerabilities fixed in 26.06.8",
          "severity": "critical",
          "exploited_in_wild": true,
          "verified": true,
          "fixed": "26.06.8",
          "source": "https://github.com/ElementsProject/lightning/releases/tag/v26.06.8",
          "status": "affected",
          "reason": "Version 26.06.7 is in the affected range < 26.06.8. Fixed in 26.06.8.",
          "manual_checks": [],
          "action": "upgrade",
          "steps": "We strongly recommend upgrading to this release."
        }
      ]
    },
    {
      "name": "alice",
      "implementation": "lnd",
      "version": "0.20.0-beta commit=v0.20.0-beta",
      "network": "regtest",
      "status": "affected",
      "error": null,
      "findings": [
        {
          "advisory_id": "FW-2026-009",
          "status": "not_affected",
          "reason": "Version 0.20.0 is in the affected range < 0.20.2 || >= 0.21.0-rc1, < 0.21.1, but it only applies on mainnet, and this node runs on regtest.",
          "manual_checks": [],
          "...": "other fields as above"
        }
      ]
    },
    {
      "name": "offline-node",
      "implementation": "cln",
      "version": null,
      "network": null,
      "status": "cant_tell",
      "error": "Can't connect to http://127.0.0.1:8182; is the node running?",
      "findings": []
    }
  ]
}
```

### Fields

**Node**

| Field | Type | Notes |
|---|---|---|
| `name` | string | Label from `firewatch.toml`. Unique. |
| `implementation` | `"cln" \| "lnd" \| "ldk"` | Display names: Core Lightning, LND, LDK. |
| `version` | string or null | Exactly as the node reports it. `null` when the node couldn't be read. |
| `network` | string or null | `regtest`, `bitcoin`, ... |
| `status` | status | The worst status across its findings. |
| `error` | string or null | Set when the node couldn't be read. A plain sentence, e.g. "Can't connect to https://127.0.0.1:8085; is the node running?". Show it; the node is amber. |
| `build_check` | build check | Whether the binary is the official release its version claims. See below. |
| `findings` | finding[] | Worst first. Empty means no advisory applies to this node's implementation. |

**Node errors.** Every `error` is a plain sentence that names what to check. These are all of them, each checked against live Polar LND and Core Lightning images:

| Cause | `error` starts with |
|---|---|
| Node stopped, or nothing on that port | `Can't connect to <url>; is the node running?` |
| Packets dropped by a firewall, or the node never answers | `Timed out after 10 s waiting for <url>` |
| `tls_cert` isn't this node's certificate | `TLS certificate not trusted; check tls_cert for <name>` |
| `https://` on a port that doesn't speak TLS (clnrest over HTTP, LND's p2p port) | `<url> doesn't answer HTTPS; ` then, for LND, `check the port: LND's REST port, not its p2p or gRPC port`, or for Core Lightning, `check the port, or use http:// if clnrest runs over HTTP` |
| A TLS server Firewatch can't agree with (rare) | `TLS with <url> failed (<reason>); check the url and port` |
| `http://` on LND's REST port, which is TLS-only | `<url> expects HTTPS; change the url to https://` |
| A port that answers, but not as a REST API (LND's gRPC port, or a proxy page) | `<url> answered, but not as a node's REST API; check the port (REST, not gRPC or p2p)`. When the reply was HTTP, its status is included: `answered with HTTP 502, but ...` |
| A port that hangs up (Core Lightning's p2p port), or a node mid-restart | `<url> closed the connection; check the port, or wait if the node is restarting` |
| Another node's macaroon or rune | `Credential rejected by the node (...)` |
| A rune or macaroon that can do more than read | `Credential is not read-only: ...` |
| A missing or unreadable credential file, or a `url` without `http://` or `https://` | `config: ...` |

**Finding**

| Field | Type | Notes |
|---|---|---|
| `advisory_id` | string | `FW-2026-003` |
| `title` | string | |
| `severity` | `"critical" \| "high" \| "medium" \| "low"` | |
| `exploited_in_wild` | bool | Worth a badge. |
| `verified` | bool | `false` only when the daemon runs with `--include-unverified`. Show a "not yet verified" badge. |
| `fixed` | string or null | `null` means no fix exists yet. |
| `source` | URL | The developers' own notes. Always link it. |
| `status` | status | |
| `reason` | string | One sentence. Show it next to the colour. |
| `in_range` | bool | The node's version is inside the affected range. `false` when it is outside or can't be parsed. A `not_affected` finding with `in_range: true` was ruled out by a condition (for example "mainnet only"): worth showing, since a version check alone would have flagged it. |
| `manual_checks` | string[] | Non-empty for amber findings: what the operator should check by hand. Show them as a checklist. |
| `action` | `"upgrade" \| "offline" \| "disable_feature"` | |
| `steps` | string | The developers' words, quoted. Show it in quotation marks, never reworded. |

**Build check**

An object tagged by `status`:

```json
{ "status": "match" }
{ "status": "mismatch", "release": "v0.20.0-beta", "expected": "b9ea7070c20a…", "actual": "fc472ef1…", "build": "v0.20.0-beta-1-gfc472ef" }
{ "status": "unreleased", "release": "v26.06.7", "build": "v26.06.7-12-gabc1234" }
{ "status": "modified", "release": "v26.06.7", "build": "v26.06.7-modded" }
{ "status": "unknown_release" }
{ "status": "not_checked" }
```

`expected` and `actual` are full 40-character hashes; show the first 7. `build` is how the node describes its build (LND's `commit=` text), absent when it doesn't say. When `actual` equals `expected`, the node runs the official commit but was built with changes, e.g. `build` is `v0.20.0-beta-dirty`: show `build` instead of the two hashes. On `mismatch`, every finding on the node is already `cant_tell` with the reason, so the card only needs a banner. `unknown_release` means the version isn't in Firewatch's table, so the build wasn't verified: a small grey note is enough. `unreleased` and `modified` are Core Lightning only. On `unreleased`, every finding is already `cant_tell`, like `mismatch`: a banner naming `build` is enough. On `modified`, red findings stay red and only the would-be-green ones are `cant_tell`, each with a hand check: a small note naming `build` is enough. `not_checked` (a Core Lightning release, or an LND node without a commit) needs nothing.

**Status**

| Value | Colour | Meaning |
|---|---|---|
| `affected` | red | The version is in range and every condition is confirmed. |
| `cant_tell` | amber | Something can't be confirmed over RPC, or the node can't be read. |
| `not_affected` | green | Outside the range, or a condition is confirmed false. |

A node with no findings is `not_affected`, but it means "no known advisory applies", not "proven safe". Consider a neutral label such as "No known advisories" so the dashboard never looks greener than the data.

## Serving the built dashboard

Set `dashboard_dir` under `[watch]` in `firewatch.toml` to the build output folder (for example `dashboard/dist`). The daemon then serves it at `/`, so the demo runs as a single process.
