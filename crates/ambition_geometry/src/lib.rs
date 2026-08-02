//! **Shapes, boxes and reference frames — the part of the engine that is not
//! about platformers.**
//!
//! Carved out of `ambition_platformer2d_core` (2026-08-01) because the census in
//! `scripts/core_import_census.py` showed general-named crates taking a
//! genre-named dependency for things with no genre in them. `ambition_vfx`
//! wanted exactly four items — `Vec2`, `Aabb`, `CombatVolume`, `VolumeShape` —
//! and nothing platformer-specific at all.
//!
//! ## Why THESE four modules, and why they were the right first carve
//!
//! They are already self-contained, which was checked rather than assumed:
//!
//! ```text
//! geometry.rs        bevy_math, parry2d, Vec2
//! combat_volume.rs   parry2d, Aabb, AabbExt, Vec2
//! volume_shape.rs    AccelerationFrame, CombatVolume, Vec2
//! reference_frame.rs Vec2
//! swing_shape.rs     AabbExt, CombatVolume, Vec2   (merged from `shaped-volumes`)
//! ```
//!
//! No rooms, no blocks, no ledges, no portals — and every trait they implement
//! is `Default`, `Deref` or `From`, so nothing moved into an orphan-rule
//! problem (which is what adjudicates crate placement here, not taste).
//!
//! ⚠ **and unlike the other general thing still stuck in the core crate — the
//! snapshot codec — none of this moves the ROLLBACK FINGERPRINT.** The
//! fingerprint hashes `std::any::type_name`, so relocating a snapshot type
//! rewrites the schema baseline; that is the S30 fork, and it is Jon's to
//! decide. This carve carries no such cost, which is why it went first.
//!
//! ## `Vec2` and `Aabb` are not ours either
//!
//! `Vec2` is `bevy_math`'s, re-exported; `Aabb` is an alias for its `Aabb2d`.
//! Half the reason the old dependency looked load-bearing was that consumers
//! reached *through* a platformer crate for a maths type. Anything that needs
//! only those should depend on `bevy_math` directly and skip this crate too.

pub mod combat_volume;
pub mod geometry;
pub mod reference_frame;
pub mod swing_shape;
pub mod volume_shape;

pub use bevy_math::Vec2;
pub use combat_volume::CombatVolume;
pub use geometry::{aabb_from_min_size, Aabb, AabbExt, CenteredAabb};
pub use reference_frame::{
    AccelerationFrame, ControlFrameModes, GameplayFramePolicy, InputFrameMode, LocalAxes,
    MotionFrame, RawDirectionEdges, ResolvedControlFrame, ScreenAxes, WorldVec2,
};
pub use swing_shape::SwingShape;
pub use volume_shape::{VolumeShape, DUMMY_HALF};
