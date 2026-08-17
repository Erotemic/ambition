#!/usr/bin/env python3
"""**A ratchet on broken intra-doc links** — ledger D103.

This repository's doc comments are load-bearing. They carry the ⛔ notes that say
why a thing is the way it is, and a campaign that deletes a type every few hours
turns every reference to it into a sentence describing a world that stopped
existing. Two examples, both made and repaired on 2026-08-12: a module doc that
still described a rewind "rerunning the roster archetype construction … tuning /
capabilities from the archetype id the binding retained" — three nouns, none of
which survived — and a note calling its counterpart the one that "rebuilds a
whole body because an archetype IS the creature", which was the DEFECT, fixed
that same day.

⛔⛔ **THE PROJECT GATE CANNOT SEE THIS CLASS.** The gate is
``cargo check -p ambition_app --all-targets``; an unresolved ``[`Item`]`` is a
RUSTDOC lint. So the entire class rots in the space between two commands, and
nothing has ever failed because of it. First measured 2026-08-12: **199**.

**A ratchet, not a sweep.** Fixing 199 links with nothing behind them is 199
links again by September. This records the count per crate and fails when one
RISES. Lowering a number is always allowed and the baseline is meant to be
updated downward in the same commit that earns it.

⚠ **it fails when it observes NOTHING, too.** A `cargo doc` that errors, or a
crate name that stops existing, produces zero warnings — which reads as a perfect
score. A check that cannot fail is worse than no check, so a crate that reports
no output at all is a failure rather than a triumph.

Usage::

    python3 scripts/check_doc_link_ratchet.py            # report
    python3 scripts/check_doc_link_ratchet.py --check    # exit 1 on a rise
    python3 scripts/check_doc_link_ratchet.py --update   # rewrite the baseline
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BASELINE = os.path.join(REPO, "dev", "doc_link_ratchet_baseline.json")

# The crates whose doc comments carry architectural reasoning. Not every crate:
# a ratchet over the whole workspace is a slow check that nobody runs, and these
# four hold the character/actor/combat model this campaign is rewriting.
CRATES = [
    "ambition_platformer2d_actor_monolith",
    "ambition_characters",
    "ambition_platformer2d_core",
    "ambition_combat",
    # ⛔⛔ **A CARVE CAN LAUNDER DEBT OFF THIS LEDGER, and one just did.** D33
    # step 2 moved `conversation` out of the monolith into its own crate, and
    # the monolith's count fell 122 → 109 — thirteen broken links that did not
    # get FIXED, they got RE-HOMED into a crate this list did not name. Banking
    # 109 without this line would have recorded a 13-link improvement nobody
    # earned. ⇒ **when a carve leaves one of these crates, the destination joins
    # the list in the same commit.**
    "ambition_conversation",
    # ⇒ and D33's `boss_encounter` carve did it again the same day: 7,635 lines
    # left the monolith, so its count falls for a reason nobody fixed. The
    # destination joins the list here, in the carve's own commit.
    "ambition_boss_encounter",
    # ⇒ and D33's outer-shell relocation (2026-08-17) did it a THIRD way: the
    # departing code went into crates that ALREADY EXISTED, so there was no new
    # `Cargo.toml` to remind anyone. `equipment` joined `ambition_items`, the
    # dialogue host glue joined `ambition_conversation` (already listed), and the
    # Map tab's renderer joined `ambition_menu`. Two more destinations, same
    # rule: **a relocation launders exactly as well as a carve does.**
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
        ["cargo", "doc", "-p", crate, "--no-deps"],
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
        # ⛔ the "observed nothing" guard: a doc build that failed, or a crate
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
