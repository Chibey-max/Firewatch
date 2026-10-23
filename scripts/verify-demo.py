#!/usr/bin/env python3
"""Full-path check (D4-4): every demo node shows exactly what demo/expected.toml
says, and every source link in the report opens.

    scripts/verify-demo.py                  # before the upgrade
    scripts/verify-demo.py --after-upgrade  # after demo/upgrade.sh

Runs `firewatch check --json` against firewatch.toml (CONFIG= to change it).
Exits 1 on any difference.
"""

import json
import os
import subprocess
import sys
import tomllib
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def live_report() -> dict:
    config = os.environ.get("CONFIG", "firewatch.toml")
    out = subprocess.run(
        ["cargo", "run", "--quiet", "--release", "--", "check", "--config", config, "--json"],
        cwd=ROOT, capture_output=True, text=True,
    )
    return json.loads(out.stdout)


def link_ok(url: str) -> bool:
    req = urllib.request.Request(url, headers={"User-Agent": "firewatch-verify"})
    try:
        with urllib.request.urlopen(req, timeout=20) as resp:
            return resp.status == 200
    except Exception:
        return False


def main() -> int:
    after = "--after-upgrade" in sys.argv
    expected = tomllib.loads((ROOT / "demo/expected.toml").read_text())["node"]
    report = live_report()
    nodes = {n["name"]: n for n in report["nodes"]}
    problems = []

    for want in expected:
        name = want["name"]
        if after and "after_upgrade" in want:
            want = {**want, **want["after_upgrade"]}
        got = nodes.get(name)
        if got is None:
            if not want.get("optional"):
                problems.append(f"{name}: missing from the report")
            else:
                print(f"skip   {name} (optional, not configured)")
            continue
        if got["error"]:
            problems.append(f"{name}: can't be read: {got['error']}")
            continue
        checks = [("node status", want["status"], got["status"]),
                  ("build check", want["build_check"], got["build_check"]["status"])]
        findings = {f["advisory_id"]: f["status"] for f in got["findings"]}
        for advisory, status in want["findings"].items():
            checks.append((advisory, status, findings.get(advisory, "missing")))
        for advisory in findings.keys() - want["findings"].keys():
            checks.append((advisory, "not expected", findings[advisory]))
        bad = [f"{what}: expected {w}, got {g}" for what, w, g in checks if w != g]
        print(f"{'ok' if not bad else 'FAIL':<6} {name}")
        problems += [f"{name}: {b}" for b in bad]

    sources = sorted({f["source"] for n in report["nodes"] for f in n["findings"]})
    for url in sources:
        ok = link_ok(url)
        print(f"{'ok' if ok else 'FAIL':<6} {url}")
        if not ok:
            problems.append(f"source link doesn't open: {url}")

    for p in problems:
        print(f"  ✗ {p}")
    print(f"\n{'All demo checks pass.' if not problems else f'{len(problems)} problem(s).'}")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
