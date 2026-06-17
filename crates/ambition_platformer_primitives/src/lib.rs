//! Reusable, content-free platformer runtime primitives.
//!
//! This crate owns shared body, gravity, projectile, transit, lifecycle, and
//! schedule seams without depending on `ambition_sandbox`, content,
//! presentation, app assembly, or devtool modules.

pub mod body;
pub mod gravity;
pub mod kinematic;
pub mod lifecycle;
pub mod markers;
pub mod math;
pub mod orientation;
pub mod prelude;
pub mod projectile;
pub mod schedule;
pub mod time;
pub mod transit;
pub mod world_query;
