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

> **RE-MEASURED AGAIN against `4149f26b6` (2026-09-03), one layer at a time
> rather than one foundation at a time — and the diagram below is half built.**
>
> | layer in the diagram | state |
> |---|---|
> | authoritative world + actor facts | ✔ exists (`world_facts`, above) |
> | observations / memory / goals | ◐ observations and memory exist (above); **goals do not** |
> | planner or policy | ⛔ **exists but is CLOSED — see below** |
> | typed engine action intent | ✔ **exists and is live** |
> | ordinary actor/control/interact systems | ✔ consume it today |
>
> ⭐ **THE ACTION SEAM IS NOT ASPIRATIONAL.** `ActionRequest`
> (`ambition_characters/src/brain/action_set/mod.rs:1255`), carried by
> `ActorActionMessage`, is consumed in production by the traversal abilities
> (`abilities/traversal/{flyline.rs:48,trapdoor.rs:62,teleport.rs:335}`), by
> `features/ecs/brain_effects.rs` and by `ambition_held_items/src/lib.rs:1454`.
> Its own doc comment still describes itself as "the *shape* of the resolver
> output" pending later wiring; that wiring landed, and the comment is stale.
> ⇒ **The requirement "typed action vocabulary rather than free-form mutation"
> is already met**, which is worth knowing before anyone designs it again.
>
> ⛔ **BUT THE POLICY LAYER ABOVE IT IS A CLOSED ENUM, AND THAT IS A SECOND GATE
> THIS PAGE DOES NOT NAME.** `CharacterBrainTemplate`
> (`ambition_characters/src/brain/mod.rs:485`) has nine variants — `StandStill`,
> `Wanderer`, `MeleeBrute`, `Skirmisher`, `Sniper`, `ChargeCrash`, `Smash`,
> `Aerial`, `Fighter` — with **no trait object, no registry and no `Custom`
> arm**. So a new policy provider is a new variant *inside*
> `ambition_characters`.
>
> ⇒ That directly contradicts two of this page's own requirements. "LLM-backed
> reasoning, scripted planners, utility AI and deterministic fallback brains
> should be interchangeable policy providers above the same action seam" is not
> satisfiable as the types stand, and "no dependency from low-level actor/world
> crates on an LLM service" cannot be honoured by *any* separate adapter crate:
> the adapter cannot supply a brain without editing the low-level crate that
> owns the enum.
>
> ⇒ **So "wait for navigation" is not the whole wait.** Navigation is owned
> ([`platformer-navigation-and-reachability.md`](platformer-navigation-and-reachability.md)).
> The policy seam is not. ⓘ
> [`control-authority-and-ai-policy.md`](control-authority-and-ai-policy.md)
> knows this enum and tracks moving `Smash` and `Fighter` behaviour out of the
> crate — but that is a placement question, and a carve that relocates two
> variants leaves the enum exactly as closed as it is now. Opening the seam is
> unowned work, and unlike navigation it is cheap to prototype: it changes a
> type, not a subsystem.
>
> ⚠ **AND THERE IS A `Custom` ARM THAT LOOKS LIKE THE ESCAPE HATCH AND IS NOT.**
> This was nearly recorded wrongly, so it is worth stating plainly: the workspace
> has **two** brain vocabularies, and the exhaustive matches outside
> `ambition_characters` are almost all on the other one.
>
> | type | shape | role |
> |---|---|---|
> | `entity_catalog::placements::CharacterBrain` | **open** — has `Custom(String)` | what a LEVEL authors on a spawn |
> | `ambition_characters::brain::CharacterBrainTemplate` | **closed** — nine variants | what the runtime actually runs |
>
> The authored `Custom(String)` names a character archetype, not a behaviour —
> e.g. `CharacterBrain::Custom("giant_gnu_hands")`
> (`actor_monolith/src/construction/mod.rs:1846`) — and the archetype it names
> carries a `BrainProfile` whose `template` field
> (`ambition_characters/src/brain/profile.rs:111`) is one of the same nine.
> ⇒ **So the string-keyed openness is an AUTHORING indirection that resolves back
> into the closed set.** It lets content name a new creature; it does not let
> anything supply a new policy. Anyone sizing this work by grepping for `Custom`
> will conclude the seam is already open, and it is not.

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
