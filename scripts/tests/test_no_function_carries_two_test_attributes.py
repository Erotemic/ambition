"""A `#[test]` may not be stolen from the function it was written for.

⛔⛔ TWO GUARDS DIED THIS WAY ON 2026-09-04 AND NEITHER FAILED. In
`ladder_rig.rs`, `mirroring_a_bout_swaps_every_per_seat_reading` had its doc
block and its `#[test]` bound to the NEXT function; the mirror check became an
ordinary private `fn` nothing called, and it had never run once. It is the guard
proving index 0 means the higher rung in both halves of a paired bout — the
assumption the whole paired reading rests on. In `moveset.rs`,
`the_side_special_is_a_command_grab_and_not_the_standing_grab_renamed` was dead
for a day, and it was the guard for the `lunge_grab` claim published above it.

⭐ THE MECHANISM IS WHY IT RECURS, and it is not "somebody forgot an attribute".
Inserting a new test between an existing test's DOC BLOCK and its FUNCTION BODY
leaves the old `///` lines and the old `#[test]` attached to the newly inserted
function. The new function then carries TWO `#[test]` attributes and a
description of something else, and the displaced function silently becomes
private dead code. ⇒ Nothing about that is visible in review: the diff looks
like an added test, and it is — plus a deleted one nobody deleted.

⛔ AND BOTH HALVES OF THE DAMAGE ARE QUIET. `duplicated attribute` reads as lint
noise, and the displaced function's `never used` hides among a crate's other
unused-function warnings. In `ambition_demo_smash_app` there were two such
warnings already, so the dead guard's was the third and looked like more of the
same.

⚠ THIS CHECKS THE ARITHMETIC, NOT THE INTENT: between one function and the
previous item, exactly one `#[test]` may accumulate. Two means one was stolen.
That is cheap, has no false positives across 1,774 files, and catches the shape
before the test-count guards would — they only notice if somebody happens to
compare a number.
"""

from __future__ import annotations

import re
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent.parent
ROOTS = ("crates", "game", "tests")

TEST_ATTR = re.compile(r"#\[\s*(?:tokio::)?test\s*[\]\(]")
FN = re.compile(r"(?:pub\s+)?(?:async\s+)?fn\s+\w+")


def _rust_files() -> list[Path]:
    found: list[Path] = []
    for root in ROOTS:
        found.extend(sorted((REPO / root).rglob("*.rs")))
    return found


def _logical_lines(text: str) -> list[tuple[int, str]]:
    """Collapse each attribute onto one logical line, keeping its first line number.

    ⛔⛔ WITHOUT THIS THE GUARD FAILS SILENTLY, AND IN THE DANGEROUS DIRECTION.
    An attribute may span lines:

        #[test]
        #[cfg_attr(
            not(has_baked_packs),
            ignore = "this tree has no ultrapack ..."
        )]
        fn intro_cart_pack_spec_resolves_at_two_tiers() {

    Walking back from the `fn`, the lines immediately above are `)]`, a bare
    string, and `not(has_baked_packs),` — none of which begin with `#[` or `//`.
    ⇒ A scanner that ends its run of attributes at "not a doc-or-attribute line"
    stops BEFORE reaching the `#[test]`s, counts one instead of two, and passes.
    ⚠ That is a FALSE NEGATIVE: a stolen attribute sitting above any multi-line
    `cfg_attr` would ship. Found by `YardratAmbition` hitting the mirror-image
    bug (a false POSITIVE, which is loud) in their own walk-back, on
    `ambition_sprite_sheet/src/sprite_packs.rs:169`.
    """
    out: list[tuple[int, str]] = []
    buffer = ""
    start = 0
    for number, raw in enumerate(text.split("\n"), 1):
        stripped = raw.strip()
        if buffer:
            buffer += " " + stripped
            if buffer.count("[") <= buffer.count("]"):
                out.append((start, buffer))
                buffer = ""
            continue
        if stripped.startswith("#[") or stripped.startswith("#!["):
            if stripped.count("[") > stripped.count("]"):
                buffer, start = stripped, number
                continue
        out.append((number, stripped))
    if buffer:
        out.append((start, buffer))
    return out


def _stolen_attributes(path: Path) -> list[str]:
    text = path.read_text(encoding="utf-8", errors="replace")
    pending: list[int] = []
    found: list[str] = []
    for number, stripped in _logical_lines(text):
        if TEST_ATTR.match(stripped):
            pending.append(number)
        elif FN.match(stripped):
            if len(pending) > 1:
                found.append(
                    f"{path.relative_to(REPO)}:{number}: `{stripped[:70]}` carries "
                    f"{len(pending)} `#[test]` attributes (lines {pending}) — the "
                    "earlier one belonged to a function that is now dead code"
                )
            pending = []
        elif stripped and not stripped.startswith(("//", "#[", "#![")):
            # Any other real code ends the run of attributes.
            pending = []
    return found


def test_no_function_carries_more_than_one_test_attribute() -> None:
    files = _rust_files()
    # ⛔ An anti-vacuity floor: this walks the tree by glob, and a walk that
    # finds nothing passes every assertion below it.
    assert len(files) > 500, (
        f"only {len(files)} Rust file(s) found under {ROOTS} — the corpus this "
        "guard walks has moved, and an empty walk cannot fail"
    )
    stolen: list[str] = []
    for path in files:
        stolen.extend(_stolen_attributes(path))
    assert not stolen, (
        "a `#[test]` is attached to a function it was not written for, which "
        "leaves the function it WAS written for as private dead code that never "
        "runs. Move the stray attribute back above its own `fn`:\n  "
        + "\n  ".join(stolen)
    )


def test_the_scanner_sees_through_a_multi_line_attribute(tmp_path) -> None:
    """⛔ THE GUARD'S OWN BLIND SPOT, pinned — it shipped with this one.

    The first version ended its run of attributes at the first line that was
    neither a doc comment nor an attribute START, so the continuation lines of a
    multi-line `#[cfg_attr(...)]` reset the count. A stolen `#[test]` sitting
    above such a block counted ONE and passed.

    ⚠ A FALSE NEGATIVE IS THE DANGEROUS DIRECTION for this check and is why this
    test exists rather than a note: the guard reports fewer attributes than are
    there, so it goes quiet exactly where it is needed. `YardratAmbition` found
    it by hitting the mirror-image bug — a false POSITIVE, which is loud — in
    their own walk-back over `sprite_packs.rs:169`.

    ⭐ MEASURED AFTER THE FIX: the repository-wide count stayed at 2, so this
    blind spot was hiding nothing in this tree. Recorded because "the fix
    changed no finding" is the result a later reader would otherwise re-derive,
    and because it would have hidden the NEXT one.
    """
    source = tmp_path / "fixture.rs"
    source.write_text(
        "mod tests {\n"
        "    /// docs for the function below\n"
        "    #[test]\n"
        "    /// docs for a function inserted between that doc and its body\n"
        "    #[test]\n"
        "    #[cfg_attr(\n"
        '        not(feature = "x"),\n'
        '        ignore = "a reason spanning lines"\n'
        "    )]\n"
        "    fn the_inserted_one() {}\n"
        "\n"
        "    fn the_displaced_one() {}\n"
        "}\n",
        encoding="utf-8",
    )
    logical = _logical_lines(source.read_text(encoding="utf-8"))
    attrs = [number for number, text in logical if TEST_ATTR.match(text)]
    assert attrs == [3, 5], (
        "both `#[test]` attributes must survive the collapse with their original "
        f"line numbers; got {attrs}"
    )
    functions = [text for _, text in logical if FN.match(text)]
    assert len(functions) == 2, (
        "the multi-line attribute must collapse to one logical line rather than "
        f"being read as code that ends the attribute run; got {functions}"
    )
