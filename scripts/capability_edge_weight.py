#!/usr/bin/env python3
"""What each unasked-for crate would actually cost to unlink.

`check_absence_contracts.py::capability-footprint-may-not-grow` reports the
headline pair — how many crates a movement-only game links, and how many of them
it never asked for. It says nothing about which of those is CHEAP to cut, and the
obvious proxy is wrong.

⛔⛔ DEPENDENT COUNT IS NOT COUPLING, and picking by it sends you at the worst
crate in the list. MEASURED 2026-09-05: `ambition_match` has TWO dependent
manifests — the fewest but one — and is used in SIXTEEN files of
`ambition_platformer2d_actor_monolith` alone, across `character_runtime/`,
`features/` and `avatar/`. A carve sized by "only two crates depend on it" would
have been sized at the wrong granularity, which is a mistake this repo has
already made more than once.

⇒ So this reports BOTH, plus the ratio, and sorts by the number that predicts the
work: how many FILES name the crate.

⚠ It is a sizing aid, not a verdict. A crate used in many files may still be
cheap if every use is one re-export, and a crate used in two may be load-bearing
in both. The point is to stop the ranking that is confidently backwards.

Usage: python3 scripts/capability_edge_weight.py
"""

from __future__ import annotations

import collections
import json
import pathlib
import re
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
BASELINE = REPO / "scripts/baselines/capability-footprint-baseline.json"


def tracked(*patterns: str) -> list[str]:
    out = subprocess.run(
        ["git", "ls-files", *patterns], cwd=REPO, capture_output=True, text=True, check=True
    ).stdout.split()
    # ⚠ Worktrees carry a full copy of the tree; counting them would multiply
    # every number by however many an agent happens to have open.
    return [p for p in out if "/.worktrees/" not in p and not p.startswith(".worktrees/")]


def main() -> int:
    try:
        base = json.loads(BASELINE.read_text(encoding="utf-8"))
    except OSError as error:
        print(f"⛔ cannot read {BASELINE.relative_to(REPO)}: {error}", file=sys.stderr)
        return 3
    targets = sorted(base["never_asked_for"])
    if not targets:
        print("⛔ the baseline lists no unasked-for crates; nothing to weigh", file=sys.stderr)
        return 1

    manifests = tracked("*Cargo.toml")
    sources = tracked("*.rs")

    dependents: dict[str, set[str]] = collections.defaultdict(set)
    mandatory: dict[str, set[str]] = collections.defaultdict(set)
    for manifest in manifests:
        text = (REPO / manifest).read_text(encoding="utf-8", errors="replace")
        owner = manifest.rsplit("/", 2)[-2] if "/" in manifest else manifest
        for crate in targets:
            # A dependency line, not a mention in a comment or a feature list.
            line = re.search(rf"^\s*{re.escape(crate)}\s*=.*$", text, re.M)
            if not line:
                continue
            dependents[crate].add(owner)
            # ⛔⛔ AN OPTIONAL DEPENDENCY DOES NOT PUT A CRATE IN THE CLOSURE, and
            # counting it as one hid the very case this column exists for.
            # `ambition_platformer2d` declares `ambition_sfx_bank` as
            # `optional = true`, so the crate reaches the closure ONLY through
            # `ambition_sfx` -- which is itself unasked-for. Treating the optional
            # line as a real dependent made the cheapest-looking row look
            # independently cuttable when it is not.
            if "optional = true" not in line.group(0):
                mandatory[crate].add(owner)

    files: dict[str, int] = collections.Counter()
    for source in sources:
        text = (REPO / source).read_text(encoding="utf-8", errors="replace")
        for crate in targets:
            if crate in text:
                files[crate] += 1

    # ⛔⛔ A CRATE CAN RIDE IN ON ANOTHER CRATE IN THE SAME LIST, and then neither
    # count means anything: removing it from the closure is impossible without
    # removing its carrier first. MEASURED 2026-09-05: `ambition_sfx_bank` has
    # the FEWEST files of all 23 -- and two of its three are COMMENTS while the
    # third is a re-export in `ambition_sfx`, which is itself unasked-for and 118
    # files wide. The cheapest-looking row in the table was not cuttable at all.
    #
    # ⇒ So the third column is the one that decides whether the other two are
    # worth reading: a crate whose dependents are ALL in the unasked-for set is
    # marked, because the work is its carrier's, not its own.
    unasked = set(targets)
    carried = {
        crate
        for crate in targets
        if mandatory[crate] and mandatory[crate] <= unasked
    }
    rows = sorted(targets, key=lambda c: (c in carried, files[c], len(dependents[c])))
    print(
        f"{'crate':32s} {'manifests':>9s} {'files':>6s}  {'':2s} "
        "cheapest first, by FILES; CARRIED = not cuttable alone"
    )
    for crate in rows:
        mark = "  CARRIED by " + ", ".join(sorted(mandatory[crate])) if crate in carried else ""
        print(f"  {crate:30s} {len(dependents[crate]):9d} {files[crate]:6d}{mark}")
    fewest_manifests = min(targets, key=lambda c: len(dependents[c]))
    fewest_files = min(targets, key=lambda c: files[c])
    print(
        "\n⚠ THREE RANKINGS, AND ALL THREE DISAGREE. Fewest manifests: "
        f"`{fewest_manifests}`. Fewest files: `{fewest_files}`"
        f"{' (CARRIED -- not cuttable alone)' if fewest_files in carried else ''}. "
        f"Cheapest actually cuttable: `{rows[0]}`."
    )
    print(
        "⇒ Read the CARRIED column first: a crate whose dependents are all in this "
        "same list cannot leave the closure until its carrier does, whatever its "
        "counts say."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
