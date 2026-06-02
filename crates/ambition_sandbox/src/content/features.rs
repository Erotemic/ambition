//! Sandbox gameplay feature systems.
//!
//! Authored and dynamic feature families live as Bevy entities/components. This
//! facade re-exports the component types, messages, and systems used by the
//! simulation, presentation, encounter, and test layers while domain logic lives
//! in `features/*.rs`.

use crate::engine_core as ae;
use crate::engine_core::AabbExt;
use bevy::prelude::*;

use crate::world::platforms::MovingPlatformState;

const ENEMY_GRAVITY: f32 = 1450.0;
const ENEMY_MAX_FALL: f32 = 760.0;
/// Vertical impulse (px/s) applied when a grounded enemy's
/// `ActorControlFrame.jump_pressed` is true. Slightly under the
/// player's `JUMP_SPEED` (630) so goblins jump a touch lower than
/// the player — enough to clear a head-height target and commit
/// an air attack, not enough to out-arc a player jumping straight
/// up. Engine y grows downward; the integration applies
/// `body.vel.y = -ENEMY_JUMP_SPEED`.
const ENEMY_JUMP_SPEED: f32 = 520.0;
/// Mid-air jump impulse (px/s). Slightly under the ground jump so
/// the second jump reads as a "boost" rather than a full re-launch
/// — matches the player's `DOUBLE_JUMP_SPEED` shape (520 → 420 step).
const ENEMY_DOUBLE_JUMP_SPEED: f32 = 430.0;
/// Mid-air jumps an enemy gets between landings. `1` = single
/// double-jump (matches the player's default). Resets when the
/// body transitions `on_ground: false → true` in `enemy.update()`.
pub(crate) const MAX_ENEMY_AIR_JUMPS: u8 = 1;
// `ENEMY_PATROL_SPEED` / `ENEMY_CHASE_SPEED` / `ENEMY_ATTACK_RANGE`
// retired by the enemy_archetypes.ron migration — each row now carries
// its own speeds / ranges. If multiple archetypes drift back to the
// same baseline, prefer one inline literal per row over re-introducing
// a shared const here.
const ENEMY_ATTACK_COOLDOWN: f32 = 1.05;
// BOSS_ATTACK_COOLDOWN retired by the boss-profile data-driven
// migration — each profile in `boss_profiles.ron` carries its own
// `attack_cooldown`. The clockwork_warden default (1.35) lives there.
const BREAK_ON_STAND_SECONDS: f32 = 0.85;

/// Gravity (px/s²) used by the falling-chest tick. Lighter than the
/// player's GRAVITY (2250) so a treasure chest reads as a heavy-but-
/// floaty drop, not a brick. Tuned by feel against the mockingbird
/// arena: at 1400 px/s² and 80 px of fall, the drop lands in ~0.34 s.
const CHEST_FALL_GRAVITY: f32 = 1400.0;
/// Terminal-velocity cap so a chest dropped from a tall arena doesn't
/// blast through the floor sweep before the sub-step kicks in.
const CHEST_FALL_MAX_SPEED: f32 = 900.0;

mod boss_attack_geometry;
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
mod util;
mod world_overlay;

pub use boss_attack_geometry::{
    active_attack_volumes, body_damage_aabb, boss_attack_damage, bounding_aabb, damageable_volumes,
    telegraph_volumes, volumes_for_profile, world_space_body_aabbs_from_metrics,
    world_space_body_aabbs_from_parts, BossAnimationFrameSample, BossVolumeContext,
};
pub use bosses::{
    boss_special_for_profile, BossAttackProfile, BossBehaviorProfile, BossMovementProfile,
    BossRuntime, BossSpriteMetrics, GNU_TON_APPLE_OWNER_PREFIX, GRADIENT_SENTINEL_ENCOUNTER_ID,
};
pub use breakables::BreakableRuntime;
pub use bus::{
    apply_flag_effects, apply_gameplay_sfx_effects, apply_quest_effects, apply_switch_effects,
    GameplayEffectsSchedulePlugin,
};
pub use chests::ChestRuntime;
pub(crate) use ecs::actor_component_snapshot;

pub use ecs::enemy_clusters::{
    EnemyConfig, EnemyKinematics, EnemyMotionPath, EnemyMut, EnemyStatus,
};
pub use ecs::ActorSpriteData;
pub use components::{
    ActorAggression, ActorAttackState, ActorCombatState, ActorCooldowns, ActorDisposition,
    ActorFaction, ActorHealth, ActorIdentity, ActorIntent, ActorPose, ActorTarget, AggressionMode,
    AggressionTarget, BossDeathAnimation,
    BossPatternTimer, BossPhase, BossRewardChest, BreakableFeature, ChestBundle, ChestFeature,
    Collected, CombatKit, DamageableVolumes, EncounterMob, EncounterRewardChest, EnemyActorBundle,
    FallingChest, FeatureAabb, FeatureBaseBundle, FeatureId, FeatureLifecycleBundle, FeatureName,
    FeatureRenderedBundle, Opened, PersistKey, PickupBundle, PickupFeature, PogoPolicy,
    PogoTargetContributor, PogoTargetVolumes, RespawnTimer, SandboxSolidContributor, StandTimer,
    SwitchFeature, SwitchOn,
};
pub use ecs::{
    apply_actor_stimuli, apply_feature_hit_events, apply_gameplay_banner_requests,
    apply_hitbox_damage, clear_encounter_reward_ecs, collect_ecs_pickups,
    derive_boss_sprite_metrics, derive_pogo_target_volumes, despawn_encounter_mobs,
    ecs_boss_anim_state, ecs_boss_anim_state_and_entity, ecs_boss_animation_frame_sample,
    ecs_boss_name, ecs_breakable_state, ecs_chest_opened, ecs_enemy_anim_state, ecs_enemy_name,
    ecs_enemy_sprite_override, ecs_hit_event_hits_actor, ecs_hit_event_hits_boss,
    ecs_hit_event_hits_breakable, ecs_npc_anim_state, ecs_npc_name, enforce_mount_rider_link,
    interact_ecs_actors_and_switches, is_composite_spawn, open_ecs_chests,
    pirate_on_shark_rider_offset, rebuild_feature_ecs_world_overlay, rebuild_feature_view_index,
    refresh_actor_damageable_volumes, refresh_boss_damageable_volumes,
    refresh_breakable_damageable_volumes, reset_ecs_room_features, select_actor_targets,
    spawn_encounter_mob, spawn_enemy_projectiles_from_brain_actions,
    spawn_eye_beam_from_special_messages, spawn_gnu_apple_rain_from_special_messages,
    spawn_gradient_cascade_minions_from_special_messages, spawn_melee_hitbox,
    spawn_minima_trap_from_special_messages, spawn_overfit_volley_from_special_messages,
    spawn_room_feature_entities, spawn_saddle_point_from_special_messages,
    start_enemy_melee_from_brain_actions, sync_actor_poses_from_feature_aabbs,
    sync_boss_actor_components, sync_boss_encounter_phase, sync_boss_reward_chests_ecs,
    sync_ecs_actors_with_save, sync_ecs_bosses_with_save, sync_ecs_switches_from_save,
    sync_encounter_reward_chests_ecs, sync_riders_to_mounts, tick_and_despawn_hitboxes,
    tick_boss_brains_system, tick_gameplay_banner, update_ecs_actors, update_ecs_bosses,
    update_ecs_breakables, update_ecs_falling_chests, update_ecs_hazards, ActorRuntime,
    AppleRainSpawnState, BossFeature, EyeBeamState, FeatureEcsWorldOverlay, FeatureSimEntity,
    FeatureViewIndex, GradientCascadeState, HazardFeature, HeldItem, Hitbox, HitboxAnchor,
    HitboxHits, HitboxLifetime, MinimaTrapState, MountSlot, Mountable, Mounted, MountedBrainCache,
    MountedSize, OverfitVolleyState, RidingOn, SaddlePointState,
};
pub use enemies::{
    ActorSpawnState, ActorSurfaceState, EnemyArchetype, EnemyRespawnPolicy, EnemyRuntime,
    ENEMY_DEAD_UNTIL_REST_SUFFIX,
};
pub use events::{
    ActorStimulus, FeatureCombatTuning, FeatureView, FeatureVisualKind, GameplayBanner,
    GameplayBannerRequested, GameplayEffect, HitEvent, HitKnockback, HitMode, HitSource, HitTarget,
    NpcDialogueRequest, ResetRoomFeaturesEvent,
};
pub use hazards::HazardRuntime;
pub use npcs::{NpcRuntime, NPC_PATROL_SPEED};
pub use path_motion::PathMotion;
pub use pickups::PickupRuntime;
pub use world_overlay::world_with_sandbox_solids;

pub(super) use npcs::NPC_HOSTILE_STRIKE_THRESHOLD;
use util::*;

/// Module-local Bevy plugin: schedules the `WorldPrep` simulation set —
/// LDtk hot-reload poll + the ECS feature-world overlay rebuild + the
/// per-frame hazard/actor/boss ticks. Sets up the collision world that
/// `PlayerInput` and `PlayerSimulation` read in the same frame.
///
/// Four of the five systems live in `content/features/ecs/`; the
/// LDtk hot-reload poller is the one outlier (lives in
/// `ldtk_world::hot_reload`). Carved out of
/// `app/plugins.rs::register_world_prep_systems` per OVERNIGHT-TODO #6.
pub struct WorldPrepSchedulePlugin;

impl bevy::prelude::Plugin for WorldPrepSchedulePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        use bevy::prelude::{IntoScheduleConfigs, Update};
        app.add_systems(
            Update,
            (
                crate::ldtk_world::poll_ldtk_file_changes,
                // Sprite-driven boss metrics must be available before
                // boss damageable/pogo volumes are derived, otherwise
                // composite bosses such as GNU-ton would briefly fall
                // back to their coarse spawn envelope.
                derive_boss_sprite_metrics,
                refresh_actor_damageable_volumes,
                refresh_boss_damageable_volumes,
                refresh_breakable_damageable_volumes,
                derive_pogo_target_volumes,
                rebuild_feature_ecs_world_overlay,
                update_ecs_hazards,
                // Target selection runs before actor / boss updates so
                // each non-player actor's per-frame "who am I looking
                // at" pointer is fresh by the time downstream ticks
                // consult `ActorTarget` (OVERNIGHT-TODO #17.8).
                select_actor_targets,
                update_ecs_actors,
                // Rider/mount pose sync. Runs immediately after the
                // per-actor brain tick so the rider's brain has had
                // a chance to emit fire intent for the target from
                // a position close to where it'll actually be after
                // the snap. update_ecs_actors integrates each
                // actor's velocity; this system zeros it again and
                // snaps the rider back to the mount-relative
                // position so the rider doesn't drift away on the
                // next frame.
                sync_riders_to_mounts,
                // Boss tick chain (post "move boss policy out of
                // BossRuntime"): the brain decides intent first, then
                // the integration system consumes its `desired_vel`.
                // `sync_boss_encounter_phase` runs before the brain
                // tick so this frame's phase is the one
                // `BossPatternContext` carries.
                sync_boss_encounter_phase,
                tick_boss_brains_system,
                crate::boss_encounter::steer_cut_rope_boss_under_anvil,
                update_ecs_bosses,
                sync_boss_actor_components,
                sync_actor_poses_from_feature_aabbs,
            )
                .chain()
                .in_set(crate::app::SandboxSet::WorldPrep),
        );
    }
}

/// Module-local Bevy plugin: schedules the `FeatureCollection`
/// simulation set — pickups collected this frame + the resulting heal
/// requests applied to player health.
///
/// Carved out of `app/plugins.rs::register_feature_collection_systems`
/// per OVERNIGHT-TODO #6. The heal-request reader lives in
/// `crate::player`; the chain still owns the ordering.
pub struct FeatureCollectionSchedulePlugin;

impl bevy::prelude::Plugin for FeatureCollectionSchedulePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        use bevy::prelude::{IntoScheduleConfigs, Update};
        app.add_systems(
            Update,
            (
                collect_ecs_pickups,
                crate::player::apply_player_heal_requests,
            )
                .chain()
                .in_set(crate::app::SandboxSet::FeatureCollection),
        );
    }
}

/// Module-local Bevy plugin: schedules the `FeatureInteraction`
/// simulation set — actor / switch interactions, chest opens, breakable
/// damage, falling chest ticks, save mirror, and the encounter switch
/// index rebuild.
///
/// Carved out of
/// `app/plugins.rs::register_feature_interaction_systems` per
/// OVERNIGHT-TODO #6. Five of the six systems live in
/// `content/features/ecs/`; the encounter switch index rebuild is the
/// one outlier (lives in `crate::encounter`).
pub struct FeatureInteractionSchedulePlugin;

impl bevy::prelude::Plugin for FeatureInteractionSchedulePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        use bevy::prelude::{IntoScheduleConfigs, Update};
        app.add_systems(
            Update,
            (
                interact_ecs_actors_and_switches,
                open_ecs_chests,
                update_ecs_breakables,
                update_ecs_falling_chests,
                sync_ecs_switches_from_save,
                crate::encounter::rebuild_encounter_switch_index,
            )
                .chain()
                .in_set(crate::app::SandboxSet::FeatureInteraction),
        );
    }
}

/// Module-local Bevy plugin: schedules the per-frame
/// [`FeatureViewIndex`] rebuild into [`crate::app::SandboxSet::FeatureViewSync`].
/// The rebuild walks every ECS feature query once per frame and feeds
/// presentation systems (sync_visuals, sprite upgraders, HUD readouts)
/// a single shared read-model instead of forcing each consumer to
/// re-scan the feature world. Carved out of
/// `app/plugins.rs::register_feature_view_sync_systems` per
/// OVERNIGHT-TODO #6.
pub struct FeatureViewSyncSchedulePlugin;

impl bevy::prelude::Plugin for FeatureViewSyncSchedulePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        use bevy::prelude::{IntoScheduleConfigs, Update};
        app.add_systems(
            Update,
            rebuild_feature_view_index.in_set(crate::app::SandboxSet::FeatureViewSync),
        );
    }
}

#[cfg(test)]
mod conversion_tests;
