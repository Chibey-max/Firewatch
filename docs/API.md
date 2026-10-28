# Dashboard API

`firewatch watch` serves this API, by default on `http://127.0.0.1:7070`. All responses are JSON.

**Who may call it.** Only pages served from this machine. The dashboard is same-origin: served by `watch --dashboard`, or in development through Vite's `/api` proxy. A page on another localhost port (for example with `VITE_API_BASE=http://127.0.0.1:7070`) gets CORS for `GET` and `POST`. Everything else gets `403 {"error": "..."}`:
- a `Host` that isn't `localhost`, `127.0.0.1` or `[::1]`, which guards against DNS rebinding. With `--allow-remote`, IP addresses are accepted too.
- an `Origin` that is neither localhost nor the server itself, which blocks other sites from reading the report or triggering `POST /api/check`.

| Method | Path | Returns |
|---|---|---|
| `GET` | `/api/report` | The latest report. `503 {"error": "..."}` until the first check finishes (a few seconds after start). |
| `POST` | `/api/check` | Runs a check now and returns the new report. Use it for a "Re-check" button. |
| `GET` | `/api/advisories` | Every advisory being checked, in the advisory file format. |
| `GET` | `/api/health` | `{"ok": true, "nodes": 3, "advisories": 6, "alerts": "nostr" \| "off", "alerts_pending": 0, "oldest_pending_since": null, "last_sent_at": 1792702526}` |

The daemon re-checks every `interval_secs` (10 in the demo config), so polling `/api/report` every 2 to 5 seconds is enough.

### Alert delivery

`/api/health` also says whether alerts are getting through. An alert that can't be sent (no internet, relays down) waits in a queue and is retried every check.

| Field | Type | Notes |
|---|---|---|
| `alerts_pending` | number | Alerts waiting to be sent. `0` when everything got through, or when alerts are off. |
| `oldest_pending_since` | Unix seconds or null | When the longest-waiting alert started waiting: "offline since 14:02". `null` when nothing waits. |
| `last_sent_at` | Unix seconds or null | When an alert last got through. `null` until one has, since this `watch` started. |

When `alerts_pending` is above 0, show it next to the live status, for example "1 alert waiting to send · offline since 14:02". It clears on delivery.

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
| `exposure` | exposure[] | Where the node's REST and gRPC APIs listen, most exposed first. **Never changes `status`.** See below. |
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
| A plain `http://` url to another machine, without `allow_plain_http = true` (nothing is sent) | `config: node '<name>': <url> is plain http:// to another machine, so the rune would cross the network unencrypted. ...` (`macaroon` for LND) |

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

**Exposure**

Whether each of the node's APIs can be reached from other machines. A leaked rune or macaroon only works for someone who can reach the API: in August 2026 a BTCPay Server flaw let attackers obtain LND macaroons, and funds were stolen; the [BTCPay Server 2.4.2 advisory](https://blog.btcpayserver.org/security-advisory-btcpay-server-2-4-2/) says *"Do not leave an affected LND deployment accessible from the internet."* It isn't an advisory verdict: a fully patched node can be exposed, so it never changes the node's `status` or findings.

```json
{ "api": "rest", "status": "exposed", "addresses": ["0.0.0.0:8080"],
  "reason": "The REST API listens on all interfaces (0.0.0.0:8080), so a leaked credential works from anywhere that can reach it.",
  "manual_check": "Check that port 8080 isn't reachable from the internet: router port forwards, firewall, Tor or a reverse proxy.",
  "source": "https://blog.btcpayserver.org/security-advisory-btcpay-server-2-4-2/" }
```

| Field | Type | Notes |
|---|---|---|
| `api` | `"rest" \| "grpc"` | Core Lightning: clnrest and cln-grpc (`grpc` only when it's serving). LND: REST and gRPC. |
| `status` | `"exposed" \| "unknown" \| "local"` | `exposed`: all interfaces (`0.0.0.0`, `::`, `:8080`) or an address other machines reach. `unknown`: couldn't be read (LND without `lnd_conf`, or an option `lnd.conf` doesn't set) or a host name. `local`: `127.0.0.1`, `::1`, `localhost` or a unix socket. |
| `addresses` | string[] | `host:port` or `unix://path`. Core Lightning: from `listconfigs`. LND: from `lnd.conf`, completed the way LND reads them, e.g. `tcp://0.0.0.0` is `0.0.0.0:10009` and `8080` is `localhost:8080`. Empty when unknown. |
| `reason` | string | One plain sentence. |
| `manual_check` | string or null | What to check by hand. `null` when `local`. |
| `source` | string or null | The BTCPay advisory. `null` when `local`. |

Show `exposed` and `unknown` as a note on the card, apart from the findings and not in the node's colour; `local` needs nothing. On Polar, Core Lightning nodes are `exposed` (Polar binds their APIs to `0.0.0.0` inside Docker) and LND nodes are `unknown` (Polar passes their listen addresses as command-line flags).

**Status**

| Value | Colour | Meaning |
|---|---|---|
| `affected` | red | The version is in range and every condition is confirmed. |
| `cant_tell` | amber | Something can't be confirmed over RPC, or the node can't be read. |
| `not_affected` | green | Outside the range, or a condition is confirmed false. |

A node with no findings is `not_affected`, but it means "no known advisory applies", not "proven safe". Consider a neutral label such as "No known advisories" so the dashboard never looks greener than the data.

## Serving the built dashboard

Set `dashboard_dir` under `[watch]` in `firewatch.toml` to the build output folder (for example `dashboard/dist`). The daemon then serves it at `/`, so the demo runs as a single process.
