"""`docs/planning/demos/moveset-reviews.md` agrees with the code about whose moves these are.

Jon, 2026-09-05: *"We should probably keep track of moves I've explicitly
authored, or decided I like, because everything else can be polished or modified
to an LLM's heart's content."* ⇒ Two lists, and this guard holds BOTH directions.

⛔⛔ THE SECOND DIRECTION IS THE ONE THAT CAN HURT SOMEBODY. A missing row makes
the page incomplete, which is annoying. A fighter wrongly listed as FREE TO
CHANGE tells a polish pass that one of Jon's own moves is nobody's — the single
failure the page exists to prevent, and the one nobody would notice until the
move had already been rewritten.

⭐ COMPARED AS SETS AGAINST THE CODE, not scraped from the prose. A guard reading
fighter names out of a table would pass on a name mentioned in passing and fail
on one spelled differently; the manifest states each list exactly once, as slugs.
Every moveset file must appear in exactly one list, which is what forces a NEW
fighter to be classified rather than defaulting to "free".

⚠ THE TEST FOR "HAS MAINTAINER INPUT" IS THE MAINTAINER'S NAME IN THE FILE. That
is the convention this repo already follows — his words are quoted verbatim
beside the move they govern — and it is deliberately crude: it cannot tell an
authored RULE from a passing mention, and it errs toward calling a file reviewed.
Erring that way is correct here, because the expensive mistake is the other one.
"""

import pathlib
import re

REPO = pathlib.Path(__file__).resolve().parents[2]
PAGE = REPO / "docs/planning/demos/moveset-reviews.md"
MOVESETS = REPO / "game/ambition_content/src"

MAINTAINER = re.compile(r"\bJon\b")


def _manifest(text, key):
    match = re.search(rf"<!--\s*{key}:\s*(.*?)\s*-->", text)
    assert match, f"the reviews page carries no `{key}` manifest comment"
    return {slug.strip() for slug in match.group(1).split(",") if slug.strip()}


def _fighters():
    """slug -> whether its moveset file quotes the maintainer."""
    found = {}
    for path in sorted(MOVESETS.glob("*_moveset.rs")):
        slug = path.name[: -len("_moveset.rs")]
        found[slug] = bool(MAINTAINER.search(path.read_text()))
    return found


def test_the_reviews_page_agrees_with_the_code_about_whose_moves_these_are():
    assert PAGE.exists(), f"the reviews page is gone: {PAGE}"
    text = PAGE.read_text()
    reviewed = _manifest(text, "reviewed-fighters")
    free = _manifest(text, "free-fighters")

    fighters = _fighters()
    assert fighters, "no `*_moveset.rs` files found — the guard is scanning nothing"

    overlap = reviewed & free
    assert not overlap, (
        "these fighters are listed as BOTH reviewed and free to change, so the "
        f"page contradicts itself: {sorted(overlap)}"
    )

    in_code = set(fighters)
    listed = reviewed | free
    assert listed == in_code, (
        "the reviews page and the roster disagree about which fighters exist.\n"
        f"  on the page, not in code: {sorted(listed - in_code)}\n"
        f"  in code, not on the page: {sorted(in_code - listed)}\n"
        "⇒ A new fighter must be classified as reviewed or free — defaulting to "
        "neither is how a maintainer-authored move ends up looking like nobody's."
    )

    # ⛔ THE DIRECTION THAT MATTERS: nothing Jon has spoken about may sit in the
    # free-to-change list.
    wrongly_free = sorted(slug for slug in free if fighters[slug])
    assert not wrongly_free, (
        f"these fighters are listed as FREE TO CHANGE but their moveset files "
        f"quote the maintainer: {wrongly_free}.\n"
        "⇒ A polish pass reading that list would treat one of Jon's own moves as "
        "nobody's and rewrite it. Move them to the reviewed list and record what "
        "he asked for."
    )

    # And the other direction, which keeps the page honest rather than safe.
    wrongly_reviewed = sorted(slug for slug in reviewed if not fighters[slug])
    assert not wrongly_reviewed, (
        f"these fighters are listed as REVIEWED but their moveset files carry no "
        f"maintainer input at all: {wrongly_reviewed}.\n"
        "⇒ Attributing an agent's design to Jon weakens the claim the roster "
        "exists to make, exactly as much as overwriting one of his does."
    )

    # ⛔ ANTI-VACUITY. Two empty manifests satisfy every assertion above, and so
    # does a roster nobody has spoken about.
    assert len(reviewed) >= 5 and len(free) >= 1, (
        f"the manifest lists {len(reviewed)} reviewed and {len(free)} free "
        "fighters, which is too few for this guard to be checking anything"
    )
