//! Generic combat kit — the content-free half of the feature-ECS world.
//!
//! Extracted from `content/features`: the component
//! vocabulary, gameplay messages/buses, hitbox lifecycle, pickups,
//! chests, breakables, hazards, target selection, path motion, and the
//! collision world-overlay. Everything here is reusable platformer
//! combat machinery: **no named Ambition content** (no archetype enums,
//! no boss ids) — that stays in `content/features`, which consumes this
//! kit and re-exports it so inbound `crate::features::…` paths are
//! unchanged.
//!
//! The named actor layer (`ActorConfig`/archetype-coupled systems:
//! actors / damage / mount / spawn / save_sync) intentionally remains
//! content-side until the archetype hub is dissolved into capability
//! components; see `docs/current/state.md` for the crate split.

use crate::engine_core as ae;
use crate::engine_core::AabbExt;
use bevy::prelude::*;

use crate::audio::SfxMessage;
use crate::platformer_runtime::lifecycle::RoomVisual;
use crate::world::physics::{DebrisBurstMessage, PhysicsDebrisCue};
use crate::world::platforms::MovingPlatformState;
use crate::WorldTime;
use ambition_vfx::vfx::{ParticleKind, VfxMessage};

/// Seconds a player must stand on a breakable before it shatters.
const BREAK_ON_STAND_SECONDS: f32 = 0.85;

/// Gravity (px/s²) used by the falling-chest tick. Lighter than the
/// player's GRAVITY (2250) so a treasure chest reads as a heavy-but-
/// floaty drop, not a brick. Tuned by feel against the mockingbird
/// arena: at 1400 px/s² and 80 px of fall, the drop lands in ~0.34 s.
const CHEST_FALL_GRAVITY: f32 = 1400.0;
/// Terminal-velocity cap so a chest dropped from a tall arena doesn't
/// blast through the floor sweep before the sub-step kicks in.
const CHEST_FALL_MAX_SPEED: f32 = 900.0;

pub mod banner;
pub mod boss_clusters;
pub mod breakables;
pub mod bus;
pub mod chests;
pub mod components;
pub mod events;
pub mod falling_chest;
pub mod hazard_runtime;
pub mod hazards;
pub mod held_items;
pub mod hitbox;
pub mod overlay;
pub mod path_motion;
pub mod pickups;
pub mod spawn_static;
pub mod targeting;
pub mod util;
pub mod variation;
pub mod world_overlay;

// The pure combat MODEL (Damage / Hitbox / AttackSpec / DamageVolume / slots) is
// the reusable `ambition_combat` foundation crate, re-exported here so the whole
// combat surface — model + the ECS systems below — lives under one `crate::combat`
// namespace (`crate::combat::AttackSpec` and `crate::combat::hazards` both resolve).
pub use ambition_combat::*;

pub use components::*;
pub use events::*;
// `FeatureSimEntity` is the generic entity-marker queried by the reusable
// mechanics; its definition lives DOWN in
// `ambition_platformer_primitives::markers` (ADR 0019).
pub use ambition_platformer_primitives::markers::FeatureSimEntity;
pub use hazard_runtime::*;
pub use path_motion::*;
pub use world_overlay::*;
