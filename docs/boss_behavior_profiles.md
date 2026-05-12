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

## Charged fireball test tuning

For rapid boss-battle iteration, charged fireball damage now ramps
exponentially: tier 0 is 1x, tier 1 is 4x, and tier 2 is 16x. This is intended
as a playtest accelerator so fully charged shots can quickly push bosses through
phase transitions while the richer boss system is being built.
