//! General geometry and reference-frame primitives.
//!
//! This crate contains platformer-independent shapes, combat volumes, swing shapes,
//! and frame math. `Vec2` is re-exported from `bevy_math` and `Aabb` aliases Bevy's
//! `Aabb2d`; callers that need only those upstream math types should depend on
//! `bevy_math` directly.

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
