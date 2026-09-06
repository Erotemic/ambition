"""The vacuous-set-pin guard, shown finding and shown declining.

⛔ every case is planted in a synthetic workspace. This guard's whole subject is
an edge that LOOKS like it works, so "it printed OK against the real tree" is
precisely the evidence it must not rest on.

The decline cases carry as much weight as the find. The first run of this check
reported twelve rows, and eight were Bevy's own sets — `UiSystem::Layout`,
`TransformSystem::Propagate`, `InputSystem::Unify` and friends, which have no
`.in_set` here because Bevy registers their members. A guard that cries wolf
eight times out of twelve gets waived into uselessness, so the restriction to
sets DEFINED in this workspace is pinned below as its own test.
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

import check_set_pins_have_engine_members as guard  # noqa: E402

SET_DEF = (
    "#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]\n"
    "pub struct FontsLoaded;\n"
)


def _tree(tmp_path: Path, files: dict[str, str]) -> Path:
    for rel, body in files.items():
        p = tmp_path / rel
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(body, encoding="utf-8")
    return tmp_path


def test_an_engine_pin_at_an_app_filled_set_is_found(tmp_path):
    root = _tree(tmp_path, {
        "crates/ambition_render/src/lib.rs": SET_DEF,
        "crates/ambition_touch_input/src/lib.rs": "overlay.after(ambition_render::FontsLoaded);",
        "game/ambition_app/src/lib.rs": "load.in_set(ambition_render::FontsLoaded);",
    })
    found = guard.collect(root)
    assert len(found) == 1, f"the planted vacuous pin was not found: {found}"
    name, owner, pinning, registering = found[0]
    assert (name, owner) == ("FontsLoaded", "ambition_render")
    assert pinning == ["ambition_touch_input"]
    assert registering == ["ambition_app"]


def test_one_engine_member_is_enough_for_the_question_this_asks(tmp_path):
    root = _tree(tmp_path, {
        "crates/ambition_render/src/lib.rs": SET_DEF + "load.in_set(FontsLoaded);",
        "crates/ambition_touch_input/src/lib.rs": "overlay.after(ambition_render::FontsLoaded);",
        "game/ambition_app/src/lib.rs": "extra.in_set(ambition_render::FontsLoaded);",
    })
    assert guard.collect(root) == [], (
        "an engine registration means every composition gets a member, so the "
        "edge holds and a game adding more is irrelevant"
    )


def test_bevys_own_sets_are_not_our_problem(tmp_path):
    """⛔ eight of the first run's twelve rows were this."""
    root = _tree(tmp_path, {
        "crates/ambition_platformer2d_host/src/lib.rs": "\n".join([
            "a.after(bevy::ui::UiSystem::Layout);",
            "b.before(bevy::transform::TransformSystem::Propagate);",
            "c.after(leafwing_input_manager::plugin::InputManagerSystem::Unify);",
        ]),
    })
    assert guard.collect(root) == [], (
        "these are defined in dependencies and filled by them; no `.in_set` for "
        "them exists here and none should"
    )


def test_a_set_with_no_members_at_all_is_found(tmp_path):
    root = _tree(tmp_path, {
        "crates/ambition_render/src/lib.rs": SET_DEF,
        "crates/ambition_touch_input/src/lib.rs": "overlay.after(ambition_render::FontsLoaded);",
    })
    found = guard.collect(root)
    assert len(found) == 1 and found[0][3] == [], (
        "a set nothing joins is the most vacuous case of all, and the one most "
        "likely to be left behind by deleting a system"
    )


def test_a_game_pinning_an_app_filled_set_is_fine(tmp_path):
    root = _tree(tmp_path, {
        "crates/ambition_render/src/lib.rs": SET_DEF,
        "game/ambition_app/src/lib.rs": "\n".join([
            "load.in_set(ambition_render::FontsLoaded);",
            "text.after(ambition_render::FontsLoaded);",
        ]),
    })
    assert guard.collect(root) == [], (
        "the app both fills and pins it — the edge holds wherever the app runs, "
        "which is the only place either half exists"
    )


def test_the_real_tree_has_no_unwaived_rows():
    """⚠ this must run against the REAL crate list.

    An earlier draft cached the set of game crates in a module global, which the
    synthetic trees above populate first — this assertion would have classified
    real crates using a fake list and passed for the wrong reason.
    """
    unwaived = [f for f in guard.collect() if f[0] not in guard.WAIVERS]
    assert not unwaived, (
        "an engine pin points at a set only a game fills: "
        f"{[f[0] for f in unwaived]}"
    )
