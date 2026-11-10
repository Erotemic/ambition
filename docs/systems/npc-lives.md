---
status: current
last_verified: 2026-11-10
---

# NPC lives and authored schedules

Ambition's NPC-life system gives ordinary actor bodies recurring schedules,
plans, and per-character state. The planner chooses a sequence of steps; the
body executor performs those steps through the existing actor/navigation
systems. The implementation is usable for the two Eureka-lab scientists; it
is not a general world-object interaction system.

## Where to author and implement

- `game/ambition_content/assets/data/npc_lives.ron` contains domains,
  character-specific life rows, and durations for timed verbs. It is a
  `npc_lives` source in Ambition's `assets/pack.ron`.
- `ambition_content_schemas::npc_lives` parses and validates the authored
  values. `game/ambition_content/src/npc_lives.rs` publishes a `LifeBook`
  during registration and content reload.
- `ambition_life_planning::LifePlanner` is the planner interface: given
  `Facts`, produce steps, then apply a completed step's effects. Ambition's
  authored domains use the `ambition_htn` implementation.
- `ambition_platformer2d_actor_monolith/src/body/life.rs` owns the `Life`
  executor, live-room claims, interruptions, offscreen catch-up, and ledger.

A `lives` entry identifies its character and planner `domain`. Its `facts`
are ongoing personal state, `authored` holds the schedule configuration,
`daily` resets selected state at each new day, `lines` supplies text for
`say`, and `observes` names world facts such as `Free("workbench")` or
`Present("npc_science_coulomba")`. The authored book is reread at planning
time; a live-reloaded domain is used on a subsequent plan, without a second
planner inside the actor.

## Current action semantics

| Action | Current execution |
| --- | --- |
| `go_to` | Start a `Visit` to a named place in the room; complete on arrival or replan on failure |
| `claim` / `release` | Reserve or free a named object for one body in a live room |
| `say` | Speak the authored topic in a bubble; a missing line stops the plan |
| `use`, `rest`, or another declared `holds` verb | Spend the authored amount of game time, then apply its facts; **no underlying object interaction is performed** |
| Unknown undeclared verb | Refuse during validation or stop with `Unsupported` |

A plan is reconsidered when the hour changes, when a claim is lost, or when
relevant world observations change. Conversation can interrupt a life and
cause replanning. The executor orders contenders by character ID rather than
ECS query iteration order.

## Live rooms, offscreen progression, and saves

When an NPC leaves live simulation, `LifeLedger` retains personal facts and
its last planned time. On return, catch-up advances its schedule by whole
hours without simulating the walk or physical object interaction; at most the
last week is replayed. The result places the NPC at its derived location.
Offscreen resource booking is based on hour buckets and is an approximation,
not a guarantee of nonoverlapping physical use of a bench within the hour.

The ledger is rollback state and is persisted to save data when changed.
A fresh game resets it. `daily` facts (for example, `ate=false` and
`talked=false`) reinitialize at the day boundary while cumulative facts can
continue across days.

## Extending the behavior

To add a new routine, start with the domain, character row, named destinations,
and authored speech in `npc_lives.ron`; verify that the character visits the
right place and reaches the next day's schedule. If a new verb must interact
with a real world object, add an **explicit executable action and outcome** to
the body executor rather than declaring a timed `holds` verb and pretending it
performed an interaction.

For current limitations and further work, see the
[active NPC planning plan](../planning/engine/radiant-npc-planning.md).
