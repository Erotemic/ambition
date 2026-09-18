"""The resource half of the sim-schedule memory census.

Its sibling reads `Local<T>`; this one reads `ResMut<T>`, and the whole
difficulty is telling an ACCUMULATION from a wholesale write. Every arm below
plants one side of that distinction, because the cheap version of this guard —
"flag every `ResMut` of an unregistered type" — reports hundreds of rows and
means nothing.
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

import check_sim_schedule_resource_memory_is_adjudicated as guard  # noqa: E402

ROLLBACK_REGISTRY_REL = "crates/ambition_platformer2d_runtime/src/rollback/mod.rs"


def _tree(tmp_path: Path, files: dict[str, str], registry: str = "") -> Path:
    path = tmp_path / ROLLBACK_REGISTRY_REL
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(registry or "fn r(app: &mut App) {}", encoding="utf-8")
    for rel, body in files.items():
        p = tmp_path / rel
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(body, encoding="utf-8")
    guard._scan.cache_clear()
    return tmp_path


def _one(body: str) -> dict[str, str]:
    return {
        "crates/ambition_x/src/lib.rs": "\n".join([
            body,
            "fn build(app: &mut App) {",
            "    let sim = app.sim_schedule();",
            "    app.add_systems(sim, tick);",
            "}",
        ])
    }


def test_a_compound_assignment_is_an_accumulation(tmp_path):
    root = _tree(tmp_path, _one("pub fn tick(mut c: ResMut<Clock>) { c.seconds += 1.0; }"))
    assert set(guard.carrying(root)) == {"tick"}


def test_a_wholesale_write_carries_nothing(tmp_path):
    """⭐ THE LOAD-BEARING CONTROL. Most host-local resources in the sim
    schedule look exactly like this, and a guard that flagged them would be
    noise rather than a census."""
    root = _tree(tmp_path, _one("pub fn tick(mut c: ResMut<Clock>) { c.seconds = 1.0; }"))
    assert guard.carrying(root) == {}


def test_a_write_that_reads_its_own_value_is_an_accumulation(tmp_path):
    """`x = x - dt` is `+=` spelled the long way, which is how the shrine pulse
    and three dev counters are written."""
    root = _tree(
        tmp_path,
        _one("pub fn tick(mut c: ResMut<Clock>) { c.left = (c.left - dt).max(0.0); }"),
    )
    assert set(guard.carrying(root)) == {"tick"}


def test_a_rollback_registered_type_is_not_this_guard_s_business(tmp_path):
    """A registered accumulator rewinds with the timeline, which is the fix."""
    root = _tree(
        tmp_path,
        _one("pub fn tick(mut c: ResMut<Clock>) { c.seconds += 1.0; }"),
        registry='fn r(app: &mut App) { app.rollback_resource_canonical::<Clock>(E, "c"); }',
    )
    assert guard.carrying(root) == {}


def test_a_reset_in_the_same_body_cannot_carry_the_previous_run(tmp_path):
    root = _tree(
        tmp_path,
        _one("pub fn tick(mut v: ResMut<View>) { v.0.clear(); v.0.push(1); }"),
    )
    assert guard.carrying(root) == {}


def test_a_domain_SPECIFIC_clear_is_still_a_clear(tmp_path):
    """⛔⛤ The first draft looked for `.clear()` and missed
    `overlay.clear_engine_contributions()`, so four `FeatureEcsWorldOverlay`
    contributors read as carrying memory while the rebuild's own comment said
    it clears first and they re-extend after it."""
    root = _tree(
        tmp_path,
        _one("pub fn tick(mut v: ResMut<View>) { v.clear_contributions(); v.0.push(1); }"),
    )
    assert guard.carrying(root) == {}


def test_a_SIBLING_system_resetting_the_type_is_the_reset_contribute_shape(tmp_path):
    """⛔⛤ AND THIS WAS A DEFECT IN THIS FILE'S OWN FIRST DRAFT. The "somebody
    resets this type" map was built only from systems that ALSO grow, so a pure
    resetter — `rebuild_body_clocks_view`, whose entire body is
    `view.0.clear()` — never registered, and every mechanic pushing into
    `BodyClocksView` read as unbounded growth."""
    root = _tree(tmp_path, {
        "crates/ambition_x/src/lib.rs": "\n".join([
            "pub fn wipe(mut v: ResMut<View>) { v.0.clear(); }",
            "pub fn contribute(mut v: ResMut<View>) { v.0.push(1); }",
            "fn build(app: &mut App) {",
            "    let sim = app.sim_schedule();",
            "    app.add_systems(sim, (wipe, contribute));",
            "}",
        ])
    })
    assert guard.carrying(root) == {}


def test_a_host_schedule_accumulator_is_not_this_guard_s_business(tmp_path):
    """Nothing rewinds outside the rewinding schedule, so nothing is lost."""
    root = _tree(tmp_path, {
        "crates/ambition_x/src/lib.rs": "\n".join([
            "pub fn tick(mut c: ResMut<Clock>) { c.seconds += 1.0; }",
            "fn build(app: &mut App) { app.add_systems(Update, tick); }",
        ])
    })
    assert guard.carrying(root) == {}


# ── the real tree ───────────────────────────────────────────────────────────


def test_every_accumulator_in_the_real_tree_is_read():
    guard._scan.cache_clear()
    unread = sorted(n for n in guard.carrying() if n not in guard.ADJUDICATED)
    assert not unread, f"unread accumulators in the rewinding schedule: {unread}"


def test_no_reading_outlives_its_accumulator():
    guard._scan.cache_clear()
    found = guard.carrying()
    stale = sorted(n for n in guard.ADJUDICATED if n not in found)
    assert not stale, f"readings whose accumulator this scan no longer sees: {stale}"


def test_a_shrinking_population_is_a_failure_not_a_clean_report():
    guard._scan.cache_clear()
    sizes = guard.population_sizes()
    for key, floor in guard.FLOORS.items():
        assert sizes[key] >= floor, f"{key}: {sizes[key]} < floor {floor}"


def test_every_reading_names_a_mechanism_and_a_date():
    mechanisms = (
        "OBSERVING THE REWIND",
        "GENERATION STAMP",
        "PRESENTATION ONLY",
        "OBSERVATION CENSUS",
        "DEMO TOOL",
    )
    for name, reading in guard.ADJUDICATED.items():
        assert "read 20" in reading, f"{name}'s reading is undated"
        assert any(m in reading for m in mechanisms), (
            f"{name}'s reading names none of the five mechanisms, so the next "
            f"reader cannot check a twelfth against it"
        )


# ── the sibling reset, which is a claim about ORDER ─────────────────────────


def _two(bodies: str, registration: str) -> dict[str, str]:
    return {
        "crates/ambition_x/src/lib.rs": "\n".join([
            bodies,
            "fn build(app: &mut App) {",
            "    let sim = app.sim_schedule();",
            f"    {registration}",
            "}",
        ])
    }


def test_a_SIBLING_reset_is_not_proof_and_lands_in_the_ordering_bucket(tmp_path):
    """⛔⛤ THE COUNTEREXAMPLE A 2026-09-18 REVIEW BUILT, NOW AN ARM.

    `carrying()` treated any sibling reset of the same type as proof of safety.
    It is not proof of anything: registered as one unordered tuple, `contribute`
    reads the PREVIOUS simulation run's contents before `wipe` ever runs.

    ⇒ `carrying()` still reports nothing for it — that is the documented split,
    not an oversight — and `sibling_reset_rows` is where it now shows up, which
    is what makes `main()` demand the ordering be named.
    """
    root = _tree(tmp_path, _two(
        "pub fn contribute(mut v: ResMut<View>) { v.0.push(1); }\n"
        "pub fn wipe(mut v: ResMut<View>) { v.0.clear(); }",
        "app.add_systems(sim, (contribute, wipe));",
    ))
    guard._sibling_reset_cached.cache_clear()
    assert guard.carrying(root) == {}
    rows = guard.sibling_reset_rows(root)
    assert [(n, ty) for n, _f, ty, _h in rows] == [("contribute", "View")], rows


def test_a_type_that_resets_ITSELF_is_not_in_the_ordering_bucket(tmp_path):
    """⭐ THE CONTROL. A system that clears before it appends needs no edge, and
    putting it in the bucket would demand a reading for nothing."""
    root = _tree(tmp_path, _one(
        "pub fn tick(mut v: ResMut<View>) { v.0.clear(); v.0.push(1); }"
    ))
    guard._sibling_reset_cached.cache_clear()
    assert guard.sibling_reset_rows(root) == ()


def test_a_type_nothing_resets_stays_in_carrying_not_the_bucket(tmp_path):
    root = _tree(tmp_path, _one("pub fn tick(mut v: ResMut<View>) { v.0.push(1); }"))
    guard._sibling_reset_cached.cache_clear()
    assert set(guard.carrying(root)) == {"tick"}
    assert guard.sibling_reset_rows(root) == ()


def test_every_ordering_reading_names_an_edge_and_a_date():
    """A reading here is a claim about a schedule EDGE, so it must point at one."""
    edges = (".chain()", ".after", "in_set")
    for ty, reading in guard.SIBLING_RESET_ADJUDICATED.items():
        assert "read 2026-" in reading, f"{ty}'s ordering reading carries no date"
        assert any(e in reading for e in edges), (
            f"{ty}'s reading names no schedule edge: {reading[:90]}"
        )


def test_the_real_ordering_bucket_is_fully_read():
    types = {ty for _n, _f, ty, _h in guard.sibling_reset_rows()}
    assert types, "the sibling-reset bucket is empty — the split stopped working"
    assert not types - set(guard.SIBLING_RESET_ADJUDICATED), types
