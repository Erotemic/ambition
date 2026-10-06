#!/usr/bin/env python3
"""Ratchet the readers that ask a body's LEVEL box outside the movement kernel.

A body that is not square has one box, turned to its DOWN: the published
footprint (`CenteredAabb`), or `BodyKinematics::collision_box(last_step)` for
the collision box. `BodyKinematics::aabb()` and `size * 0.5` are the level box.
A reader that asks the level box is right in normal gravity and wrong in a
room whose gravity is turned (LEVEL-BOX-READERS, `docs/planning/queue.md`).

This check runs one search over the Rust sources and compares each line it
finds with a baseline (`scripts/baselines/level-box-readers.json`). Each line
of the baseline is classified: it is a legitimate read with its reason (a
shot has no frame, the centre only, a game with no turned gravity), or it is a
read still to convert. The check fails when:

- the search finds a line that is not in the baseline: a NEW level-box read.
  Convert it to the rule, or classify it in the baseline with its reason;
- the baseline has a line the search no longer finds: delete it. The counts
  may only fall, and a stale line would let a new read hide behind it.

What this check can NOT show: that no reader asks the level box. It finds the
spellings in `SEARCH` and no other. A reader that builds the box another way
(`Aabb::new(pos, half)` from a local, a destructured `size`) is not found.

Usage::

    python3 scripts/check_level_box_readers.py            # check
    python3 scripts/check_level_box_readers.py --list     # the lines found now
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from collections import Counter
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
BASELINE = REPO / "scripts" / "baselines" / "level-box-readers.json"

#: The search. A line of Rust that matches is a read of a body's level box.
SEARCH = r"(kinematics|kin|body)\.aabb\(\)|(kinematics|kin)\.size \* 0\.5|(kinematics|kin)\.size / 2"
PATTERN = re.compile(SEARCH)

#: Where the sources are, under the repository root.
ROOTS = ("crates", "game")

#: A path with one of these parts is a test file, and is not searched.
TEST_PATH_PARTS = ("/tests/", "/tests_")
TEST_FILE_SUFFIXES = ("tests.rs", "_tests.rs")

INLINE_TEST_MODULE = re.compile(r"\s*(pub(\([a-z]+\))? )?mod \w+ *\{")


def is_test_path(relative: str) -> bool:
    return any(part in f"/{relative}" for part in TEST_PATH_PARTS) or relative.endswith(TEST_FILE_SUFFIXES)


def inline_test_ranges(lines: list[str]) -> list[tuple[int, int]]:
    """The 1-based line ranges of each `#[cfg(test)] mod name { ... }` block.

    A path filter does not find these: the tests of a file can be in the file.
    """

    ranges = []
    index = 0
    while index < len(lines):
        if lines[index].strip().startswith("#[cfg(test)]"):
            opener = index + 1
            while opener < len(lines) and lines[opener].strip().startswith("#["):
                opener += 1
            if opener < len(lines) and INLINE_TEST_MODULE.match(lines[opener]):
                depth = 0
                end = opener
                while end < len(lines):
                    depth += lines[end].count("{") - lines[end].count("}")
                    if depth <= 0:
                        break
                    end += 1
                ranges.append((opener + 1, end + 1))
                index = end
        index += 1
    return ranges


def find(repo: Path = REPO) -> list[dict]:
    """Each line the search finds: its file, its line number, and its code."""

    found = []
    for root in ROOTS:
        base = repo / root
        if not base.is_dir():
            continue
        for path in sorted(base.rglob("*.rs")):
            relative = path.relative_to(repo).as_posix()
            if "/target/" in f"/{relative}" or is_test_path(relative):
                continue
            lines = path.read_text(encoding="utf-8").split("\n")
            tests = inline_test_ranges(lines)
            for number, line in enumerate(lines, start=1):
                code = line.strip()
                if not PATTERN.search(line) or code.startswith("//"):
                    continue
                if any(start <= number <= end for start, end in tests):
                    continue
                found.append({"file": relative, "line": number, "code": code})
    return found


def load_baseline(path: Path = BASELINE) -> dict:
    with open(path, encoding="utf-8") as handle:
        return json.load(handle)


def compare(found: list[dict], baseline: dict) -> tuple[list[dict], list[dict], list[str]]:
    """(new reads, stale baseline lines, baseline errors)."""

    classes = baseline.get("classes", {})
    errors = []
    wanted: Counter = Counter()
    for entry in baseline.get("reads", []):
        if entry.get("class") not in classes:
            errors.append(f"{entry.get('file')}: `{entry.get('code')}` has the class `{entry.get('class')}`, which the baseline does not define")
        wanted[(entry["file"], entry["code"])] += 1
    seen: Counter = Counter()
    new = []
    for line in found:
        key = (line["file"], line["code"])
        seen[key] += 1
        if seen[key] > wanted[key]:
            new.append(line)
    stale = []
    for (file, code), count in sorted(wanted.items()):
        for _ in range(count - seen[(file, code)]):
            stale.append({"file": file, "code": code})
    return new, stale, errors


def main(argv: list[str] | None = None, repo: Path = REPO, baseline_path: Path = BASELINE) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--list", action="store_true", help="print the lines found now, as baseline entries with no class")
    args = parser.parse_args(argv)

    found = find(repo)
    print("the search, over `*.rs` under " + " and ".join(f"`{root}`" for root in ROOTS) + ", outside test files,")
    print("inline `#[cfg(test)]` modules and comment lines:")
    print(f"    {SEARCH}")
    if args.list:
        print(json.dumps([{"file": line["file"], "code": line["code"], "class": ""} for line in found], indent=2))
        return 0

    baseline = load_baseline(baseline_path)
    new, stale, errors = compare(found, baseline)
    by_class: Counter = Counter(entry["class"] for entry in baseline.get("reads", []))
    print(f"found {len(found)} level-box read(s); the baseline classifies {sum(by_class.values())}:")
    for name, reason in baseline.get("classes", {}).items():
        print(f"  {by_class[name]:3}  {name}: {reason}")
    if not found and baseline.get("reads"):
        print("\nFAIL: the search found nothing at all, and the baseline is not empty: the search is broken, not the code.")
        return 1
    for error in errors:
        print(f"\nFAIL: {error}")
    if new:
        print("\nFAIL: a level-box read that the baseline does not classify:")
        for line in new:
            print(f"    {line['file']}:{line['line']}: {line['code']}")
        print(
            "⇒ ask the rule (`BodyKinematics::collision_box(last_step)`, or the published footprint), or add the\n"
            f"  line to {baseline_path.relative_to(repo) if baseline_path.is_relative_to(repo) else baseline_path} with the class that says why the level box is right there."
        )
    if stale:
        print("\nFAIL: the baseline has a line the search no longer finds (the counts may only fall: delete it):")
        for line in stale:
            print(f"    {line['file']}: {line['code']}")
    if new or stale or errors:
        return 1
    print("\nok: every level-box read the search finds is classified. This does not show that no other reader asks the level box.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
