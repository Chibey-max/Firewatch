#!/usr/bin/env python3
"""Check that every verified advisory quotes its source word for word.

For each advisory with verified = true, download its `source` page, turn the
HTML into plain text, and look for `remediation.steps` in it. Markdown
backticks may or may not survive on the page (GitHub release notes keep
them, rendered pages use <code>), so the quote passes either way.
Whitespace is collapsed on both sides. Nothing else is normalised, so a
changed word or punctuation mark fails.

    scripts/audit-quotes.py            # exits 1 if any quote is missing
"""

import html
import re
import sys
import tomllib
import urllib.request
from html.parser import HTMLParser
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


class Text(HTMLParser):
    SKIP = {"script", "style", "svg", "noscript"}

    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.parts, self.skipping = [], 0

    def handle_starttag(self, tag, attrs):
        if tag in self.SKIP:
            self.skipping += 1

    def handle_endtag(self, tag):
        if tag in self.SKIP and self.skipping:
            self.skipping -= 1

    def handle_data(self, data):
        if not self.skipping:
            self.parts.append(data)


def squash(text: str) -> str:
    return re.sub(r"\s+", " ", html.unescape(text)).strip()


def page_text(url: str) -> str:
    req = urllib.request.Request(url, headers={"User-Agent": "firewatch-audit"})
    with urllib.request.urlopen(req, timeout=30) as resp:
        parser = Text()
        parser.feed(resp.read().decode("utf-8", errors="replace"))
    return squash(" ".join(parser.parts))


def main() -> int:
    failures = 0
    for path in sorted((ROOT / "advisories").glob("*.toml")):
        advisory = tomllib.loads(path.read_text())
        if not advisory.get("verified"):
            continue
        quote = squash(advisory["remediation"]["steps"])
        try:
            text = page_text(advisory["source"])
            found = quote in text or quote.replace("`", "") in text
        except Exception as e:  # network errors are failures too
            print(f"ERROR     {advisory['id']}  {e}")
            failures += 1
            continue
        print(f"{'MATCH' if found else 'NOT FOUND'} {advisory['id']}  \"{quote}\"")
        failures += not found
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
