# Bevy ECS stale-component visibility after removing a sync system

**Date:** 2026-05-15  
**Tags:** `bevy-resource`, `architecture-seam`, `game-input`, `cross-system-signal`, `bevy-0.18`  
**Prompt level:** A (pre-error operation)

---

## Background

The Ambition ECS player migration (`c3791bb`) flipped `PlayerMovementAuthority`
to be the per-frame authority for player movement. As part of that commit, two
book-keeping systems were removed from the schedule:

- `sync_runtime_from_player_entity` — read ECS components, wrote `SandboxRuntime`
- `sync_player_entity_from_runtime` — read `SandboxRuntime`, wrote ECS components

The goal was correct: these systems maintained a two-way mirror that should go
away once the authority is settled. The runtime's shadow copy is now written at
the end of `sandbox_update` via `runtime.player = player.clone()`.

**What was missed:** Several ECS components on the player entity (`PlayerBody`,
`PlayerCombatState`, `PlayerInteractionState`) are queried by *downstream*
systems (rendering, hazards, interaction). These components were **only updated
by `sync_player_entity_from_runtime`**. Removing that system left them frozen at
their spawn-time values for the rest of the game session.

The rendering and camera systems have a fallback:

```rust
let player_body = player.single().copied().unwrap_or_else(|_| {
    PlayerBody::from_player(&runtime.player)
});
```

But the fallback only fires when `player.single()` **fails**. If the component
exists on the entity (just stale), `single()` succeeds and the stale value is
used. The fallback is not a freshness guard.

---

## Observable symptoms

- Player collision box moves; sprite and camera stay at spawn position.
- No invincibility frames after being hit (player takes damage every frame while
  inside a hazard's AABB).
- Chest and NPC interaction never triggers (interact buffer never reads as filled).

---

## Level A prompt

You are doing Chunk B + C of the Ambition ECS player migration. The prior commit
(`c3791bb`) made `PlayerMovementAuthority` the per-frame authority for player
movement and removed the two-way sync pair
(`sync_player_entity_from_runtime` / `sync_runtime_from_player_entity`) from the
schedule. The runtime shadow (`runtime.player`) is now written once at frame end
inside `sandbox_update`.

Here is the relevant shape of the system schedule for `SandboxSet::CoreSimulation`:

```
update_ecs_hazards          ← queries PlayerBody + PlayerCombatState
update_ecs_actors
update_ecs_bosses
sandbox_update              ← writes runtime.player, runtime.damage_invuln_timer,
                              runtime.interact_buffer_timer, etc.
reset_ecs_room_features
update_projectiles
apply_feature_damage_events
```

And in `SandboxSet::FeatureInteraction` (runs after CoreSimulation):

```
interact_ecs_actors_and_switches  ← queries PlayerBody + PlayerInteractionState
open_ecs_chests                   ← queries PlayerBody + PlayerInteractionState
```

And in the presentation schedule (runs after FeatureInteraction):

```
sync_visuals       ← queries Option<&PlayerBody>, Option<&PlayerCombatState>
camera_follow      ← queries &PlayerBody
```

`PlayerBody`, `PlayerCombatState`, and `PlayerInteractionState` are distinct ECS
components on the player entity. They are populated at entity spawn time and are
intended to be updated each frame. After the sync pair removal they are no longer
updated.

**Question:** What minimal system (or extension to an existing one) should be
added, where in the schedule should it run, and what invariant must it maintain
so that all downstream readers see correct per-frame values?

---

## Expected answer

Add a single system that runs **after `sandbox_update`** (and therefore after
`apply_feature_damage_events`) but **before** the FeatureInteraction and
presentation sets:

```rust
pub fn write_player_ecs_components(
    runtime: Res<SandboxRuntime>,
    mut players: Query<
        (
            &PlayerMovementAuthority,
            &mut PlayerBody,
            &mut PlayerCombatState,
            &mut PlayerInteractionState,
        ),
        With<PlayerEntity>,
    >,
) {
    let Ok((authority, mut body, mut combat, mut interaction)) = players.single_mut() else {
        return;
    };
    *body = PlayerBody::from_player(&authority.player);
    *combat = PlayerCombatState::from_runtime(&runtime);
    *interaction = PlayerInteractionState::from_runtime(&runtime);
}
```

Register it at the tail of the `CoreSimulation` chain:

```rust
.add_systems(
    Update,
    (
        // ... existing systems ...
        apply_feature_damage_events,
        write_player_ecs_components,   // ← Chunk B + C
    )
        .chain()
        .in_set(SandboxSet::CoreSimulation),
)
```

**Key invariants:**
- `update_ecs_hazards` runs in the *same* frame as `write_player_ecs_components`
  but *before* `sandbox_update`. It therefore reads last frame's `PlayerCombatState`,
  which is correct: invuln granted by a hit in frame N is reflected in frame N+1.
- `interact_ecs_actors_and_switches` and `open_ecs_chests` run *after*
  `write_player_ecs_components` (in FeatureInteraction), so they see this
  frame's interaction buffer state.
- Do **not** write `PlayerCombatState` from `PlayerMovementAuthority` — those
  fields (`flash_timer`, `damage_invuln_timer`, etc.) live in `SandboxRuntime`,
  not in `ae::Player`.

---

## Validation

```bash
~/.cargo/bin/cargo build -p ambition_actors
~/.cargo/bin/cargo test -p ambition_actors --lib
# Expect: 445 passed, 0 failed
```

Runtime check: move the player into a hazard — invuln frames should apply
(second hit only after `damage_invuln_timer` expires). Press interact near an
NPC or chest — dialogue / open should trigger.

---

## Why this is a good candidate

The mistake is tempting: the agent correctly removed the sync pair and correctly
wrote `runtime.player` at frame end — but missed that ECS component consumers
use `query.single()` and only fall back to `runtime` on query *failure*, not on
stale values. The component exists, so no fallback fires, and the stale data
silently drives game behavior. The bug is invisible to the compiler and to tests
that don't exercise the rendering/interaction path.
