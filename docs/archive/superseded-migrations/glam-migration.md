# Archived: glam-migration.md

Superseded migration or transition note. Preserve as historical evidence; do not treat as current procedure.

Original path: `docs/recipes/glam-migration.md`

---

# Bevy math migration

The engine no longer owns a bespoke vector type, and it no longer depends on `glam` directly. Instead, `ambition_engine::Vec2` re-exports `bevy_math::Vec2`, the same vector type Bevy uses internally.

This keeps simulation-space math aligned with Bevy without forcing sandbox or story crates to translate between vector types. It also makes future use of Bevy reflection, bounding volumes, gizmos, and inspector tooling easier because the engine is now Bevy-math-native.

Rules of thumb:

- Use `ambition_engine::Vec2` for engine-facing simulation code.
- Use `ambition_engine::Aabb`, which is Bevy's `Aabb2d`, for rectangular bounds.
- Use `ambition_engine::AabbExt` only for Ambition-specific semantics such as strict platformer overlap and swept collision.
- Use `parry2d` for lower-level collision queries instead of adding more bespoke geometry math.
