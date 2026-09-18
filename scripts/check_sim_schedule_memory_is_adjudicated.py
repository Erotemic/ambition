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

⛔⛤ **AND THIS CENSUS SEES 13 OF 112, BECAUSE A `MessageReader` IS A `Local`
AND DOES NOT SPELL IT.** In `bevy_ecs` 0.19.1 the type is literally
`struct MessageReader<'w, 's, M> { reader: Local<'s, MessageCursor<M>>, messages:
Res<'w, Messages<M>> }` (`message/message_reader.rs:34-38`) — the cursor saying
*"I have already read up to here"* is per-system host storage, exactly the thing
this file's opening paragraph is about, and the detector matches the literal
token `Local<` in a parameter list. MEASURED 2026-09-18 over the same 594
sim-schedule registrations:

    systems with a literal `Local<`          13
    plus one reached through a `SystemParam`   1   ← `tick_actor_brains`, added
                                                    2026-09-18 after a review
                                                    named the miss
    systems with a `MessageReader` cursor    99   (106 cursors)
    overlap                                   1

⇒ 14 adjudicated here and 99 owned by the ingress census. ⚠ The 13 stood in this
block for part of a day as *"what this census adjudicates"*, which was true and
was not the population — the number and the population are different claims and
this file had been making the second while measuring the first.

⭐⭐ **AND ALL 99 ARE ADJUDICATED BY ONE RULE, WHICH IS WHY THEY ARE NOT LISTED
HERE.** Read 2026-09-18 in `bevy_ecs` 0.19.1: `Messages::clear`
(`message/messages.rs:228-232`) empties both buffers and sets each buffer's
`start_message_count = self.message_count`, so **`message_count` is MONOTONIC
across a clear** — it is never rewound. A cursor's unread count is
`message_count.saturating_sub(last_message_count).min(len())`
(`message/message_cursor.rs:120-129`). ⇒

> A `MessageReader` inside the rewinding schedule loses its message exactly
> when nothing inside that schedule re-raises it.

A sim-raised message IS re-raised by the resimulation, which bumps
`message_count` past the cursor, so the re-raise is read and the cursor carries
no defect. A HOST-raised one is not re-raised, the cursor reads `n - n = 0`, and
the intent is gone. **That is the Q136 ingress question**, and
`check_host_produced_sim_consumed_requests.py` already owns it: joined over
these 99 at 2026-09-18, 73 read only sim-produced messages, 4 read a
host-produced one (exactly that script's four message crossings), and 22 read a
type with no production writer at all.

⛤ **THE ROUTE TO THAT RULE WAS A POISON THAT PASSED.** Removing
`clear_message_on_rollback::<PlayerHealRequested>` changed its witness's outcome
not at all — the registration was assumed to be the loss mechanism, and the
arithmetic above says why it cannot be: the cursor has already refused to read
on the first resimulated frame, whether or not the channel still holds the
message. ⇒ Listing 99 readings here would duplicate the other census's
population rather than adjudicate anything. What this file owes is to SAY the
number every run, which `main()` does, so a clean `13/13` cannot imply 13 is the
population, and to name the owner of the rest.

⚠ **THE THIRD ONE IS WHY THIS FILE'S POPULATION IS A FLOOR AND SAYS SO.**
`PlayerCloneClock` was a `Resource`, not a `Local` — the same failure with a
different spelling, and this scan cannot see it. Answering for the resource half
needs "is this `ResMut<T>` accumulating, and is `T` unregistered", where the
accumulating test is the hard part: most host-local resources are written
wholesale every frame and are perfectly safe.
⇒ A green here is a statement about `Local`, not about memory.

✅ **THAT INSTRUMENT NOW EXISTS**, built 2026-09-18:
`scripts/check_sim_schedule_resource_memory_is_adjudicated.py`, also in
`--maintenance`. Its discriminator for "accumulating" is SELF-REFERENCE — a
write that reads its own previous value — and it reports 11 accumulators, all
read. The two guards are siblings and neither subsumes the other.

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
import functools
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

#: A `#[derive(SystemParam)]` field holding a `Local`.
#:
#: ⛔⛤ **A `Local` REACHED THROUGH A BUNDLE IS STILL PER-SYSTEM MEMORY, AND THIS
#: CENSUS COULD NOT SEE ONE UNTIL 2026-09-18.** A review named the miss:
#: `tick_actor_brains` runs in the rewinding schedule and takes `PerceivedWorld`,
#: whose `empty_relations: Local<'s, FactionRelations>`
#: (`features/ecs/perception.rs:1191`) never appeared in this file's population.
#: ⇒ It is harmless — an immutable all-peaceful fallback returned by
#: `unwrap_or(&self.empty_relations)` — but *"13 systems with a `Local`, every
#: one read"* was not the population, and a green that names the wrong
#: population is the failure mode this file's own docstring is about.
#:
#: ⭐ The Q136 checker had already learned this lesson for `ResMut` and grew
#: `system_param_mutable_fields` for it. This is the same concept for `Local`,
#: reusing the same bundle enumeration and the same dotted-path nesting, so the
#: two agree about what a bundle IS.
_LOCAL_FIELD = sim.bundle_field_pattern("Local")


def _bundle_local_fields(repo: Path) -> tuple[tuple[str, tuple[tuple[str, str], ...]], ...]:
    """`bundle -> ((field path, Local's type), ..)`, nested paths included."""
    return sim.bundle_fields_matching(_LOCAL_FIELD, repo)


def bundle_local_fields(repo: Path = REPO) -> dict[str, dict[str, str]]:
    return {name: dict(fields) for name, fields in _bundle_local_fields(repo) if fields}

#: system -> the reading, dated. ⛔ NOT a waiver list: each row names the
#: MECHANISM that makes remembering harmless here, so the next reader can check
#: that it still holds rather than trusting the row.
ADJUDICATED: dict[str, str] = {
    # ── NEVER WRITTEN: a sixth mechanism, and the strongest one ─────────────
    "tick_actor_brains": (
        "NEVER WRITTEN, which is a stronger reason than any of the five below "
        "and arrived with the first bundle-reached `Local` this census could "
        "see. The binding is `PerceivedWorld.empty_relations: "
        "Local<'s, FactionRelations>` "
        "(`features/ecs/perception.rs:1191`), and its only use is "
        "`self.relations.as_deref().unwrap_or(&self.empty_relations)` at `:1197` "
        "— a borrowable all-peaceful table so `relations()` can hand out a "
        "reference whether or not the live resource is registered. ⇒ A `Local` "
        "nothing ever writes holds `Default::default()` on the speculative run "
        "and on every replay, so there is no value for a rewind to fail to "
        "restore. ⚠ THE CLAIM IS MECHANICAL AND IS CHECKED: "
        "`test_the_never_written_claim_is_verified_not_trusted` fails if any "
        "production line writes that path, because *\"never written\"* is exactly "
        "the kind of true-today sentence a later edit falsifies in silence "
        "(read 2026-09-18, found by a review 2026-09-18)"
    ),
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
#:
#: ⛤ THE REGISTRATION POPULATION JUMPED 589 -> 694 on 2026-09-18 when
#: `_sim_schedule_systems` started following registration WRAPPERS, and the
#: floor deliberately did NOT move with it. 450 was already chosen far under
#: 589 so ordinary churn could not trip it; raising it to hug 694 would make
#: the floor a second, worse copy of the population reading, which is the shape
#: this repository has been collapsing all week.
FLOORS = {"sim-schedule registrations": 450, "systems with a Local": 10}


@functools.cache
def _sim_schedule_systems_cached(repo: Path) -> frozenset[str]:
    return frozenset(_sim_schedule_systems(repo))


def sim_schedule_systems(repo: Path = REPO) -> set[str]:
    """Every name appearing in an `add_systems` whose label IS the sim schedule."""
    return set(_sim_schedule_systems_cached(repo))


def _sim_schedule_systems(repo: Path) -> set[str]:
    found: set[str] = set()
    for _src, text in sim._production_sources(repo):
        for body in sim.add_systems_bodies(text):
            schedule, _, rest = body.partition(",")
            if sim.normalize_schedule(schedule.strip()) not in SIM_LABELS:
                continue
            rest = sim.strip_run_conditions(rest)
            found |= set(re.findall(r"\b([a-z_][a-z0-9_]*)\b", rest))
    # ⛔⛤ **A REGISTRATION WRAPPER IS STILL A REGISTRATION, and leaving it out
    # shrank the wrong number.** `install_technique(app, KEY, offer,
    # (..systems..))` ends in `app.add_systems(sim, systems)`, so a scan for the
    # literal call answers *"not in the simulation"* about systems that ship in
    # it. MEASURED 2026-09-18: 103 wrapper-registered names sat outside this
    # population and 30 of them carry a `Local` or a `MessageReader`.
    #
    # ⭐ **AND THE WIDENING MOVES NO ADJUDICATION ROW**, which is why it is one
    # commit rather than a campaign: `remembering_systems` — the population this
    # module actually adjudicates — stays at 14, because none of the 103 carries
    # a bare `Local`. What grows is `hidden_cursor_systems`, 99 systems/107
    # cursors -> 129/138, and that is the number this module prints to say how
    # much it CANNOT adjudicate. A lower bound getting bigger is the honest
    # direction for it to move.
    return found | set(sim.wrapper_registered_systems(repo))


@functools.cache
def _remembering_cached(repo: Path) -> tuple[tuple[str, str, tuple[str, ...]], ...]:
    return tuple(
        (name, rel, tuple(binds))
        for name, (rel, binds) in sorted(_remembering_systems(repo).items())
    )


def remembering_systems(repo: Path = REPO) -> dict[str, tuple[str, list[str]]]:
    """`{system: (file, [Local binding, ..])}` for sim-schedule systems with a `Local`."""
    return {n: (rel, list(b)) for n, rel, b in _remembering_cached(repo)}


def _remembering_systems(repo: Path) -> dict[str, tuple[str, list[str]]]:
    registered = sim_schedule_systems(repo)
    bundles = bundle_local_fields(repo)
    found: dict[str, tuple[str, list[str]]] = {}
    for src, text in sim._production_sources(repo):
        for match in sim._PUB_FN.finditer(text):
            name = match.group(1)
            if name not in registered:
                continue
            params = sim._params(text, match.end())
            binds = _LOCAL.findall(params)
            # ⇒ And the same question asked of every bundle the signature holds.
            for bundle, fields in bundles.items():
                if re.search(rf":\s*(?:[A-Za-z_][A-Za-z_0-9]*::)*{re.escape(bundle)}\b", params):
                    binds.extend(f"{bundle}.{path}" for path in fields)
            if binds:
                found[name] = (str(src.relative_to(repo)), sorted(binds))
    return found


#: `MessageReader<T>` — a `Local<MessageCursor<T>>` that does not say `Local`.
#: See the blind-spot block in this module's docstring.
_MESSAGE_READER = re.compile(r"\bMessageReader\s*<")
#: A `MessageReader` FIELD of a `#[derive(SystemParam)]` bundle, with its message
#: type. Same shape as `_LOCAL_FIELD` and for the same reason — a bundle field
#: cannot elide its lifetimes, so the parameter spelling and the field spelling
#: are different strings for one thing.
_MESSAGE_READER_FIELD = sim.bundle_field_pattern("MessageReader")


def _bundle_cursor_fields(repo: Path) -> tuple[tuple[str, tuple[tuple[str, str], ...]], ...]:
    """`bundle -> ((field path, message type), ..)`, nested paths included.

    ⛔⛤ **THE CURSOR HALF OF THE BUNDLE EXPANSION, AND IT WAS MISSING WHILE THE
    `Local` HALF SHIPPED.** `_bundle_local_fields` has expanded `Local` through
    `#[derive(SystemParam)]` structs since 2026-09-18; `hidden_cursor_systems`
    read only a system's own parameter list on the same day, so a cursor one
    level down was invisible to the number this module prints to say how much it
    cannot see. Found by review with a production specimen rather than a syntax
    poison: `FreshAttempt` (`crates/ambition_combat/src/events.rs:194`) carries
    two `MessageReader` fields and
    `void_pending_player_hits_at_lifecycle_boundaries`
    (`crates/ambition_damage/src/lib.rs:1249`) takes it — a registered sim system
    whose two cursors this census reported as zero.

    ⇒ An UNDERCOUNT in the "what I cannot adjudicate" number is the worst
    direction a number can be wrong in, because it makes the adjudicated part
    look more complete than it is.
    """
    return tuple(
        (name, fields)
        for name, fields in sim.bundle_fields_matching(_MESSAGE_READER_FIELD, repo)
        if fields
    )


def bundle_cursor_fields(repo: Path = REPO) -> dict[str, dict[str, str]]:
    """`{bundle: {field path: message type}}` for every bundle holding a cursor."""
    return {name: dict(fields) for name, fields in _bundle_cursor_fields(repo)}


def hidden_cursor_systems(repo: Path = REPO) -> dict[str, tuple[str, int]]:
    """`{system: (file, cursor count)}` — the part of the population this census
    cannot adjudicate, measured so a clean run cannot imply it does not exist.

    Counts a system's own `MessageReader` parameters AND the cursors inside any
    `#[derive(SystemParam)]` bundle it takes [`_bundle_cursor_fields`].
    """
    registered = sim_schedule_systems(repo)
    bundles = bundle_cursor_fields(repo)
    found: dict[str, tuple[str, int]] = {}
    for src, text in sim._production_sources(repo):
        for match in sim._PUB_FN.finditer(text):
            name = match.group(1)
            if name not in registered:
                continue
            params = sim._params(text, match.end())
            count = len(_MESSAGE_READER.findall(params))
            for bundle, fields in bundles.items():
                if re.search(
                    rf":\s*(?:[A-Za-z_][A-Za-z_0-9]*::)*{re.escape(bundle)}\b", params
                ):
                    count += len(fields)
            if count:
                found[name] = (str(src.relative_to(repo)), count)
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
    hidden = hidden_cursor_systems()
    print(
        f"ok: {len(found)} system(s) in the rewinding schedule carry a `Local`, every one read "
        f"(of {sizes['sim-schedule registrations']} sim-schedule registrations)"
    )
    # ⛔⛤ SAID OUT LOUD EVERY RUN, BECAUSE AN INSTRUMENT THAT CANNOT SEE
    # SOMETHING MUST BE THE ONE TO SAY SO. A clean `13/13` above would otherwise
    # read as "13 is the population", and it is not.
    print(
        f"⚠ ADJUDICATED ELSEWHERE: {len(hidden)} further system(s) carry a "
        f"`MessageReader`, which IS a `Local<MessageCursor<T>>` and does not say so "
        f"({sum(c for _, c in hidden.values())} cursors). One rule covers them — a "
        "cursor loses its message exactly when nothing in the rewinding schedule "
        "re-raises it — and that is Q136's ingress question, owned by "
        "`check_host_produced_sim_consumed_requests.py`. See this module's cursor block."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
