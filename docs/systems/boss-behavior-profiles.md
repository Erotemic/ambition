# Boss behavior profiles

Bosses should scale by adding authored profiles, not by growing named branches in
runtime systems.

A boss profile owns the full content-facing bundle for a fight:

- encounter progression (`BossEncounterSpec`)
- sandbox movement/contact behavior (`BossBehaviorProfile`)
- attack hitbox vocabulary and timing
- music ids
- death reward behavior

The engine encounter state machine remains responsible for deterministic phase
progression and HP thresholds. The sandbox profile layer decides how a live
`BossRuntime` actually moves, damages the player, and pays out rewards.

## Current profiles

- `clockwork_warden` uses the old gradient-sentinel encounter tuning with a
  grounded/hovering `AnchorSway` movement profile.
- `mockingbird` uses a wider `AirSwoop` behavior profile and a data-declared
  `DropChest` reward.

## Direction

Future bosses should add or extend profiles instead of adding more `if boss_id ==
...` branches. Useful next profile fields include:

- per-phase movement overrides
- vulnerability windows
- attack projectiles / summons
- arena anchors and hazards
- cutscene hooks
- reward chains

The short-term implementation still uses `BossRuntime` as the live physics-ish
actor, but the profile now gives us a place to move toward per-boss behavior
without changing the generic encounter machine.

## Brain→sim seam

As of 2026-05-21 (commit `66c8b0b`), `BossRuntime::update` runs through
the same `ActorControlFrame` seam every other actor uses:

1. **BRAIN** — `build_control_frame` consults
   `BossMovementProfile::target` plus the apple-rain dodge layer and a
   world-bounds clamp to produce a `desired_vel`.
2. **INTEGRATION** — a uniform `step_kinematic` call with `gravity=0`
   integrates the velocity against world solids. The bespoke
   `move_toward_target` + `boss_space_is_free` collision path is
   deleted. Multi-part bosses still collide against `combat_size`.
3. **EFFECTS** — `Cycle` / `Scripted` attack-pattern timers and the
   apple-rain spawn tick run unchanged.

Practical consequence for new profiles: the brain owns "where do I want
to be this tick" (sway, dodge, chase math, world-bounds clamp); the
simulation half guarantees collision. Profiles should keep their
`movement` math velocity-space-friendly (a target the brain converts to
`desired_vel`) rather than asking for position-space teleports.

See `docs/systems/character-ai-refactor.md` for the parallel enemy
migration and `crates/ambition_sandbox/src/actor_control.rs` for the
shared frame definition.

## Universal-brain shadow (2026-05-24)

Bosses now also carry a sandbox-side
`Brain::StateMachine(BossPattern{encounter_id})` + `ActionSet`
shadow alongside `BossRuntime`. The shadow runs each frame, fills
the actor's `ActorControl` frame, and the `emit_brain_action_messages`
resolver writes `ActorActionMessage`s for each resolved
`ActionRequest`. The default boss `ActionSet` carries a `Bolt`
ranged + `BossSpotlight` special so a possessed-then-released boss
has an offensive baseline; per-encounter overrides land during the
daytime EFFECTS-flip.

Today the messages aren't consumed by combat spawners — the
`BossRuntime` apple-rain / scripted-pattern path still drives
behavior. Daytime work threads each boss's encounter id through
`BossPattern.tick` to drive the phase schedule from the brain, then
flips combat spawns to read `ActorActionMessage`. See
`docs/systems/brain-driver.md` and
`docs/recipes/extending-brains-and-action-sets.md` (Daytime
EFFECTS-consumer flip — concrete procedure).

## Charged fireball test tuning

For rapid boss-battle iteration, charged fireball damage now ramps
exponentially: tier 0 is 1x, tier 1 is 4x, and tier 2 is 16x. This is intended
as a playtest accelerator so fully charged shots can quickly push bosses through
phase transitions while the richer boss system is being built.
