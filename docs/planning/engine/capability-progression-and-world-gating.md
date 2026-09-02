# Capability progression and world gating — Engine 1.0 program

**State:** OPEN — systemic gating is the preferred direction; capability ownership details are intentionally unresolved.

> **RE-MEASURED against `bc85c059d` (2026-09-02). ⭐ THE INPUTS TO FOUR GATE FAMILIES
> ALREADY EXIST; THE GATE THAT READS THEM DOES NOT.** That is a materially
> different starting position from "design the whole thing", and it says where
> the first slice is.
>
> | gate family | vocabulary at HEAD |
> |---|---|
> | body capability | ⭐ `AbilitySet` (`platformer2d_core/src/abilities.rs`) with **14** fields: `move_horizontal`, `jump`, `variable_jump`, `double_jump`, `fast_fall`, `wall_jump`, `wall_cling`, `wall_climb`, `dash`, `double_dash`, `fly`, `fly_toggle`, `blink`, `precision_blink` |
> | body property | ⭐ `mass` (`Option<f32>`, read at spawn, rollback-registered as `mount.mass`), `standing_height` (authored; "IS the body's height — not a hint"), `Locomotion` |
> | item/equipment | `crates/ambition_items`, `crates/ambition_inventory_ui` |
> | world mechanism | partial — `PersistedSwitch`, `GravityFlipSwitch`, `Empowered`; no general "mechanism is in state X" fact |
> | soft systemic pressure | nothing route-facing |
> | social/knowledge | nothing route-facing |
> | story gate | the authored road, as intended |
>
> ⛔ **AND NOTHING GATES A ROUTE ON ANY OF THEM.** The types named `*Gate` in the
> workspace are `ActionGate` (`entity_catalog/src/action_scheme.rs`),
> `EncounterGate` (`ambition_encounter/src/timeline.rs`) and `OutOfShieldGate`
> (`platformer2d_core/src/movement/abilities.rs`) — action and encounter
> sequencing, not world traversal. A search for a route requirement reading a
> body capability finds only cooldown and brain-action gating.
>
> ⇒ **So the open question is narrower than the page's framing suggests.** It is
> not "how do we represent seven gate families"; it is "what reads `AbilitySet`
> and `mass` when a body meets a route, and who owns that predicate". The
> capability-ownership detail the page leaves unresolved is exactly the thing
> that first slice would have to decide, and it can now be decided against real
> types rather than proposed ones.

## Goal

Make exploration and progression primarily emerge from **what the controlled
body can do, what it carries/equips, and what the world has physically become**,
rather than from a long chain of story-stage flags.

Ambition should be able to gate routes through body size, movement abilities,
portal use, environmental resistance, tools, keys, powered machinery and other
mechanical facts. Explicit narrative gates remain available when sequencing is
actually the design.

## Gate families

- **body capability:** climb, fly, morph, blink, portal use, attack/tool ability;
- **body property:** size, mass class, locomotion type, damage/resistance facts;
- **item/equipment:** physical key, tool, wearable or held capability source;
- **world mechanism:** bridge repaired, machine powered, door physically opened;
- **soft systemic pressure:** danger, difficult traversal, hostile population;
- **social/knowledge:** character cooperation or discovered information;
- **story gate:** explicit authored sequencing, used deliberately rather than as
  the default progression representation.

## Engine/game boundary

The engine owns reusable requirement/capability facts and queries. Ambition owns
which theorems, characters, areas and progression meanings those facts represent.

Do not turn every gate into a generic quest condition. Conversely, if world
interaction, navigation, AI and authoring all independently need the same typed
requirement expression, promote that common vocabulary rather than duplicating
flag checks.

## Candidate crate / Bevy shape

A small capability/requirement vocabulary may eventually deserve its own crate,
but only if body construction, world interactions and reachability genuinely
share it. Prefer typed data and queries over a stringly universal expression
language.

## Open design questions — deliberately unresolved

- Which capabilities belong intrinsically to a body versus participant-level
  permanent progression?
- When possession changes bodies, which theorem abilities transfer and which do
  not?
- Can an item temporarily satisfy a capability requirement without becoming a
  body capability?
- How expressive should compound requirements be before they become an
  accidental scripting language?
- Should "knowledge" ever be an engine fact, or remain Ambition/social AI data?
- How should co-op gates behave when one participant can traverse and another
  cannot?
- What constitutes a soft gate that AI/navigation should still consider
  reachable?
