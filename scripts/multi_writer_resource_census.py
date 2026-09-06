#!/usr/bin/env python3
"""Which `Resource`s are written (`ResMut<T>`) from more than one FILE?

⛔⛔ THIS IS A SHORTLIST, NOT A FINDING LIST, and the docstring says so because the
count is the part that misleads. A fact written from N places is NOT an authority
violation by count. Two measured cases from 2026-09-06, same grep, opposite
verdicts:

  * `CutRopeBossArenaState` — TWO systems retracted it on the SAME message. A
    poison proved it: deleting either retractor alone left an end-to-end test
    GREEN, deleting both turned it RED. ⇒ Two copies did not merely risk drifting
    apart; each hid the other's absence, so NEITHER could be tested. Real defect.
  * `ActiveRoomTransitionLoad::asset_readiness_complete` — SIX `= true` sites
    across two crates and FOUR distinct meanings (no contributor, host cannot
    answer, assets failed, genuinely ready). CORRECT. `commit.rs` never reads it:
    the commit is gated by `phase`, every failure path sets `phase = Failed`, and
    the outcome lives in that single-authority sibling.

⭐ SO THE DISCRIMINATOR IS ON THE READER'S SIDE: ask what READS the fact, and
whether an ambiguity in it can reach a DECISION. The fighter lane's writer-side
version of the same rule: two branches of ONE function is one authority with two
exits; two SYSTEMS on one trigger is two authorities.

⇒ AND THE CONFIRMING EXPERIMENT IS A POISON, not a reading: delete one writer and
run the test that should care. If it stays green, either nothing tests the fact or
another writer is covering — and only the second is a finding. Confirm the edit
landed (`grep -c`) before believing either.

Usage:  python3 scripts/multi_writer_resource_census.py [PATH ...]
"""

from __future__ import annotations

import argparse
import collections
import pathlib
import re
import subprocess
import sys

RESMUT = re.compile(r"ResMut<\s*([A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z0-9_]+)*)\s*>")
DEFAULT_PATHS = ("crates", "game")


def rust_files(paths: tuple[str, ...]) -> list[str]:
    out = subprocess.run(
        ["git", "ls-files", *paths], capture_output=True, text=True, check=True
    ).stdout.split()
    return [f for f in out if f.endswith(".rs")]


def writers(files: list[str]) -> dict[str, set[str]]:
    """`{short type name: {file, ...}}` over NON-TEST code.

    ⚠ Test modules are cut at the first `#[cfg(test)]`. A fixture that builds a
    resource by hand is not a second authority over it, and counting fixtures is
    how a census manufactures findings nobody can act on.
    """
    found: dict[str, set[str]] = collections.defaultdict(set)
    for f in files:
        src = pathlib.Path(f).read_text(encoding="utf-8", errors="replace")
        src = src.split("#[cfg(test)]")[0]
        for m in RESMUT.finditer(src):
            found[m.group(1).split("::")[-1]].add(f)
    return found


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("paths", nargs="*", default=list(DEFAULT_PATHS))
    args = ap.parse_args(argv)

    files = rust_files(tuple(args.paths))
    if not files:
        # ⛔ An empty corpus would print "0 multi-writer types" and read as a
        # clean bill of health.
        print("no .rs files matched — the instrument found nothing to read")
        return 2

    found = writers(files)
    multi = {t: fs for t, fs in found.items() if len(fs) > 1}
    print(f"{len(files)} files; {len(found)} `ResMut<T>` types; {len(multi)} written from >1 file\n")
    for ty, fs in sorted(multi.items(), key=lambda kv: (-len(kv[1]), kv[0])):
        print(f"  {ty}  ({len(fs)} files)")
        for f in sorted(fs):
            print(f"      {f}")
    print(
        "\n⇒ A SHORTLIST, NOT FINDINGS. For each: what READS this, and can an"
        "\n  ambiguity in it reach a decision? Then POISON one writer and run the"
        "\n  test that should care — a green means nothing tests it OR another"
        "\n  writer covers it, and only the second is a finding."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
