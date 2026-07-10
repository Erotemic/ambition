//! Reusable, content-free platformer runtime primitives.
//!
//! This crate owns shared body, gravity, projectile, transit, lifecycle, and
//! schedule seams without depending on `ambition_actors`, content,
//! presentation, app assembly, or devtool modules.

pub mod body;
pub mod camera_ease;
pub mod camera_layers;
pub mod class_b;
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

pub mod feature_overlay;

pub mod physics;

pub mod shrine;
