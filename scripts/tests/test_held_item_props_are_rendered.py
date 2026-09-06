"""The held-item art guard has to FIRE, and has to keep its population honest.

`check_held_item_props_are_rendered.py` is green against the live tree, so a
test that only ran it would prove nothing — the recurring lesson on this repo
is that a check green at minute zero guards nothing. These tests do what the
live run cannot:

* run it against the real tree, so a genuine divergence between the game's
  `HeldItemArtEntry` registrations and the renderer's `HELD_ITEM_ICON_SPECS`
  reddens a lane instead of sitting in a script nobody invokes;
* SKIP rather than pass when the renderer submodule is absent, because exit 3
  means the art went UNEXAMINED and reporting that as success is how a check
  ends up off on every machine without a tool venv;
* pin the ASYMMETRY (a drawing the game has not wired is a note, not a
  failure — the submodule is separately pinned and art legitimately lands
  first); and
* pin the FAMILY RESTRICTION, which is the one a later reader is most likely
  to "tighten" back into a bug. The first draft failed any self-named art path
  and immediately fired on `axe` and `javelin`, both correct and both drawn by
  a different renderer target. `test_art_named_after_an_unowned_item_is_not_a
  _failure` is that measurement, kept executable.
"""

from __future__ import annotations

import importlib.util
import subprocess
import sys
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parents[2]
CHECK = REPO / "scripts" / "check_held_item_props_are_rendered.py"


def _module():
    spec = importlib.util.spec_from_file_location("held_item_check", CHECK)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    if module.drawn_art() is None:
        pytest.skip("renderer submodule not importable; held-item art unexamined")
    return module


def test_every_rendered_held_item_prop_is_the_path_the_game_points_at() -> None:
    result = subprocess.run(
        [sys.executable, str(CHECK)], capture_output=True, text=True, cwd=REPO
    )
    if result.returncode == 3:
        pytest.skip("renderer submodule not importable; held-item art unexamined")
    assert result.returncode == 0, result.stdout + result.stderr


def test_the_check_fails_when_the_game_wears_art_the_renderer_did_not_draw() -> None:
    """The rename, in either repo, that this guard exists for."""
    module = _module()
    real = module.registered_art
    key = next(iter(module.drawn_art()))
    try:
        module.registered_art = lambda: {**real(), key: "sprites/props/borrowed.png"}
        assert module.main() == 1
    finally:
        module.registered_art = real


def test_the_check_fails_when_art_named_after_an_owned_item_is_undrawn() -> None:
    """A spec deleted while the registration stays: nothing writes that file."""
    module = _module()
    real_drawn, real_registered = module.drawn_art, module.registered_art
    family = next(iter(real_drawn())).split("_", 1)[0]
    orphan = f"{family}_vanished"
    try:
        module.registered_art = lambda: {
            **real_registered(),
            orphan: f"sprites/props/{orphan}.png",
        }
        assert module.main() == 1
    finally:
        module.drawn_art, module.registered_art = real_drawn, real_registered


def test_art_named_after_an_unowned_item_is_not_a_failure() -> None:
    """⛔ THE `axe`/`javelin` MEASUREMENT, kept executable.

    A prop named after itself but outside every family the spec list owns is
    drawn by a DIFFERENT renderer target and is none of this check's business.
    Failing it is the over-broad population the first draft shipped with.
    """
    module = _module()
    real = module.registered_art
    try:
        module.registered_art = lambda: {
            **real(),
            "halberd": "sprites/props/halberd.png",
        }
        assert module.main() == 0
    finally:
        module.registered_art = real


def test_a_drawing_the_game_has_not_wired_is_a_note_not_a_failure() -> None:
    """The loose direction, which keeps the normal authoring order legal.

    ⛔⛔ THE FIXTURE USED TO ADD `polygon_unwired`, AND THAT WAS NOT ISOLATED.
    Adding any `polygon_*` drawing puts `polygon` into `families`, which
    immediately makes the three REAL registered items (`polygon_bomb`,
    `polygon_mine`, `polygon_ponytail`) orphans and turns this test red for a
    reason that has nothing to do with its subject. It went unnoticed only
    because the whole file was SKIPPING. A fixture that names a live family is
    testing the tree, not the rule.
    """
    module = _module()
    real = module.drawn_art
    try:
        module.drawn_art = lambda: {
            **real(),
            "zzfixture_unwired": "sprites/props/zzfixture_unwired.png",
        }
        assert module.main() == 0
    finally:
        module.drawn_art = real


def test_a_self_named_orphan_fails_only_once_its_family_is_covered() -> None:
    """⭐ THE STRUCTURAL BLIND SPOT, pinned on both sides.

    The orphan rule is scoped to families derived from the DRAWN keys, so the
    FIRST id of a family this list does not cover can never trip it. That is why
    `polygon_bomb` and friends draw an ERROR every spawn while this check reports
    ok — and why the same ids go red the moment ANY `polygon_*` drawing lands.
    """
    module = _module()
    real_drawn, real_registered = module.drawn_art, module.registered_art
    # ⛔ A MATCHING PAIR UNDERNEATH THE SUBJECT, because the check now refuses an
    # EMPTY INTERSECTION — it printed `ok: 0 compared` for a commit, so comparing
    # nothing is a failure rather than a cheerful zero. Without this pair the
    # fixture would exercise the intersection floor instead of the family rule,
    # and the test would be green for the wrong reason in the "abstains" arm.
    agreed = {"shared_thing": "sprites/props/shared_thing.png"}
    try:
        module.registered_art = lambda: {
            **agreed,
            "fam_thing": "sprites/props/fam_thing.png",
        }
        # No `fam` spec: the family is uncovered, so the rule abstains.
        module.drawn_art = lambda: {
            **agreed,
            "other_thing": "sprites/props/other_thing.png",
        }
        assert module.main() == 0, "an uncovered family must not fail"
        # One `fam` drawing arrives and the SAME registration is now an orphan.
        module.drawn_art = lambda: {
            **agreed,
            "fam_other": "sprites/props/fam_other.png",
        }
        assert module.main() == 1, "a covered family must fail the self-named orphan"
    finally:
        module.drawn_art, module.registered_art = real_drawn, real_registered


def test_an_empty_spec_list_is_a_failure_not_a_vacuous_pass() -> None:
    module = _module()
    real = module.drawn_art
    try:
        module.drawn_art = lambda: {}
        assert module.main() == 1
    finally:
        module.drawn_art = real


def test_an_empty_intersection_is_a_failure_not_a_cheerful_zero() -> None:
    """⛔⛔ THE STATE THIS CHECK WAS IN FOR THE LENGTH OF ONE COMMIT.

    Repointed at a spec list whose ids the game does not register, it printed
    `ok: 0 held-item prop(s) compared` and exited 0. Both halves were non-empty —
    7 drawn, 8 registered — so every anti-vacuity arm was satisfied; what was
    empty was the OVERLAP, which is the only thing this file verifies.

    ⇒ A guard whose population is an INTERSECTION has to floor the intersection,
    not its operands. That is the general form, and it is why this test exists
    rather than a comment saying to be careful.
    """
    module = _module()
    real = module.drawn_art
    try:
        module.drawn_art = lambda: {
            "an_id_the_game_does_not_register": "sprites/props/nobody.png"
        }
        assert module.main() == 1
    finally:
        module.drawn_art = real


def test_a_stale_checkout_is_cannot_check_but_real_drift_still_fails() -> None:
    """⛔⛔ THE TWO CASES LOOK IDENTICAL FROM THE IMPORT and want opposite answers.

    An `ImportError` on a NAME means either the pinned renderer really dropped it
    (DRIFT — the thing this file exists to catch) or this working copy is behind
    the pin (STALE — the ordinary state in a repo where `git submodule update` is
    not run casually). Asking the PIN tells them apart.

    ⚠ I got this wrong in both directions on 2026-09-06: first by reading a stale
    checkout as drift and repointing the check at the wrong list, then by making
    drift a hard failure that also fired on staleness — eight red tests on the
    normal state of the repo.
    """
    spec = importlib.util.spec_from_file_location("held_item_check_probe", CHECK)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)

    # Not an import-NAME error at all: not evidence of staleness.
    assert module._pin_still_exports("some other error") is False

    # The pin has lost the name too ⇒ real drift ⇒ hard failure, never a skip.
    real = module._pin_still_exports
    try:
        module._pin_still_exports = lambda _message: False
        try:
            module.drawn_art()
        except SystemExit as exit_:
            assert "does not export what this check reads" in str(exit_)
        except ImportError:
            pytest.skip("the renderer imports cleanly here; drift arm unexercised")
        else:
            # Import succeeded, so this machine cannot exercise the arm.
            pytest.skip("the renderer imports cleanly here; drift arm unexercised")
    finally:
        module._pin_still_exports = real
