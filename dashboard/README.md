# Firewatch dashboard

React + TypeScript + Vite + Tailwind. It reads the report from `firewatch watch` (see [`docs/API.md`](../docs/API.md)) and shows one dial per node.

## Run it

```sh
npm install
npm run dev          # http://localhost:5173, proxies /api to 127.0.0.1:7070
```

Start the daemon from the repo root in another terminal: `cargo run -- watch`.

### Live, without Polar

`dev/sim.sh` runs the real `firewatch watch` against two simulated Core Lightning endpoints (`old-cln` on v26.06.7, `current-cln` on v26.06.9) plus one address with nothing on it, so every state comes from real checks:

```sh
dashboard/dev/sim.sh            # terminal 1, from the repo root; Ctrl+C stops it
cd dashboard && npm run dev     # terminal 2
dashboard/dev/sim.sh upgrade    # old-cln now reports v26.06.9: press Re-check now
dashboard/dev/sim.sh reset      # back to v26.06.7
```

The simulated endpoints only answer `getinfo` and `listconfigs` (`dev/fake_cln.py`). They are not Lightning nodes; use Polar for the real demo.

### Without the daemon

Open the app with a fixture to see sample data:

| URL | Shows |
|---|---|
| `/?fixture=demo` | the `docs/DEMO.md` network before the demo: `old-cln` and `alice` red, FW-2026-009 ruled out on `alice` |
| `/?fixture=demo-after` | after beats 4 and 5: `old-cln` upgraded (green), `current-cln` stopped (amber) |
| `/?fixture=mixed` | one node affected, one not affected, one can't tell (includes an unverified advisory) |
| `/?fixture=green` | every node after upgrading, including one with no known advisories |
| `/?fixture=error` | one affected node and one that can't be read |
| `/?fixture=empty` | no nodes configured |

The fixtures in `src/fixtures/` come from the real CLI; `demo` and `demo-after` are the stage fallback if Polar fails. Regenerate `report.mixed.json` from the repo root with:

```sh
cargo run -- check --nodes examples/nodes.json --json --include-unverified > dashboard/src/fixtures/report.mixed.json
```

## Build for the demo

```sh
npm run build        # outputs dashboard/dist
```

Then set `dashboard_dir = "dashboard/dist"` under `[watch]` in `firewatch.toml`, and the daemon serves the dashboard at `http://127.0.0.1:7070`.

## Contract

`src/types/report.ts` mirrors `crates/firewatch-core/src/report.rs`. If the Rust report changes, update the types in the same PR.
