# Projectiles and motion inputs

Projectiles and command-input abilities are reusable combat/movement primitives. The backend should express the verb without baking in one content example.

## Current status

- `crates/ambition_actors/src/projectile/` owns Ambition's projectile vocabulary and player command-input handling.
- `ambition_platformer_primitives::projectile` owns reusable projectile body/gameplay pieces.
- In-flight projectiles are ECS entities, not Vec entries in player/enemy resources.
- Player Fireball / Hadouken / HadoukenSuper examples are wired through the sandbox projectile systems.
- Projectile trace events report fire, resource block, hit, and expiry events.

## Implemented projectile tiers

| Variant | Input shape | Cost | Damage | Cooldown | Speed | Lifetime | Behavior |
|---|---|---:|---:|---:|---:|---:|---|
| Fireball | Press/hold/release Projectile without a recent motion gesture | 1.0 | 1 base, scaled by charge tier | 0.30 s | 360 px/s | 1.20 s | Mild downward arc; can bounce; charge affects size and damage. |
| Hadouken | Grace quarter-circle plus Projectile, currently `Down -> Right` or mirrored | 3.0 | 3 | 0.55 s | 520 px/s | 1.60 s | Straight shot; expires on solid contact. |
| HadoukenSuper | Full quarter-circle or half-circle plus Projectile | 5.0 | 5 | 0.85 s | 640 px/s | 1.80 s | Larger, stronger, more expensive straight shot. |

The default projectile meter caps at 8 resource units and regenerates at 1.5 units/second. Outgoing projectile damage also respects `user_settings.gameplay.player_damage_multiplier`.

## Fireball charge contract

`FireballChargeTuning::DEFAULT` defines three tiers:

- tier 0: release before 0.35 s,
- tier 1: hold for at least 0.35 s,
- tier 2: hold for at least 0.85 s.

Only Fireball uses charge tiers. Motion gestures consume the press immediately and do not start a Fireball charge.

## Motion-input recognizer

`MotionInputBuffer` stores recent quantized directions in a short sliding window. The sandbox samples the current control axis, pushes directions into the buffer, and checks motion recognizers on the Projectile press edge.

Ordering rule: check the most-specific motion first. The grace quarter-circle is a subsequence of the full quarter-circle, so Super must be checked before weak Hadouken.

## Collision and surface behavior

Projectile collision follows the authored surface vocabulary but is intentionally simpler than player movement:

- `Solid` and `BlinkWall` block projectiles.
- Fireball bounces when it lands on the top of a one-way platform.
- Side, ceiling, or out-of-budget contacts on one-way platforms pass through.
- Solids are checked before one-way platforms when both overlap.
- Hadouken variants spawn with no bounce budget, so a solid hit expires them on first contact.
- Portal transit is entity-based; tests should use live authored portal pairs rather than hard-coded channel colors.

## Update shape

Projectile systems:

1. tick spawner cooldown/resource regeneration;
2. consume the player's projectile action request and motion axis;
3. tick existing projectile entities and resolve lifetime/surface hits;
4. on Projectile press, try motion-input upgrades before starting Fireball charge;
5. accumulate Fireball charge while held;
6. release a charged Fireball if a charge was active;
7. emit trace/events for diagnostics and tests.

## Rules

- Separate command recognition from the effect that fires.
- Keep projectile simulation deterministic enough for tests.
- Presentation owns sprites, SFX, particles, and camera feedback.
- Authored unlocks and content restrictions belong in sandbox content/progression code.
- Do not parse raw device input in projectile logic; consume semantic action/control state.

## Validation anchors

```bash
cargo test -p ambition_actors --lib projectile
cargo test -p ambition_actors projectile
cargo test -p ambition_app --test projectile_portal_transit --features "rl_sim portal"
```

Useful anchors:

```text
crates/ambition_actors/src/projectile/
crates/ambition_platformer_primitives/src/projectile/
```
