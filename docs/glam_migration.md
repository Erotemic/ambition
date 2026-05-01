# Glam migration

The engine now uses `glam::Vec2` directly instead of maintaining an Ambition-owned vector type.

This keeps the reusable `ambition_engine` crate Bevy-independent while still sharing the same battle-tested math stack that Bevy uses internally. Bevy 0.18's `bevy_math` depends on `glam 0.30.7`, so the engine pins the same compatible `glam` line and re-exports `ambition_engine::Vec2` for downstream code.

The former `math.rs` module was removed. Ambition-specific scalar helpers that do not belong to glam, such as `approach`, now live in `scalar.rs`.

## Why not use `bevy::math::Vec2` in the engine?

`bevy::math::Vec2` is a Bevy re-export of glam's vector type. Using `glam::Vec2` directly gives the engine the same vector semantics without making the core simulation crate depend on Bevy's app, ECS, renderer, or feature graph. The sandbox can still use Bevy's `Vec2` alias for rendering-facing code; the types resolve to the same glam vector version when dependency versions are aligned.

## Rules

- Use `ambition_engine::Vec2` or `glam::Vec2` for simulation-space vectors.
- Do not add new handwritten vector, rectangle, angle, matrix, or interpolation primitives unless there is a clear gameplay-specific semantic wrapper.
- Prefer `glam` for vector math, `parry2d` for geometry queries, Bevy Gizmos for debug drawing, and data-driven specs for tunable values.
