"""The pinned-renderer check must DISCRIMINATE, not merely run.

⛔⛔ THIS TESTS THE INSTRUMENT, NOT THE PIN, and deliberately so. The pin does
not satisfy the requirement today — that is a maintainer decision, recorded in
`docs/planning/yardrat-open-measurements.md`, and asserting it here would be a
red nobody in CI can clear.

What CAN be asserted, and is the thing that would silently rot, is that the
checker still tells a refusing tree from a non-refusing one. A check whose
healthy answer is "no" and whose broken answer is also "no" says nothing, and
this one's subject lives in a submodule where every ordinary way of looking
(running the suite, grepping the files) reads the CHECKED-OUT tree instead.

⇒ So: the branch that implements the refusal must read as refusing, the pinned
commit must read as not refusing, and both answers must come from the same
function the script calls.
"""

from __future__ import annotations

import importlib.util
import subprocess
import sys
from pathlib import Path

import pytest

REPO = Path(
    subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True
    ).stdout.strip()
)
SPEC = importlib.util.spec_from_file_location(
    "check_pinned_music_renderer_refuses_gm",
    REPO / "scripts" / "check_pinned_music_renderer_refuses_gm.py",
)
mod = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = mod
SPEC.loader.exec_module(mod)

#: The branch that implements the refusal. Named rather than pinned to a SHA:
#: the branch gains commits, and a SHA here would make this test assert about a
#: tree nobody is on — which is the exact defect the whole check exists for.
REFUSING_BRANCH = "origin/agent/sfizz-source-fallback-and-cue-fanout"


def _submodule_has(ref: str) -> bool:
    return subprocess.run(
        ["git", "rev-parse", "--verify", "--quiet", f"{ref}^{{commit}}"],
        cwd=REPO / mod.SUBMODULE, capture_output=True, text=True,
    ).returncode == 0


def test_the_checker_reads_the_pin_at_all():
    """A population floor: no pinned commit means the check measures nothing."""
    sha = mod.pinned_sha()
    assert sha and len(sha) == 40, f"could not read the pinned commit: {sha!r}"
    assert mod.file_at(sha, mod.REQUIRED_IN) is not None, (
        f"{mod.REQUIRED_IN} does not exist in the pinned tree at all — the check "
        "would report 'no refusal' for a path typo just as loudly"
    )


def test_it_tells_a_refusing_tree_from_a_non_refusing_one():
    """⭐ THE DISCRIMINATION ARM. Both answers, from the same function."""
    if not _submodule_has(REFUSING_BRANCH):
        pytest.skip(f"{REFUSING_BRANCH} is not fetched in this checkout")

    refusing, why_yes = mod.refusal_present(REFUSING_BRANCH)
    assert refusing, (
        f"the branch that implements the refusal reads as NOT refusing: {why_yes}"
    )

    pinned, why_no = mod.refusal_present(mod.pinned_sha())
    # ⚠ NOT asserted as False. The day the pointer moves this becomes True, and a
    # test that hard-codes today's answer would fail on the fix -- which is the
    # one moment nobody wants a red. What must hold is that the two are answered
    # INDEPENDENTLY and the reason is legible either way.
    assert isinstance(pinned, bool) and why_no, "the pinned answer carries no reason"
    if not pinned:
        assert mod.REQUIRED_TOKEN in why_no or "does not exist" in why_no, (
            f"the refusal's absence is reported without naming why: {why_no}"
        )


def test_the_token_it_looks_for_is_not_vacuous():
    """The token must be absent from a tree that genuinely lacks the refusal.

    ⛔ If `REQUIRED_TOKEN` appeared in both trees the check would pass forever.
    It is the ENV OVERRIDE rather than the words "General MIDI" precisely because
    the prose appears in comments on BOTH sides — and a comment is what the
    pinned tree turned out to have.
    """
    body = mod.file_at(mod.pinned_sha(), mod.REQUIRED_IN)
    assert body, "no pinned cli.py to check"
    if mod.REQUIRED_TOKEN not in body:
        assert "General" in body or "midi" in body.lower(), (
            "the pinned tree mentions the CONCEPT nowhere either, so this check "
            "may be looking at the wrong file"
        )
