# Projectiles and motion inputs

Projectiles and command-input abilities are reusable combat/movement primitives. The backend should make the verb expressible without baking in one specific content example.

## Current status

- `crates/ambition_sandbox/src/projectile/` owns the projectile vocabulary: `ProjectileKind`, `ProjectileSpec`, `ProjectileBody`, `ProjectileSpawner`, `FireballChargeTuning`, and `MotionInputBuffer`.
- The sandbox wires player Fireball / Hadouken / HadoukenSuper examples through `crates/ambition_sandbox/src/projectile/`.
- Projectile state is stored in `PlayerProjectileState` rather than spawning a Bevy entity per projectile. That keeps headless tests and trace output simple; presentation/VFX can still observe the resource.
- Projectile trace events map to `GameplayTraceEvent::Projectile { tick, kind, event, damage }` for `fired`, `blocked_by_resource`, `hit`, and `expired`.

## Implemented projectile tiers

| Variant | Input shape | Cost | Damage | Cooldown | Speed | Lifetime | Behavior |
|---|---|---:|---:|---:|---:|---:|---|
| Fireball | Press/hold/release Projectile without a recent motion gesture | 1.0 | 1 base, scaled by charge tier | 0.30 s | 360 px/s | 1.20 s | Mild downward arc; can bounce; charge affects size and damage. |
| Hadouken | Grace quarter-circle plus Projectile, currently `Down -> Right` or mirrored | 3.0 | 3 | 0.55 s | 520 px/s | 1.60 s | Straight shot; expires on solid contact. |
| HadoukenSuper | Full quarter-circle or half-circle plus Projectile | 5.0 | 5 | 0.85 s | 640 px/s | 1.80 s | Larger, stronger, more expensive straight shot. |

The default projectile meter is `ResourceMeter::new(8.0, 1.5, 0.0)`, so it caps at 8 resource units and regenerates at 1.5 units/second. Outgoing projectile damage is also multiplied by `user_settings.gameplay.player_damage_multiplier`.

## Fireball charge contract

`FireballChargeTuning::DEFAULT` defines three tiers:

- tier 0: release before 0.35 s,
- tier 1: hold for at least 0.35 s,
- tier 2: hold for at least 0.85 s.

Only Fireball uses charge tiers. `ProjectileSpec::with_charge_tier` stores the tier, increases hitbox size, and applies the current exponential damage ramp. Hadouken/HadoukenSuper ignore Fireball charge; motion gestures consume the press immediately and do not start a charge.

## Motion-input recognizer

`MotionInputBuffer` stores recent quantized directions in a short sliding window. The sandbox samples `ControlFrame.axis_x/y`, pushes directions into the buffer, and checks motion recognizers on the Projectile press edge.

Important ordering rule: check the most-specific motion first. The grace quarter-circle is a subsequence of the full quarter-circle, so the sandbox checks Super before weak Hadouken. This prevents a clean full input from being downgraded by the easier recognizer.

Run with `RUST_LOG=ambition_sandbox::projectile=info` to see press diagnostics: what the motion recognizer saw, which gesture matched, and why a press did or did not upgrade.

## Collision and surface behavior

Projectile collision is intentionally not identical to player movement, but it should follow the same authored surface vocabulary:

- `Solid` and `BlinkWall` block projectiles.
- Fireball bounces when the projectile lands on the top of a one-way platform.
- Side, ceiling, or out-of-budget contacts on one-way platforms pass through; one-way platforms are not general solid blockers from below or the sides.
- Solids are checked before one-way platforms when both overlap, matching the harder-surface priority used by player physics.
- Hadouken variants spawn with no bounce budget, so a solid hit expires them on first contact.

Keep `TODO.md`'s projectile one-way item open until this behavior is covered by a visible room/test that makes the expected bounce/pass-through rules obvious.

## Sandbox update shape

`update_projectiles` does the following:

1. Tick spawner cooldown/resource regeneration.
2. Consume this player's `ActionRequest::PlayerProjectileTick` from the brain/action stream, then sample its axis into the motion buffer.
3. Tick existing projectile bodies and resolve lifetime/surface hits.
4. On Projectile press: try motion-input upgrades first, otherwise start Fireball charging.
5. While held: accumulate Fireball charge time.
6. On release: spawn a charged Fireball if a Fireball charge was active.
7. Push trace events for spawn, resource block, hit, and expiry.

## Rules

- Separate command recognition from the effect that fires.
- Keep projectile simulation deterministic enough for tests.
- Presentation owns sprites, SFX, particles, and camera feedback.
- Authored unlocks and content-specific restrictions belong in sandbox content/progression code.
- Do not parse raw device input in projectile logic; consume `ActionRequest::PlayerProjectileTick` and the `MotionInputBuffer` abstraction.

## Validation anchors

```bash
cargo test -p ambition_sandbox --lib projectile
cargo test -p ambition_sandbox --lib combat
cargo test -p ambition_sandbox projectile
```

Useful code anchors:

```text
crates/ambition_sandbox/src/projectile/
crates/ambition_sandbox/src/projectile/
crates/ambition_sandbox/src/projectile/systems.rs
```
