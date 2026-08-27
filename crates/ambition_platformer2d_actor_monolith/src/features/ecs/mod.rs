//! ECS backbone of the actor / world-object simulation.
//!
//! Despite the `features/` name, this is NOT a set of toggleable feature
//! slices — it is the enemy / NPC / boss ACTOR SIMULATION plus the authored
//! room objects they share a world with (pickups, chests, breakables,
//! switches, hazards, mounts). Every one is a Bevy entity spawned and ticked
//! here; this is the authoritative implementation.
//!
//! Module map (each sibling owns one slice of that sim):
//! - `spawn*` — spawn authored room objects, encounter mobs, mounts/riders;
//! - `actors` / `actor_clusters` / `bosses` — the per-frame actor tick over the
//!   unified ECS cluster components that hold actor state (NPCs + enemies share
//!   one cluster; bosses are their own);
//! - `damage*` / `aggression` / `interact` — hit routing, provocation, and
//!   player interactions;
//! - `encounter_rewards` / `reset` / `save_sync` — reward chests, room reset,
//!   and save-state mirroring;
//! - `view_index` / `anim_helpers` / `target_volumes` — per-frame read models
//!   and overlays consumed by presentation, engine, and combat code.
//!
//! Facade: many `ecs::<name>` paths re-export from the reusable
//! `combat` kit (`banner`, `breakables`, `chests`, `hazards`,
//! `hitbox`, `overlay`, `pickups`, ...) so call sites stay
//! stable while the generic mechanics live DOWN in that kit (ADR 0019).

use super::*;
// `BodyCombat`/`BodyHealth` live on the reusable actor crate. This module surfaces them to the
// `ecs/` submodules that name `super:BodyCombat` — the `super:*` glob no longer carries them
// since the `features` facade stopped re-exporting the shared body vocabulary.
use crate::platformer_runtime::lifecycle::RoomVisual;
use ambition_characters::actor::BodyCombat;
use ambition_vfx::vfx::{ParticleKind, VfxMessage};
use bevy::prelude::{
    Commands, Component, Entity, MessageReader, MessageWriter, NextState, Query, Res, ResMut, With,
    Without,
};

use ambition_time::WorldTime;

pub mod actor_bundles;
pub mod actor_clusters;
mod actors;
mod aggression;
mod anim_helpers;
pub mod attack;
mod bosses;
mod brain_builders;
pub(crate) use brain_builders::enemy_default_brain;
/// The ladder projection, registered in the actor pipeline beside the brain tick.
pub use brain_builders::project_authored_fighter_ladder;
/// The dismount reaction: mount announces, this rebuilds. See its own note.
pub use brain_builders::rebuild_dismounted_rider_brains;
pub(crate) mod autonomous_reconcile;
mod brain_effects;
pub(crate) mod character_policy;
pub mod chests;
mod damage;
// ⛔ `damage_apply` LEFT FOR `crates/ambition_damage`, 2026-08-26. It named no
// monolith type by the time it went — five re-export facades had been hiding
// what it actually depended on — so the carve was a move rather than a design.
mod damage_drops;
mod damage_predicates;
pub mod dormancy;
pub mod effect_bus;
mod encounter_rewards;
#[cfg(test)]
mod fighter_harness;
mod interact;
pub mod ledge_trump;
pub mod perception;
pub mod pickups;
mod reset;
mod save_sync;
mod spawn;
mod spawn_actors;
pub mod spawn_static;
mod target_volumes;

// Combat-kit aliases keep `ecs::<module>` paths stable for callers.
// (attack/chests/damage_apply/effect_bus/pickups/spawn_static
// are LOCAL modules now — the E2 glue re-home.)
pub use ambition_combat::{
    banner, breakables, falling_chest, hazards, held_items, hitbox, targeting, variation,
};

pub use actors::{
    actor_component_snapshot, enemy_component_snapshot, sync_actor_components_from_cluster,
};
pub use actors::{
    apply_actor_contact_damage, integrate_sim_bodies, route_boss_strikes_to_limbs,
    snapshot_body_contact, sync_actor_poses_from_feature_aabbs, sync_actor_read_model,
    tick_actor_brains, tick_npc_idle_barks, ActorSteering, AxisSweptMotion, MomentumMotion,
    MotionModel,
};
pub(crate) use actors::{
    maintain_actor_pre_decision_state, observe_actor_decision_inputs,
    publish_actor_decision_frames, ActorDecisionFacts, ActorDecisionFrames,
};
pub use aggression::{
    apply_actor_stimuli, arm_requested_challenges, tick_pending_challenges, ChallengeRequested,
    PendingChallenge, CHALLENGE_GRACE_S,
};
pub use anim_helpers::{
    advance_actor_anim_overlays, boss_anim_state_for, ecs_boss_anim_state,
    ecs_boss_anim_state_and_entity, ecs_boss_animation_frame_sample, ecs_breakable_state,
    ecs_chest_opened,
};
pub use banner::{apply_gameplay_banner_requests, tick_gameplay_banner};
// `boss_component_snapshot` is pub: the observation-boundary contract tests
// (ambition_sim_view) build boss read-model components from a scratch boss.
pub use crate::world::overlay::{
    rebuild_feature_ecs_world_overlay, FeatureEcsWorldOverlay, FeatureWorldOverlaySet,
};
pub use ambition_combat::hit_camera_shake::shake_camera_on_landed_hits;
pub use bosses::boss_component_snapshot;
#[allow(
    unused_imports,
    reason = "marker re-exported for tests / external visualizers"
)]
pub use bosses::BossSpriteMetricsApplied;
pub use bosses::{
    boss_spawn_hurtboxes, derive_boss_sprite_metrics, drive_boss_animators, integrate_boss_bodies,
    project_boss_attack_state_from_move, sync_boss_actor_components, sync_boss_encounter_phase,
    tick_boss_brains_system, trigger_boss_attack_moves, update_ecs_bosses,
};
pub use brain_effects::spawn_projectiles_from_brain_actions;
pub use breakables::update_ecs_breakables;
pub use chests::open_ecs_chests;
pub use damage::apply_feature_hit_events;
pub use damage_predicates::{
    ecs_hit_event_hits_actor, ecs_hit_event_hits_boss, ecs_hit_event_hits_breakable,
};
pub use encounter_rewards::{clear_encounter_reward_ecs, sync_encounter_reward_chests_ecs};
pub use falling_chest::update_ecs_falling_chests;
pub use hazards::{update_ecs_hazards, HazardTickSet};
pub use held_items::HeldItem;
pub use hitbox::{
    apply_hitbox_damage, tick_and_despawn_hitboxes, Hitbox, HitboxAnchor, HitboxHits,
    HitboxKnockback, HitboxLifetime,
};
pub use interact::interact_ecs_actors_and_switches;
// ⭐ THE MOUNT PAIR'S TESTS STAYED, because their fixtures are this crate's
// construction road. They exercise `ambition_mount` from the composition.
#[cfg(test)]
mod mount_pair_tests;

// ⛔ THE MOUNT PAIR LEFT THIS CRATE (D33, 2026-08-26) and is NOT re-exported.
// `ambition_mount` owns it; a `pub use` here would let every caller keep
// spelling it `ambition_mount::MountSlot` and hide whose domain it is — the same rule
// `Mass`, `MountDied` and `TemporaryControl` each moved under.
pub use pickups::{
    collect_ecs_pickups, magnetize_pickups, PickupArt, PickupCollect, PickupCollectLock,
    PickupMagnetize,
};
pub use reset::{reset_ecs_room_features, SpawnedThisAttempt};
pub use save_sync::{
    sync_ecs_actors_with_save, sync_ecs_bosses_with_save, sync_ecs_switches_from_save,
};
pub use spawn::{
    spawn_encounter_mob, spawn_room_feature_entities_from_plan, ActorConstructionContext,
    OccurrenceContinuity, RoomContentStagingError, RoomContentStagingRegistrationError,
    RoomContentStagingRegistry, RoomFeatureConstructionError, RoomFeatureConstructionPlan,
    RoomFeatureConstructionReceipt,
};
pub(crate) use spawn::{spawn_runtime_minion, spawn_runtime_minion_into};
pub use spawn_actors::{
    apply_spawn_actor_requests, apply_summon_effects, EncounterMobSeed, GiantHandPlan,
    SpawnActorKind, SpawnActorRequest,
};
pub(crate) use spawn_actors::{
    giant_hand_plans, is_limbed_host, spawn_boss_with_overrides_into,
    spawn_enemy_with_faction_into, spawn_staged_actor_into,
};
pub use target_volumes::{
    derive_pogo_target_volumes, refresh_body_damageable_volumes, refresh_boss_damageable_volumes,
    refresh_breakable_damageable_volumes,
};
pub use targeting::{
    can_damage, damage_lands, dissolve_settled_grudges, select_actor_targets, FactionRelations,
    FriendlyFire,
};

// `FeatureSimEntity` is a generic entity-marker queried by the reusable
// mechanics, so its definition lives DOWN in
// `ambition_platformer2d_shared_tangle::markers` (ADR 0019). Re-exported here so all
// existing `crate::features::ecs::FeatureSimEntity` call sites compile
// unchanged.
pub use ambition_platformer2d_shared_tangle::lifecycle::FeatureSimEntity;

// `HazardFeature` moved to the combat kit with the hazard runtime.
pub use ambition_combat::hazard_runtime::HazardFeature;

#[cfg(test)]
mod tests;
