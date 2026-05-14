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

/// Gravity (px/s²) used by the falling-chest tick. Lighter than the
/// player's GRAVITY (2250) so a treasure chest reads as a heavy-but-
/// floaty drop, not a brick. Tuned by feel against the mockingbird
/// arena: at 1400 px/s² and 80 px of fall, the drop lands in ~0.34 s.
const CHEST_FALL_GRAVITY: f32 = 1400.0;
/// Terminal-velocity cap so a chest dropped from a tall arena doesn't
/// blast through the floor sweep before the sub-step kicks in.
const CHEST_FALL_MAX_SPEED: f32 = 900.0;

mod bosses;
mod breakables;
mod bus;
mod chests;
pub mod components;
mod ecs;
mod enemies;
mod events;
mod hazards;
mod npcs;
mod path_motion;
mod pickups;
mod runtime;
mod util;
mod world_overlay;

pub use bosses::{BossAttackProfile, BossBehaviorProfile, BossMovementProfile, BossRuntime};
pub use breakables::BreakableRuntime;
pub use bus::{
    apply_boss_damage_effects, apply_flag_effects, apply_gameplay_sfx_effects,
    apply_npc_strike_effects, apply_quest_effects, apply_switch_effects,
};
pub use chests::ChestRuntime;
pub use components::{
    BossRewardChest, BreakableFeature, ChestFeature, Collected, EncounterMob,
    EncounterRewardChest, FallingChest, FeatureAabb, FeatureId, FeatureName, Opened,
    PersistKey, PickupFeature, PogoTargetContributor, RespawnTimer, SandboxSolidContributor,
    StandTimer, SwitchFeature, SwitchOn,
};
pub use ecs::{
    apply_ecs_breakable_damage_queue, clear_encounter_reward_ecs, collect_ecs_pickups,
    despawn_encounter_mobs, ecs_breakable_state,
    ecs_chest_opened, ecs_damage_event_hits_actor, ecs_damage_event_hits_boss,
    ecs_damage_event_hits_breakable, ecs_actor_view_compat, ecs_boss_anim_state,
    ecs_boss_name, ecs_enemy_anim_state, ecs_enemy_sprite_override, ecs_feature_view,
    ecs_npc_anim_state, ecs_npc_name, interact_ecs_actors_and_switches, open_ecs_chests,
    rebuild_feature_ecs_world_overlay, reset_ecs_room_features, spawn_encounter_mob,
    spawn_room_feature_entities, sync_boss_reward_chests_ecs, sync_ecs_actors_with_save,
    sync_ecs_bosses_with_save, sync_ecs_switches_from_save, sync_encounter_reward_chests_ecs, update_ecs_actors,
    update_ecs_bosses, update_ecs_breakables, update_ecs_falling_chests, update_ecs_hazards,
    ActorDisposition, ActorRuntime, BossFeature, FeatureEcsWorldOverlay,
    FeatureSimEntity, HazardFeature, tick_gameplay_banner, apply_gameplay_banner_requests,
};
pub use enemies::{EnemyArchetype, EnemyRuntime};
pub use events::{
    DamageEvent, DamageReport, DamageSource, FeatureCombatTuning, FeatureEvents,
    FeaturePhysicsBurst, FeaturePhysicsCue, FeatureView, FeatureVisualKind,
    GameplayBanner, GameplayBannerRequested, GameplayEffect, NpcDialogueRequest, PogoBounceEvent,
    ResetRoomFeaturesEvent, PlayerDamageEvent, PlayerDamageMode, PlayerDamageSource,
};
pub use hazards::HazardRuntime;
pub use npcs::NpcRuntime;
pub use path_motion::PathMotion;
pub use pickups::PickupRuntime;
pub use world_overlay::world_with_sandbox_solids;

pub(super) use npcs::NPC_HOSTILE_STRIKE_THRESHOLD;
use util::*;

#[cfg(test)]
mod conversion_tests;
