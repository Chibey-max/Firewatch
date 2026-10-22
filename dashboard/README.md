# Firewatch dashboard

React + TypeScript + Vite + Tailwind. It reads the report from `firewatch watch` (see [`docs/API.md`](../docs/API.md)) and shows one dial per node.

## Run it

```sh
npm install
npm run dev          # http://localhost:5173, proxies /api to 127.0.0.1:7070
```

Start the daemon from the repo root in another terminal: `cargo run -- watch`.

### Without the daemon

Open the app with a fixture to see sample data:

| URL | Shows |
|---|---|
| `/?fixture=mixed` | one node affected, one not affected, one can't tell (includes an unverified advisory) |
| `/?fixture=green` | every node after upgrading, including one with no known advisories |
| `/?fixture=error` | one affected node and one that can't be read |
| `/?fixture=empty` | no nodes configured |

The fixtures in `src/fixtures/` come from the real CLI. Regenerate `report.mixed.json` from the repo root with:

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
