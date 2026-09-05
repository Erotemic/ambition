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
    # D33 cut 1 (2026-09-03): the body seed left the monolith, taking
    # ActorClusterSeed / ActorMotionPath / ActorBody and their doc comments with
    # it. Added AFTER the fact rather than in the carve's own commit, which is
    # the failure this list's comment predicts — the monolith's count falls and
    # reads as a repair.
    "ambition_body_seed",
    # D33 cut 2b (2026-09-03): the match preparation crate. Added in the carve's
    # own window this time, which is what the comment above asks for.
    "ambition_match",
    # Added by the post-carve pass rather than by the carves themselves
    # (2026-09-03): `ambition_abilities` carries 434 doc-comment lines and
    # `ambition_encounter_features` 142, so the monolith's count falls as they
    # take theirs and nothing counts the destination. That is the "reduced
    # coverage reads as improvement" failure this list's own comment predicts.
    "ambition_abilities",
    "ambition_encounter_features",
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

    # ⛔⛔ THE COVERAGE GUARD, and it exists because this file's own CRATES
    # comment predicted the failure and then only ASKED a human to avoid it:
    # *"When architecture moves out of a tracked crate, add its destination in
    # the same change so the ratchet does not mistake reduced coverage for
    # improvement."* Nothing enforced that. Dropping a name from CRATES leaves
    # its baseline entry orphaned, stops measuring it, and the TOTAL falls —
    # which reads exactly like a repair. ⇒ a baselined crate that is no longer
    # measured is a SHRINKING GUARD, and it fails here.
    orphaned = sorted(set(baseline) - set(CRATES))
    if orphaned and not args.update:
        print()
        print(f"⛔ {', '.join(orphaned)} is in the baseline and NOT in CRATES —")
        print("   the tracked set SHRANK. The total falls and reads as a repair.")
        print("   If a carve moved that code, add its DESTINATION crate to CRATES")
        print("   in this commit; only then `--update` to re-baseline.")
        return 1

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

    if fell and not risen:
        print()
        print(f"⭐ {', '.join(fell)} improved — run --update to bank it, in this commit.")
    elif fell and risen:
        # ⛔⛔ THE ADVICE AND THE FINDING WERE ASYMMETRIC, and the asymmetry
        # pointed one way: "run --update to bank it" printed ALWAYS, while the
        # risen block printed only under --check. A plain run therefore told you
        # to bank an improvement without ever showing you the regressions --
        # and `--update` rewrites EVERY count, so following that advice converts
        # a rise into the new normal. Found 2026-09-03 with two crates risen.
        print()
        print(f"⭐ {', '.join(fell)} improved — but ⛔ DO NOT --update YET.")
        print("   `--update` rewrites EVERY count, so it would bank the rises")
        print("   below as the new baseline. Fix or account for those first.")

    # Printed whether or not --check was passed: a run that shows "ROSE" in its
    # table and then says nothing about it is why the rises above went unread.
    if risen:
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
