#!/usr/bin/env python3
"""How much of the compile baseline is a PLACEHOLDER rather than a measurement?

⭐ WHY THIS IS A SCRIPT AND NOT A NUMBER IN A ROW. Three DIFFERENT true numbers
live around this question and they are easy to mistake for one another — I did,
and published the mistake before catching it:

  * the crates whose `seconds_source` is `estimated` (this script's answer);
  * the frozen file's own `unpriced_crates` LIST, deliberately held at an older
    value by `compile_ratchet.py:1533` so the finding stays loud; and
  * the DELTA between them, which is what the gate reports as UNPRICED.

⛔ So a gap between the second and the first is the guard WORKING, not a bug.
This script answers only the first, and says so, because every `seconds` figure
derived for a placeholder crate is wrong by an unknown factor — the ratchet's own
tooling reports size predicts compile cost at R²=0.12 — and the SHARE that rests
on the guess is the honest way to read any number the ratchet prints.

⇒ It reads the committed baseline JSON and reports, so it costs nothing and can
be re-run before quoting a figure. It does NOT build anything;
`scripts/compile_collect.py` is what turns an estimate into a measurement.

⚠ NOT A GUARD, and deliberately: a placeholder is not a defect. Every D33 carve
creates a new destination with no measured cost, so this number RISING is the
campaign working as designed. What it must not do is stay unstated while
`seconds` figures get quoted off it.
"""
from __future__ import annotations

import collections
import json
import pathlib
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
BASELINE = REPO / "dev" / "compile_ratchet_baseline.json"
MEASURED = "measured"


def main() -> int:
    if not BASELINE.exists():
        # ⚠ NOT `relative_to(REPO)`: it raises for any path outside the repo, so
        # the error path itself blew up instead of reporting the error. Caught by
        # this script's own wrapper the first time it ran.
        print(f"FAIL: no baseline at {BASELINE} — this script reports on a file "
              "that must exist.", file=sys.stderr)
        return 1
    data = json.loads(BASELINE.read_text(encoding="utf-8"))
    crates = data.get("crates") or {}
    if not crates:
        print("FAIL: the baseline holds no crates; every share below would be "
              "a division by zero dressed as a fact.", file=sys.stderr)
        return 1

    by_source = collections.Counter(v.get("seconds_source") for v in crates.values())
    placeholder = sorted(k for k, v in crates.items()
                         if v.get("seconds_source") != MEASURED)
    share = 100.0 * len(placeholder) / len(crates)

    print(f"baseline commit {data.get('commit')} "
          f"(carried_from {data.get('carried_from')})")
    print(f"{len(crates)} crates: " + ", ".join(
        f"{n} {src}" for src, n in sorted(by_source.items(), key=lambda kv: -kv[1])))
    print(f"\n⚠ {len(placeholder)} crate(s) priced by PLACEHOLDER "
          f"— {share:.0f}% of the baseline:")
    for name in placeholder:
        print(f"  {name}")
    print("\n⇒ Every `seconds` figure the ratchet derives for those is wrong by "
          "an unknown factor.\n  `python3 scripts/compile_collect.py` is what "
          "prices them.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
