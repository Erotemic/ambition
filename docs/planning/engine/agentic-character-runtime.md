# Agentic character runtime — Engine 1.0 program

**State:** OPEN / LATER — architecture direction is useful now; implementation should wait for actor/navigation/world-fact foundations.

## Current state

| layer | state |
|---|---|
| authoritative world and actor facts | exists: `world_facts`, `WorldFactConditionsPlugin` |
| observations and memory | exist: `WorldMemory`, `PerceptionMemory`, `AgentObservation`, `CombatObservation` |
| goals | absent |
| navigation and reachability over world geometry | in one room, for one body: exists since 2026-10-09 (a surface graph checked in the kernel, and a brain that follows it). Between rooms: absent. Owner: [`platformer-navigation-and-reachability.md`](platformer-navigation-and-reachability.md) |
| planner or policy | exists but closed (see below) |
| typed engine action intent | exists and is live: `ActionRequest` in `ActorActionMessage`, consumed by traversal abilities, `brain_effects.rs` and `ambition_held_items` |

The implementation waits for two things, not one:

1. **Navigation between rooms.** In-room navigation landed 2026-10-09 (the
   owner page has "The first slice"). The other reachability code that exists
   is not world navigation:
   `reachable_from_start` / `reaches_finish` in `ambition_entity_catalog` walk
   an authored flow graph, the rollback session walks the system dependency
   graph, and the fighter recovery code reasons over local perceived terrain.
2. **An open policy seam.** `CharacterBrainTemplate`
   (`crates/ambition_characters/src/brain/mod.rs`) is a closed enum of nine
   variants (`StandStill`, `Wanderer`, `MeleeBrute`, `Skirmisher`, `Sniper`,
   `ChargeCrash`, `Smash`, `Aerial`, `Fighter`) with no trait object, registry
   or `Custom` arm. A new policy provider must therefore edit
   `ambition_characters`, and an LLM adapter crate cannot supply a brain. No
   plan owns this work; it changes a type, not a subsystem.

   MEASURED 2026-10-09, the cost of the closed seam: the `Roam` brain, one new
   policy, is ten edits in six files of two crates, and its own file. In `ambition_characters`:
   the `StateMachineCfg` variant, its hostility, its perception requirement and
   its dispatch (`state_machine/mod.rs`), its label and its authored-equality
   arm (`brain/mod.rs`), the `BrainPreset` variant (`entry.rs`), its resolver
   arm (`resolver.rs`) and its checksum cursor (`snapshot_impls.rs`). In the
   actor monolith: the second dispatcher's exhaustive match (`brain_tick.rs`).
   The compiler finds each one, which is the good half. The bad half is that a
   game cannot add a policy from its own crate. `Roam` is a catalog-preset
   brain (`BrainPreset` / `StateMachineCfg`); it is not a
   `CharacterBrainTemplate`, so an enemy archetype cannot name it.

`entity_catalog::placements::CharacterBrain::Custom(String)` is not that seam.
It names a character archetype, whose `BrainProfile.template` resolves back to
one of the nine variants.

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

## Runtime agent boundary versus authoring agent boundary

The [authoring control plane](authoring-and-tools.md) edits, validates and publishes
content outside the simulation tick. A runtime agent instead receives bounded
observations and proposes semantic intentions under the normal actor-control
acceptance rules. These are different trust, latency and lifetime contracts.

A delayed model response needs subject/session identity, observation revision,
expiry and cancellation policy before acceptance. It cannot mutate a Bevy World,
install a provider, alter a moveset graph in place or bypass action eligibility.
Playback/rollback must consume the accepted intent, not re-query a remote model.

Trusted Rust extensions remain trusted code, not a sandbox. Data-authored flows
need A11/A12 validation and execution bounds. Do not create a universal runtime
service bus to make an agent capable of everything the engine can express.
