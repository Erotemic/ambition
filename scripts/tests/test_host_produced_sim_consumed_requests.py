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
import check_rollback_mutators_run_in_sim as sim  # noqa: E402

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

    A fully-qualified label is not rare and not synthetic: while the label test
    compared bare names, every system registered this way was invisible as a
    host producer. The specimen that first showed it, `request_player_clone_on_key`,
    was deleted with the clone hotkey in `89d78a4a5` — so the arm is re-pointed
    at a LIVE one that is also one of this script's four current crossings,
    `apply_ambient_gravity_requests`
    (`crates/ambition_platformer2d_shared_tangle/src/gravity.rs:753`).
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

    A real producer writes `request.0 = true` and never mentions the type at
    all — `outstanding.0 = true` in
    `crates/ambition_platformer2d_actor_monolith/src/session/checkpoint.rs:414`
    is the shape, and it is what the deleted `SpawnPlayerCloneRequest` producer
    looked like too. So "no assignment names the type" must
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


# ── the message channel ─────────────────────────────────────────────────────
#
# ⚠ **THE THREE ARMS BELOW SHARE ONE GATE, `message_crossings`'s `side()`, AND
# EACH POISON OF IT REDDENS EXACTLY ONE OF THEM** (verified 2026-09-18):
#
#   * swap the two sides (`"sim" if all(is_non_rewinding) else "host"`)
#     → the crossing arm reports no crossing, and the real tree's four vanish;
#   * count any writer, not only a host one → the in-sim control fires;
#   * count any reader, not only a sim one  → the host→host control fires.
#
# The real-tree completeness arm reddens under all three, which is the point of
# having the synthetic arms: they say WHICH half of the gate moved.


def test_a_host_written_sim_read_message_is_a_crossing(tmp_path):
    """⛔⛤ THE CHANNEL THIS SCRIPT'S `Resource` FILTER EXCLUDED ENTIRELY.

    The filter is right — the first version reported 67 rows because `App`,
    `Commands` and `Sprite` are not resources — but an intent raised as a
    `Message` is the same defect on another road, and four of them exist.
    """
    root = _tree(tmp_path, {
        "crates/ambition_x/src/lib.rs": "\n".join([
            "pub fn raise_it(mut w: MessageWriter<Heal>) { w.write(Heal); }",
            "pub fn apply_it(mut r: MessageReader<Heal>) { for _ in r.read() {} }",
            "fn build(app: &mut App) {",
            "    let sim = app.sim_schedule();",
            "    app.add_systems(Update, raise_it);",
            "    app.add_systems(sim, apply_it);",
            "}",
        ]),
    })
    assert guard.message_crossings(root) == {"Heal": (["raise_it"], ["apply_it"])}


def test_a_message_raised_INSIDE_the_simulation_is_not_a_crossing(tmp_path):
    """⭐ THE CONTROL, and it is the majority case: two of the three spawn-request
    seams in the tree are messages raised and read inside the simulation, so the
    resimulation re-raises them and nothing crosses the boundary."""
    root = _tree(tmp_path, {
        "crates/ambition_x/src/lib.rs": "\n".join([
            "pub fn raise_it(mut w: MessageWriter<Shot>) { w.write(Shot); }",
            "pub fn apply_it(mut r: MessageReader<Shot>) { for _ in r.read() {} }",
            "fn build(app: &mut App) {",
            "    let sim = app.sim_schedule();",
            "    app.add_systems(sim, (raise_it, apply_it));",
            "}",
        ]),
    })
    assert guard.message_crossings(root) == {}


def test_a_host_written_host_read_message_never_meets_a_rewind(tmp_path):
    root = _tree(tmp_path, {
        "crates/ambition_x/src/lib.rs": "\n".join([
            "pub fn raise_it(mut w: MessageWriter<Ui>) { w.write(Ui); }",
            "pub fn apply_it(mut r: MessageReader<Ui>) { for _ in r.read() {} }",
            "fn build(app: &mut App) { app.add_systems(Update, (raise_it, apply_it)); }",
        ]),
    })
    assert guard.message_crossings(root) == {}


def test_every_message_crossing_in_the_real_tree_is_read():
    unread = sorted(set(guard.message_crossings()) - set(guard.MESSAGE_ADJUDICATED))
    assert not unread, f"host->sim message crossings nobody has read: {unread}"


def test_no_message_reading_outlives_its_crossing():
    found = guard.message_crossings()
    stale = sorted(set(guard.MESSAGE_ADJUDICATED) - set(found))
    assert not stale, f"message readings this script no longer sees a crossing for: {stale}"


def test_every_message_reading_states_when_it_was_read():
    for name, reading in guard.MESSAGE_ADJUDICATED.items():
        assert "read 20" in reading, f"{name}'s reading is undated"


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


# ── the second spelling of a message production ─────────────────────────────


def test_a_world_write_message_is_a_production(tmp_path):
    """⛔⛤ THE GAP THAT MADE `NewGameResetCommitted` READ AS NEVER WRITTEN.

    Its four sim-schedule readers were all visible; the writer was a `&mut
    World` call, and this pass looked only at `MessageWriter<T>` parameters.
    """
    root = _tree(tmp_path, {
        "crates/ambition_x/src/lib.rs": "\n".join([
            "fn reg(app: &mut App) { app.add_message::<Committed>(); }",
            "pub fn raise_it(world: &mut World) { world.write_message(Committed); }",
            "pub fn apply_it(mut r: MessageReader<Committed>) { for _ in r.read() {} }",
            "fn build(app: &mut App) {",
            "    let sim = app.sim_schedule();",
            "    app.add_systems(Update, raise_it);",
            "    app.add_systems(sim, apply_it);",
            "}",
        ]),
    })
    assert guard.message_crossings(root) == {"Committed": (["raise_it"], ["apply_it"])}


def test_a_variant_argument_resolves_to_its_TYPE_not_its_variant(tmp_path):
    """⭐ `ShellCommand::GoTo(..)` writes a `ShellCommand`.

    Reading the LAST path segment invents types called `GoTo`, `QuitToHome` and
    `Success` — measured: six of the thirteen names the naive reading produced
    were variants, which is why the argument is resolved against the
    `add_message::<T>()` universe instead.
    """
    root = _tree(tmp_path, {
        "crates/ambition_x/src/lib.rs": "\n".join([
            "fn reg(app: &mut App) { app.add_message::<ShellCommand>(); }",
            "pub fn raise_it(world: &mut World) {",
            '    world.write_message(ShellCommand::GoTo("route".into()));',
            "}",
            "pub fn apply_it(mut r: MessageReader<ShellCommand>) { for _ in r.read() {} }",
            "fn build(app: &mut App) {",
            "    let sim = app.sim_schedule();",
            "    app.add_systems(Update, raise_it);",
            "    app.add_systems(sim, apply_it);",
            "}",
        ]),
    })
    assert guard.message_crossings(root) == {"ShellCommand": (["raise_it"], ["apply_it"])}


def test_an_UNREGISTERED_type_written_by_call_is_not_invented(tmp_path):
    """⚠ THE CONTROL ON THE UNIVERSE. Without it the pass would attribute every
    `.write_message(anything)` and manufacture message types from locals."""
    root = _tree(tmp_path, {
        "crates/ambition_x/src/lib.rs": "\n".join([
            "pub fn raise_it(world: &mut World) { world.write_message(NotAMessage); }",
            "pub fn apply_it(mut r: MessageReader<NotAMessage>) { for _ in r.read() {} }",
            "fn build(app: &mut App) {",
            "    let sim = app.sim_schedule();",
            "    app.add_systems(Update, raise_it);",
            "    app.add_systems(sim, apply_it);",
            "}",
        ]),
    })
    assert guard.message_crossings(root) == {}


def test_an_unresolvable_argument_is_reported_not_dropped():
    """⛔ The tree has 21 of these. A silently dropped write is the gap itself."""
    unresolved = guard.unresolved_message_writes()
    assert unresolved, "no unresolved `.write_message(..)` arguments found — the pattern moved"
    assert all(isinstance(f, str) and isinstance(a, str) for f, a in unresolved)


# ── the shape of a reading ──────────────────────────────────────────────────


def test_every_reading_opens_with_a_verdict_and_benign_names_its_escape():
    assert not guard.misshapen_readings()


def test_a_benign_reading_without_an_escape_is_refused(monkeypatch):
    """⛔⛤ THIS CAUGHT A REAL ONE ON ITS FIRST RUN.

    `ResetToCheckpoint` read *"BENIGN, BY AN ORDERING THE ROLLBACK LAYER
    ENFORCES ON PURPOSE"* — which IS the outside-the-timeline escape and did not
    say so in words a reader could match against the other rows. The escape is
    now named.
    """
    monkeypatch.setitem(
        guard.MESSAGE_ADJUDICATED, "Synthetic", "✅ BENIGN, it seemed fine (read 2026-09-18)"
    )
    problems = guard.misshapen_readings()
    assert any("Synthetic" in p and "escape" in p for p in problems), problems


def test_a_reading_with_no_verdict_is_refused(monkeypatch):
    monkeypatch.setitem(
        guard.MESSAGE_ADJUDICATED, "Synthetic", "this one is probably OK (read 2026-09-18)"
    )
    problems = guard.misshapen_readings()
    assert any("Synthetic" in p and "open with" in p for p in problems), problems


def test_a_LIVE_reading_needs_no_escape(monkeypatch):
    """⭐ THE CONTROL. A live defect has no escape by definition, so requiring
    one of every reading would force the word into rows where it is a lie."""
    monkeypatch.setitem(
        guard.MESSAGE_ADJUDICATED, "Synthetic", "⛔ LIVE, and nothing saves it (read 2026-09-18)"
    )
    assert not [p for p in guard.misshapen_readings() if "Synthetic" in p]


def test_the_escape_vocabulary_covers_an_answer_that_does_not_exist_yet():
    """⚠ `REGISTERED CONSUMPTION` has no instance in the tree.

    A check whose vocabulary only covers what already exists cannot accept a
    correct new answer — and that escape is the addition Q136's option 2 needs.
    """
    assert "REGISTERED CONSUMPTION" in guard.BENIGN_ESCAPES
    used = {
        escape
        for table in (guard.ADJUDICATED, guard.MESSAGE_ADJUDICATED)
        for reading in table.values()
        for escape in guard.BENIGN_ESCAPES
        if escape in reading.upper()
    }
    assert "REGISTERED CONSUMPTION" not in used, (
        "something now uses the registered-consumption escape — good news, and this arm's "
        "premise is gone. Point it at whichever escape is still unused, or delete it."
    )


# ── the population's own lower bound ───────────────────────────────────────


def test_the_schedule_map_gap_is_measured_not_assumed():
    """⛔⛤ THE CROSSING SET IS A LOWER BOUND AND THE BOUND IS MEASURED.

    A system absent from `schedules_by_system` reads as "no schedule", which
    `message_crossings` treats as unclassifiable and drops — safe for a verdict
    and unsafe for a POPULATION. This arm fails if the number moves in either
    direction, because a FALL is a road landing (say which) and a RISE is the
    attribution losing ground.

    ⭐ 77 on 2026-09-18 before registration wrappers were followed, 47 after,
    **61** once `MessageReader`/`MessageWriter` fields inside
    `#[derive(SystemParam)]` bundles joined the population later the same day.

    ⛔⛤ **AND THAT THIRD READING IS A CAUSE THIS ARM DID NOT HAVE A CATEGORY
    FOR.** It offered two — a fall is a road landing, a rise is the scan losing
    ground — and the rise from 47 to 61 is neither: the SCAN did not change, the
    POPULATION did. Fourteen systems that were never counted became countable
    and most of them are still unplaceable, which is the honest shape of
    widening a census. ⇒ A rise now has to say WHICH of the two it is, because a
    band that treats them alike would have read a 30% growth in what the
    instrument admits it cannot see as a regression, and a real regression
    hiding under a simultaneous widening would read as neither.
    """
    unlocated = guard.unlocated_message_systems()
    assert 50 <= len(unlocated) <= 72, (
        f"{len(unlocated)} unlocated message systems; the 2026-09-18 reading after "
        "bundle fields joined the population was 61 (47 before them, 77 before the "
        "wrapper road). A fall means another attribution road landed — name it here "
        "and re-measure the exposed-type count in the same commit. A rise is either "
        "the scan losing ground or the population widening, and the two are not the "
        "same finding: say which."
    )


def test_the_wrapper_road_still_reaches_its_specimen():
    """⭐ THE REGRESSION GUARD ON THE ROAD THAT CLOSED 39% OF THE GAP.

    `apply_feature_hit_events` is registered into the simulation schedule
    through `install_technique(app, KEY, offer, (..systems..))`
    (`crates/ambition_platformer2d_runtime/src/combat_schedule.rs:650`), whose
    body is `app.add_systems(sim, systems)` (`:78`). It appears in no
    `add_systems` argument list anywhere in the tree and never has, so it is
    exactly the shape a literal-call scan reports as unscheduled.

    ⛔⛤ THIS ARM REPLACES ONE THAT ASSERTED THE OPPOSITE, and the swap is the
    point. The previous version held the name as PROOF OF A PARSER DEFECT, on a
    docstring claiming `add_systems_bodies` truncated the call at `:640` to 272
    characters. Measured when the fix was attempted: that body is 452 characters
    and closes correctly, 0 of 639 bodies in the tree grow when comments are
    blanked, and no `add_systems` body in that file has ever contained this
    name. The arm was right to be specific and its reason was wrong — which is
    how the wrong reason got caught, because a named specimen can be re-read.
    """
    by_system = guard.schedules_by_system()
    assert by_system.get("apply_feature_hit_events") == {"sim"}, (
        "the wrapper road stopped reaching `apply_feature_hit_events`: it is registered "
        "through `install_technique`, so either `find_sim_forwarders` no longer finds that "
        f"wrapper or the call site moved. Got {by_system.get('apply_feature_hit_events')!r}."
    )


def test_the_residual_is_a_different_shape_than_the_wrapper_gap():
    """⚠ WHAT IS LEFT IS MOSTLY NOT A REGISTRATION PROBLEM AT ALL.

    Measured 2026-09-18 over the 47: **35 appear in no `add_systems` body
    anywhere in the tree**, tests included — `main`, `fire`,
    `finalize_room_publication`, `dispatch_menu_action` are plain functions a
    system CALLS, not systems. Attributing those needs a call graph, not a
    better registration parser. The other 12 are registered only inside
    `#[cfg(test)]` modules, which `_production_sources` strips on purpose: a
    test-only registration is not a schedule fact about the shipped game.

    ⇒ So the honest residual exposure is smaller than the raw count suggests,
    and this arm keeps a specimen of the REAL remaining shape so the next person
    widens the right thing.
    """
    unlocated = set(guard.unlocated_message_systems())
    assert "finalize_room_publication" in unlocated, (
        "the residual specimen resolved. `finalize_room_publication` is called from "
        "`world/rooms/transaction.rs`, not registered — if it now has a schedule, either a "
        "call-graph road landed (say so) or something is attributing a non-system."
    )


def test_a_write_bound_to_a_local_first_is_still_in_the_population(tmp_path):
    """⛔⛤ THE REVIEW POISON THAT USED TO PASS, kept as an arm.

    A 2026-09-18 review wrote `let event = Heal; world.write_message(event);`
    into the tree, watched the crossing leave `message_crossings` for
    `unresolved_message_writes`, and watched this script print `ok:` anyway. An
    argument the parser cannot read was a write that silently left the
    population — the one failure mode a population instrument may not have.
    """
    root = _tree(tmp_path, {
        "crates/ambition_x/src/lib.rs": _CROSSING,
        "crates/ambition_y/src/lib.rs": (
            "pub fn smuggle(world: &mut World) {\n"
            "    let hidden = Unregistered::default();\n"
            "    world.write_message(hidden);\n"
            "}\n"
        ),
    })
    unresolved = guard.unresolved_message_writes(root)
    assert ("crates/ambition_y/src/lib.rs", "hidden") in unresolved, (
        f"a bare local argument is no longer reported as unresolved: {unresolved}"
    )
    assert guard.unresolved_head("hidden") not in guard.UNRESOLVED_ADJUDICATED, (
        "the fixture's name is adjudicated, so this arm cannot show the failure"
    )


def test_the_three_resolvable_binding_shapes_resolve():
    """⭐ AGAINST PRODUCTION, because all three shapes are in the tree.

    `for event in router.advance_pending(..)` -> `ShellEvent` by the callee's
    `-> Vec<ShellEvent>` (`crates/ambition_game_shell/src/plugin.rs:369`,
    `router.rs:703`), and `fn stage_actor(&mut self, request: SpawnActorRequest)`
    -> `SpawnActorRequest` by the PARAMETER
    (`crates/ambition_sim_harness/src/runtime.rs:833`). Both were in the
    unresolved list until 2026-09-18; the list went 21 -> 18 and the remainder
    is one adjudicated type.
    """
    universe = guard._registered_messages(guard.REPO)
    shell = (guard.REPO / "crates/ambition_game_shell/src/plugin.rs").read_text()
    assert guard._resolve_binding(shell, "event", universe, guard.REPO) == "ShellEvent"
    harness = (guard.REPO / "crates/ambition_sim_harness/src/runtime.rs").read_text()
    assert (
        guard._resolve_binding(harness, "request", universe, guard.REPO)
        == "SpawnActorRequest"
    )


def test_every_unresolved_write_left_in_the_tree_is_adjudicated():
    """⚠ THE RATCHET. A new unreadable spelling must be resolved or argued for.

    Today's remainder is one type across 18 sites: `AppExit`, which this
    workspace never passes to `add_message`, so it cannot be in the universe
    however it is spelled.
    """
    heads = {guard.unresolved_head(arg) for _file, arg in guard.unresolved_message_writes()}
    assert heads <= set(guard.UNRESOLVED_ADJUDICATED), (
        f"unadjudicated unresolved write head(s): {sorted(heads - set(guard.UNRESOLVED_ADJUDICATED))}"
    )
    assert heads == {"AppExit"}, (
        f"the remainder changed to {sorted(heads)}; re-read UNRESOLVED_ADJUDICATED "
        "rather than widening it"
    )


def test_no_message_system_is_classified_only_by_an_opaque_schedule_parameter():
    """⚠ THE THIRD WAY A SIDE COULD BE WRONG, and today it is closed.

    `schedules_by_system` records the LABEL as written, so a generic installer
    such as `install_attempt_scoped(app, schedule, when)`
    (`crates/ambition_platformer2d_actor_monolith/src/session/reset/mod.rs:125`)
    contributes the parameter's NAME. `is_non_rewinding` treats any schedule
    variable as rewinding, so an opaque label reads as `sim` — the safe
    direction for a sim READER and the unsafe one for a host WRITER, which
    would be dropped from the crossing set.

    ⭐ MEASURED 2026-09-18: of 172 message writers/readers whose every label is
    a variable, 164 are labelled `sim` (the workspace spelling of
    `let sim = app.sim_schedule()`) and the other 8 carry an opaque `schedule`
    or `schedule.clone()` BESIDE `sim`. None is classified by an opaque label
    alone, so nothing rests on the guess. This arm fails the day one does.
    """
    by_system = guard.schedules_by_system()
    sides = {
        name
        for _ty, writers, readers in guard._message_sides(guard.REPO)
        for name in list(writers) + list(readers)
    }
    opaque_only = sorted(
        name
        for name in sides
        if (labels := by_system.get(name))
        and all(sim.is_schedule_variable(label) for label in labels)
        and "sim" not in labels
    )
    assert not opaque_only, (
        "these message systems are placed ONLY by a generic installer's parameter "
        f"name, so their host/sim side is a guess: {opaque_only}. Resolve the call "
        "site's real schedule argument, or report them with the other lower bounds "
        "rather than classifying them."
    )


def test_a_message_channel_inside_a_bundle_reaches_the_population():
    """⛔⛤ THE SPECIMEN THE REVIEW NAMED, AND THE FIVE TYPES IT WAS HIDING.

    Both sides of this census read a system's OWN parameter list, so a
    `MessageReader` or `MessageWriter` one level down inside a
    `#[derive(SystemParam)]` bundle was invisible. Two consequences, and only
    the second was predictable from the first: a real sim reader read as no
    reader, AND five registered message types — `BodyKnockedOut`,
    `LandedBodyHit`, `OwnedSfxMessage`, `ParriedBodyHit`, `WalletShieldSpent` —
    were written ONLY through a bundle field and so had no writer at all, which
    drops a type out of the population entirely rather than merely misattributing
    it.

    ⚠ THE WRITE SIDE IS AN UPPER BOUND AND THE ROW SAYS SO. Possession is not
    use: `grid_menu_nav` takes `MenuDispatchParams` and never writes the heal.
    The type is admitted so the population is right, the name is kept so
    detection stays conservative, and the printed row labels it — see
    `held_writer`.
    """
    bundles = {name: (w, r) for name, w, r in guard._bundle_message_fields(guard.REPO)}
    assert "FreshAttempt" in bundles, "the review's specimen left the tree"
    assert set(bundles["FreshAttempt"][1]) == {"RoomLoaded", "RoomReplayAdmitted"}

    sides = {ty: (w, r) for ty, w, r in guard._message_sides(guard.REPO)}
    assert "void_pending_player_hits_at_lifecycle_boundaries" in sides["RoomLoaded"][1]

    for ty in (
        "BodyKnockedOut",
        "LandedBodyHit",
        "OwnedSfxMessage",
        "ParriedBodyHit",
        "WalletShieldSpent",
    ):
        assert ty in sides, f"{ty} is written only through a bundle field and left again"

    # Possession is labelled, a real write is not.
    assert guard.held_writer("PlayerHealRequested", "grid_menu_nav")
    assert not guard.held_writer(
        "PlayerHealRequested", "kaleidoscope_menu_action_activated"
    )


def test_a_sim_schedule_bound_to_an_unusual_name_is_still_the_sim_schedule():
    """⭐ `is_schedule_variable` DEFERRED THIS TO "DATAFLOW"; IT IS ONE IDIOM.

    Measured 2026-09-18: every local ever bound to a `sim_schedule()` call in
    the production corpus is `sim` (45 files) or `pre_collect_sim` (one). The
    exception is the one that mattered — `refuse_a_weaker_form_pickup` is
    registered through it (`game/ambition_demo_mary_o/src/lib.rs:1893`) and
    entered this census through `BodySfxWriter`, where it became the first
    message system placed by an opaque label alone.
    """
    text = (REPO / "game/ambition_demo_mary_o/src/lib.rs").read_text()
    assert "pre_collect_sim" in sim.sim_schedule_bindings(text)
    assert "sim" in guard.schedules_by_system()["refuse_a_weaker_form_pickup"]
