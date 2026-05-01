# Core / Bevy Boundary

Ambition currently has two layers:

- `ambition_engine`: reusable, mostly deterministic game rules.
- `ambition_sandbox`: Bevy-powered presentation, input plumbing, room experiments, effects, audio, and debug UI.

This split is still useful, but `ambition_engine` should not try to be a second Bevy. Bevy is the host engine. Ambition's core should be the portable rules library that can be tested without opening a window.

## What belongs in `ambition_engine`

Keep logic here when it should be headless-testable or shared by multiple Ambition games/sandboxes:

- player movement state and movement tuning;
- ability toggles and compatibility rules;
- collision geometry and room/world primitives;
- blink resolution and movement-theorem-style verbs;
- enemy/dummy state and collision rules;
- combat hitboxes and damage/knockback semantics;
- simulation events emitted for the presentation layer;
- tests for movement, collision, blink, room safety, and enemy behavior.

## What belongs in `ambition_sandbox`

Keep logic here when it is presentation, tooling, or experiment-specific:

- Bevy plugins, systems, resources, and entities;
- rendering and debug overlays;
- audio playback and particles;
- keyboard/gamepad bindings;
- temporary room graph experiments;
- UI/HUD text;
- transition visuals and camera behavior.

## Vector type direction

The engine used to have a home-grown `Vec2`. That was useful during the first backend-neutral prototype, but it has now been removed.

A better direction is to migrate the core crate to `glam::Vec2` directly:

- Bevy already uses glam internally, so conversion friction goes down.
- `glam::Vec2` is lightweight and not Bevy-specific.
- It gives us a mature vector implementation instead of maintaining our own math type.
- The engine can remain backend-neutral by depending on `glam`, not `bevy`.

Recommended future refactor:

1. Add `glam` as a dependency of `ambition_engine`.
2. Done: `ambition_engine::Vec2` now re-exports `glam::Vec2` directly.
3. Done: custom vector helper methods were removed in favor of glam native methods such as `normalize_or`.
4. Keep `approach()` and other game-feel helpers in `scalar.rs`.
5. Add tests before/after the migration so movement behavior does not drift.

This was done as a dedicated pass because `Vec2` is used across nearly every subsystem.

## Why keep an engine/core crate at all?

The useful abstraction is not an engine in the Unity/Godot/Bevy sense. It is a reusable game-rules core. That core lets us:

- run headless unit tests;
- validate AI-generated room/ability specs without graphics;
- replay deterministic movement traces;
- keep gameplay behavior independent of visual effects;
- build multiple sandboxes or games from the same movement/ability toolkit.

If the name `ambition_engine` starts to imply too much, consider renaming later to `ambition_core` and adding a separate `ambition_bevy` adapter crate.
