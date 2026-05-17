# Projectiles and motion inputs

Projectiles and command-input abilities are reusable combat/movement primitives. The backend should make the verb expressible without baking in one specific content example.

## Current status

- `ambition_engine` has projectile/combat vocabulary for directional intents and projectile behavior.
- The sandbox wires Fireball and Hadouken-style motion-input examples through input, player state, presentation, and content rules.
- Keep motion-input parsing testable without requiring the visible app.

## Rules

- Separate command recognition from the effect that fires.
- Keep projectile simulation deterministic enough for tests.
- Presentation owns sprites, SFX, particles, and camera feedback.
- Authored unlocks and content-specific restrictions belong in sandbox content/progression code.

## Validation anchors

```bash
cargo test -p ambition_engine projectile
cargo test -p ambition_engine combat
cargo test -p ambition_sandbox projectile
```
