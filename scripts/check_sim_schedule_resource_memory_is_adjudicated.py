#!/usr/bin/env python3
"""Which unregistered RESOURCES accumulate inside the rewinding schedule?

⛔⛤ **THIS IS THE HALF `check_sim_schedule_memory_is_adjudicated.py` NAMED AS ITS
OWN RESIDUE AND COULD NOT ANSWER.** That guard reads `Local<T>`. The third
instance of the defect it was built for was not a `Local` at all:
`PlayerCloneClock` was a `Resource`, `init_resource`d and never registered for
rollback, accumulating wall dt inside the sim schedule — measured as `GGRS
sync-test checksum mismatch at frames [14, 15, ..]`. Its docstring says so and
says the instrument for the resource half *"does not exist"*. This is it.

⭐⭐ **THE HARD PART IS "ACCUMULATING", AND THE DISCRIMINATOR IS SELF-REFERENCE.**
Most host-local resources in the sim schedule are written WHOLESALE every run and
are perfectly safe; a census that flagged every `ResMut` would report hundreds of
rows and mean nothing. What makes a write carry memory across a rewind is that it
READS ITS OWN PREVIOUS VALUE — `+=`, `x = x + 1`, a `push` onto what is already
there. A wholesale write cannot carry anything, however unregistered it is.

⚠ **AND THE RESET SIDE IS WHERE THIS WAS WRONG THREE TIMES, ALWAYS BY SPELLING.**
A run that resets the value first cannot carry the previous run's into this one,
so the reset test decides most of the population — and it kept being keyed on how
a reset is SPELLED:

    `.clear()`                    the first draft, and it missed the next two
    `clear_engine_contributions()` a domain-specific clear; four
                                  `FeatureEcsWorldOverlay` contributors read as
                                  carrying memory while the rebuild's own comment
                                  said it clears first and they re-extend after
    `begin_rebuild()` + `end_rebuild()`  not a clear at ALL: a generation stamp
                                  and a `retain` that sweeps last run's entries

⇒ The reset prefixes are a concept list, and the generation case is adjudicated
rather than detected. The remaining structural fix was a defect in this file's
own first draft: the "some sibling resets this type" map was built only from
systems that ALSO grow, so a pure resetter like `rebuild_body_clocks_view`
(whose whole body is `view.0.clear()`) never registered, and every mechanic
pushing into `BodyClocksView` read as unbounded growth. Both passes now run over
the whole population.
"""

from __future__ import annotations

import functools
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "scripts"))

import check_rollback_mutators_run_in_sim as sim  # noqa: E402
import check_sim_schedule_memory_is_adjudicated as memory  # noqa: E402

_RESMUT = re.compile(
    r"(?:mut\s+)?([a-z_][a-z0-9_]*)\s*:\s*(?:[A-Za-z_][A-Za-z_0-9]*::)*ResMut\s*<\s*"
    r"(?:'[a-z_]+\s*,\s*)?((?:[A-Za-z_][A-Za-z_0-9]*::)*[A-Z][A-Za-z_0-9]*)"
)

#: A write that reads its own previous value.
_COMPOUND = ("+=", "-=", "*=", "/=", "|=", "&=", "^=")
#: Adding to what is already there.
_GROWERS = ("push", "push_str", "insert", "extend", "entry", "append", "push_back")
#: A call that starts a fresh value. Keyed on the CONCEPT — see the docstring.
_RESET_CALL = r"\.\s*(?:clear|reset|truncate|wipe|drain|take|replace)\w*\s*\("


def _grows(body: str, bind: str) -> list[str]:
    hits: list[str] = []
    esc = re.escape(bind)
    for op in _COMPOUND:
        if re.search(rf"(?:\*\s*)?\b{esc}\b[\w\.\[\]0-9\(\)]*\s*{re.escape(op)}", body):
            hits.append(op)
    for grower in _GROWERS:
        if re.search(rf"\b{esc}\b[\w\.\[\]0-9]*\.\s*{grower}\s*\(", body):
            hits.append(grower)
    for match in re.finditer(rf"\b{esc}\b([\w\.\[\]0-9]*)\s*=(?!=)([^;]*);", body):
        # Not after a `.`: `world_time.sim_dt()` names another value's method,
        # not the binding `sim_dt`, and `x.dt = clock.sim_dt()` is a wholesale
        # write.
        if re.search(rf"(?<![\w.]){esc}\b", match.group(2)):
            hits.append("reads its own value")
            break
    return sorted(set(hits))


def _resets(body: str, bind: str) -> str | None:
    esc = re.escape(bind)
    match = re.search(rf"\b{esc}\b[\w\.\[\]0-9]*{_RESET_CALL}", body)
    if match:
        return match.group(0).split(".")[-1].rstrip("( ")
    if re.search(rf"\*\s*{esc}\s*=\s*[^;]*(?:default\(\)|new\(\))", body):
        return "wholesale reset"
    return None


@functools.cache
def _scan(repo: Path) -> tuple[tuple[str, str, str, tuple[str, ...], str | None], ...]:
    """`(system, file, type, grow hits, reset)` per sim-schedule `ResMut` binding."""
    population = memory.sim_schedule_systems(repo)
    rows: list[tuple[str, str, str, tuple[str, ...], str | None]] = []
    for src, text in sim._production_sources(repo):
        for match in sim._PUB_FN.finditer(text):
            name = match.group(1)
            if name not in population:
                continue
            params = sim._params(text, match.end())
            binds = _RESMUT.findall(params)
            if not binds:
                continue
            brace = text.find("{", match.end() + len(params))
            if brace < 0:
                continue
            body = sim._braced(text, brace)
            rel = str(src.relative_to(repo))
            for bind, ty in binds:
                short = ty.rsplit("::", 1)[-1]
                rows.append((name, rel, short, tuple(_grows(body, bind)), _resets(body, bind)))
    return tuple(sorted(set(rows)))


def carrying(repo: Path = REPO) -> dict[str, tuple[str, str, tuple[str, ...]]]:
    """`{system: (file, type, hits)}` for each accumulation nothing resets.

    Both passes run over the WHOLE population: a type reset by any sim-schedule
    system is rebuilt across a group of systems rather than carried, which is
    the ordinary `Reset` set / `Contribute` set shape.
    """
    rows = _scan(repo)
    registered = sim.rollback_types(repo)
    reset_types = {ty for _n, _f, ty, _h, reset in rows if reset}
    found: dict[str, tuple[str, str, tuple[str, ...]]] = {}
    for name, rel, ty, hits, reset in rows:
        if not hits or reset or ty in registered or ty in reset_types:
            continue
        found[name] = (rel, ty, hits)
    return found


@functools.cache
def _sibling_reset_cached(
    repo: Path,
) -> tuple[tuple[str, str, str, tuple[str, ...]], ...]:
    rows = _scan(repo)
    registered = set(sim.rollback_types(repo))
    reset_types = {ty for _n, _f, ty, _h, reset in rows if reset}
    return tuple(
        (name, rel, ty, hits)
        for name, rel, ty, hits, reset in rows
        if hits and not reset and ty not in registered and ty in reset_types
    )


def sibling_reset_rows(repo: Path = REPO) -> tuple[tuple[str, str, str, tuple[str, ...]], ...]:
    """`(system, file, type, grow hits)` for accumulations a SIBLING resets.

    ⛔⛤ **`carrying()` TREATS A SIBLING RESET AS PROOF OF SAFETY AND IT IS NOT
    PROOF OF ANYTHING — A 2026-09-18 REVIEW BUILT THE COUNTEREXAMPLE.** Given

        app.add_systems(sim, (contribute, wipe));

    where `contribute` does `v.0.push(..)` and `wipe` later does `v.0.clear()`,
    `carrying()` reports nothing — and yet `contribute` reads the PREVIOUS
    simulation run's contents before the later reset ever runs. The exemption
    needs reset-BEFORE-contribute, and membership in the same schedule does not
    give that.

    ⇒ So these rows are no longer exempt; they are a second bucket that must be
    adjudicated by NAMING the ordering. That is the conservative repair rather
    than the complete one: proving a schedule path would mean modelling
    `.chain()`, `.before`, `.after` and set graphs, and the population here is
    4 rows over 2 types — both of which turn out to have an explicit, named
    ordering, one of them already held by its own shipped guard. A reading that
    cites the ordering is worth more than a parser that infers it, at this size.
    """
    return _sibling_reset_cached(repo)


#: type → the ordering that makes *reset before contribute* true for it.
#:
#: ⚠ A row here is NOT waived. It is a claim about a schedule edge, and the
#: reading must name the edge so the next reader can check it still exists.
SIBLING_RESET_ADJUDICATED: dict[str, str] = {
    "BodyClocksView": (
        "AN EXPLICIT TWO-SET CHAIN. `rebuild_body_clocks_view` clears the whole "
        "row set (`view.0.clear()`, `ambition_sim_view/src/facts.rs:355-356`) "
        "and is `.in_set(BodyClockViewSet::Reset)` at `:881`; the sets are "
        "registered `(BodyClockViewSet::Reset, BodyClockViewSet::Contribute)"
        ".chain()` at `:893-895`, inside the `FeatureViewSync` phase. Every "
        "contributor writes `.in_set(BodyClockViewSet::Contribute)` and nothing "
        "else — the type's own doc block at `:347-348` says so and names the "
        "prerequisite it depends on (read 2026-09-18)"
    ),
    "FeatureEcsWorldOverlay": (
        "AN EXPLICIT `.after` ON THE RESETTER'S SET, AND IT ALREADY HAS ITS OWN "
        "GUARD. `rebuild_feature_ecs_world_overlay` "
        "(`actor_monolith/src/features/ecs/world_overlay.rs:25`) calls "
        "`overlay.clear_engine_contributions()` at `:53` and runs in "
        "`FeatureWorldOverlaySet`; contributors declare "
        "`.after(FeatureWorldOverlaySet)` "
        "(`platformer2d_runtime/src/world_gating.rs:44`), and that file asserts "
        "the edge in words at `:115`: *\"every gate_solids writer must run AFTER "
        "FeatureWorldOverlaySet\"*. ⇒ This is the case where the ordering is not "
        "merely present but held, which is what the other row should grow toward "
        "(read 2026-09-18)"
    ),
}


def population_sizes(repo: Path = REPO) -> dict[str, int]:
    rows = _scan(repo)
    return {
        "sim-schedule ResMut bindings": len(rows),
        "accumulating bindings": sum(1 for r in rows if r[3]),
        "systems carrying across a rewind": len(carrying(repo)),
    }


#: A floor, not a target: a scan that suddenly sees far less has broken.
FLOORS = {"sim-schedule ResMut bindings": 180, "accumulating bindings": 45}


#: system → the reading, dated. An entry says a human looked at this accumulator
#: and can say why a rewind replaying it does not corrupt anything.
#:
#: ⛔ THE FIVE MECHANISMS ARE THE USEFUL PART — check a twelfth against them:
#:
#:     OBSERVING THE REWIND     counting resimulations IS the purpose; carrying
#:                              across a rewind is required, not tolerated
#:     A GENERATION STAMP       a monotonic tag re-applied every run, swept
#:                              against at the end; the absolute value is never
#:                              compared to anything a rewind restores
#:     PRESENTATION ONLY        everything that reads it is outside the timeline,
#:                              so over-applying the accumulation is a cosmetic
#:                              imprecision and not a divergence
#:     AN OBSERVATION CENSUS    nothing in production reads it at all
#:     A DEV OR DEMO TOOL       not in a shipped composition
ADJUDICATED: dict[str, str] = {
    "count_advance_run": (
        "⭐ OBSERVING THE REWIND. `RollbackExecutionStats` exists to count how "
        "often `AdvanceWorld` ran and how much of that was resimulation — it "
        "tracks `advance_runs`, `lifetime_advance_runs` and "
        "`highest_simulated_frame` precisely so a replayed frame is VISIBLE. An "
        "accumulator a rewind reset would answer the opposite question "
        "(read 2026-09-18)"
    ),
    "count_advance_world_run": (
        "⭐ OBSERVING THE REWIND, and it says so in the field names: "
        "`RollbackProofState` increments `advance_runs` and, when "
        "`state.resimulating`, `resimulated_runs`. The dev rollback proof pulse "
        "is a measurement OF the rewind (read 2026-09-18)"
    ),
    "rebuild_actor_anim_index": (
        "⭐ A GENERATION STAMP, NOT A CLEAR — which is why the reset test did not "
        "see it. `begin_rebuild` does `self.generation = "
        "self.generation.wrapping_add(1)` "
        "(`crates/ambition_sim_view/src/anim_index.rs:235-237`) and `end_rebuild` "
        "does `self.frames.retain(|_, (_, g)| *g == gen)` (`:239-242`), so every "
        "entry is re-stamped each run and last run's are swept. The counter is a "
        "monotonic tag: a rewind leaves it higher than the frame count would "
        "suggest and nothing compares it to a restored value. `wrapping_add` "
        "makes the overflow defined (read 2026-09-18)"
    ),
    "rebuild_boss_frame_index": (
        "⭐ THE SAME GENERATION STAMP, same file (`:411`), same sweep "
        "(read 2026-09-18)"
    ),
    "tick_shrine_activation_pulse": (
        "⭐ PRESENTATION ONLY, and the type's own module doc is the evidence: "
        "*\"Shared presentation pulse state for save/heal shrines\"* "
        "(`crates/ambition_platformer2d_shared_tangle/src/shrine.rs:1`). It is "
        "SET in the sim on activation "
        "(`crates/ambition_platformer2d_actor_monolith/src/shrine.rs:91`) and the "
        "only reader is `ambition_render`'s shrine visuals "
        "(`crates/ambition_render/src/rendering/shrine_visuals.rs:184`, "
        "`Res<..>`). ⭐ It also uses the RIGHT clock — `world_time.scaled_dt`, "
        "not wall dt — so this is the `PlayerCloneClock` shape with the clock "
        "already correct. A resimulated frame decays `remaining` again, which "
        "makes the pulse fade marginally early on a rewinding host and cannot "
        "reach a checksum (read 2026-09-18)"
    ),
    "observe_brain_action_counter": (
        "⭐ AN OBSERVATION CENSUS WITH NO READER. `BrainActionCounter.total` "
        "accumulates `ActorActionMessage` counts and `last_frame` is written "
        "wholesale. Measured: `crates/ambition_characters/src/brain/mod.rs:254` "
        "is the only other mention in the tree and it is the `init_resource` — "
        "nothing reads either field (read 2026-09-18)"
    ),
    "observe_damageable_body_identity": (
        "⭐ AN OBSERVATION CENSUS. `BodyIdentityCensus` is read only from the "
        "test module in its own file "
        "(`crates/ambition_platformer2d_actor_monolith/src/features/ecs/body_identity.rs:191`); "
        "no production system reads it (read 2026-09-18)"
    ),
    "observe_unminted_bodies": (
        "⭐ AN OBSERVATION CENSUS, A2's witness surface. `UnmintedBodyCensus` is "
        "read only from the test module in its own file "
        "(`crates/ambition_platformer2d_runtime/src/sim_identity.rs:400,424`) "
        "(read 2026-09-18)"
    ),
    "sync_plugin_spawned_ambition_entities": (
        "⭐ AN OBSERVATION CENSUS WITH NO READER AT ALL. `LdtkRuntimeSpineStats` "
        "counts `spawned_entities` and bumps a `revision`; a tree-wide grep for "
        "`Res<LdtkRuntimeSpineStats>` returns nothing (read 2026-09-18)"
    ),
    "force_a_grab_in_range": (
        "⭐ A DEMO TOOL, NOT A SHIPPED COMPOSITION. "
        "`game/ambition_demo_smash_app/src/tools/capture_probe.rs` is a capture "
        "probe; `Forced` is read at `:473` inside the same tool (read 2026-09-18)"
    ),
}


def main() -> int:
    rows = _scan(REPO)
    sizes = population_sizes(REPO)
    short = [
        f"{key}: {sizes[key]} < floor {floor}"
        for key, floor in FLOORS.items()
        if sizes[key] < floor
    ]
    found = carrying(REPO)

    print(
        f"{sizes['sim-schedule ResMut bindings']} `ResMut` binding(s) in the rewinding "
        f"schedule; {sizes['accumulating bindings']} accumulate; "
        f"{len(found)} accumulate a type nothing resets and nothing rolls back."
    )
    print()
    for name, (rel, ty, hits) in sorted(found.items()):
        reading = ADJUDICATED.get(name)
        print(f"`{ty}` via {', '.join(hits)}")
        print(f"    {name}  ({rel})")
        print(f"    {reading}" if reading else "    ⛔ NOBODY HAS READ THIS ONE")
        print()

    unread = sorted(name for name in found if name not in ADJUDICATED)
    stale = sorted(name for name in ADJUDICATED if name not in found)
    if short:
        print("⛔ FAILED\n")
        for line in short:
            print(f"  population shrank: {line}")
        print("  ⇒ A scan that suddenly sees less has broken, not improved.")
        return 1
    if unread:
        print("⛔ FAILED\n")
        for name in unread:
            print(f"  unread accumulator: {name}")
        print(
            "  ⇒ Say which of the five mechanisms makes it harmless, or fix it. A\n"
            "    value accumulated inside the rewinding schedule that nothing\n"
            "    restores is the `PlayerCloneClock` defect until somebody says why\n"
            "    it is not."
        )
        return 1
    if stale:
        print("⛔ FAILED\n")
        for name in stale:
            print(f"  reading with no accumulator: {name}")
        print(
            "  ⇒ Either it was repaired — delete the reading in the same change —\n"
            "    or this scan can no longer see it, which is the worse of the two."
        )
        return 1
    # ⛔⛤ THE SECOND BUCKET, WHICH USED TO BE A SILENT EXEMPTION. A sibling
    # reset is a claim about ORDER, and membership in one schedule does not
    # establish it — the reviewer's counterexample is in `sibling_reset_rows`.
    siblings = sibling_reset_rows()
    sibling_types = sorted({ty for _n, _f, ty, _h in siblings})
    unread_siblings = [ty for ty in sibling_types if ty not in SIBLING_RESET_ADJUDICATED]
    stale_siblings = sorted(set(SIBLING_RESET_ADJUDICATED) - set(sibling_types))
    if unread_siblings:
        print("⛔ FAILED\n")
        for ty in unread_siblings:
            contributors = sorted({n for n, _f, t, _h in siblings if t == ty})
            print(f"  accumulation exempted only by a SIBLING reset: {ty}")
            print(f"      contributors: {', '.join(contributors)}")
        print(
            "  ⇒ A sibling that resets the same type does NOT establish that it runs FIRST.\n"
            "    Name the ordering — a `.chain()` of two sets, or an `.after` on the\n"
            "    resetter's set — in `SIBLING_RESET_ADJUDICATED`. If there is no such\n"
            "    edge, this is an accumulation that reads the speculative run's leftovers."
        )
        return 1
    if stale_siblings:
        print("⛔ FAILED\n")
        for ty in stale_siblings:
            print(f"  ordering reading with no sibling-reset accumulation left: {ty}")
        print("  ⇒ the shape changed; re-derive the row or remove it.")
        return 1

    print(f"ok: {len(found)} accumulator(s) in the rewinding schedule, every one read")
    print(
        f"   plus {len(siblings)} contribution(s) across {len(sibling_types)} type(s) whose "
        "safety is a RESET-BEFORE-CONTRIBUTE ordering, each with the edge named: "
        + ", ".join(sibling_types)
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
