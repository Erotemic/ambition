"""Q136's population instrument, and the false-positive class that hid its shape.

The first run reported eight crossings and five of them were the SAME
misreading: a session-edge reset holds thirty resources through a `SystemParam`
bundle and writes every one of them to `T::default()` inside a helper, so the
scan saw a mutable holder with no visible write and called it a producer.

⭐ A WRITE OF `T::default()` IS THE ABSENCE OF AN INTENT. Each arm below plants
one side of that distinction, because the cheap version of this script will be
re-derived the cheap way otherwise.
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

import check_host_produced_sim_consumed_requests as guard  # noqa: E402

ROLLBACK_REGISTRY_REL = "crates/ambition_platformer2d_runtime/src/rollback/mod.rs"


def _tree(tmp_path: Path, files: dict[str, str]) -> Path:
    registry = tmp_path / ROLLBACK_REGISTRY_REL
    registry.parent.mkdir(parents=True, exist_ok=True)
    registry.write_text(
        'fn r(app: &mut App) { app.rollback_resource_canonical::<Flag>(E, "flag"); }',
        encoding="utf-8",
    )
    for rel, body in files.items():
        p = tmp_path / rel
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(body, encoding="utf-8")
    return tmp_path


_CROSSING = "\n".join([
    "pub fn raise_it(mut req: ResMut<Flag>) { req.0 = true; }",
    "pub fn spend_it(mut req: ResMut<Flag>) {",
    "    if !req.0 { return; }",
    "    req.0 = false;",
    "}",
    "fn build(app: &mut App) {",
    "    let sim = app.sim_schedule();",
    "    app.add_systems(Update, raise_it);",
    "    app.add_systems(sim, spend_it);",
    "}",
])


def test_a_host_raise_spent_in_the_sim_is_a_crossing(tmp_path):
    root = _tree(tmp_path, {"crates/ambition_x/src/lib.rs": _CROSSING})
    assert guard.crossings(root) == {"Flag": (["raise_it"], ["spend_it"])}


def test_a_raise_inside_the_sim_is_not_a_crossing(tmp_path):
    """The control. If both halves are in the rewind, the replay re-produces it."""
    root = _tree(tmp_path, {
        "crates/ambition_x/src/lib.rs": _CROSSING.replace(
            "app.add_systems(Update, raise_it);", "app.add_systems(sim, raise_it);"
        ),
    })
    assert guard.crossings(root) == {}


def test_a_qualified_host_label_is_still_the_host_side(tmp_path):
    """Inherited from the sibling guard's sixth spelling, and it matters here.

    `request_player_clone_on_key` is registered into `bevy::app::Update`; while
    the label test compared bare names this crossing did not exist.
    """
    root = _tree(tmp_path, {
        "crates/ambition_x/src/lib.rs": _CROSSING.replace(
            "app.add_systems(Update, raise_it);",
            "app.add_systems(bevy::app::Update, raise_it);",
        ),
    })
    assert guard.crossings(root) == {"Flag": (["raise_it"], ["spend_it"])}


# ── the false-positive class: a clear is not a production ───────────────────


def test_writing_a_default_is_clearing_not_producing(tmp_path):
    root = _tree(tmp_path, {
        "crates/ambition_x/src/lib.rs": _CROSSING.replace(
            "pub fn raise_it(mut req: ResMut<Flag>) { req.0 = true; }",
            "pub fn raise_it(mut req: ResMut<Flag>) { *req = Flag::default(); }",
        ),
    })
    assert guard.crossings(root) == {}, (
        "a session-edge reset writing a default is emptying the slot; an empty "
        "slot cannot be an intent a rewind loses"
    )


def test_a_clear_hidden_in_a_helper_is_still_a_clear(tmp_path):
    """⛔⛤ THE ACTUAL DEFECT, 2026-09-18: five of eight reported crossings.

    `reset_session_scoped_resources_on_activation`'s own body writes NOTHING —
    it calls `reset(resources)` and every `*field = Type::default()` lives in
    that helper. Reading the system body alone found a mutable holder with no
    visible write and, with nothing to demote it, called it a producer.
    """
    root = _tree(tmp_path, {
        "crates/ambition_x/src/lib.rs": _CROSSING.replace(
            "pub fn raise_it(mut req: ResMut<Flag>) { req.0 = true; }",
            "\n".join([
                "pub fn raise_it(req: ResMut<Flag>) { clear_them(req); }",
                "fn clear_them(mut req: ResMut<Flag>) { *req = Flag::default(); }",
            ]),
        ),
    })
    assert guard.crossings(root) == {}


def test_a_write_through_a_binding_that_never_names_the_type_is_a_production(tmp_path):
    """⚠ THE LOAD-BEARING HALF OF `_only_clears`, and the easy way to break it.

    A real producer writes `request.0 = true` and never mentions
    `SpawnPlayerCloneRequest` at all. So "no assignment names the type" must
    mean CANNOT DEMOTE, not "clears" — a `_only_clears` that returned True on an
    empty match would silently empty this script's whole population.
    """
    root = _tree(tmp_path, {"crates/ambition_x/src/lib.rs": _CROSSING})
    body = "pub fn raise_it(mut req: ResMut<Flag>) { req.0 = true; }"
    assert not guard._only_clears(body, "Flag")
    assert guard.crossings(root) == {"Flag": (["raise_it"], ["spend_it"])}


def test_a_system_that_both_clears_and_raises_is_a_producer(tmp_path):
    """One default among real writes does not demote it."""
    root = _tree(tmp_path, {
        "crates/ambition_x/src/lib.rs": _CROSSING.replace(
            "pub fn raise_it(mut req: ResMut<Flag>) { req.0 = true; }",
            "pub fn raise_it(mut req: ResMut<Flag>) {",
            1,
        ).replace(
            "pub fn spend_it",
            "\n".join([
                "    *req = Flag::default();",
                "    *req = Flag::raised();",
                "}",
                "pub fn spend_it",
            ]),
            1,
        ),
    })
    assert guard.crossings(root) == {"Flag": (["raise_it"], ["spend_it"])}


def test_the_bundle_half_is_what_found_the_prompting_defect(tmp_path):
    """`NewGameResetRequested` is held as a `#[derive(SystemParam)]` FIELD.

    A scan of `pub fn` signatures alone reported ZERO host producers for it, so
    the script's own subject passed.
    """
    root = _tree(tmp_path, {
        "crates/ambition_x/src/lib.rs": "\n".join([
            "#[derive(SystemParam)]",
            "pub struct MenuState<'w> { flag: ResMut<'w, Flag> }",
            "pub fn raise_it(mut menu: MenuState) { menu.flag.0 = true; }",
            "pub fn spend_it(mut req: ResMut<Flag>) {",
            "    if !req.0 { return; }",
            "    req.0 = false;",
            "}",
            "fn build(app: &mut App) {",
            "    let sim = app.sim_schedule();",
            "    app.add_systems(Update, raise_it);",
            "    app.add_systems(sim, spend_it);",
            "}",
        ]),
    })
    assert guard.crossings(root) == {"Flag": (["raise_it"], ["spend_it"])}


def test_a_request_SPENT_through_a_bundle_field_is_found(tmp_path):
    """⛔⛤ A REVIEW REPRODUCED THIS AS A FALSE NEGATIVE ON 2026-09-18.

    The bundle half was applied to mutable HOLDERS only; spend detection looked
    at direct `ResMut` parameters, so a consumer spending through `p.req` was
    invisible and the script reported no crossing at all. Both sides now work
    from the same `(access path, type)` pairs.
    """
    root = _tree(tmp_path, {
        "crates/ambition_x/src/lib.rs": "\n".join([
            "#[derive(SystemParam)]",
            "pub struct Spend<'w> { req: ResMut<'w, Flag> }",
            "pub fn raise_it(mut f: ResMut<Flag>) { f.0 = true; }",
            "pub fn spend_it(mut p: Spend) {",
            "    if !p.req.0 { return; }",
            "    p.req.0 = false;",
            "}",
            "fn build(app: &mut App) {",
            "    let sim = app.sim_schedule();",
            "    app.add_systems(Update, raise_it);",
            "    app.add_systems(sim, spend_it);",
            "}",
        ]),
    })
    assert guard.crossings(root) == {"Flag": (["raise_it"], ["spend_it"])}


def test_merely_HOLDING_a_bundle_field_does_not_make_a_producer(tmp_path):
    """⛔⛤ THE CONVERSE ERROR, FROM THE SAME REVIEW, AND IT REACHED THE OUTPUT.

    Reduced to a set of types, a bundle cannot say which field a body touched,
    so every system holding the bundle counted as a writer of everything in it.
    That is where this script's claim of *"seven kaleidoscope systems"* producing
    `NewGameResetRequested` came from; the measured answer is two, and neither is
    visible without reading four hops (see `PRODUCER_BY_INSPECTION`).

    Here `bystander` holds the bundle and touches its OTHER field only, so the
    crossing must have exactly one producer rather than two.
    """
    root = _tree(tmp_path, {
        "crates/ambition_x/src/lib.rs": "\n".join([
            "#[derive(SystemParam)]",
            "pub struct Menu<'w> { flag: ResMut<'w, Flag>, other: ResMut<'w, Other> }",
            "pub fn raise_it(mut menu: Menu) { menu.flag.0 = true; }",
            "pub fn bystander(mut menu: Menu) { menu.other.0 = true; }",
            "pub fn spend_it(mut req: ResMut<Flag>) {",
            "    if !req.0 { return; }",
            "    req.0 = false;",
            "}",
            "fn build(app: &mut App) {",
            "    let sim = app.sim_schedule();",
            "    app.add_systems(Update, raise_it);",
            "    app.add_systems(Update, bystander);",
            "    app.add_systems(sim, spend_it);",
            "}",
        ]),
    })
    assert guard.crossings(root) == {"Flag": (["raise_it"], ["spend_it"])}


def test_a_raise_spelled_as_a_METHOD_CALL_is_a_production(tmp_path):
    """⛔ THE SIXTH SPELLING: the raise is a call, not an assignment.

    `NewGameResetRequested`'s only production write is `self.request = true`
    inside `NewGameResetRequested::request`, reached as `self.reset.request()`.
    A body that assigns nothing still produces.
    """
    root = _tree(tmp_path, {
        "crates/ambition_x/src/lib.rs": "\n".join([
            "impl Flag { pub fn request(&mut self) { self.0 = true; } }",
            "pub fn raise_it(mut f: ResMut<Flag>) { f.request(); }",
            "pub fn spend_it(mut req: ResMut<Flag>) {",
            "    if !req.0 { return; }",
            "    req.0 = false;",
            "}",
            "fn build(app: &mut App) {",
            "    let sim = app.sim_schedule();",
            "    app.add_systems(Update, raise_it);",
            "    app.add_systems(sim, spend_it);",
            "}",
        ]),
    })
    assert guard.crossings(root) == {"Flag": (["raise_it"], ["spend_it"])}


def test_the_by_inspection_table_is_load_bearing_and_says_why():
    """An entry must name producers AND explain why no scan can reach them."""
    for ty, (producers, why) in guard.PRODUCER_BY_INSPECTION.items():
        assert producers, f"{ty} declares no producer"
        assert len(why) > 200, f"{ty}'s entry does not explain the gap it fills"
        assert ty in guard.ADJUDICATED, f"{ty} is declared but never read"


# ── the real tree ───────────────────────────────────────────────────────────


def test_the_real_tree_has_no_unadjudicated_crossing():
    unread = sorted(set(guard.crossings()) - set(guard.ADJUDICATED))
    assert not unread, (
        "an intent raised outside the rewinding schedule and SPENT inside it is "
        f"lost: {unread}. Read the producer and the consumer and record which of "
        "Q136's two mechanisms applies."
    )


def test_no_adjudication_names_a_crossing_that_no_longer_exists():
    """⛔ A row that stops being a crossing has been FIXED or LOST SIGHT OF.

    Leaving it banked would silently absorb the next intent to take its place —
    the same property `check_rollback_mutators_run_in_sim.py`'s ACKNOWLEDGED
    table protects.
    """
    stale = sorted(set(guard.ADJUDICATED) - set(guard.crossings()))
    assert not stale, (
        f"{stale} no longer reports as a crossing. If it was fixed, delete the "
        "entry and say so in the commit; if the scan stopped seeing it, that is "
        "the finding."
    )


def test_a_shrinking_population_is_a_failure_not_a_clean_report():
    """An empty population satisfies 'every crossing is adjudicated' vacuously."""
    sizes = guard.population_sizes()
    for what, floor in guard.FLOORS.items():
        assert sizes[what] >= floor, (
            f"`{what}` is {sizes[what]}, below the recorded floor of {floor}"
        )


def test_every_adjudication_states_when_it_was_read():
    for ty, reading in guard.ADJUDICATED.items():
        assert "read 2026-" in reading, f"{ty}'s reading carries no date"
        assert len(reading) > 200, f"{ty}'s reading is a label, not a reading"
