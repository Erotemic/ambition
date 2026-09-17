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
CFG_TEST = re.compile(r"#\[cfg\(test\)\]")
DEFAULT_PATHS = ("crates", "game")


def rust_files(paths: tuple[str, ...]) -> list[str]:
    out = subprocess.run(
        ["git", "ls-files", *paths], capture_output=True, text=True, check=True
    ).stdout.split()
    return [f for f in out if f.endswith(".rs")]


def strip_test_modules(src: str) -> str:
    """Source with each `#[cfg(test)]` ITEM removed — and nothing after it.

    ⛔⛤ **THE CUT USED TO BE `src.split("#[cfg(test)]")[0]`, WHICH THROWS AWAY
    THE FILE TAIL, AND IN THIS TREE THE TAIL IS USUALLY PRODUCTION CODE.** A
    module declares its tests near the top — `#[cfg(test)] mod tests;` at
    `ambition_platformer2d_runtime/src/lib.rs:25` — so everything below that line
    was invisible to the census. MEASURED 2026-09-17: **777 of 1,302 production
    files carry a `#[cfg(test)]`, and 40 of them hold 77 `ResMut<T>`
    occurrences after the first one.** The census was UNDERCOUNTING writers, in a
    direction nobody would notice, and a poison appended to a file's end proved
    it by not firing.

    ⇒ Cut per ITEM: a declaration (`mod tests;`) ends at its semicolon, and an
    inline module or `#[cfg(test)] fn` ends at the brace that closes it.

    ⚠ The brace match is naive about braces inside string literals inside a test
    body. Over-cutting loses production writers, which is the direction the old
    rule already erred in; under-cutting would count a fixture as an authority.
    Both are visible as a population change, which is why the ratchet on this
    census records the count.
    """
    out: list[str] = []
    pos = 0
    while True:
        match = CFG_TEST.search(src, pos)
        if match is None:
            out.append(src[pos:])
            return "".join(out)
        out.append(src[pos : match.start()])
        rest = src[match.end() :]
        brace = rest.find("{")
        semi = rest.find(";")
        if semi != -1 and (brace == -1 or semi < brace):
            pos = match.end() + semi + 1
            continue
        if brace == -1:
            return "".join(out)
        depth = 0
        cursor = match.end() + brace
        while cursor < len(src):
            if src[cursor] == "{":
                depth += 1
            elif src[cursor] == "}":
                depth -= 1
                if depth == 0:
                    break
            cursor += 1
        pos = cursor + 1


def is_test_file(path: str) -> bool:
    """A whole FILE of tests, which has no `#[cfg(test)]` marker to cut at.

    ⛔⛤ **THE `#[cfg(test)]` CUT BELOW IS HALF THE RULE, AND FOR 564 FILES IT IS
    THE WRONG HALF.** An integration test under `tests/` and a `mod tests`
    carried in its own `tests.rs` contain no `#[cfg(test)]` line at all — the
    parent module carries it — so every `ResMut<T>` in them counted as a writer
    while this module's docstring said the census reads NON-TEST code. MEASURED
    2026-09-17: 1,866 files become 1,302; `ResMut<T>` types 368 become 310; and
    the shortlist 83 becomes **75**. The eight that leave are multi-writer only
    because a fixture writes them (`Captured`, `CapturedHits`, `DeathsSeen`,
    `FixedStepsTaken`, `PortalWorldFrame`, `RoomResetsSeen`, `SaveRestored`,
    `SeatMenuFrames`).

    ⚠ The three shapes are measured, not guessed: 284 files under a `tests/`
    directory, 214 named `tests.rs`, 66 named `*_tests.rs`. No production
    directory in this tree is named `tests`, and nothing named `test_*` is
    excluded — a production helper used by tests is production.
    """
    parts = pathlib.PurePosixPath(path).parts
    name = parts[-1] if parts else ""
    return "tests" in parts or name == "tests.rs" or name.endswith("_tests.rs")


def production_files(paths: tuple[str, ...] = DEFAULT_PATHS) -> list[str]:
    return [f for f in rust_files(paths) if not is_test_file(f)]


def writers(files: list[str]) -> dict[str, set[str]]:
    """`{short type name: {file, ...}}` over the files it is GIVEN.

    ⚠ Each `#[cfg(test)]` ITEM is cut by [`strip_test_modules`]. A fixture that
    builds a resource by hand is not a second authority over it, and counting
    fixtures is how a census manufactures findings nobody can act on.

    ⛔ **A WHOLE TEST FILE HAS NO SUCH MARKER**, so the caller filters those out
    with [`is_test_file`] — see what that cost when it did not.
    """
    found: dict[str, set[str]] = collections.defaultdict(set)
    for f in files:
        src = pathlib.Path(f).read_text(encoding="utf-8", errors="replace")
        src = strip_test_modules(src)
        for m in RESMUT.finditer(src):
            found[m.group(1).split("::")[-1]].add(f)
    return found


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("paths", nargs="*", default=list(DEFAULT_PATHS))
    args = ap.parse_args(argv)

    files = [f for f in rust_files(tuple(args.paths)) if not is_test_file(f)]
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
    print(
        "\n⇒ A SHORTLIST, NOT FINDINGS. For each: what READS this, and can an"
        "\n  ambiguity in it reach a decision? Then POISON one writer and run the"
        "\n  test that should care — a green means nothing tests it OR another"
        "\n  writer covers it, and only the second is a finding."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
