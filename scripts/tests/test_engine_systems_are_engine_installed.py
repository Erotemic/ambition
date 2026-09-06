"""Poison tests for the engine-installation boundary checker.

Fixtures prove the checker fires when engine-owned systems are registered only
from app composition while remaining quiet on prose that merely names the same
symbols."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from check_engine_systems_are_engine_installed import (  # noqa: E402
    REPO,
    UNCLAIMED_BUDGET,
    WAIVERS,
    add_systems_bodies,
    app_only_systems,
    registered_engine_systems,
    strip_comments,
    strip_run_conditions,
)

APP_ROOTS = ["game"]


def test_a_registration_inside_add_systems_is_seen():
    source = """
        app.add_systems(Update, ambition_platformer2d::render::rendering::sync_parallax_layers);
    """
    bodies = add_systems_bodies(source)
    assert len(bodies) == 1
    assert "sync_parallax_layers" in bodies[0]


def test_a_nested_call_does_not_end_the_body_early():
    """Paren balance, not the first `)`. Every real registration in this repo has
    a `.after(..)` or a `.run_if(..)` on it, so a naive scan would truncate the
    body before the system it is looking for."""
    source = """
        app.add_systems(
            Update,
            (a::b::first_system, a::b::second_system)
                .chain()
                .after(a::b::something_else)
                .run_if(x::y::in_mode(Mode::Play)),
        );
    """
    body = add_systems_bodies(source)[0]
    assert "first_system" in body
    assert "second_system" in body


def test_a_run_condition_is_not_a_system():
    body = "a::b::real_system.run_if(x::y::in_mode(Mode::Play))"
    stripped = strip_run_conditions(body)
    assert "real_system" in stripped
    assert "in_mode" not in stripped


def test_an_ordering_edge_is_not_a_registration():
    """The expensive kind of wrong. `.after(morph_ball::sync_morph_ball_visual)`
    made this script report two ability visuals as app-only — and
    `ambition_render` registers both; the app is only ordering its dev sprite
    overrides against them. A guard that sends you to fix something already
    correct burns more than a guard that says nothing."""
    body = (
        "a::b::real_system"
        ".after(render::rendering::morph_ball::sync_morph_ball_visual)"
        ".before(render::rendering::bubble_shield::sync_bubble_shield_visual)"
    )
    stripped = strip_run_conditions(body)
    assert "real_system" in stripped
    assert "sync_morph_ball_visual" not in stripped
    assert "sync_bubble_shield_visual" not in stripped


def test_a_doc_comment_naming_a_system_is_not_a_registration():
    """The recurrence this repo has already paid for three times on its absence
    checks: documenting a thing must never look like doing it. The comment below
    is exactly what the fix for S12 left behind in `plugins.rs`."""
    source = """
        // ⚠ `sync_parallax_layers` left this list: SessionRoomVisualsPlugin
        // registers it for every composition now.
        app.add_systems(Update, ambition_platformer2d::render::rendering::sync_health_overlays);
    """
    body = add_systems_bodies(strip_comments(source))[0]
    assert "sync_health_overlays" in body
    assert "sync_parallax_layers" not in body


def test_the_live_tree_matches_the_ratchet():
    """The budget is the count, exactly. Over it is a new app-only system; under
    it is a fix that did not tighten the ratchet, and a budget nobody tightens
    becomes a permanent allowance."""
    offenders = app_only_systems(REPO)
    assert len(offenders) == UNCLAIMED_BUDGET, (
        f"unclaimed app-only presentation systems: {sorted(offenders)}. "
        f"The ratchet allows {UNCLAIMED_BUDGET}."
    )


def test_every_waiver_still_names_something_the_app_registers():
    """A waiver whose system has since moved into an engine plugin is dead
    weight that reads as a live decision. Deleting it is part of the move."""
    registered = registered_engine_systems(REPO, APP_ROOTS)
    stale = sorted(name for name in WAIVERS if name not in registered)
    assert not stale, (
        f"these waivers name systems no game composition registers any more: "
        f"{stale}. Delete them — a stale waiver reads as a decision somebody made."
    )


def test_every_waiver_has_a_reason():
    empty = sorted(name for name, why in WAIVERS.items() if not why.strip())
    assert not empty, f"waivers with no reason: {empty}"


def test_open_rows_and_waivers_never_overlap():
    """The two registries answer OPPOSITE questions.

    A waiver says the ENGINE should not own it; an open row says the engine
    should and names the blocker. A name in both means nobody has decided, while
    the check reports a decision either way.
    """
    import check_engine_systems_are_engine_installed as guard

    both = set(guard.WAIVERS) & set(guard.OPEN_ROWS)
    assert not both, f"decided and undecided at once: {sorted(both)}"


def test_every_open_row_names_a_BLOCKER_not_an_intention():
    """⛔ this registry is the one most able to rot into 'we will get to it'.

    A waiver at least states a decision. An open row states that work remains,
    which is what every unfixed thing says — so the entry has to carry the
    specific question it waits on, or it is a TODO with a ratchet's authority.
    """
    import check_engine_systems_are_engine_installed as guard

    thin = [
        name
        for name, why in guard.OPEN_ROWS.items()
        if "⛔" not in why or len(why) < 120
    ]
    assert not thin, (
        "these open rows do not name a blocker: "
        f"{thin}. Say what the move waits ON — a schedule question, an ownership "
        "question, a missing seam — or fix it and delete the entry."
    )
