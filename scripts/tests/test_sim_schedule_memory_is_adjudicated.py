"""The sim-schedule `Local` census, and the two ways its cheap version lies.

⛔ A `Local` in a system registered into `app.sim_schedule()` is host storage no
rewind restores, so a resimulated frame reads what the speculative run left. This
file plants both halves of the population test — a sim-schedule `Local` must be
FOUND, and a host-schedule one must not be — because a census that answers for
the wrong schedule reads exactly like a clean one.
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))

import check_sim_schedule_memory_is_adjudicated as guard  # noqa: E402

ROLLBACK_REGISTRY_REL = "crates/ambition_platformer2d_runtime/src/rollback/mod.rs"


def _tree(tmp_path: Path, files: dict[str, str]) -> Path:
    registry = tmp_path / ROLLBACK_REGISTRY_REL
    registry.parent.mkdir(parents=True, exist_ok=True)
    registry.write_text("fn r(app: &mut App) {}", encoding="utf-8")
    for rel, body in files.items():
        p = tmp_path / rel
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(body, encoding="utf-8")
    return tmp_path


def _planted(schedule: str, param: str = "mut seen: Local<bool>") -> str:
    return "\n".join([
        f"pub fn remembers({param}) {{}}",
        "fn build(app: &mut App) {",
        "    let sim = app.sim_schedule();",
        f"    app.add_systems({schedule}, remembers);",
        "}",
    ])


def test_a_local_in_the_sim_schedule_is_found(tmp_path):
    root = _tree(tmp_path, {"crates/ambition_x/src/lib.rs": _planted("sim")})
    found = guard.remembering_systems(root)
    assert "remembers" in found, f"the planted sim-schedule `Local` was missed: {found}"
    assert found["remembers"][1] == ["seen"]


def test_a_local_in_a_host_schedule_is_not_this_guard_s_business(tmp_path):
    """THE CONTROL. `Update` never rewinds, so a `Local` there remembers nothing
    a replay could disagree with — and a census that flagged it would bury the
    thirteen real members in hundreds of irrelevant ones."""
    root = _tree(tmp_path, {"crates/ambition_x/src/lib.rs": _planted("Update")})
    assert guard.remembering_systems(root) == {}


def test_the_ggrs_schedule_spelling_counts_too(tmp_path):
    """`sim` is the idiom; `GgrsSchedule` is what it resolves to on the shipped
    host, and a test or a plugin may name it directly."""
    root = _tree(tmp_path, {"crates/ambition_x/src/lib.rs": _planted("bevy_ggrs::GgrsSchedule")})
    assert "remembers" in guard.remembering_systems(root)


def test_a_qualified_local_path_is_still_a_local(tmp_path):
    """⚠ THE TREE USES BOTH SPELLINGS. `bevy::prelude::Local` and
    `bevy::ecs::system::Local` both appear at real sim-schedule systems, so a
    pattern anchored on the bare name is half blind — the same failure the
    sibling guard's schedule labels had."""
    for spelling in ("Local", "bevy::prelude::Local", "bevy::ecs::system::Local"):
        root = _tree(
            tmp_path / spelling.replace(":", "_"),
            {"crates/ambition_x/src/lib.rs": _planted("sim", f"mut seen: {spelling}<bool>")},
        )
        assert "remembers" in guard.remembering_systems(root), spelling


def test_a_system_never_registered_is_not_in_the_population(tmp_path):
    """A helper with a `Local` that nothing registers has no schedule and so no
    rewind to be wrong about."""
    root = _tree(tmp_path, {
        "crates/ambition_x/src/lib.rs": "\n".join([
            "pub fn remembers(mut seen: Local<bool>) {}",
            "fn build(app: &mut App) { let sim = app.sim_schedule(); }",
        ]),
    })
    assert guard.remembering_systems(root) == {}


def test_an_ordering_edge_is_not_a_registration(tmp_path):
    """`.after(x)` names a system somebody ELSE registered — inherited from the
    sibling parser rather than reimplemented, and pinned here so a rewrite
    cannot lose it."""
    root = _tree(tmp_path, {
        "crates/ambition_x/src/lib.rs": "\n".join([
            "pub fn remembers(mut seen: Local<bool>) {}",
            "pub fn other() {}",
            "fn build(app: &mut App) {",
            "    let sim = app.sim_schedule();",
            "    app.add_systems(sim, other.after(remembers));",
            "}",
        ]),
    })
    assert guard.remembering_systems(root) == {}


# ── the real tree ───────────────────────────────────────────────────────────


def test_every_remembered_value_in_the_real_tree_is_read():
    unread = sorted(set(guard.remembering_systems()) - set(guard.ADJUDICATED))
    assert not unread, (
        "system(s) inside the rewinding schedule remember something no rewind "
        f"restores, with no reading: {unread}"
    )


def test_no_reading_outlives_its_local():
    """⛔ A row that stops being a member was REPAIRED or LOST SIGHT OF, and
    leaving it banked would absorb the next system to take its place."""
    stale = sorted(set(guard.ADJUDICATED) - set(guard.remembering_systems()))
    assert not stale, f"{stale} no longer carries a `Local` in the sim schedule"


def test_a_shrinking_population_is_a_failure_not_a_clean_report():
    sizes = guard.population_sizes()
    for what, floor in guard.FLOORS.items():
        assert sizes[what] >= floor, (
            f"`{what}` is {sizes[what]}, below the recorded floor of {floor}"
        )


def test_every_reading_names_a_mechanism_and_a_date():
    """A reading that says only "harmless" is a waiver wearing a census's
    clothes. Each must name WHICH mechanism applies.

    ⭐ **THE LIST GREW TO SIX ON 2026-09-18, AND THIS ARM IS WHY THAT WAS A
    DECISION RATHER THAN A DRIFT.** The bundle-field expansion brought
    `tick_actor_brains` into the population, whose `Local` is NEVER WRITTEN —
    a mechanism none of the first five covers, and a stronger one than any of
    them: a `Local` nothing writes holds `Default::default()` on the
    speculative run and on every replay. Widening a closed vocabulary should
    cost a red test, so that the sixth entry is argued for instead of
    appearing.
    """
    reasons = (
        "SCRATCH",
        "LATCH",
        "NOT ROLLBACK STATE",
        "CACHED QUERY",
        "PER-INSTANCE",
        "NEVER WRITTEN",
    )
    for name, reading in guard.ADJUDICATED.items():
        assert "read 2026-" in reading, f"{name}'s reading carries no date"
        assert any(r in reading for r in reasons), (
            f"{name}'s reading names none of the {len(reasons)} mechanisms: {reading[:80]}"
        )


# ── the bundle-reached Local, and the claim its reading makes ───────────────


def test_a_Local_reached_through_a_bundle_is_in_the_population():
    """⛔⛤ A 2026-09-18 REVIEW FOUND THIS MISS IN THE REAL TREE.

    `tick_actor_brains` runs in the rewinding schedule and takes
    `PerceivedWorld`, whose `empty_relations` is a `Local`. Before the bundle
    expansion the census said *"13 systems with a `Local`, every one read"* —
    and 13 was not the population.
    """
    found = guard.remembering_systems()
    assert "tick_actor_brains" in found, (
        "the bundle-field expansion stopped seeing `PerceivedWorld.empty_relations`"
    )
    binds = found["tick_actor_brains"][1]
    assert any("empty_relations" in b for b in binds), binds


def test_the_bundle_expansion_finds_the_dotted_path_not_just_the_type():
    fields = guard.bundle_local_fields()
    assert "PerceivedWorld" in fields, sorted(fields)
    assert fields["PerceivedWorld"].get("empty_relations") == "FactionRelations"


def test_the_never_written_claim_is_verified_not_trusted():
    """⛔ ITS READING SAYS "NEVER WRITTEN", AND THAT IS CHECKABLE.

    A true-today sentence about absence is exactly what a later edit falsifies
    in silence. If a production line writes that path, the mechanism named in
    the reading is gone and the reading must change with it.
    """
    import re

    writes = []
    for path, text in guard.sim._production_sources(guard.REPO):
        for match in re.finditer(r"empty_relations", text):
            window = text[max(0, match.start() - 60) : match.end() + 60]
            if re.search(r"empty_relations\s*(?:=[^=]|\.\s*(?:clear|push|insert|set)\w*\s*\()", window):
                writes.append(f"{path.relative_to(guard.REPO)}: {window.strip()[:90]}")
            if re.search(r"&\s*mut\s+self\s*\.\s*empty_relations", window):
                writes.append(f"{path.relative_to(guard.REPO)}: taken &mut")
    assert not writes, (
        "`PerceivedWorld.empty_relations` is written now, so `tick_actor_brains`'"
        f"s NEVER WRITTEN reading no longer holds: {writes}"
    )
