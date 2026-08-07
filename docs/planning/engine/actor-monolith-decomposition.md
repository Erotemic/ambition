# Actor monolith decomposition

**Status: OPEN — active incremental engine work.**

This plan replaces the July-era conclusion that no further
`ambition_platformer2d_actor_monolith` carve was currently justified. That
conclusion was useful while the remaining crate still behaved like one tightly
woven actor authority. It is no longer true of the current tree.

At HEAD `43373f72ddc2` on 2026-08-07:

- `crates/ambition_platformer2d_actor_monolith/src/**/*.rs` is **110,911 lines**;
- the generated `.agent` packet reports **42 root modules**, **176 registered
  systems**, **47 resources**, and **28 normal internal `ambition_*`
  dependencies**;
- [`../test-iteration-cost-2026-08-02.md`](../test-iteration-cost-2026-08-02.md)
  records that Cargo incremental compilation is disabled for this repository, so
  the crate is the recompilation unit, and the measured suite compiled the actor
  monolith 16 times;
- the public-API campaign's movement-only consumer still inherited 15 capability
  crates it did not request because the actor monolith brought them into the
  resolved graph: audio, cutscene, dialog, encounter, items, LDtk, menu,
  persistence, portal, projectiles, settings menu, SFX, SFX bank, UI nav, and
  VFX. See [`api-1.0-campaign.md`](api-1.0-campaign.md).

The decomposition trigger has therefore fired for **two independent reasons**:
consumer dependency leakage and compile-unit cost. The work is no longer
conditional on discovering whether a carve is justified. The remaining design
question is where each incremental boundary belongs.

## Goal

Drain the current monolith until the remaining crate is honestly the reusable
actor/body/control simulation domain, then rename that residue to a durable
actor crate.

The desired property is not merely fewer lines. Editing movement or body control
should not require recompiling dialogue, menus, audio, persistence, LDtk
integration, encounter orchestration, character presentation, and other
independent capabilities. Likewise, changing dialogue or presentation should not
recompile the movement implementation merely because both once lived under the
same package.

The public `ambition_platformer2d` facade remains the compatibility boundary.
Internal crate ownership may change without forcing consumer-facing module paths
to mirror the historical implementation layout.

## Operating rule: subtract one concept at a time

This is a **ratcheted subtraction campaign**, not a flag day.

A good slice makes the actor monolith stop knowing about one coherent concept.
The strongest closure evidence is usually one or more of:

1. a direct dependency disappears from the actor monolith's `Cargo.toml`;
2. the movement-only consumer's resolved capability footprint shrinks;
3. an internal cycle or shared-authority knot disappears;
4. a substantial independent compile unit can rebuild separately;
5. a historical compatibility facade or duplicate path is deleted.

Moving files without improving any of those measures is not a successful carve.
Do not create a new forwarding crate that simply imports every extracted domain
back into the monolith; that preserves both dependency leakage and compile
fanout.

Prefer an existing owning crate when one already exists. Create a new crate only
when the extracted code names a durable reusable engine concept with a clear
dependency direction and a plausible independent consumer/test surface.

## Evidence used to choose each slice

Use the repository's generated navigation data before inventing a boundary:

- `.agent/index/crates/graph-declared.json` — declared crate edges;
- `.agent/index/crates/graph-resolved.json` — resolved build graph;
- `.agent/index/crates/ambition_platformer2d_actor_monolith.json` — files,
  symbols, declared dependencies, tests, and module map;
- `.agent/ecs_inventory/crates/ambition_platformer2d_actor_monolith.{md,json}` —
  module ECS ownership, schedule registrations, message producers/consumers, and
  resource readers/writers;
- `crates/ambition_platformer2d_actor_monolith/MODULES.md` — intended concern of
  each root module;
- [`../../../dev/journals/code_smells.md`](../../../dev/journals/code_smells.md)
  — known duplicate mechanisms, compatibility debris, and second-order
  monoliths.

Generated data is navigation evidence, not source authority. For an extraction
candidate, confirm the important edges in Rust before changing ownership.

For each candidate, record at least:

```text
module/domain
    approximate size
    outgoing actor-module dependencies
    incoming actor-module consumers
    external crate dependencies
    resources it mutates / messages it owns
    expected Cargo edge(s) removed
    expected compile-isolation payoff
    destination owner
```

Incoming edges and shared mutable authorities matter as much as imports. A small
module with twenty consumers can be harder to peel than a larger leaf. The ECS
inventory already exposes several high-coupling authorities — for example
`AmbitionGameSave`, `OwnedItems`, `SlotInteractionState`, `ActiveConversation`,
and `ControlledSubject` — that must not accidentally acquire two owners during a
carve.

## Slice procedure

For every incremental extraction:

1. **Recompute the slice-local graph.** Start from `.agent` data and source.
2. **Name the authority and lifetime.** State what is authoritative, what is
   derived, who mutates it, and whether it belongs to process, experience,
   session, participant, body, room, simulation tick, or presentation frame.
3. **Repoint compatibility imports first.** A dependency through
   `actor_monolith::foo` may only be a stale re-export of a lower canonical
   owner. Remove that false coupling before designing a new interface.
4. **Choose the destination by semantics, not directory shape.** Existing domain
   owners beat a new crate whose only rationale is file location.
5. **Put integration above the domains it joins.** If code genuinely needs both
   actor state and a leaf capability, keep the leaf model independent and place
   the adapter at a composition layer rather than introducing a wrong-way edge.
6. **Migrate all production consumers.** Avoid dual authorities and parallel
   implementations.
7. **Delete the displaced path.** Do not retain internal compatibility shims in
   a pre-release repository.
8. **Remove the old Cargo edge when the slice makes it unnecessary.**
9. **Run focused checks and remeasure.** The useful scorecard is dependency
   closure, actor-crate compile cost, and affected package tests — not a mandatory
   full-workspace suite.

## Current shape: peel the outside before splitting the actor kernel

The largest root modules at the 2026-08-07 baseline are:

| Root area | Rust lines under `src/` | Planning interpretation |
|---|---:|---|
| `features` | 37,884 | deeply coupled actor/NPC/boss ECS core; late carve |
| `character_runtime` | 12,543 | several lifetimes/owners hidden under one name |
| `boss_encounter` | 6,778 | encounter + actor + content orchestration |
| `avatar` | 6,716 | historical mixture that should dissolve by owner |
| `construction` | 5,124 | planner plus domain-specific recipes; split by owner |
| `abilities` | 4,826 | optional gameplay capabilities around body state |
| `character_sprites` | 3,989 | presentation/loading candidate |
| `world` | 3,324 | world/runtime/LDtk integration candidate |
| `items` | 2,388 | item/body integration; vertical carve |
| `schedule` | 2,375 | global ordering vocabulary; requires deliberate ownership |

Do **not** begin by slicing `features` into arbitrary crates. The code-smell log
already found genuinely tight private coupling in the actor-update machinery.
Peel independent capabilities and misplaced outer-layer work first; the central
actor SCC should become smaller and more legible as its spokes disappear.

Two historical aggregate directories should probably **dissolve** rather than
become crates:

- `avatar`: body mechanics move to actor/body owners; participant/home-body and
  starting-character policy move to session/control/provider owners;
  presentation reactions move downstream;
- `construction`: retain a small neutral planning/construction vocabulary where
  useful, while actor, item, encounter, shrine, boss, and other domain-specific
  construction recipes move with their domains.

Likewise, `features` is not a candidate crate name. Its current module map says
it is the enemy/NPC/boss actor simulation. As capabilities peel away, the residue
should be renamed according to the actor concept it actually implements.

## Architecture specimen: Blink must be body capability, not session identity

The current default-Blink behavior is a concrete example of the ownership error
this campaign should eliminate.

The movement kernel correctly gates Blink on the body's capability. The bad
policy is upstream in `session/setup.rs`: when a character catalog row does not
author an ability set, body construction falls back to the session-wide
`EditableAbilitySet`. That developer/session mask defaults to the unrestricted
implemented ability set, including Blink. Consequently an omitted character
capability declaration can manufacture an intrinsic body ability.

The intended authority is:

```text
prepared character/body definition
    -> intrinsic BodyAbilities
    -> effective abilities after explicit session/dev restrictions
    -> movement/action systems
```

The protagonist may explicitly own Blink. Other controlled bodies do not acquire
Blink because control authority moved to them. A development restriction mask
may remove an intrinsic ability; it must not serve as the source of character
identity.

This is useful as an early seam correction because the newer prepared-character
path should eventually carry intrinsic body capabilities alongside motion model,
vitals, moveset, and other body identity. It also provides a concrete test of the
decomposition: answering “can this body Blink?” should not require session setup,
host policy, or protagonist-specific state.

Avoid using the historical word “sandbox” for this concept. Retain that word
only for literal artifacts or environments whose actual role is an experimental
sandbox. Prefer game, experience, session, runtime, host, world, room, dev policy,
or body capability according to the owner being described.

## Candidate extraction waves

These are **priority hypotheses**, not a promise to create one crate per row.
Recheck the current graph before each slice and change order when a cheaper,
higher-payoff boundary becomes available.

### Wave A — easy outer dependencies and stale facades

Start with leaves and false edges that can make the rest of the graph truthful:

- menu/settings integration;
- host/persistence compatibility adapters;
- cutscene integration where schedule ownership can be made neutral;
- touch/input dependencies that only exist through actor compatibility re-exports.

The code-smell backlog specifically identifies actor compatibility-facade debris
and the second-order `spawn_actors.rs` dispatcher. Delete compatibility paths as
their last consumer moves. Do not extract `spawn_actors.rs` as a crate; shrink it
by moving each domain-specific branch with the domain it constructs.

### Wave B — character preparation and presentation

`character_runtime` plus `character_sprites` is a large compile-isolation target.
Split by lifecycle rather than by current directory:

```text
authored character
    -> provider/preparation
    -> prepared body definition
    -> actor construction

prepared presentation
    -> sprite/art loading
    -> presentation
```

Hurtbox/combat adaptation belongs with combat integration; physical body
baseline belongs with actor/body construction; match seating belongs to session
or match topology. “Character” is the subject of these operations, not one shared
runtime authority.

### Wave C — world authoring/backend integration

The actor kernel should consume lowered world/session semantics, not depend on
LDtk as an authoring backend. Use the existing `ambition_platformer2d_world`,
`ambition_platformer2d_ldtk`, provider, and runtime owners to move backend
loading/hot-reload and room orchestration outward until the actor crate can drop
its LDtk edge.

### Wave D — items/equipment and optional gameplay capabilities

Treat items, equipment, abilities, projectiles, portals, and similar mechanics as
body/gameplay capabilities composed around the actor kernel. Use vertical slices:
model/vocabulary stays in the leaf domain, actor-facing execution uses a narrow
body interface, and integration lives above both when necessary.

Do not simply move the current `items/` or `abilities/` directory wholesale if it
still orchestrates unrelated actor/session concerns.

### Wave E — dialogue, conversation, encounter, and boss orchestration

These are attractive dependency removals but higher-risk authority seams.
Conversation changes simulation lifetime and currently crosses the rollback
boundary; preserve the rule that simulation-affecting narrative transitions are
replayable at the same simulation point. Encounter owns orchestration, not actor
identity. Boss-specific policy remains content/game policy while reusable
encounter state stays in `ambition_encounter`.

### Wave F — presentation effects and audio

Simulation publishes deterministic/confirmed effect intent; audio, SFX, VFX,
character visuals, and other presentation consumers live downstream. Reuse the
confirmed-frame external-effect seam rather than making actor simulation depend
on playback/rendering implementations.

Be precise about types currently named like presentation effects that actually
spawn simulation entities or change combat state: those belong to simulation
vocabulary even if their historical name says VFX.

### Wave G — central actor residue

Only after the outer domains have left should we reassess `features`, actor
updates, body-mode integration, control, scheduling, and the remaining
construction path. At this point the residual dependency graph should tell us
whether one actor crate is honest or whether there are still multiple durable
simulation domains.

When the residue really is actor/body/control simulation, rename
`ambition_platformer2d_actor_monolith` to the final actor-domain name and update
the public facade to re-export semantic API modules from their actual owners.

## Scoreboard

Record these measurements after meaningful waves, not after every tiny edit:

| Measure | 2026-08-07 baseline | Direction |
|---|---:|---|
| Actor `src/**/*.rs` lines | 110,911 | down substantially |
| Normal internal `ambition_*` dependencies | 28 | down |
| Unwanted movement-only capability crates inherited through actors | 15 | toward 0 |
| Actor-monolith builds in the measured full-suite workflow | 16 | suite target remains <=2; carves also reduce cost per build |
| Root modules | 42 | descriptive only; do not optimize this count directly |

Add a focused compile-time measurement once the first substantial carve lands so
later waves can compare clean-build and representative incremental-edit cost.
The compile-time goal is both **less work per actor edit** and **more independent
work available for Cargo/rustc parallelism**.

## Definition of done

The campaign is done when:

- the residual actor crate owns body/actor simulation, control integration, and
  genuinely actor-local state rather than miscellaneous platformer gameplay;
- optional capabilities do not enter a minimal consumer solely through the actor
  crate;
- presentation, authoring backend, menu, audio, persistence, dialogue, and other
  independent domains compile outside the actor implementation unit;
- intrinsic body capabilities come from prepared body/character identity rather
  than session/dev defaults;
- historical `avatar`, `features`, and broad construction aggregates have either
  dissolved or been renamed to honest durable concepts;
- the public SDK remains semantic and stable even though implementation ownership
  moved underneath it;
- the compile-time and resolved-dependency measurements demonstrate a material
  improvement over the 2026-08-07 baseline.
