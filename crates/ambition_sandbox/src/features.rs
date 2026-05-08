//! Runtime feature probes for the basement sandbox rooms.
//!
//! The engine owns the reusable data vocabulary. This module is deliberately a
//! sandbox-side adapter: it turns authored `World::objects` into a small playable
//! proving ground for hazards, enemies, bosses, breakables, pickups, chests, and
//! NPC interactions without committing final production behavior yet.
//!
//! Implementation is split by gameplay domain so future LLM passes can load the
//! area they are changing without dragging the entire feature runtime into
//! context. Keep `features.rs` as a facade: public types are re-exported here,
//! while domain logic lives in `features/*.rs`.

use ambition_engine as ae;
use ambition_engine::AabbExt;
use bevy::prelude::*;

use crate::platforms::MovingPlatformState;

const ENEMY_GRAVITY: f32 = 1450.0;
const ENEMY_MAX_FALL: f32 = 760.0;
const ENEMY_PATROL_SPEED: f32 = 105.0;
const ENEMY_CHASE_SPEED: f32 = 155.0;
const ENEMY_ATTACK_RANGE: f32 = 150.0;
const ENEMY_ATTACK_COOLDOWN: f32 = 1.05;
const BOSS_ATTACK_COOLDOWN: f32 = 1.35;
const BREAK_ON_STAND_SECONDS: f32 = 0.85;

mod bosses;
mod breakables;
mod bus;
mod chests;
mod enemies;
mod events;
mod hazards;
mod npcs;
mod path_motion;
mod pickups;
mod runtime;
mod util;
mod world_overlay;

pub use bosses::BossRuntime;
pub use breakables::BreakableRuntime;
pub use bus::{
    apply_save_to_features, drain_feature_event_bus, sync_features_with_save, FeatureEventBus,
};
pub use chests::ChestRuntime;
pub use enemies::{EnemyArchetype, EnemyRuntime};
pub use events::{
    DamageEvent, DamageReport, DamageSource, FeatureCombatTuning, FeatureEvents,
    FeaturePhysicsBurst, FeaturePhysicsCue, FeatureView, FeatureVisualKind, NpcDialogueRequest,
    PlayerDamageEvent, PlayerDamageMode, PlayerDamageSource,
};
pub use hazards::HazardRuntime;
pub use npcs::NpcRuntime;
pub use path_motion::PathMotion;
pub use pickups::PickupRuntime;
pub use runtime::{FeatureRuntime, SwitchRuntime};
pub use world_overlay::world_with_sandbox_solids;

pub(super) use npcs::NPC_HOSTILE_STRIKE_THRESHOLD;
use util::*;

#[cfg(test)]
mod conversion_tests;
