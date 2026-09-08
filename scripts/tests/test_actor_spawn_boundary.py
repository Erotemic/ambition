"""The actor-spawn crate builds bodies; the actor kernel runs them.

`ambition_platformer2d_actor_spawn` was carved out of the monolith on 2026-09-07
and the first cut took the LIVE actor vocabulary with it: the per-tick cluster
view (`ActorMut` / `ActorClusterQueryData`), the damage i-frame constant, in-place
provocation, the fighter-ladder projection and the dismounted-rider rebuild.  The
kernel then imported its own tick-time view from the crate whose contract said
"spawn", and the module-graph SCC receipt (11 -> 9) could not see it because the
graph stops at the monolith directory.

This guard states the boundary the other way round, from the SPAWN side:

* the spawn crate declares no live query view (`QueryData`), no timing constant,
  and exactly one system (the spawn-request drain) — a builder crate has nothing
  else to schedule;
* the spawn crate owns no pickup/chest construction — those are feature bundles;
* the kernel's ordinary live roads (actor tick, damage, perception, aggression,
  save-sync, integration) consume the spawn crate only through its BUILDERS
  (`brain_builders`, `conversion` snapshots, `npc_policy`), never through spawn-only
  vocabulary (`SpawnActorRequest`, `spawn_*_into`, `actor_bundles`, installers);
* and, positively, the kernel's spawn roads DO consume it, and the live view DOES
  live in the kernel — so a rename cannot turn this file into an always-green
  historical assertion.

The no-back-edge invariant (spawn never names the monolith) stays in
`test_actor_construction_inversion.py`; it is a separate promise.
"""

from __future__ import annotations

import pathlib
import re

REPO = pathlib.Path(__file__).resolve().parents[2]
MONOLITH_SRC = REPO / "crates" / "ambition_platformer2d_actor_monolith" / "src"
SPAWN_SRC = REPO / "crates" / "ambition_platformer2d_actor_spawn" / "src"
SPAWN_CRATE = "ambition_platformer2d_actor_spawn"

LINE_COMMENT = re.compile(r"//.*$", re.MULTILINE)

# The one system a builder crate legitimately owns: draining spawn requests into
# spawns.  A second entry here is a boundary decision, not a convenience.
SPAWN_SYSTEMS = {"apply_spawn_actor_requests"}

# Spawn-crate modules a LIVE kernel road may import: pure builders (data in,
# components out).  Anything else the crate exports is spawn-only vocabulary.
BUILDER_MODULES = {"brain_builders", "conversion", "npc_policy"}

# Kernel files that ARE spawn/materialization roads, by path fragment.  They may
# consume the whole spawn crate.  Every other production file in the monolith is
# a live road for the purpose of this guard.
SPAWN_ROADS = (
    "/construction/",
    "/features/ecs/spawn/",
    "/features/ecs/spawn_static.rs",
    "/features/ecs/summon.rs",
    "/features/feature_bundles.rs",
    "/world/rooms/stage.rs",
    "/character_runtime/match_activation.rs",
    "/character_runtime/presentation.rs",
    "/rollback_registration.rs",
)


def _production_files(root: pathlib.Path) -> list[pathlib.Path]:
    return [
        p
        for p in sorted(root.rglob("*.rs"))
        if "tests" not in p.name and "/tests/" not in str(p)
    ]


def _code(path: pathlib.Path) -> str:
    text = path.read_text(encoding="utf-8", errors="ignore")
    # Drop `#[cfg(test)] mod ... { ... }` tails: a builder crate may test its
    # builders through an App without owning a system.
    cut = text.find("#[cfg(test)]")
    if cut != -1:
        text = text[:cut]
    return LINE_COMMENT.sub("", text)


def _fn_containing(code: str, offset: int) -> str | None:
    head = code[:offset]
    match = None
    for match in re.finditer(r"\bfn\s+([A-Za-z_][A-Za-z_0-9]*)", head):
        pass
    return match.group(1) if match else None


def test_the_spawn_crate_declares_no_live_query_view() -> None:
    offenders = [
        str(p.relative_to(REPO))
        for p in _production_files(SPAWN_SRC)
        if re.search(r"\bQueryData\b", _code(p))
    ]
    assert not offenders, (
        "the spawn crate derives or names a `QueryData` view; the per-tick actor "
        "view belongs to the kernel (`crate::actor_clusters`):\n  "
        + "\n  ".join(offenders)
    )


def test_the_spawn_crate_owns_no_timing_constant() -> None:
    offenders: list[str] = []
    for p in _production_files(SPAWN_SRC):
        for m in re.finditer(r"pub(?:\([a-z]+\))?\s+const\s+([A-Z_0-9]+)\s*:\s*f32", _code(p)):
            if m.group(1).endswith(("_S", "_MS", "_SECS", "_SECONDS")):
                offenders.append(f"{p.relative_to(REPO)}  {m.group(1)}")
    assert not offenders, (
        "a construction crate publishes live timing; move it to the owner that "
        "ticks it:\n  " + "\n  ".join(offenders)
    )


def test_the_spawn_crate_schedules_exactly_its_request_drain() -> None:
    systems: set[str] = set()
    for p in _production_files(SPAWN_SRC):
        code = _code(p)
        for m in re.finditer(r"\.add_systems\(", code):
            # The system named inside the call, not the installer around it.
            tail = code[m.end() : m.end() + 400]
            names = re.findall(r"\b([a-z_][a-z_0-9]*)\s*[\.,)]", tail)
            names = [n for n in names if n not in {"schedule", "sim", "app"}]
            systems.add(names[0] if names else f"<unparsed in {p.name}>")
        for m in re.finditer(r"\bQuery<", code):
            owner = _fn_containing(code, m.start())
            if owner:
                systems.add(owner)
            else:
                systems.add(f"<Query outside fn in {p.name}>")
    assert systems == SPAWN_SYSTEMS, (
        f"the spawn crate schedules or queries in {sorted(systems)}; a builder crate "
        f"owns exactly {sorted(SPAWN_SYSTEMS)}. A system that reads live entities "
        "belongs to the kernel."
    )


def test_the_spawn_crate_builds_no_pickup_or_chest() -> None:
    offenders = [
        str(p.relative_to(REPO))
        for p in _production_files(SPAWN_SRC)
        if re.search(r"\b(PickupFeature|ChestFeature)\b", _code(p))
    ]
    assert not offenders, (
        "an actor-named crate constructs pickups/chests; those bundles belong to "
        "the feature layer (`features::feature_bundles`):\n  " + "\n  ".join(offenders)
    )


def _spawn_imports(code: str) -> list[str]:
    return re.findall(rf"{SPAWN_CRATE}::([A-Za-z_][A-Za-z_0-9]*)", code)


def test_live_kernel_roads_consume_only_builders() -> None:
    offenders: list[str] = []
    for p in _production_files(MONOLITH_SRC):
        rel = "/" + str(p.relative_to(MONOLITH_SRC))
        if any(road in rel for road in SPAWN_ROADS):
            continue
        for name in _spawn_imports(_code(p)):
            if name not in BUILDER_MODULES:
                offenders.append(f"{p.relative_to(REPO)}  {SPAWN_CRATE}::{name}")
    assert not offenders, (
        "a live kernel road reaches spawn-only vocabulary; a running actor needs a "
        "builder at most, never a spawn request, a bundle or an installer:\n  "
        + "\n  ".join(sorted(set(offenders)))
    )


def test_the_positive_controls_hold() -> None:
    roads = [
        p
        for p in _production_files(MONOLITH_SRC)
        if any(road in "/" + str(p.relative_to(MONOLITH_SRC)) for road in SPAWN_ROADS)
    ]
    road_refs = sum(len(_spawn_imports(_code(p))) for p in roads)
    # MEASURED 2026-09-07: 7 distinct import sites across the roads (a `use
    # a::{b, c}` counts once).  A floor, not a ceiling — growth is fine.
    assert road_refs >= 5, (
        f"the kernel's spawn roads reference the spawn crate only {road_refs} times; "
        "if construction moved, re-point SPAWN_ROADS instead of letting this guard "
        "certify an empty population"
    )
    view = MONOLITH_SRC / "actor_clusters.rs"
    assert view.is_file(), "the live actor view left the kernel again"
    code = _code(view)
    assert "QueryData" in code and "ACTOR_DAMAGE_IFRAME_S" in code
    live_users = [
        p
        for p in _production_files(MONOLITH_SRC / "features")
        if "crate::actor_clusters::" in _code(p)
    ]
    assert len(live_users) >= 5, (
        f"only {len(live_users)} live feature files read `crate::actor_clusters`; the "
        "view is supposed to be the kernel's shared tick vocabulary"
    )
