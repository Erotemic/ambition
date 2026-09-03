#!/usr/bin/env python3
"""Ratchet broken intra-document links.

Known broken anchors are tolerated only while they remain in the baseline; newly
broken links fail the check and repaired links reduce the baseline."""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

sys.path.insert(0, os.path.join(REPO, "scripts", "lib"))
from cargo_bin import cargo_binary  # noqa: E402
BASELINE = os.path.join(REPO, "dev", "doc_link_ratchet_baseline.json")

# Keep the ratchet on crates whose doc comments define the main runtime model;
# scanning the whole workspace would make the check too expensive for routine use.
CRATES = [
    "ambition_platformer2d_actor_monolith",
    "ambition_characters",
    "ambition_platformer2d_core",
    "ambition_combat",
    # When architecture moves out of a tracked crate, add its destination in the
    # same change so the ratchet does not mistake reduced coverage for improvement.
    "ambition_conversation",
    # The destination joins the list here, in the carve's own commit.
    "ambition_boss_encounter",
    "ambition_items",
    "ambition_menu",
]

# rustdoc's two shapes for this class.
WARNING = re.compile(
    r"^warning: (unresolved link to|public documentation for)",
    re.MULTILINE,
)


def measure(crate: str) -> tuple[int, str]:
    """`(broken link count, raw output)` for one crate."""
    result = subprocess.run(
        [cargo_binary(), "doc", "-p", crate, "--no-deps"],
        cwd=REPO,
        capture_output=True,
        text=True,
    )
    output = result.stdout + result.stderr
    return len(WARNING.findall(output)), output


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="exit 1 when a count rises")
    parser.add_argument("--update", action="store_true", help="rewrite the baseline")
    args = parser.parse_args()

    baseline = {}
    if os.path.exists(BASELINE):
        baseline = json.load(open(BASELINE, encoding="utf-8")).get("crates", {})

    counts: dict[str, int] = {}
    risen: list[str] = []
    fell: list[str] = []
    silent: list[str] = []

    for crate in CRATES:
        count, output = measure(crate)
        # the "observed nothing" guard: a doc build that failed, or a crate
        # that no longer exists, emits no warnings and would read as zero.
        if "Documenting" not in output and "Finished" not in output:
            silent.append(crate)
        counts[crate] = count
        previous = baseline.get(crate)
        mark = ""
        if previous is None:
            mark = "  (new)"
        elif count > previous:
            mark = f"  ⛔ ROSE from {previous}"
            risen.append(crate)
        elif count < previous:
            mark = f"  ⭐ fell from {previous}"
            fell.append(crate)
        print(f"{crate:40s} {count:4d}{mark}")

    total = sum(counts.values())
    print(f"{'TOTAL':40s} {total:4d}")

    if silent:
        print()
        print(f"⛔ {', '.join(silent)} produced no rustdoc output at all — the")
        print("   build failed or the crate is gone, and zero warnings from a")
        print("   build that did not happen is not a score.")
        return 1

    if args.update:
        os.makedirs(os.path.dirname(BASELINE), exist_ok=True)
        with open(BASELINE, "w", encoding="utf-8") as handle:
            json.dump(
                {
                    "_comment": (
                        "Broken intra-doc links per crate (ledger D103). A RATCHET: "
                        "these may fall and must not rise. Lower them in the same "
                        "commit that earns it — scripts/check_doc_link_ratchet.py --update."
                    ),
                    "crates": counts,
                },
                handle,
                indent=2,
                sort_keys=True,
            )
            handle.write("\n")
        print(f"\nbaseline written: {BASELINE}")
        return 0

    if fell:
        print()
        print(f"⭐ {', '.join(fell)} improved — run --update to bank it, in this commit.")

    if risen and args.check:
        print()
        print(f"⛔ {len(risen)} crate(s) gained broken doc links: {', '.join(risen)}")
        print("   Run `cargo doc -p <crate> --no-deps` and read the warnings: each is")
        print("   a `[`Item`]` naming something renamed, moved or deleted. A deletion")
        print("   that leaves its references behind turns a doc comment into a")
        print("   description of a world that no longer exists — which in this")
        print("   repository is where the reasoning lives.")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
