"""A planning heading is a public interface, and renaming one used to break
inbound links in silence.

`check_planning_citations.py` asks two questions well — does this path name a
file, does this name have a definition — and until 2026-09-16 it asked nothing
at all about a link's `#fragment`. Measured that day: renaming one heading in
`awaiting-maintainer-decision.md` broke five inbound links while the checker
printed *"all resolved"*, and a sweep of the corpus found two links already
broken by earlier renames (`ROLLBACK-BAG-DESYNC`, `A10`).

⛔ THE SUBJECT HERE IS THE SLUG FUNCTION, because it is a REIMPLEMENTATION of
GitHub's algorithm and nothing else in the repo can check it. A slug rule that
is wrong in the permissive direction reports nothing and restores the silence
this guard exists to end, which is indistinguishable from a clean corpus — so
the anchors below are pinned as known answers taken from live headings.
"""

from __future__ import annotations

import importlib.util
import subprocess
import sys
from pathlib import Path

REPO = Path(
    subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True
    ).stdout.strip()
)
SCRIPT = REPO / "scripts/check_planning_citations.py"


def load():
    spec = importlib.util.spec_from_file_location("citations", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    sys.modules["citations"] = module
    spec.loader.exec_module(module)
    return module


# Heading text -> the anchor GitHub actually serves for it. Every row is a live
# heading whose inbound links resolve in the browser today.
KNOWN = [
    (
        "## Q135 — should GGRS start before the durable restore has finished?",
        "q135--should-ggrs-start-before-the-durable-restore-has-finished",
    ),
    (
        "### ROLLBACK-BAG-DESYNC — `AmbitionGameSave` disagrees with its own rollback replay",
        "rollback-bag-desync--ambitiongamesave-disagrees-with-its-own-rollback-replay",
    ),
    # ⚠ THE EMOJI CASE, AND IT IS THE ONE A HAND-WRITTEN LINK GETS WRONG. The
    # ✅ is DROPPED rather than transliterated or percent-encoded, and the two
    # spaces that surrounded it survive -- so the anchor carries THREE hyphens
    # where the heading shows "— ✅ DONE". A link written by URL-encoding the
    # emoji (`%E2%9C%85`) does not resolve, which is exactly how `status.md`'s
    # row broke.
    (
        "### A10 — candidate world / last-good-world publication — ✅ DONE, DEMOLITION CLOSED 2026-09-16",
        "a10--candidate-world--last-good-world-publication---done-demolition-closed-2026-09-16",
    ),
]


def test_the_slug_rule_agrees_with_github_on_live_headings():
    slugs = load().heading_slugs
    for heading, expected in KNOWN:
        assert slugs(heading) == {expected}, (
            f"the slug rule no longer mints the anchor that {heading.split(' ')[1]!r}'s "
            f"inbound links use. Changing this rule silently repoints every "
            f"anchor in docs/planning"
        )


def test_a_renamed_heading_is_reported_rather_than_resolved():
    """The whole point: the checker must FAIL on a fragment naming no heading."""
    module = load()
    doc = REPO / "docs/planning/awaiting-maintainer-decision.md"
    rows, examined = module.anchor_findings([doc])
    assert examined > 0, (
        "no anchor link was examined in a file known to contain them, so this "
        "guard would pass over a corpus it never read -- the failure mode the "
        "checker's own 'all resolved' already demonstrated once"
    )
    assert not rows, f"live anchors stopped resolving: {rows}"


def test_the_corpus_has_no_broken_anchors():
    module = load()
    docs = sorted((REPO / "docs/planning").rglob("*.md"))
    rows, examined = module.anchor_findings(docs)
    assert examined >= 60, (
        f"only {examined} anchor links were examined across {len(docs)} planning "
        "files. The sweep that motivated this guard found 70; a collapse here "
        "means the link pattern stopped matching, not that the corpus got tidy"
    )
    assert not rows, "\n".join(f"{r[0]}:{r[1]} {r[3]}" for r in rows)
