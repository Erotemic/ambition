//! Reusable, content-free platformer runtime primitives.
//!
//! This crate owns shared body, gravity, projectile, transit, lifecycle, and
//! schedule seams without depending on actor-monolith, content, presentation,
//! app assembly, or devtool modules.
//!
//! A type belongs here only when at least two distinct domains consume it and
//! neither can own it without creating the wrong dependency edge. Being generic,
//! potentially reusable, or awkward to place is not sufficient.

pub mod app_finalization;
pub mod authored_logic;
pub mod binding;
pub mod body;
pub mod camera_ease;
pub mod camera_layers;
pub mod class_b;
pub mod construction;
pub mod developer_hotkeys;
pub mod frame_env;
pub mod gameplay_presentation;
pub mod gravity;
pub mod lifecycle;
pub mod markers;
pub mod math;
pub mod orientation;
pub mod prelude;
pub mod projectile;
pub mod safe_position;
pub mod schedule;
pub mod time;
pub mod transit;
pub mod world_log;

pub mod block_nudge;
pub mod feature_kind;
pub mod feature_overlay;

pub mod held_item_art;
pub mod world_item_art;

pub mod physics;

pub mod shrine;

/// The ONE identity vocabulary for snapshot / replay / netcode (N3.1).
pub mod sim_id;
mod snapshot_impls;
/// Whether an autonomous body is currently masked by a transient controller.
///
/// ⭐ HERE FOR THE REASON `body::Mass` AND `body::MountDied` ARE: TWO domains
/// share it — possession and the mount pair — and neither owns the other. It was
/// in the actor monolith's `features`, which made "who controls this body across
/// time" read as a features fact.
pub mod temporary_control;

// Domain-owned rollback declaration; the host supplies the backend registrar.
mod rollback_registration;
pub use rollback_registration::register_rollback_state;
