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
    # ⛔⛤ FAST-ITERATION I1 (2026-09-11): twenty `smash_*` modules — 4,767 lines
    # of pure move authoring — left `ambition_characters` for the pure value
    # crate, and this list's own comment predicted exactly what happened. The
    # run reported `ambition_characters 19 ⭐ 3 repaired (was 22)` and the three
    # "repairs" were `smash_capture.rs` and `smash_limit.rs` DOC LINKS THAT HAD
    # MOVED, not links anybody fixed. The total fell by one while a new broken
    # link was added in the same commit. ⇒ Added in the carve's own window, and
    # the three come back as this crate's baseline where they can be paid off.
    "ambition_entity_catalog",
]

# rustdoc's two shapes for this class.
UNRESOLVED = re.compile(r"^warning: unresolved link to `(?P<name>.+)`$")
PRIVATE = re.compile(
    r"^warning: public documentation for `(?P<item>.+)` links to private item "
    r"`(?P<target>.+)`$"
)
LOCATION = re.compile(r"^\s*--> (?P<path>[^:]+):\d+:\d+$")


def identities(output: str) -> list[str]:
    """Every broken link in one crate's rustdoc output, NAMED.

    ⛔⛔ THE BASELINE USED TO BE A COUNT, AND A COUNT CANNOT SEE A SWAP. Repair
    one link and break another in the same crate and the number is unchanged, so
    the new break is banked silently and forever. That is not hypothetical: on
    2026-09-07 this ratchet went red at +7 across three crates, and because the
    baseline held only totals it could not say WHICH seven. The repair that
    turned it green fixed seven OTHER broken links — the number came back, the
    regression did not have to. Storing the set makes paying a regression off
    with unrelated repairs impossible rather than merely discouraged.

    ⚠ Identity deliberately OMITS the line and column: a link that has not
    changed must not churn the baseline because something above it grew. The file
    and the link's own text are what identify it.

    ⚠ rustdoc emits some of these with no `-->` at all (8 of 88 measured
    2026-09-07, the `[text](path)` form among them). Those keep their name and
    lose their file; they are still attributed to their crate, which is the unit
    this ratchet ratchets.
    """
    found: list[str] = []
    lines = output.splitlines()
    for index, line in enumerate(lines):
        match = UNRESOLVED.match(line)
        if match:
            kind, name = "unresolved", match.group("name")
        else:
            match = PRIVATE.match(line)
            if not match:
                continue
            kind = "private"
            name = f"{match.group('item')} -> {match.group('target')}"
        path = ""
        for ahead in lines[index + 1 : index + 4]:
            if ahead.startswith("warning:"):
                break
            located = LOCATION.match(ahead)
            if located:
                path = located.group("path")
                break
        found.append(f"{path or '<no location>'}: {kind} `{name}`")
    return sorted(found)


def measure(crate: str) -> tuple[list[str], str, int]:
    """`(broken links, raw output, cargo's exit status)` for one crate.

    ⛔⛔ **THE EXIT STATUS IS PART OF THE MEASUREMENT, AND IT WAS THROWN AWAY —
    2026-09-18.** This returned two values and `main` scored the first of them
    whatever cargo had done. On 2026-09-18 a `--maintenance` run of this guard
    measured ZERO for all thirteen crates, printed *"⭐ 42 repaired"* down the
    table with `TOTAL 0`, exited 0, and advised `--update` — which would have
    written an all-empty baseline and retired the ratchet. Nothing was repaired:
    the thirteen `cargo doc` invocations produced no warnings, and a build that
    did not report cannot report a zero.
    """
    result = subprocess.run(
        [cargo_binary(), "doc", "-p", crate, "--no-deps"],
        cwd=REPO,
        capture_output=True,
        text=True,
    )
    output = result.stdout + result.stderr
    return identities(output), output, result.returncode


def documented(crate: str, output: str) -> bool:
    """Positive evidence that THIS crate's doc unit was accounted for.

    ⭐ PER CRATE, AND POSITIVE. The arm this replaces asked whether `Documenting`
    or `Finished` appeared ANYWHERE in one crate's output, and a warm unit
    prints no `Documenting` line at all while `Finished` prints either way — so
    it could not separate *"fresh, diagnostics replayed"* from *"this command
    did nothing"*. Both spellings are accepted because both are real: cargo
    prints `Documenting <crate>` when it re-documents and
    `Generated .../doc/<crate>/index.html` in both cases. ⚠ If a future cargo
    stops printing both, every crate reads as silent and this guard REFUSES
    rather than scoring a zero, which is the safe direction to fail in.
    """
    return f"Documenting {crate}" in output or f"/doc/{crate}/index.html" in output


def _difference(left: list[str], right: list[str]) -> list[str]:
    """Members of `left` not covered by `right`, counting repeats."""
    remaining = list(right)
    out = []
    for item in left:
        if item in remaining:
            remaining.remove(item)
        else:
            out.append(item)
    return out


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="exit 1 when a count rises")
    parser.add_argument("--update", action="store_true", help="rewrite the baseline")
    # ⭐ `--update` CONFLATES TWO OPERATIONS, and the conflation is a trap: it
    # rewrites EVERY count, so the only way to give a newly-tracked crate a floor
    # was also to bank every outstanding rise. `--adopt` does the first alone —
    # it writes entries ONLY for crates missing from the baseline and leaves
    # every existing count untouched.
    parser.add_argument(
        "--adopt",
        action="store_true",
        help="baseline ONLY crates that have no entry yet; never touch existing counts",
    )
    args = parser.parse_args()

    baseline: dict[str, list[str]] = {}
    if os.path.exists(BASELINE):
        baseline = json.load(open(BASELINE, encoding="utf-8")).get("crates", {})

    # ⛔ A BASELINE IN THE OLD COUNT SHAPE IS NOT A BASELINE THIS CHECK CAN READ,
    # and reading an int as "no links recorded" would make every existing break
    # look new. Refuse and name the migration instead of guessing.
    # ⚠ `--update` IS EXEMPT, because it is the migration this refuses toward.
    # A guard that forbids its own remedy leaves no road out of the state it
    # detects; the first version of this block did exactly that and could not
    # migrate the baseline it was written to migrate. `--adopt` is NOT exempt: it
    # merges into the existing values and would write a half-migrated file.
    stale = sorted(c for c, v in baseline.items() if not isinstance(v, list))
    if stale and args.update:
        # Migrating: a COUNT is not a comparable previous, so those crates read
        # as `(new)` for this one run rather than being diffed against an int.
        baseline = {c: v for c, v in baseline.items() if c not in set(stale)}
    if stale and not args.update:
        print()
        print(f"⛔ {', '.join(stale)} carry a COUNT, not a link list — this")
        print("   baseline predates the identity ratchet. A count cannot say which")
        print("   link broke, and cannot see a repair and a new break cancel.")
        print("   Migrate with: scripts/check_doc_link_ratchet.py --update")
        return 1

    # ⛔⛔ THE COVERAGE GUARD, and it exists because this file's own CRATES
    # comment predicted the failure and then only ASKED a human to avoid it:
    # *"When architecture moves out of a tracked crate, add its destination in
    # the same change so the ratchet does not mistake reduced coverage for
    # improvement."* Nothing enforced that. Dropping a name from CRATES leaves
    # its baseline entry orphaned, stops measuring it, and the TOTAL falls —
    # which reads exactly like a repair. ⇒ a baselined crate that is no longer
    # measured is a SHRINKING GUARD, and it fails here.
    orphaned = sorted(set(baseline) - set(CRATES))
    if orphaned and not (args.update or args.adopt):
        print()
        print(f"⛔ {', '.join(orphaned)} is in the baseline and NOT in CRATES —")
        print("   the tracked set SHRANK. The total falls and reads as a repair.")
        print("   If a carve moved that code, add its DESTINATION crate to CRATES")
        print("   in this commit; only then `--update` to re-baseline.")
        return 1

    # ⛔⛔ AND THE MIRROR: a crate in CRATES with no baseline entry is TRACKED BUT
    # UNRATCHETED. It prints "(new)", contributes to TOTAL, and has no floor — so
    # its links may rise freely and nothing says so. The documented flow is to
    # add the crate and `--update` IN THE SAME COMMIT; this makes that the only
    # flow, instead of asking for it. ⚠ `--update` is exempt, because that is the
    # command that establishes the floor.
    unbaselined = sorted(set(CRATES) - set(baseline))
    if unbaselined and baseline and not (args.update or args.adopt):
        print()
        print(f"⛔ {', '.join(unbaselined)} is in CRATES with NO baseline entry —")
        print("   tracked but UNRATCHETED: it prints (new), counts toward TOTAL,")
        print("   and has no floor, so its links can rise and nothing reports it.")
        print("   Add the crate and `--update` in the SAME commit.")
        return 1

    counts: dict[str, list[str]] = {}
    # (crate, cargo's output) for every crate whose `cargo doc` did not succeed.
    refused: list[tuple[str, str]] = []
    risen: list[str] = []
    fell: list[str] = []
    silent: list[str] = []
    appeared: dict[str, list[str]] = {}
    repaired: dict[str, list[str]] = {}

    for crate in CRATES:
        links, output, code = measure(crate)
        if code != 0:
            refused.append((crate, output))
            continue
        # the "observed nothing" guard: a doc build that failed, or a crate
        # that no longer exists, emits no warnings and would read as zero.
        if not documented(crate, output):
            silent.append(crate)
        counts[crate] = links
        previous = baseline.get(crate)
        mark = ""
        if previous is None:
            mark = "  (new)"
        else:
            # ⭐ MULTISET, NOT SET. The same link text can legitimately appear
            # twice in one file; collapsing them would hide the second one's
            # arrival and its repair alike.
            gained = _difference(links, previous)
            lost = _difference(previous, links)
            if gained:
                appeared[crate] = gained
                risen.append(crate)
                mark = f"  ⛔ {len(gained)} NEW (was {len(previous)}, now {len(links)})"
            elif lost:
                repaired[crate] = lost
                fell.append(crate)
                mark = f"  ⭐ {len(lost)} repaired (was {len(previous)})"
        print(f"{crate:40s} {len(links):4d}{mark}")

    total = sum(len(v) for v in counts.values())
    print(f"{'TOTAL':40s} {total:4d}")

    if refused:
        print()
        print(f"⛔ `cargo doc` FAILED for {', '.join(c for c, _ in refused)} — so")
        print("   there is no measurement to compare, and an empty warning list")
        print("   from a command that exited non-zero is not a repair.")
        for crate, output in refused:
            tail = [line for line in output.strip().splitlines() if line.strip()][-4:]
            print(f"   {crate}:")
            for line in tail:
                print(f"     {line}")
        return 1

    if silent:
        print()
        print(f"⛔ {', '.join(silent)} produced no rustdoc output at all — the")
        print("   build failed or the crate is gone, and zero warnings from a")
        print("   build that did not happen is not a score.")
        return 1

    # ⛔⛔ A REPAIR NOBODY MADE. Every tracked crate reading zero against a
    # baseline that records links is the shape of an instrument failure, not of
    # thirteen simultaneous repairs — it is what 2026-09-18 printed, and the two
    # arms above would not have caught it if cargo had exited 0 while replaying
    # no diagnostics. A genuine universal repair is a deliberate act and can say
    # so with `--update`; this refuses to infer one.
    banked = sum(len(v) for v in baseline.values() if isinstance(v, list))
    if baseline and banked and total == 0 and not (args.update or args.adopt):
        print()
        print(f"⛔ every tracked crate measured ZERO against a baseline of {banked}")
        print("   broken link(s). Thirteen crates are not repaired at once: this is")
        print("   the shape of a measurement that did not happen. Run one crate by")
        print("   hand (`cargo doc -p <crate> --no-deps`) and read its warnings")
        print("   before believing the table. A real universal repair is banked")
        print("   deliberately with --update.")
        return 1



    if args.adopt:
        # ⛔ MERGE, NEVER REWRITE. Existing counts are the floor and are not this
        # command's business; only absent crates gain an entry.
        merged = dict(baseline)
        added = {c: counts[c] for c in counts if c not in baseline}
        if not added:
            print("\nevery tracked crate already has a baseline entry; nothing to adopt.")
            return 0
        merged.update(added)
        os.makedirs(os.path.dirname(BASELINE), exist_ok=True)
        with open(BASELINE, "w", encoding="utf-8") as handle:
            json.dump(
                {
                    "_comment": (
                        "Broken intra-doc links per crate (ledger D103), NAMED. A "
                        "RATCHET: a link here may be repaired and must not be joined "
                        "by a new one. The list is the fact and the count is only its "
                        "length -- a baseline of totals could not tell a repair plus a "
                        "new break from no change at all. Bank a repair in the same "
                        "commit that earns it: scripts/check_doc_link_ratchet.py --update."
                    ),
                    "crates": merged,
                },
                handle,
                indent=2,
                sort_keys=True,
            )
            handle.write("\n")
        for crate, count in sorted(added.items()):
            print(f"\nadopted {crate} at {count}")
        print(f"baseline written: {BASELINE}   (existing counts untouched)")
        return 0

    if args.update:
        os.makedirs(os.path.dirname(BASELINE), exist_ok=True)
        with open(BASELINE, "w", encoding="utf-8") as handle:
            json.dump(
                {
                    "_comment": (
                        "Broken intra-doc links per crate (ledger D103), NAMED. A "
                        "RATCHET: a link here may be repaired and must not be joined "
                        "by a new one. The list is the fact and the count is only its "
                        "length -- a baseline of totals could not tell a repair plus a "
                        "new break from no change at all. Bank a repair in the same "
                        "commit that earns it: scripts/check_doc_link_ratchet.py --update."
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
        for crate in fell:
            for link in repaired[crate]:
                print(f"⭐ {crate}: repaired  - {link}")
        print("   run --update to bank it, in this commit.")
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
        print(f"⛔ {len(risen)} crate(s) gained broken doc links:")
        for crate in risen:
            print(f"   {crate}:")
            for link in appeared[crate]:
                print(f"     + {link}")
        print("   Each is a `[`Item`]` naming something renamed, moved or deleted.")
        print("   A deletion that leaves its references behind turns a doc comment")
        print("   into a description of a world that no longer exists — which in")
        print("   this repository is where the reasoning lives.")
        print("   ⛔ FIX THESE, not some other broken link in the same crate. The")
        print("   baseline records WHICH links are broken, so paying a regression")
        print("   off with an unrelated repair no longer restores the number.")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
