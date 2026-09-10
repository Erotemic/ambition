"""A citation can resolve perfectly and still name the wrong KIND of line.

⛔⛔ THE DEFECT THIS PINS: A4's writer map named four production readers of
`body_driving_seat`. The count was still four 156 commits later and TWO of the
four members were wrong — one of them a line inside `#[cfg(test)]`. Every other
check in the citation lane asks whether a citation RESOLVES. That one resolves.
Only its ROLE is wrong, so 156 commits of green said nothing about it.

⛔ AND THE FIRST VERSION OF THE REGION SCANNER PRODUCED THE SAME DEFECT ITSELF.
Counting braces from `#[cfg(test)]` without stopping at a BODYLESS item ran on
past `#[cfg(test)] mod slot_gesture_tests;` and swallowed `CharacterBrainTemplate`
— a production enum — reporting 21 findings where there are 18. That arm is the
reason this file tests the region scanner directly rather than only end to end.

⭐ SYNTHETIC ON PURPOSE. The live defect is about to be corrected in the doc that
carries it, and a control pinned to a live defect passes for the wrong reason
from the day it is fixed.
"""

from __future__ import annotations

import importlib.util
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
CHECKER = REPO / "scripts" / "check_planning_citations.py"

_spec = importlib.util.spec_from_file_location("_citations", CHECKER)
_mod = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_mod)

#: Two test modules, one bodyless `#[cfg(test)] mod`, and production items both
#: BETWEEN and BELOW them. Every trap the naive scanners fall into is here.
SYNTHETIC = """\
pub fn production_above() -> u32 {
    1
}

#[cfg(test)]
mod first_tests {
    #[test]
    fn inside_the_first_region() {
        assert!(true);
    }
}

pub fn production_between() -> u32 {
    2
}

#[cfg(test)]
mod named_for_its_subject;

pub enum ProductionBelowABodylessCfgTest {
    Alpha,
    Beta,
}

#[cfg(test)]
mod second_tests {
    #[test]
    fn inside_the_second_region() {
        assert!(true);
    }
}

pub fn production_at_the_bottom() -> u32 {
    3
}
"""


def _regions(tmp_path: Path) -> list[tuple[int, int]]:
    src = tmp_path / "synthetic.rs"
    src.write_text(SYNTHETIC)
    return _mod.cfg_test_regions(src)


def _line_of(needle: str) -> int:
    for n, line in enumerate(SYNTHETIC.splitlines(), 1):
        if needle in line:
            return n
    raise AssertionError(f"{needle!r} is not in the fixture")


def _covered(regions: list[tuple[int, int]], line: int) -> bool:
    return any(start <= line <= end for start, end in regions)


def test_every_cfg_test_opens_its_own_region(tmp_path) -> None:
    """Not one region, and not the whole file below the first attribute."""
    regions = _regions(tmp_path)
    assert len(regions) == 2, regions
    assert _covered(regions, _line_of("fn inside_the_first_region")), regions
    assert _covered(regions, _line_of("fn inside_the_second_region")), regions


def test_production_between_two_test_modules_is_not_swallowed(tmp_path) -> None:
    """The trap in `split at the first #[cfg(test)]`."""
    regions = _regions(tmp_path)
    assert not _covered(regions, _line_of("fn production_between")), regions
    assert not _covered(regions, _line_of("fn production_at_the_bottom")), regions
    assert not _covered(regions, _line_of("fn production_above")), regions


def test_a_bodyless_cfg_test_mod_opens_no_region(tmp_path) -> None:
    """⛔ THE ARM THAT WAS WRONG. `#[cfg(test)] mod x;` ends at its semicolon.

    Without this, brace counting runs to the next unrelated block and reports the
    item below it as test-only — which is the very defect being hunted, produced
    by the hunter.
    """
    regions = _regions(tmp_path)
    enum_line = _line_of("pub enum ProductionBelowABodylessCfgTest")
    assert not _covered(regions, enum_line), (
        f"line {enum_line} is production and a bodyless `#[cfg(test)] mod x;` "
        f"above it must not put it in a test region: {regions}"
    )


def _run(tmp_path: Path, body: str) -> str:
    doc = tmp_path / "fixture.md"
    doc.write_text(body)
    proc = subprocess.run(
        [sys.executable, str(CHECKER), "--roles", str(doc)],
        cwd=REPO, capture_output=True, text=True,
    )
    return proc.stdout + proc.stderr


def _a_tracked_file_with_a_test_region() -> tuple[str, int]:
    """A real tracked `.rs` with a `#[cfg(test)]` region, and a line inside it.

    ⭐ DERIVED AT RUN TIME, NOT WRITTEN DOWN. Citation resolution runs against
    `git ls-files`, so this arm cannot use a temporary file — and a hand-written
    `file.rs:NN` here would rot the first time that file grew a line. Sorting
    makes the choice deterministic.
    """
    tracked = subprocess.run(
        ["git", "ls-files", "*.rs"], cwd=REPO, capture_output=True, text=True,
    ).stdout.split("\n")
    for rel in sorted(p for p in tracked if p):
        regions = _mod.cfg_test_regions(REPO / rel)
        if regions:
            start, end = regions[0]
            return rel, (start + end) // 2
    raise AssertionError("no tracked .rs file has a #[cfg(test)] region")


def test_a_citation_into_a_test_region_is_reported(tmp_path) -> None:
    """⭐ THE POSITIVE ARM, and it is the one that matters.

    A lane that reported nothing would still pass the marker test below.
    """
    rel, line = _a_tracked_file_with_a_test_region()
    out = _run(tmp_path, f"A citation that names a test line: `{rel}:{line}`.\n")
    assert "1 land inside a #[cfg(test)] region" in out, out
    assert f"{rel}:{line}" in out, out
    assert "only in a TEST build" in out, out


def test_the_cite_test_marker_silences_one(tmp_path) -> None:
    """The deliberate case: a fixture named in a coverage table is not a defect."""
    rel, line = _a_tracked_file_with_a_test_region()
    out = _run(
        tmp_path,
        f"A citation that names a test line ON PURPOSE: `{rel}:{line}`. "
        "<!-- cite-test -->\n",
    )
    assert "none of them land inside a #[cfg(test)] region" in out, out


def test_the_role_lane_does_not_gate(tmp_path) -> None:
    """⛔ REPORT-ONLY. 18 of 336 live citations land in a test region and about
    half are deliberate, so gating here would redden the planning corpus on rows
    that are correct."""
    doc = tmp_path / "fixture.md"
    doc.write_text("Nothing to see.\n")
    proc = subprocess.run(
        [sys.executable, str(CHECKER), "--roles", "--strict", str(doc)],
        cwd=REPO, capture_output=True, text=True,
    )
    assert proc.returncode == 0, proc.stdout + proc.stderr
