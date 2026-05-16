# ADR 0016: Unify NPC and Enemy into a single `Actor` entity

## Status

Proposed.

## Context

The Ambition design doc treats "NPC" and "Enemy" as a single
concept differentiated only by aggression level: a calm shopkeeper
and a hostile raider are both *actors* — characters that occupy a
room, have a name + dialogue line, possibly patrol a path, and
respond to the player. The distinction between "talks to you" and
"attacks you" is one field, not one entity type.

The current implementation does NOT mirror that:

- **LDtk authoring:** `NpcSpawn` and `EnemySpawn` are separate
  entity definitions with overlapping but distinct field lists
  (`dialogue_id` on NpcSpawn, `brain` on EnemySpawn, both with
  `name` + `path_id`).
- **Runtime:** `ActorRuntime::Peaceful(NpcRuntime)` and
  `ActorRuntime::Hostile(EnemyRuntime)` are separate variants. A
  story beat that flips a peaceful NPC to hostile (the design's
  "calm Erdish suddenly becomes aggressive") routes through
  `hostile_from_npc(npc)` and rebuilds the runtime state.
- **Combat / dialogue:** Two separate spawn paths feed two
  separate (but very similar) systems. Adding a new actor
  capability (e.g., a peaceful enemy that flees when attacked)
  means touching both sides.

This split is acceptable while the cast is small, but the design's
"every guard could be a shopkeeper, every shopkeeper could be a
guard" intent demands a single entity vocabulary.

## Decision

Replace `NpcSpawn` and `EnemySpawn` with a single `Actor` LDtk
entity definition. Replace `ActorRuntime::{Peaceful, Hostile}`
with a single `ActorRuntime` that holds an `aggression` field.

### LDtk authoring

```yaml
- type: Actor
  px: [400, 688]
  size: [32, 48]
  fields:
    name: Gate Janitor
    aggression: Peaceful     # Peaceful | Wary | Hostile | Aggressive
    dialogue_id: gate_janitor_ripple   # optional
    brain: Idle              # optional; defaults follow aggression
    path_id: ""              # optional patrol
```

Aggression tiers — chosen to match the design doc's "ethical
funding axis" language:

- `Peaceful`: never initiates combat. Always interactable for
  dialogue. Becomes `Wary` if attacked.
- `Wary`: doesn't initiate, but defends if attacked. Dialogue
  still works.
- `Hostile`: pursues + attacks on sight. Dialogue blocked.
- `Aggressive`: faster-reacting Hostile; tuned for elite enemies
  + bosses' minions.

The `aggression` field is the **single switch**. Story beats flip
it via `actor.set_aggression(new_tier)`; the runtime composes the
correct AI brain + interactable behavior from there.

### Runtime

```rust
#[derive(Component, Clone, Debug)]
pub struct ActorRuntime {
    pub id: String,
    pub name: String,
    pub aggression: Aggression,
    pub dialogue_id: Option<String>,
    pub brain: ActorBrain,        // unified; aggression picks default
    pub patrol: Option<PatrolSpec>,
    pub combat: Option<ActorCombatState>,
}
```

`ActorBrain` is the union of today's `EnemyBrain` + a new `Idle` /
`Patrol` variant for peaceful actors. The brain registry picks a
default based on aggression when the authored field is empty:

| Aggression | Default brain          |
| ---------- | ---------------------- |
| Peaceful   | `Patrol(path_id?)`     |
| Wary       | `PatrolOrDefend(...)`  |
| Hostile    | `Pursue(...)`          |
| Aggressive | `PursueFast(...)`      |

Authors can override per-actor by setting `brain` explicitly.

### Combat / dialogue integration

The combat damage pipeline already routes through `ActorRuntime`;
the change is internal — no more `match Peaceful / Hostile` arms.
`apply_feature_damage_events` (tech debt §5 in
[intro_handoff_to_next_agent]) splits naturally into one path
keyed by aggression rather than the variant.

Dialogue: `Interactable` only attaches when `aggression !=
Hostile` and `dialogue_id.is_some()`. A hostile actor with
`dialogue_id: erdish_corrupted` becomes interactable the moment
its aggression flips back to Peaceful — without re-spawning the
entity.

### Migration

The migration is wide (every authored NpcSpawn + EnemySpawn in
sandbox.ldtk and intro.ldtk) but mechanical:

1. `python -m ambition_ldtk_tools def register-entity` adds the
   `Actor` entity def.
2. New tool `entity convert-type` (or one-off script) rewrites
   every `NpcSpawn` to `Actor { aggression: Peaceful }` and every
   `EnemySpawn` to `Actor { aggression: Hostile }`, preserving px,
   size, and overlapping fields.
3. `entity_to_runtime` grows a single `"Actor"` arm; the
   `"NpcSpawn"` / `"EnemySpawn"` arms become deprecated stubs that
   emit a warning + route to the Actor path (so any unmigrated
   LDtk file still loads during the transition).
4. After all LDtk files migrate, the deprecated arms + the old
   entity defs come out in one cleanup commit.

Save format: `ActorRuntime` is the in-game type, not the save type.
Persistent actor state today lives in the per-feature save record
(`ActorHealth`, `ActorCombatState`, `ActorDisposition`). Those
keep their save shape; only the in-memory composition changes.

## Consequences

- **Combat system simplification.** `apply_feature_damage_events`
  no longer needs two parallel branches; the branch becomes one
  arm with an aggression check. Future damage modifiers (e.g., a
  pacifist build that gives extra reward XP for non-lethal
  takedowns) need one knob instead of two.
- **AI extensibility.** New brain variants (e.g., `Coward`,
  `Bodyguard`, `Sleepwalker`) drop in as `ActorBrain` enum
  variants and pick up via the default-brain table.
- **Authoring simplification.** One entity in the LDtk editor
  instead of two. The aggression field is a dropdown; mistakes
  are visible at a glance.
- **Wider blast radius.** Combat, AI, dialogue,
  content_validation, save serialization, conversion_tests,
  ecs_actor_view_compat, and the encounter system all touch
  `ActorRuntime`. Each needs a careful pass. This is the highest-
  risk refactor on the current backlog.
- **Backwards compatibility window.** The deprecated NpcSpawn /
  EnemySpawn arms in step 3 stay until all LDtk migrations land;
  estimate one or two commits between "Actor exists" and "old
  types removed."

## Initial implementation target

A six-commit sequence keeps each step reviewable:

1. **`feat(actor): introduce Aggression + unified ActorRuntime`** —
   pure Rust types; no LDtk change. `From<NpcRuntime>` /
   `From<EnemyRuntime>` constructors keep callers compiling.
2. **`refactor(combat): apply damage to ActorRuntime by
   aggression`** — collapse the dual branches in
   `apply_feature_damage_events`. Tests stay green via the
   `From` constructors.
3. **`feat(ldtk): register Actor entity def + entity_to_runtime
   "Actor" arm`** — additive; old types still work.
4. **`tools(entity): convert-type subcommand`** — migration
   tooling.
5. **`migrate(intro): convert all intro NpcSpawn / EnemySpawn to
   Actor`** — one LDtk diff per file. Validate clean.
6. **`cleanup(actor): remove deprecated NpcSpawn / EnemySpawn
   arms + entity defs`** — final cleanup once authoring is fully
   migrated.

If the project also wants ADR 0015 (tileset rendering) done,
recommend Actor unification first: it's purely runtime + schema
work that doesn't touch the renderer; tilesets touch the renderer
and benefit from having a stable actor schema underneath.

## Alternatives considered

**Keep NpcSpawn and EnemySpawn separate, layer an `Actor` trait
on top.** Rejected: doesn't simplify authoring (still two LDtk
entities), and the runtime split survives unchanged. The whole
point is to make the unification visible at every layer.

**Inline `aggression` as a single bool `hostile: true/false`.**
Rejected: the design doc explicitly calls out Wary as a distinct
beat (the calm-then-defensive Erdish moment). A bool can't model
"reacts to threats but won't initiate." The four-tier enum is the
minimum that captures the design intent.

[intro_handoff_to_next_agent]: see
  `docs/intro_handoff_to_next_agent.md` §5 (tech debt).
