"""Arms for `check_raw_key_reads_do_not_collide_with_presets.py`.

The guard's subject is a defect that SHIPPED: the player-clone hotkey read
`KeyCode::KeyK` raw while two presets bound `K` to a gameplay action. These arms
exist so the guard cannot quietly stop being able to see that shape.
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

import check_raw_key_reads_do_not_collide_with_presets as guard  # noqa: E402


def test_the_binding_filter_is_not_the_label_table():
    """⛔⛤ THE MISCOUNT THIS SCRIPT WAS WRITTEN AROUND, kept as an arm.

    `presets.rs` ends with a `KeyCode::KeyM => "M"` match that turns a keycode
    into a label for the rebinding UI. A grep for `KeyCode::(\\w+)` sweeps all
    36 of those arms in beside the real bindings and reports 39 bound keys where
    the answer is 35 — and that inflation alone produced a false finding, on
    `KeyCode::KeyM` in the map menu, which no preset binds at all.
    """
    bound = guard.bound_keys()
    label_only = guard.label_only_keys()
    assert "KeyM" in label_only, (
        "`KeyM` is no longer label-only. If a preset now BINDS it, the map menu's "
        f"raw `KeyCode::KeyM` read is a real collision. bound={sorted(bound)}"
    )
    assert not (bound & label_only), "a key cannot be both bound and label-only"
    assert 20 <= len(bound) <= 60, f"{len(bound)} bound keys is outside the sane band"


def test_the_shipped_presets_still_bind_the_key_that_killed_the_clone():
    """⭐ THE ORIGINAL SPECIMEN, asserted against the live preset table.

    The clone read `K` raw; `wasd_jkl()` binds it to `burst` and `wasd_uipo()`
    to `utility`. If `K` ever stops being bound, this guard stops being able to
    demonstrate the defect it was built for, and the arm below that re-creates
    the shape stops proving anything.
    """
    assert "KeyK" in guard.bound_keys(), (
        "no shipped preset binds `K` any more — re-pick the specimen this guard's "
        "docstring uses, or the story it tells is about a tree that no longer exists"
    )


def test_every_collision_is_read_and_every_reading_has_a_subject():
    """⛔ BOTH DIRECTIONS. An unread collision is the defect; a reading whose
    subject is gone pre-approves whatever takes its place — the same rule the
    rollback waiver table had to learn.
    """
    found = set(guard.collisions())
    adjudicated = set(guard.ADJUDICATED)
    assert not (found - adjudicated), f"unread collision(s): {sorted(found - adjudicated)}"
    assert not (adjudicated - found), f"reading(s) with no subject: {sorted(adjudicated - found)}"


def test_the_live_collision_is_named_rather_than_excused():
    """⚠ ONE OF THE FOUR READINGS IS A FINDING, and the guard says so on stdout.

    `handle_map_menu_hotkeys` reads raw `KeyCode::KeyN` to toggle the minimap and
    runs whenever a session world exists; BOTH shipped presets bind `N` to
    `taunt`. It is milder than the clone — no rollback state, no simulation
    write — and it is the same mechanism, so it is recorded with its reading
    instead of being waived.
    """
    reading = guard.ADJUDICATED[("KeyN", "crates/ambition_menu/src/map/input.rs")]
    assert reading.startswith("⛔"), (
        "the `KeyN` row stopped being marked LIVE. If the toggle was routed "
        "through a bound action, delete the row (the stale-row arm will ask for "
        "it); do not soften the reading in place"
    )


def test_the_raw_read_pattern_sees_the_spellings_the_tree_uses():
    """⚠ ANTI-VACUITY ON THE READ SIDE, which is the half a floor cannot hold.

    A pattern that matched nothing would report zero collisions and pass. The
    tree spells the read bare (`keys.just_pressed(KeyCode::X)`) and qualified
    (`input.just_pressed(KeyCode::X)`), and both must land.
    """
    reads = guard.raw_reads()
    assert len(reads) >= guard.FLOORS["raw reads"], f"only {len(reads)} raw read(s) found"
    files = {file for _key, file in reads}
    assert len(files) >= 3, f"raw reads found in only {len(files)} file(s): {sorted(files)}"
    assert all(not f.startswith(guard.OWNER) for f in files), (
        f"`{guard.OWNER}` leaked into the population; it OWNS key meaning"
    )
