"""The features whose ONLY coverage is the union run must not be denied from it.

⛔⛔ MEASURED 2026-09-06: `kira`, `basic_presentation` and `falling_sand` are NOT in
`ambition_app`'s `default` feature closure (`ambition_content`'s own default is
`[]`, and `falling_sand = ["dep:bevy_falling_sand"]` is optional). So a
default-features run compiles none of their code, and the ONE thing that does is
the union run — which reaches them precisely because they are absent from
`run_tests.DENY_EXACT`.

⇒ ADDING ONE OF THEM TO `DENY_EXACT` WOULD DROP THEIR COVERAGE TO ZERO, silently
and with every lane still green. That is the failure this guards: not a broken
test, an ABSENT one.

⚠ WHY THE HAZARD IS REAL RATHER THAN THEORETICAL. `run_tests.py`'s own comment used
to justify that list by saying the suite "already exercises them via default
features" — false for exactly these three. A future reader denying `falling_sand`
for some headless reason would read that sentence, believe default features still
cover it, and be wrong. The sentence is fixed; this is the part a sentence cannot do.
"""

from __future__ import annotations

import importlib.util
import pathlib
import re
import sys
import tomllib

_ROOT = pathlib.Path(__file__).resolve().parents[2]
_SCRIPT = _ROOT / "scripts" / "run_tests.py"
_spec = importlib.util.spec_from_file_location("run_tests_for_features", _SCRIPT)
run_tests = importlib.util.module_from_spec(_spec)
assert _spec.loader is not None
sys.modules["run_tests_for_features"] = run_tests
_spec.loader.exec_module(run_tests)

# Measured, not assumed — `test_they_are_still_outside_the_default_closure` below
# re-derives this from the manifests every run.
UNION_ONLY = ("kira", "basic_presentation", "falling_sand")


def _default_closure() -> set[str]:
    """Every `ambition_app` feature reachable from `default`."""
    app = tomllib.loads((_ROOT / "game/ambition_app/Cargo.toml").read_text())["features"]
    seen: set[str] = set()
    stack = list(app.get("default", []))
    while stack:
        f = stack.pop()
        if f in seen:
            continue
        seen.add(f)
        stack.extend(app.get(f, []))
    return seen


def test_union_only_features_are_not_denied_from_the_union():
    denied = set(run_tests.DENY_EXACT) & set(UNION_ONLY)
    assert not denied, (
        f"{sorted(denied)} is denied from the union run AND outside the default "
        f"feature closure, so nothing compiles its code at all. Either restore it "
        f"to the union or say in the commit what else now covers it."
    )


def test_they_are_still_outside_the_default_closure():
    """⚠ ANTI-VACUITY, and it is the arm that keeps the test above meaningful.

    If one of these were later added to `default`, the guard above would still
    pass while guarding nothing — the feature would be covered anyway. This fails
    instead, so the list gets re-derived rather than quietly becoming decoration.
    """
    closure = _default_closure()
    # ⛔⛔ FLOOR THE CLASSIFIER, NOT ONLY THE FINDING. "Outside the default
    # closure" is vacuously true of EVERY feature when the closure is empty, so a
    # collapsed walk — a renamed `default`, a manifest shape change, a `stack`
    # that never seeds — turns this test into a guarantee it cannot make. Measured
    # 2026-09-06: emptying the walk left all three tests GREEN.
    # ⇒ `visible` is in the closure by measurement, so its absence means the walk
    # broke rather than that the manifest changed meaning.
    assert "visible" in closure and len(closure) > 5, (
        f"the default-feature walk resolved {len(closure)} feature(s) and did not "
        f"reach `visible`; it is broken, and every 'outside the closure' claim "
        f"below would be vacuously true"
    )
    covered = sorted(f for f in UNION_ONLY if f in closure)
    assert not covered, (
        f"{covered} reached `ambition_app`'s default closure, so 'union-only' is no "
        f"longer true of it. Drop it from UNION_ONLY and re-measure the rest."
    )


def test_the_comment_no_longer_claims_default_features_exercise_them():
    """⛔ The false sentence this file was written around, pinned so it stays gone.

    `run_tests.py` justified its deny-list by saying the suite "already exercises
    them via default features" — untrue for a third of the features it named.
    """
    lines = _SCRIPT.read_text(encoding="utf-8").split("\n")
    claim = "already exercises them via default features"
    # ⛔ NOT "the string is absent". The correction QUOTES the false sentence in
    # order to say it was false, so a bare substring check fails on the very fix it
    # is meant to protect — the same trap as a guard that pins the advice its rule
    # forbids. What must not come back is the CLAIM, so a line carrying the phrase
    # is only allowed when it is reporting it as history.
    asserted = [l for l in lines if claim in l and "It said" not in l]
    assert not asserted, (
        f"the false justification is being MADE again, not quoted: {asserted}. "
        f"Default features do NOT compile kira, basic_presentation or falling_sand."
    )
    # And the replacement must still name what DOES cover them.
    assert any("THE UNION RUN" in l for l in lines), "the corrected reason went missing"
