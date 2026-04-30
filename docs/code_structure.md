# Ambition code structure notes

This document records the first extensibility pass after the Bevy port. The goal
was not to perfect the architecture, but to remove the most obvious hard-coded
pressure points so the sandbox can keep evolving without turning into one large
file.

## What changed

`ambition_sandbox/src/main.rs` was split into focused modules:

- `config.rs` — window size, z layers, grid spacing, and world-to-Bevy coordinate conversion.
- `input.rs` — generic action names, keyboard presets, gamepad semantic mapping, and `ControlFrame`.
- `dummies.rs` — sandbox dummy/enemy test fixtures and respawn behavior.
- `audio.rs` — procedural sound specs, WAV generation, `SoundBank`, and playback helper.
- `fx.rs` — particles, impact rings, slash previews, and reset effects.
- `rendering.rs` — render-only Bevy components, grid/block spawning, and visual state sync.
- `main.rs` — Bevy app wiring, high-level sandbox update flow, HUD text, and attack orchestration.

This keeps the current behavior close to the working Bevy prototype while making
future changes more local.

## Remaining hard-coded areas

These are intentionally still simple, but they are the next things to extract
when they become painful:

1. **Room generation**
   - `ambition_engine::build_endgame_sandbox()` still directly defines the test room.
   - Next step: represent rooms as `SandboxSpec` or `RoomSpec` data structures so the engine can load/generated multiple rooms.

2. **Movement constants**
   - `ambition_engine` has `MovementTuning`, but the default constants are still compiled in.
   - Next step: allow a tuning preset resource and hot reload from RON/TOML.

3. **Attack definitions**
   - `slash_hitbox()` is still in `main.rs` and hard-codes hitbox sizes.
   - Next step: create `combat.rs` with `AttackSpec`, `AttackEvent`, and collision queries.

4. **Dummy behavior**
   - Dummies are still test fixtures, not real enemies.
   - Next step: add `EnemySpec` and simple behavior states once combat needs more than sandbags.

5. **Audio backend**
   - The audio module generates WAV bytes for Bevy audio. This is good enough for now.
   - Next step: move symbolic sound specs into `ambition_audio` if/when we adopt Kira/CPAL.

6. **Particle backend**
   - Particles are CPU-side sprite entities.
   - Next step: keep the public calls (`spawn_burst`, `spawn_impact`, etc.) and swap the backend to GPU particles later if needed.

7. **Input remapping persistence**
   - Presets are code-defined.
   - Next step: make presets serializable and load user bindings from a config file.

## Design rule

The sandbox should remain a backend adapter around `ambition_engine`, not a second engine.
When adding a feature, prefer this direction:

```text
symbolic/gameplay state -> ambition_engine or data spec
backend representation   -> ambition_sandbox Bevy modules
```

That means movement, collision, story-state, and generation rules should migrate
toward backend-neutral crates over time, while `ambition_sandbox` remains the
playable Bevy shell.

## Debug overlays

The Bevy port restores debug drawing through `crates/ambition_sandbox/src/debug_overlay.rs`.
This module is deliberately presentation-only: it reads `SandboxRuntime` and `ambition_engine`
state, then draws Bevy gizmos for body boxes, velocity/facing vectors, contact normals,
attack hitboxes, dummy HP bars, room bounds, and rebound impulse arrows.

Keep this layer out of `ambition_engine`. The engine should expose deterministic state and
events; the Bevy adapter decides which vectors/boxes are useful for tuning.
