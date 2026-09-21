"""A grab's reach is computed in exactly one place.

⛔⛔ IT WAS COMPUTED IN THREE, AND THEY ANSWER DIFFERENT QUESTIONS ABOUT THE SAME
MOVE. `offset + half_extents` on the x axis fed:

  * the BRAIN's `AttackCandidate::reach` — how close a fighter gets before it
    tries a grab (`features/ecs/actors/update.rs`);
  * the tether LINE a player sees, twice over, once for the player road and once
    for the actor road (`pose_view.rs`, `view_index.rs`).

⇒ Change one and what a fighter SEES stops matching the distance the brain AIMS
at — and nothing fails, because both numbers stay perfectly self-consistent. That
is the failure mode this guard exists for: a divergence with no error.

⭐ THE TETHER IS WHY IT MATTERS. Projectile Polygon's grab reaches 150 px because
that reach IS the character, so the two readings are furthest apart on exactly the
fighter built around them.

⚠ TEXTUAL, AND HONEST ABOUT IT. It cannot see a copy that renames its locals or
spells the sum differently. What it does hold is that the OBVIOUS re-inlining —
adding the two fields at a call site because it is right there — fails loudly
instead of quietly.
"""

import pathlib
import re

REPO = pathlib.Path(__file__).resolve().parents[2]
HOME = REPO / "crates/ambition_entity_catalog/src/smash_capture.rs"

# The sum, however the receiver is spelled: `attempt.offset.0 + attempt.half_extents.0`,
# `offset.x + half.x`, `params.offset.0 + params.half_extents.0`.
# ⛔ `half_extents` SPELLED IN FULL, and that is a deliberate narrowing after a
# second false positive: `pointed_polygon_moveset.rs:701` sums a HitVolume's
# `offset.0 + half.0` inside an assertion message, in a file that authors a grab
# elsewhere and so passed the file-level filter. A `HitVolume` and a
# `CaptureAttemptParams` are different types answering different questions with
# the same arithmetic.
#
# ⚠ WHAT THIS TRADES AWAY, stated rather than discovered later: it no longer
# catches the bare `offset.x + half.x` form the view sites used, which existed
# only because `live_capture_reach` returned the box as a tuple. It returns the
# REACH now, so that form cannot arise without also reverting the return type —
# a change whose own doc explains why not. What remains covered is the realistic
# regression: somebody holding a `CaptureAttemptParams` adding its two fields
# because they are right there.
COPY = re.compile(r"(\w+\.)?offset\.(0|x)\s*\+\s*(\w+\.)?half_extents\.(0|x)")


# ⛔⛔ SCOPED TO THE ROADS THAT COULD RE-INLINE *THIS* FORMULA, because the first
# version was not and reported three FALSE POSITIVES on its first run: the
# catalog computes the same leading-edge sum for an attack `HitVolume`
# (`lib.rs:2046`, `:2061`) and a moveset authors its own rect geometry
# (`pointed_polygon_moveset.rs:701`). Those are the same ARITHMETIC on different
# TYPES answering a different question, and forbidding them would make the guard
# a nuisance that gets deleted.
#
# ⇒ A file can only duplicate the capture reach if it can NAME the capture reach:
# it mentions `CaptureAttemptParams` or reads `live_capture_reach`. That is the
# population, and it is derived rather than listed, so a fourth road added
# tomorrow is covered without anybody remembering this file.
MENTIONS = ("CaptureAttemptParams", "live_capture_reach")


# ⛔⛤ **THE SECOND HOME, AND IT IS A DIFFERENT FORMULA WEARING THE SAME
# ARITHMETIC.** A `HitVolume`'s box is also `offset ± half_extent`, and this
# guard's population filter is a FILE filter: `lib.rs` names
# `CaptureAttemptParams` in code (it hydrates capture params to price a grab),
# so the whole file entered scope and the volume box's own declaration was
# reported as a copy of the capture reach. Exempting the file would drop three
# real roads; exempting the ARITHMETIC would delete the guard. ⇒ Exempt exactly
# the body of the one function that owns the volume box, by brace span, and
# assert below that the sum is actually in it — so moving the formula out
# reddens this instead of quietly widening the hole.
VOLUME_BOX_HOME = (
    "crates/ambition_entity_catalog/src/lib.rs",
    "pub fn coverage_box(&self) -> MoveCoverage {",
)


def _exempt_span(rel: str, text: str) -> range:
    """The line numbers (1-indexed) of `VOLUME_BOX_HOME`'s body, or nothing."""
    path, signature = VOLUME_BOX_HOME
    if rel != path:
        return range(0)
    lines = text.splitlines()
    start = next(
        (n for n, line in enumerate(lines, start=1) if signature in line), None
    )
    assert start is not None, (
        f"{path} no longer declares `{signature}` — the volume box's one home "
        "moved, and this guard is exempting a span that does not exist"
    )
    depth = 0
    for n in range(start, len(lines) + 1):
        depth += lines[n - 1].count("{") - lines[n - 1].count("}")
        if depth <= 0:
            return range(start, n + 1)
    raise AssertionError(f"{path}: `{signature}` has no closing brace")


def _rust_sources():
    for root in ("crates", "game"):
        for path in sorted(REPO.glob(f"{root}/**/*.rs")):
            if "/target/" in str(path):
                continue
            text = path.read_text()
            # ⛔⛔ THE POPULATION IS DECIDED BY CODE, NOT BY PROSE, and this file
            # already learned that lesson once — the COPY scan strips comments so
            # a doc comment quoting the formula cannot satisfy it. The POPULATION
            # test did not, and a single sentence naming `CaptureAttemptParams`
            # in a doc comment was enough to pull an entire crate into scope: it
            # dragged `ambition_entity_catalog` in and reported two of its
            # pre-existing volume sums, neither of which is a capture reach and
            # neither of which had changed.
            #
            # ⇒ A file can only DUPLICATE the capture reach if its CODE can name
            # it. Explaining the rule must never enlarge the rule's subject.
            code = "\n".join(line.split("//", 1)[0] for line in text.splitlines())
            if any(name in code for name in MENTIONS):
                yield path, text


def test_the_grab_reach_sum_lives_only_on_capture_attempt_params():
    assert HOME.exists(), f"the formula's home is gone: {HOME}"
    home_text = HOME.read_text()

    # ⛔ ANTI-VACUITY FIRST. If `reach_x` stopped containing the sum, every
    # assertion below would pass while the formula had moved somewhere this
    # guard is not looking.
    assert "pub fn reach_x(&self) -> f32 {" in home_text, (
        "`CaptureAttemptParams::reach_x` is gone — the guard is checking that "
        "nothing duplicates a formula that no longer has a home"
    )
    # ⛔⛔ CODE ONLY, AND THIS ARM PASSED UNDER POISON UNTIL IT DID. Blanking the
    # sum inside `reach_x` left this check green, because the DOC COMMENT above
    # `reach_x` quotes the formula while explaining it. ⇒ A rule that scans for a
    # string matches the prose discussing the string — the same trap as a
    # staleness check that flags the row correcting the error it names.
    home_code = "\n".join(
        line.split("//", 1)[0] for line in home_text.splitlines()
    )
    assert COPY.search(home_code), (
        "`smash_capture.rs` no longer computes the reach sum in CODE, so this "
        "guard is forbidding a copy of something that does not exist. (Its own "
        "doc comment still describes the formula, which is why this check reads "
        "code only.)"
    )

    strays = []
    checked = 0
    for path, text in _rust_sources():
        if path.resolve() == HOME.resolve():
            continue
        checked += 1
        rel = path.relative_to(REPO).as_posix()
        exempt = _exempt_span(rel, text)
        found_in_home = False
        for number, line in enumerate(text.splitlines(), start=1):
            code = line.split("//", 1)[0]
            if not COPY.search(code):
                continue
            if number in exempt:
                found_in_home = True
                continue
            strays.append(f"{rel}:{number}: {line.strip()}")
        # ⛔ ANTI-VACUITY ON THE EXEMPTION ITSELF. An exemption that covers
        # nothing is an exemption nobody will notice has stopped being needed.
        if exempt:
            assert found_in_home, (
                f"{rel}: the exempt span for `{VOLUME_BOX_HOME[1]}` no longer "
                "contains the sum, so this guard is holding a hole open for a "
                "formula that has moved"
            )

    # ⛔ ANTI-VACUITY ON THE POPULATION. A rename of `CaptureAttemptParams` would
    # empty this sweep and every assertion would pass on a corpus of nothing.
    assert checked >= 3, (
        f"only {checked} file(s) outside the formula's home name the capture "
        "reach at all; the three known roads are the brain's candidate, the "
        "player pose view and the actor view index, so this sweep has lost its "
        "corpus"
    )

    assert not strays, (
        "the grab reach is computed outside `CaptureAttemptParams::reach_x`:\n  "
        + "\n  ".join(strays)
        + "\n⇒ Use `reach_x()` (or `coverage()`, whose `max.0` IS the reach). "
        "Two copies of this sum answer the same question for one move — the "
        "line a player reads and the distance the AI aims at — and they drift "
        "without failing."
    )
