# Firewatch dashboard

React + TypeScript + Vite + Tailwind. It reads the report from `firewatch watch` (see [`docs/API.md`](../docs/API.md)) and shows every node in a list, worst first, with the selected node's findings beside it. `#node=<name>` in the URL selects a node; `?theme=dark` (or the toggle, remembered per browser) switches to the dark theme. Motion uses the `motion` library and turns off under the system's reduced-motion setting.

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
| `/?fixture=demo` | the `docs/DEMO.md` network: `old-cln`, `alice` and `lnd-old` red, FW-2026-013 amber on `lnd-old`, FW-2026-009 ruled out on regtest |
| `/?fixture=demo-after` | the same network after `old-cln` is upgraded to 26.06.9 (green) |
| `/?fixture=mixed` | one node affected, one not affected, one can't tell (includes an unverified advisory) |
| `/?fixture=green` | every node after upgrading, including one with no known advisories |
| `/?fixture=error` | one affected node and one that can't be read |
| `/?fixture=empty` | no nodes configured |

The fixtures in `src/fixtures/` come from the real CLI; `demo` and `demo-after` are the stage fallback if Polar fails. Regenerate `report.mixed.json` from the repo root with:

```sh
cargo run -- check --nodes examples/nodes.json --json --include-unverified > dashboard/src/fixtures/report.mixed.json
```

## Backup demo video

With `dev/sim.sh` and `npm run dev` running, `npm run record:backup` records the upgrade beat (red, upgrade, Re-check now, green) at 1920×1080 into `firewatch-backup-demo.mp4`. It drives the system Chrome (set `CHROME` to use another) and needs `ffmpeg`. Every frame carries a caption that the nodes are simulated. Re-record after UI changes.

## Build for the demo

```sh
npm run build        # outputs dashboard/dist
```

Then set `dashboard_dir = "dashboard/dist"` under `[watch]` in `firewatch.toml`, and the daemon serves the dashboard at `http://127.0.0.1:7070`.

## Contract

`src/types/report.ts` mirrors `crates/firewatch-core/src/report.rs`. If the Rust report changes, update the types in the same PR.
