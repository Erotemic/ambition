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

# ⛔⛤ **BOTH TEST PREDICATES ARE IMPORTED, NOT RESPELLED, AND THIS CENSUS BRIEFLY
# HAD ITS OWN COPY OF EACH — 2026-09-17.** `scripts/lib/test_paths.py` exists
# because there were FIVE spellings of *"is this Rust file test-only?"* giving
# five different answers, and it owns the inline-module half too (which MOVED
# there from `check_rollback_mutators_run_in_sim.py` once this census and
# `architecture_census.py` turned out to hold copies). Mine were a sixth and a
# second:
# the file rule missed `test.rs`, `test_support.rs` and the four files whose
# first attribute is an inner `#![cfg(test)]`, which no name rule can see.
#
# ⭐ MEASURED BEFORE COLLAPSING, all four combinations of the two rules against
# this corpus: **333 `ResMut<T>` types and 85 multi-writer, identically.** So the
# de-duplication changes nothing here and the keepers are strictly better
# informed — which is the only kind of collapse worth doing without re-poisoning
# every consumer.
#
# ⚠ WHAT THE SHARED STRIPPER LEAVES: it removes inline `#[cfg(test)] mod X { }`
# blocks only, so a `#[cfg(test)] fn helper(mut r: ResMut<T>)` sitting in a
# production file still counts as a writer. There are none today (the measurement
# above would differ), and widening that function reaches every one of its
# consumers — its own docstring says not to do that without reading each one's
# counts.
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent / "lib"))
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from rust_source import strip_comments  # noqa: E402
from test_paths import is_test_path, strip_test_modules  # noqa: E402
DEFAULT_PATHS = ("crates", "game")


def rust_files(paths: tuple[str, ...]) -> list[str]:
    out = subprocess.run(
        ["git", "ls-files", *paths], capture_output=True, text=True, check=True
    ).stdout.split()
    return [f for f in out if f.endswith(".rs")]


def production_files(paths: tuple[str, ...] = DEFAULT_PATHS) -> list[str]:
    """Every tracked Rust file that is not test-only, by the shared predicate.

    ⛔⛤ **THE CENSUS COUNTED WHOLE TEST FILES AS WRITERS UNTIL 2026-09-17**, and
    its docstring said it read NON-TEST code the whole time: a `mod tests` in its
    own `tests.rs` and an integration test under `tests/` carry no
    `#[cfg(test)]` line for the stripper to cut at — the parent module carries
    it. MEASURED: 1,866 tracked files, **1,294 production**, and eight types left
    the shortlist (`Captured`, `CapturedHits`, `DeathsSeen`, `FixedStepsTaken`,
    `PortalWorldFrame`, `RoomResetsSeen`, `SaveRestored`, `SeatMenuFrames`), each
    multi-writer only because a fixture wrote it.
    """
    return [f for f in rust_files(paths) if not is_test_path(pathlib.Path(f))]


def writers(files: list[str]) -> dict[str, set[str]]:
    """`{short type name: {file, ...}}` over the files it is GIVEN.

    ⚠ Each `#[cfg(test)]` ITEM is cut by [`strip_test_modules`]. A fixture that
    builds a resource by hand is not a second authority over it, and counting
    fixtures is how a census manufactures findings nobody can act on.

    ⛔ **A WHOLE TEST FILE HAS NO SUCH MARKER**, so the caller filters those out
    with [`production_files`] — see what that cost when it did not.
    """
    found: dict[str, set[str]] = collections.defaultdict(set)
    for f in files:
        src = pathlib.Path(f).read_text(encoding="utf-8", errors="replace")
        src = strip_test_modules(strip_comments(src))
        for m in RESMUT.finditer(src):
            found[m.group(1).split("::")[-1]].add(f)
    return found


#: A `ResMut<T>` parameter's BINDING, so an access through it can be found.
#: `mut save: ResMut<AmbitionGameSave>` binds `save`; the `mut` is optional
#: because a system can take `ResMut` immutably-bound and still call `&mut self`
#: methods through it.
_BINDING = r"(?:mut\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*:\s*ResMut<\s*%s\s*(?:<[^<>]*>)?\s*>"

#: An assignment, excluding `==` and `=>`. `+=`, `|=` and friends count.
_ASSIGN = re.compile(r"\s*(?:[+\-*/|&^%]=|=(?![=>]))")

#: The whole-resource target: `*state = Default::default()` replaces every field
#: at once, which is a different kind of authority from touching one of them.
WHOLE = "*<the resource>"


def shared_targets(ty: str, files: set[str] | list[str]) -> dict[str, tuple[set[str], set[str]]]:
    """`{target: (files touching it, files touching it MUTATION-SHAPED)}`.

    ⭐ **THIS IS THE NARROWING STEP, AND IT USED TO BE DONE BY HAND.** "Which
    resources have many writers" is a shortlist; "which FIELD OR METHOD do two of
    those writers both reach for" is what turns one into a question somebody can
    answer. The table it produces lived in the adjudication guard's docstring as
    hand-carried numbers until 2026-09-17.

    ⛔⛤ **TWO COUNTS, BECAUSE ONE OF THEM CANNOT BE DERIVED HONESTLY.** A call
    through a `ResMut` binding may be a read (`save.data()`) or a write
    (`save.data_mut()`), and no regex can tell `queue.record(..)` from
    `queue.len()` without the signature. So this returns BOTH: every writer file
    that TOUCHES the target, and the subset whose access is mutation-shaped —
    an assignment, or a `*_mut` name. Read the wider set as *"where to look"* and
    the narrower as *"where a write is certain"*. Neither is the type's writer
    count, which is a third number: a file can be a `ResMut<T>` writer and share
    no target with anyone.

    ⚠ Only targets reached by TWO OR MORE writer files are returned; a field one
    writer owns alone is the shape this census is looking for, not against.
    """
    binding = re.compile(_BINDING % re.escape(ty))
    touched: dict[str, set[str]] = collections.defaultdict(set)
    mutated: dict[str, set[str]] = collections.defaultdict(set)
    for f in sorted(files):
        src = strip_test_modules(
            strip_comments(pathlib.Path(f).read_text(encoding="utf-8", errors="replace"))
        )
        # A short type name can be bound under its qualified path, so fall back
        # to the suffix spelling the census already collapses on.
        names = set(binding.findall(src))
        if not names:
            names = set(
                re.findall(
                    r"(?:mut\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*:\s*ResMut<[^>]*\b"
                    + re.escape(ty)
                    + r"\s*>",
                    src,
                )
            )
        for name in names:
            for m in re.finditer(r"\*" + re.escape(name) + r"\b", src):
                if _ASSIGN.match(src[m.end() :]):
                    touched[WHOLE].add(f)
                    mutated[WHOLE].add(f)
            for m in re.finditer(re.escape(name) + r"\.([A-Za-z_][A-Za-z0-9_]*)", src):
                target = m.group(1)
                touched[target].add(f)
                rest = src[m.end() :]
                if target.endswith("_mut") or _ASSIGN.match(rest):
                    mutated[target].add(f)
    return {
        target: (fs, mutated.get(target, set()))
        for target, fs in touched.items()
        if len(fs) > 1
    }


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("paths", nargs="*", default=list(DEFAULT_PATHS))
    ap.add_argument(
        "--shared-targets",
        action="store_true",
        help=(
            "for each multi-writer type, also print the fields and methods that "
            "TWO OR MORE of its writer files reach for — the narrowing step that "
            "turns the shortlist into a question somebody can answer"
        ),
    )
    args = ap.parse_args(argv)

    files = production_files(tuple(args.paths))
    if not files:
        # ⛔ An empty corpus would print "0 multi-writer types" and read as a
        # clean bill of health.
        print("no .rs files matched — the instrument found nothing to read")
        return 2

    found = writers(files)
    multi = {t: fs for t, fs in found.items() if len(fs) > 1}
    print(
        f"{len(files)} production files; {len(found)} `ResMut<T>` types; "
        f"{len(multi)} written from >1 file\n"
    )
    for ty, fs in sorted(multi.items(), key=lambda kv: (-len(kv[1]), kv[0])):
        print(f"  {ty}  ({len(fs)} files)")
        for f in sorted(fs):
            print(f"      {f}")
        if not args.shared_targets:
            continue
        targets = shared_targets(ty, fs)
        if not targets:
            print("      shared targets: NONE — no field or method is reached")
            print("        for by two of these files, so there is nothing to join")
            continue
        for target, (touching, writing) in sorted(
            targets.items(), key=lambda kv: (-len(kv[1][1]), -len(kv[1][0]), kv[0])
        ):
            certain = f", {len(writing)} mutation-shaped" if writing else ", none certain"
            print(f"      -> {target}  ({len(touching)} of {len(fs)} files{certain})")
    print(
        "\n⇒ A SHORTLIST, NOT FINDINGS. For each: what READS this, and can an"
        "\n  ambiguity in it reach a decision? Then POISON one writer and run the"
        "\n  test that should care — a green means nothing tests it OR another"
        "\n  writer covers it, and only the second is a finding."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
