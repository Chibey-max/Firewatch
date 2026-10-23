# Firewatch dashboard

React + TypeScript + Vite + Tailwind. It reads the report from `firewatch watch` (see [`docs/API.md`](../docs/API.md)) and shows every node in a list, worst first, with the selected node's findings beside it, on panes of glass over a mountain photo. Below 1024px wide the controls move to a bar pinned at the top, and a node's findings open over the list: as a sheet from the bottom on a phone (a drag down closes it), as a dialog in the middle of the page from 640px. Back, Escape or the backdrop closes either.

| URL / control                              | Does                                                                                                     |
| ------------------------------------------ | -------------------------------------------------------------------------------------------------------- |
| `#node=<name>`                             | selects a node (a presenter can link straight to one; below 1024px it opens that node's sheet or dialog) |
| `?theme=dark`, or the sun/moon toggle      | the dark theme; the toggle opens it in a circle from the button and is remembered per browser            |
| `?font=<set>`, or the picker with `?fonts` | type experiment: `satoshi` (default), `space`, `unbounded`, `inter`, `sora`, `editorial`                 |

Motion uses the `motion` library and turns off under the system's reduced-motion setting. The photos are in `public/bg/` (credits in its README); Satoshi is self-hosted in `public/fonts/satoshi/`.

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

| URL                    | Shows                                                                                                                                                                                    |
| ---------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `/?fixture=demo`       | the `docs/DEMO.md` network: `old-cln`, `alice` and `lnd-old` red, FW-2026-013 amber on `lnd-old`, FW-2026-009 ruled out on regtest, and the optional `lnd-unofficial` amber on its build |
| `/?fixture=demo-after` | the same network after `old-cln` is upgraded to 26.06.9 (green)                                                                                                                          |
| `/?fixture=mixed`      | one node affected, one not affected, one can't tell (includes an unverified advisory)                                                                                                    |
| `/?fixture=green`      | every node after upgrading                                                                                                                                                               |
| `/?fixture=error`      | one affected node and one that can't be read                                                                                                                                             |
| `/?fixture=empty`      | no nodes configured                                                                                                                                                                      |

The fixtures in `src/fixtures/` come from the real CLI; `demo` and `demo-after` are the stage fallback if Polar fails. After any change to the report or the advisories, regenerate all of them from the repo root:

```sh
dashboard/dev/regen-fixtures.sh
```

## Tests

```sh
npm test             # Vitest: the logic behind what the operator sees, run against the fixtures
```

CI runs them with the lint, format check and build.

## Backup demo video

With `dev/sim.sh` and `npm run dev` running, `npm run record:backup` records the upgrade beat (red, upgrade, Re-check now, green) at 1920×1080 into `firewatch-backup-demo.mp4`. It drives the system Chrome (set `CHROME` to use another) and needs `ffmpeg`. Every frame carries a caption that the nodes are simulated. It refuses to record a page without its styles (after many quick edits the dev server can serve an empty stylesheet: restart `npm run dev` after `rm -rf node_modules/.vite`). Re-record after UI changes.

## Build for the demo

`scripts/demo-up.sh` (repo root) builds the dashboard and serves it from `firewatch watch` at `http://127.0.0.1:7070`. By hand:

```sh
npm run build        # outputs dashboard/dist
```

then run `firewatch watch --dashboard dashboard/dist`, or set `dashboard_dir = "dashboard/dist"` under `[watch]` in `firewatch.toml`.

## Contract

`src/types/report.ts` mirrors `crates/firewatch-core/src/report.rs`. If the Rust report changes, update the types in the same PR.
