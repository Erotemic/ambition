"""No Cargo target carries `test = false` while it still holds `#[test]` arms.

⛔⛤ **A GUARD THAT DOES NOT EXIST LOOKS EXACTLY LIKE A GUARD THAT PASSES.**
MEASURED 2026-09-12: four `[[bin]]` targets in `game/ambition_app_tools` carried
`test = false` while holding **23 live arms** between them — `capture_scene` 7,
`moveset_takes` 8, `moveset_export` 5, `trace_replay` 3. The `--rust` lane
COMPILED every one of them (their clippy warnings are in the lane log) and
executed not a single arm. One of the silenced arms was the bounds-vs-hull
contact regression the take recorder is tuned against; another two were written
the same day and would have been born dead.

⇒ `test = false` is a build-time saving for a target that genuinely has no tests.
The moment somebody adds one, it becomes a silent deletion of their work, and
nothing in the lane says so — the test count simply does not go up, and nobody
is watching a number that only ever grows.

⚠ **THE CHECK IS OVER MANIFESTS AND SOURCES, NOT OVER A LIST.** An amnesty list
here would be the same defect one level up: the way to exempt a target would be
to add a line nobody re-reads. A target either has arms or it does not.
"""
from __future__ import annotations

import pathlib
import re

REPO = pathlib.Path(__file__).resolve().parent.parent.parent

TARGET_BLOCK = re.compile(r"\[\[(bin|test|bench|example)\]\]([^\[]*)")


def _settings(block: str) -> str:
    """The block's SETTINGS, with comments stripped.

    ⛔⛤ **THE FIRST VERSION SUBSTRING-MATCHED THE WHOLE BLOCK AND CAUGHT ITS OWN
    EXPLANATION.** The comment I left where a `test = false` was REMOVED says the
    words `test = false`, so the scan reported the repaired target as still
    silenced. A scanner that recognises PROSE has the rule backwards: decide the
    region first, then match inside it. Same family as reading a `#[cfg(test)]`
    marker out of a doc comment.
    """
    return "\n".join(
        line for line in block.splitlines() if not line.lstrip().startswith("#")
    )


def _scan() -> tuple[int, int, list[tuple[str, str, int]]]:
    """One pass: targets seen, targets setting `test = false`, targets silencing arms.

    ⛔⛤ **THE FLOOR AND THE SUBJECT READ THE SAME SCAN, AND A POISON IS WHY.**
    They used to walk the tree separately, so breaking the SUBJECT's glob left
    both tests green: the subject found nothing and passed, and the floor
    certified its own healthy walk. Two scans are two authorities, and the one
    that is meant to prove the other is not blind must be the same one.
    """
    seen = 0
    silencing = 0
    silenced: list[tuple[str, str, int]] = []
    for manifest in sorted(REPO.rglob("Cargo.toml")):
        parts = manifest.parts
        # ⚠ WORKTREES ARE OTHER AGENTS' CHECKOUTS, not this tree's contract.
        if "target" in parts or ".worktrees" in parts:
            continue
        text = manifest.read_text()
        for match in TARGET_BLOCK.finditer(text):
            seen += 1
            block = match.group(2)
            if "test = false" not in _settings(block):
                continue
            silencing += 1
            path = re.search(r'path = "([^"]+)"', block)
            name = re.search(r'name = "([^"]+)"', block)
            if not path:
                continue
            source = manifest.parent / path.group(1)
            if not source.exists():
                continue
            arms = source.read_text().count("#[test]")
            if arms:
                silenced.append(
                    (str(source.relative_to(REPO)), name.group(1) if name else "?", arms)
                )
    return seen, silencing, silenced


def test_no_cargo_target_silences_arms_it_still_carries():
    seen, silencing, silenced = _scan()
    # ⛔ THE ANTI-VACUITY FLOOR, ASKED FIRST AND FROM THIS SCAN. An empty corpus
    # prints `ok`: a walk that found no manifests, or a regex that matched no
    # target block, satisfies the real assertion for a reason that has nothing to
    # do with the repository.
    #
    # ⚠ THE NUMBERS ARE MEASURED, NOT INVENTED: 36 target blocks at 2026-09-12,
    # of which several legitimately set `test = false`. A round number I made up
    # is a floor that either never fires or fires for an unrelated reason.
    assert seen > 25, (
        f"only {seen} Cargo target block(s) scanned (36 at 2026-09-12) — the scan "
        "is not reaching the tree, so the assertion below certifies nothing"
    )
    assert silencing > 0, (
        f"{seen} target block(s) scanned and NOT ONE sets `test = false` — the "
        "subject of this check does not exist in this tree. If the setting is "
        "genuinely gone, delete this file rather than leave a green tick."
    )

    assert not silenced, (
        "these Cargo targets set `test = false` and still carry `#[test]` arms, "
        "so those arms run in NO lane:\n"
        + "\n".join(f"  {arms:>3} arm(s)  {name}  ({src})" for src, name, arms in silenced)
        + "\n⇒ Remove `test = false`, or delete the arms and say why. A compiled "
        "target whose tests never execute is indistinguishable from a passing one."
    )
