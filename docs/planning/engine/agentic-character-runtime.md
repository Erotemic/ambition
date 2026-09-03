# Agentic character runtime — Engine 1.0 program

**State:** OPEN / LATER — architecture direction is useful now; implementation should wait for actor/navigation/world-fact foundations.

> **RE-MEASURED against `3c3b0d695` (2026-09-03): TWO of the three stated
> foundations now EXIST, and the wait is gated on exactly one.**
>
> | foundation | state |
> |---|---|
> | world facts | ✔ **exists** — `world_facts` module and `WorldFactConditionsPlugin` (`crates/ambition_platformer2d_actor_monolith/src/world_facts.rs:132`), installed by `ambition_platformer2d_runtime` |
> | observations / memory | ✔ **exists** — `WorldMemory` (`crates/ambition_characters/src/perception.rs:740`), `PerceptionMemory` (`crates/ambition_platformer2d_actor_monolith/src/features/ecs/perception.rs:388`), plus `AgentObservation` / `CombatObservation` in the sim harness |
> | navigation / reachability | ⛔ **absent** — no reachability type, no nav graph, no pathfinding of any kind |
>
> ⇒ **So "wait for the foundations" now means "wait for navigation".** The other
> two arrived without this page noticing, and a reader deciding whether to start
> would have been told to wait for three things when two are already here. Owner
> of the remaining one:
> [`platformer-navigation-and-reachability.md`](platformer-navigation-and-reachability.md).
>
> ⚠ **The absence was double-checked, because it is the load-bearing half.** A
> broad `reachab` grep returns only incidental English (`unreachable!`, "not
> reachable in today's content"), and a `pathfind|a_star|astar|navmesh` sweep
> returned nothing but SUBSTRING noise — `a_star` matching
> `the_visual_follows_a_stored_set` and `EXTRA_STARTUP`. A pattern that matches
> inside unrelated identifiers is not evidence either way, which is why the
> conclusion rests on the concept sweep and not on that one.

## Goal

Let persistent characters pursue goals, move through the world, choose engine
actions and participate in dialogue without giving an AI model authority to
rewrite simulation truth.

This is broader than combat `Brain` behavior and narrower than "LLM controls the
game".

## Layering

```text
authoritative world + actor facts
        ↓
observations / memory / goals
        ↓
planner or policy
        ↓
typed engine action intent
        ↓
ordinary actor/control/interact systems
```

LLM-backed reasoning, scripted planners, utility AI and deterministic fallback
brains should be interchangeable policy providers above the same action seam.

## Requirements

- typed action vocabulary rather than free-form mutation;
- deterministic simulation remains authoritative;
- planning failure/timeouts degrade to safe behavior;
- NPC movement uses ordinary navigation/body mechanics;
- dialogue context comes from structured facts/memory;
- headless inspection can explain goal, plan, action and rejection reason;
- no dependency from low-level actor/world crates on an LLM service.

## Candidate crate / Bevy ecosystem value

A small generic "agent controller" plugin could become ecosystem-worthy if it
only defines observation/action/planner contracts and does not assume Ambition's
story or remote model provider. Any actual LLM service adapter should be a
separate optional crate/tool boundary.

## Open design questions — deliberately unresolved

- Which decisions must be deterministic for rollback/network play?
- Can remote/LLM decisions be authoritative, advisory, or only outside rollback
  windows?
- What is the offline/no-model fallback?
- How much world context can an actor inspect directly?
- How are long-running goals represented and interrupted?
- How should characters coordinate or negotiate shared plans?
- What is the latency/cost budget for model-backed characters?
- What safety/content constraints belong in the game rather than the engine?
