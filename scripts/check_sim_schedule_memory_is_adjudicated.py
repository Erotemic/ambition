#!/usr/bin/env python3
"""What does a system inside the rewinding schedule REMEMBER that no rewind restores?

⛔⛔ **A `Local` IS PER-SYSTEM-INSTANCE HOST STORAGE. NOTHING SNAPSHOTS IT, SO
NOTHING RESTORES IT.** A system registered into `app.sim_schedule()` is
resimulated by GGRS after every rollback. If the value it carries between runs
lives in a `Local`, the replay reads what the SPECULATIVE run left rather than
what the restored frame held — and the two diverge. Nothing crashes; the peer's
checksum simply stops matching.

⛤ **THIS SHAPE HAS NOW BEEN THE DEFECT THREE TIMES IN ONE WEEK, AND EVERY
EXISTING CENSUS WAS BLIND TO ALL THREE.** `check_rollback_mutators_run_in_sim.py`
asks whether rollback state is mutated from a schedule that never rewinds — the
opposite direction. `resources_crossing_the_rewind_boundary.py` asks which
resources are written on both sides. `check_multi_writer_resources_are_adjudicated.py`
asks who else writes one value. A system that remembers privately is not a
multi-writer, does not cross a schedule boundary, and mutates nothing registered,
so it appears in none of them:

    sync_live_player_dev_edits_system   wrote five movement clusters from a live
                                        inspector resource inside `GgrsSchedule`
    sync_developer_body_profile         "arbitrated by a `Local` that runs once
                                        per ADVANCE and therefore remembered
                                        across a rewind" (`sim_plugin.rs`)
    tick_player_clone_brains            accumulated wall dt into an unregistered
                                        `PlayerCloneClock`; measured `GGRS
                                        sync-test checksum mismatch at frames
                                        [14, 15, ..]` on the first clone press

⚠ **THE THIRD ONE IS WHY THIS FILE'S POPULATION IS A FLOOR AND SAYS SO.**
`PlayerCloneClock` was a `Resource`, not a `Local` — the same failure with a
different spelling, and this scan cannot see it. Answering for the resource half
needs "is this `ResMut<T>` accumulating, and is `T` unregistered", where the
accumulating test is the hard part: most host-local resources are written
wholesale every frame and are perfectly safe. That instrument does not exist.
⇒ A green here is a statement about `Local`, not about memory.

⭐ **ALL 13 CURRENT MEMBERS ADJUDICATE HARMLESS, AND THE FIVE REASONS ARE THE
USEFUL PART** — they are what a reader should check a fourteenth against:

    SCRATCH             cleared or reset before use, so nothing carries over
    A LOG OR WARN LATCH the remembered value gates a message and nothing else
    NOT ROLLBACK STATE  everything it reaches is unregistered, so no comparison
                        anyone makes can disagree
    A ONCE-PER-APP LATCH a condition no rewind and no session rebase recreates
    A CACHED QUERY      storage, not state

Usage:
    python3 scripts/check_sim_schedule_memory_is_adjudicated.py
    python3 scripts/check_sim_schedule_memory_is_adjudicated.py --list
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "scripts"))

import check_rollback_mutators_run_in_sim as sim  # noqa: E402

#: Schedule labels that ARE the rewinding simulation. ⚠ the lower-case entries
#: are the `app.sim_schedule()` idiom's usual bindings; a composition that binds
#: it to some other name is invisible here, which is the same residual
#: `is_schedule_variable` documents on the sibling guard.
SIM_LABELS = frozenset(
    {"sim", "sim_schedule", "schedule", "pre_collect_sim", "GgrsSchedule", "AdvanceWorld"}
)

_LOCAL = re.compile(r"(?:mut\s+)?([a-z_][a-z0-9_]*)\s*:\s*(?:[A-Za-z_][A-Za-z_0-9]*::)*Local\s*<")

#: system -> the reading, dated. ⛔ NOT a waiver list: each row names the
#: MECHANISM that makes remembering harmless here, so the next reader can check
#: that it still holds rather than trusting the row.
ADJUDICATED: dict[str, str] = {
    # ── SCRATCH: cleared or reset before use ────────────────────────────────
    "integrate_sim_bodies": (
        "SCRATCH. `contact_scratch` is handed to `BodyContactField::field_for`, "
        "whose first statement is `out.clear()` (`shared_tangle/src/body.rs:178`) "
        "— so the `Local` is an allocation the system reuses and never a value it "
        "reads. ⚠ checked in the CALLEE, because the caller only passes `&mut`: a "
        "`field_for` that appended would be a defect with or without rollback "
        "(read 2026-09-18)"
    ),
    "project_particles_to_movement_world": (
        "SCRATCH, and it says so in its own first line: `scratch.reset_per_frame()`. "
        "Its second `Local`, `cap_warned`, is a WARN LATCH — see "
        "`emit_sand_into_grid` for the same shape (read 2026-09-18)"
    ),
    "spawn_cut_rope_victory_npc": (
        "SCRATCH. `released_hosts` is a `Vec<Entity>` the system's own comment "
        "calls *\"reused across frames\"* and which is DRAINED where it is used, so "
        "no run observes what a previous one left (read 2026-09-18)"
    ),
    "advance_room_transition_content_epoch_system": (
        "TWO REASONS, EITHER SUFFICIENT — SCRATCH, AND NOT ROLLBACK STATE. Its "
        "`last_rooms` is cleared and refilled "
        "the moment it differs, so it holds only the comparison it just made; and "
        "what it gates is `epoch.bump()` on a content-version counter that is NOT "
        "rollback-registered — a value about which assets to reload, not about the "
        "simulation (read 2026-09-18)"
    ),
    # ── A LOG OR WARN LATCH ─────────────────────────────────────────────────
    "sync_boss_encounter_phase": (
        "LOG LATCH. `last_logged` exists to print `Dormant -> Intro -> Phase1` "
        "once per transition; the phase itself comes from the entity-local copy on "
        "each boss. Nothing the `Local` holds reaches a write, so a replay that "
        "logs a transition twice or skips one has changed only the log (read "
        "2026-09-18)"
    ),
    "emit_falling_sand_spouts": (
        "LOG LATCH, verified in the guarded block rather than inferred from the "
        "name: the body under `if last_logged.as_ref() != Some(&state.spouts)` is a "
        "`bevy::log::info!` and the assignment, nothing else "
        "(`falling_sand.rs:607`) (read 2026-09-18)"
    ),
    "emit_sand_into_grid": (
        "WARN LATCH. `budget_warned` is a `bool` flipped once so a budget warning "
        "is not printed every tick. A rewind can only cause the warning to be "
        "printed a second time (read 2026-09-18)"
    ),
    # ── NOT ROLLBACK STATE: nothing it reaches is registered ────────────────
    "despawn_bfs_particles_when_the_room_changes": (
        "NOT ROLLBACK STATE. It despawns `Particle`, which belongs to the external "
        "`bevy_falling_sand` crate and which NOTHING in this workspace registers — "
        "no `require_rollback`, no component registration — so the entities it "
        "removes are invisible to the snapshot and no restore can disagree with it "
        "(read 2026-09-18)"
    ),
    "rebuild_control_prompt": (
        "NOT ROLLBACK STATE. `last` memoizes the prompt it published so an "
        "unchanged frame skips the rebuild, and `ControlPrompt` carries no rollback "
        "registration at all — it is a HUD readout. ⚠ If it is ever registered, "
        "this entry is void: a memo that survives a rewind would skip the rebuild "
        "the replay owes (read 2026-09-18)"
    ),
    "tick_npc_idle_barks": (
        "NOT ROLLBACK STATE. `NpcIdleBarkState`'s `timers` and `rotations` decide "
        "WHEN an idle bark fires and WHICH line it picks, and the only thing the "
        "system emits is `VfxMessage`. A rewind can make a bark repeat or a "
        "rotation skip — an audible artefact, not a divergence (read 2026-09-18)"
    ),
    # ── A ONCE-PER-APP LATCH no rewind or rebase recreates ──────────────────
    "advance_sim_tick": (
        "⭐ A ONCE-PER-APP LATCH, AND THE INTERESTING ONE, because a session REBASE "
        "looks like it recreates the condition and does not. `first_step` buys the "
        "off-by-one its doc describes: *\"the head of step 0 must not increment\"*. "
        "Measured 2026-09-18: `SimTick` is `rollback_resource_canonical` AND nothing "
        "in the workspace resets it on a session edge — it is absent from "
        "`reset_session_scoped_resources_on_activation`'s exhaustive destructure — so "
        "the counter is monotonic across sessions and step 0 happens exactly once in "
        "an App's life. ⛔ If anything ever zeroes `SimTick` on activation, the new "
        "session's step 0 would increment and this entry is void (read 2026-09-18)"
    ),
    # ── A CACHED QUERY: storage, not state ──────────────────────────────────
    "sync_authored_gated_lock_walls": (
        "A CACHED QUERY. `Local<Option<RoomSetQuery>>` holds a lazily built "
        "`QueryState`, which is an index into the world's archetypes rather than a "
        "value about the world. Carrying it across a rewind is what every cached "
        "query does (read 2026-09-18)"
    ),
    # ── per-instance and idempotent ─────────────────────────────────────────
    "stamp_causal_frame": (
        "PER-INSTANCE AND IDEMPOTENT, adjudicated the same day in "
        "`resources_crossing_the_rewind_boundary.py`: the function is registered "
        "into `bevy::app::First` AND into `sim`, and a `Local` belongs to a SYSTEM "
        "INSTANCE, so the two registrations do not share one. Each call is a pure "
        "setter derived from fresh `Res` reads, so nothing it writes depends on a "
        "prior write (read 2026-09-18)"
    ),
}

#: ⛔ THE FLOOR. A regex that stops matching reports an empty set, and an empty
#: set satisfies "every remembered value is adjudicated" vacuously.
#:
#: ⚠ AND IT PROTECTS AGAINST A SHRINKING POPULATION, NOT AGAINST SOMEBODY
#: LOWERING THIS LINE — measured, because the obvious poison is the wrong one.
#: Editing these numbers to 0 reddens nothing, since the live population is
#: healthy; what proves them load-bearing is poisoning the SCAN. Anchoring
#: `_LOCAL` on the bare name (so the two qualified spellings the tree uses stop
#: matching) takes three arms red at once, this floor among them.
FLOORS = {"sim-schedule registrations": 450, "systems with a Local": 10}


def sim_schedule_systems(repo: Path = REPO) -> set[str]:
    """Every name appearing in an `add_systems` whose label IS the sim schedule."""
    found: set[str] = set()
    for _src, text in sim._production_sources(repo):
        for body in sim.add_systems_bodies(text):
            schedule, _, rest = body.partition(",")
            if sim.normalize_schedule(schedule.strip()) not in SIM_LABELS:
                continue
            rest = sim.strip_run_conditions(rest)
            found |= set(re.findall(r"\b([a-z_][a-z0-9_]*)\b", rest))
    return found


def remembering_systems(repo: Path = REPO) -> dict[str, tuple[str, list[str]]]:
    """`{system: (file, [Local binding, ..])}` for sim-schedule systems with a `Local`."""
    registered = sim_schedule_systems(repo)
    found: dict[str, tuple[str, list[str]]] = {}
    for src, text in sim._production_sources(repo):
        for match in sim._PUB_FN.finditer(text):
            name = match.group(1)
            if name not in registered:
                continue
            binds = _LOCAL.findall(sim._params(text, match.end()))
            if binds:
                found[name] = (str(src.relative_to(repo)), sorted(binds))
    return found


def population_sizes(repo: Path = REPO) -> dict[str, int]:
    return {
        "sim-schedule registrations": len(sim_schedule_systems(repo)),
        "systems with a Local": len(remembering_systems(repo)),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--list", action="store_true", help="print every member and its reading")
    args = parser.parse_args()

    found = remembering_systems()
    sizes = population_sizes()
    short = [
        f"`{what}`: {sizes[what]}, below the recorded floor of {floor}. This script is "
        "looking at less than it was built against, so a clean result here means nothing."
        for what, floor in FLOORS.items()
        if sizes[what] < floor
    ]

    unread = sorted(set(found) - set(ADJUDICATED))
    stale = sorted(set(ADJUDICATED) - set(found))

    if args.list:
        for name, (rel, binds) in sorted(found.items()):
            print(f"{name}  ({', '.join(binds)})  {rel}")
            print(f"    {ADJUDICATED.get(name, '⛔ UNADJUDICATED')}\n")
        for what, size in sizes.items():
            print(f"{size} {what} (floor {FLOORS[what]})")

    problems = list(short)
    if unread:
        problems.append(
            f"{len(unread)} system(s) inside the rewinding schedule remember something "
            f"no rewind restores, with no reading: {unread}\n"
            "  ⇒ Read the `Local` and say which of the five reasons applies — scratch, a "
            "log/warn latch, nothing rollback-registered downstream, a once-per-App latch, "
            "or a cached query. ⛔ If none does, it is a divergence: the replay reads what "
            "the speculative run left."
        )
    if stale:
        problems.append(
            f"{len(stale)} adjudicated system(s) no longer carry a `Local` in the sim "
            f"schedule: {stale}\n"
            "  ⇒ Either it was repaired, in which case delete the entry in the same change, "
            "or this scan can no longer see it — which is the more likely and the worse."
        )

    if problems:
        print("\n".join(f"\n⛔ {p}" for p in problems))
        return 1
    print(
        f"ok: {len(found)} system(s) in the rewinding schedule carry a `Local`, every one read "
        f"(of {sizes['sim-schedule registrations']} sim-schedule registrations)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
