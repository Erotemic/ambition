//! Reusable, content-free platformer runtime primitives.
//!
//! This crate owns shared body, gravity, projectile, transit, lifecycle, and
//! schedule seams without depending on `ambition_platformer2d_actor_monolith`, content,
//! presentation, app assembly, or devtool modules.
//!
//! # WHEN something belongs here — the admission rule
//!
//! the sentence above is a DEPENDENCY refusal, and it is not enough.
//! Nothing that respects those edges is turned away by it, so a crate named
//! *shared tangle* will accept anything awkward to place — which is how a
//! tangle becomes one. This is the missing half: a
//! destination that states only what it cannot depend on still accepts
//! everything else.
//!
//! A TYPE LIVES HERE BECAUSE TWO DOMAINS SHARE IT AND THE ORPHAN RULE
//! FORBIDS IT LIVING IN EITHER. The rule was already written down — on one
//! type, where nobody placing a second one would look. `MountDied` (`body.rs`)
//! says it: *"it lives HERE, below the domains, because two of them share it.
//! The writer is the mount coupling in the actor monolith and the reader is
//! `ambition_boss_encounter`; a message owned by one of the two would make the
//! other depend on it for a type carrying nothing but a pair of entities."*
//! `FeatureInteractionSet` is here for the same reason — so a carved module can
//! name the ordering it participates in.
//!
//!  the test is TWO REAL CONSUMERS IN DIFFERENT DOMAINS, today. Not "this
//! is generic", not "something might share it later", and not "it was awkward
//! where it was". A type with one consumer belongs in that consumer's crate; a
//! type with two consumers in the SAME domain belongs in that domain. moving
//! something here to unblock a carve, without a second domain that reads it,
//! launders the debt rather than paying it — the carve looks finished and the
//! concept is now split across two crates instead of one.

pub mod authored_logic;
pub mod app_finalization;
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
pub mod schedule;
pub mod time;
pub mod transit;
pub mod world_log;

pub mod feature_kind;
pub mod block_nudge;
pub mod feature_overlay;

pub mod held_item_art;
pub mod world_item_art;

pub mod physics;

pub mod shrine;

/// The ONE identity vocabulary for snapshot / replay / netcode (N3.1).
pub mod sim_id;
mod snapshot_impls;

// Domain-owned rollback declaration; the host supplies the backend registrar.
mod rollback_registration;
pub use rollback_registration::register_rollback_state;
