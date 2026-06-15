//! Sandbox gameplay feature systems.
//!
//! Authored and dynamic feature families live as Bevy entities/components. This
//! facade re-exports the component types, messages, and systems used by the
//! simulation, presentation, encounter, and test layers while domain logic lives
//! in `features/*.rs`.

use crate::engine_core as ae;
use crate::engine_core::AabbExt;
use bevy::prelude::*;

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
// Archetype data owns enemy speed/range tuning; keep only shared fallback
// clocks here.
const ENEMY_ATTACK_COOLDOWN: f32 = 1.05;
// Boss/profile and combat-kit data own their own cooldown/timing constants.

pub mod banter;
// Stable facade for boss attack geometry.
pub use crate::boss_encounter::attack_geometry as boss_attack_geometry;
pub mod bosses;
mod ecs;
pub use ecs::rider_hand_world_pos;
mod enemies;
mod npcs;

// Re-export the generic combat kit so existing feature-facing paths stay stable.
pub use crate::mechanics::combat::components;
pub use crate::mechanics::combat::events;
pub use crate::mechanics::combat::hazard_runtime as hazards;
pub use crate::mechanics::combat::path_motion;
pub use crate::mechanics::combat::world_overlay;
pub use crate::mechanics::combat::{bus, util};

pub use boss_attack_geometry::{
    active_attack_volumes, body_damage_aabb, boss_attack_damage, bounding_aabb, damageable_volumes,
    telegraph_volumes, volumes_for_profile, world_space_body_aabbs_from_metrics,
    world_space_body_aabbs_from_parts, BossAnimationFrameSample, BossVolumeContext,
};
pub use bosses::{
    boss_special_for_profile, BossAttackProfile, BossBehaviorProfile, BossMovementProfile,
    BossRewardProfile, BossSpriteMetrics, GNU_TON_APPLE_OWNER_PREFIX,
    GRADIENT_SENTINEL_ENCOUNTER_ID,
};
pub use bus::{
    apply_flag_effects, apply_gameplay_sfx_effects, apply_quest_effects, apply_switch_effects,
};
pub use ecs::npc_component_snapshot;
// Runtime minion/summon spawner, re-exported so non-feature modules (e.g. the
// puppy-slug gun) can summon actors without reaching into the private `ecs` tree.
pub(crate) use ecs::spawn_runtime_minion;

pub use components::{
    ActorAggression, ActorAttackState, ActorCombatState, ActorCooldowns, ActorDisposition,
    ActorFaction, ActorHealth, ActorIdentity, ActorIntent, ActorPose, ActorTarget, AggressionMode,
    AggressionTarget, BossDeathAnimation, BossPatternTimer, BossPhase, BossRewardChest,
    BreakableFeature, ChestBundle, ChestFeature, Collected, CombatKit, DamageableVolumes,
    EncounterMob, EncounterRewardChest, EnemyActorBundle, FallingChest, FeatureAabb,
    FeatureBaseBundle, FeatureId, FeatureLifecycleBundle, FeatureName, FeatureRenderedBundle,
    Opened, PersistKey, PickupBundle, PickupFeature, PogoPolicy, PogoTargetContributor,
    PogoTargetVolumes, PostBossNpc, RespawnTimer, SandboxSolidContributor, StandTimer,
    SwitchFeature, SwitchOn,
};
pub use ecs::enemy_clusters::{
    ActorMotionPath, BodyKinematics, EnemyConfig, EnemyMut, EnemyStatus,
};
pub use ecs::npc_clusters::{NpcClusterScratch, NpcConfig, NpcMut, NpcStatus};
pub use ecs::ActorSpriteData;
pub use ecs::{
    apply_actor_stimuli, apply_feature_hit_events, apply_gameplay_banner_requests,
    apply_hitbox_damage, apply_summon_effects, boss_spawn_hurtboxes, clear_encounter_reward_ecs,
    collect_ecs_pickups, derive_boss_sprite_metrics, derive_pogo_target_volumes,
    despawn_encounter_mobs, ecs_boss_anim_state, ecs_boss_anim_state_and_entity,
    ecs_boss_animation_frame_sample, ecs_boss_name, ecs_breakable_state, ecs_chest_opened,
    ecs_enemy_anim_state, ecs_enemy_is_surface_walker, ecs_enemy_name, ecs_enemy_sprite_override,
    ecs_hit_event_hits_actor,
    ecs_hit_event_hits_boss, ecs_hit_event_hits_breakable, ecs_npc_anim_state, ecs_npc_name,
    enforce_mount_rider_link, interact_ecs_actors_and_switches, magnetize_pickups, open_ecs_chests,
    pirate_on_shark_rider_offset, rebuild_feature_ecs_world_overlay, rebuild_feature_view_index,
    refresh_actor_damageable_volumes, refresh_boss_damageable_volumes,
    refresh_breakable_damageable_volumes, reset_ecs_npc_actors, reset_ecs_room_features,
    select_actor_targets, spawn_encounter_mob, spawn_enemy_projectiles_from_brain_actions,
    spawn_melee_hitbox, spawn_room_feature_entities, start_enemy_melee_from_brain_actions,
    sync_actor_poses_from_feature_aabbs, sync_boss_actor_components, sync_boss_encounter_phase,
    sync_boss_reward_chests_ecs, sync_ecs_actors_with_save, sync_ecs_bosses_with_save,
    sync_ecs_npc_actors_with_save, sync_ecs_switches_from_save, sync_encounter_reward_chests_ecs,
    sync_riders_to_mounts, tick_and_despawn_hitboxes, tick_boss_brains_system,
    tick_gameplay_banner, update_ecs_actors, update_ecs_bosses, update_ecs_breakables,
    update_ecs_falling_chests, update_ecs_hazards, update_ecs_npcs, ActorRuntime,
    BossClusterQueryData, BossClusterRef, BossClusterScratch, BossConfig, BossMut, BossRef,
    BossStatus, FeatureEcsWorldOverlay, FeatureSimEntity, FeatureViewIndex, HazardFeature,
    HeldItem, Hitbox, HitboxAnchor, HitboxHits, HitboxLifetime, MountSlot, Mountable, Mounted,
    MountedBrainCache, MountedSize, RidingOn,
};
pub use enemies::{
    composite_visual_plan, enemy_visual_kind, install_enemy_roster, ActorSpawnState,
    ActorSurfaceState, CompositeVisualPlan, EnemyRespawnPolicy, EnemyRoster,
    ENEMY_DEAD_UNTIL_REST_SUFFIX,
};
pub use events::{
    ActorStimulus, FeatureCombatTuning, FeatureView, FeatureVisualKind, GameplayBanner,
    GameplayBannerRequested, GameplaySfxRequested, HitEvent, HitKnockback, HitMode, HitSource,
    HitTarget, NpcDialogueRequest, QuestAdvanceRequested, ResetRoomFeaturesEvent, RoomResetReason,
    SetFlagRequested, SwitchActivated,
};
pub use hazards::HazardRuntime;
pub use npcs::NPC_PATROL_SPEED;
pub use path_motion::PathMotion;
pub use world_overlay::{world_with_portal_carves, world_with_sandbox_solids};

pub(super) use npcs::NPC_HOSTILE_STRIKE_THRESHOLD;
use util::*;

/// Schedules the gameplay-effect bus chain into
/// [`crate::app::SandboxSet::GameplayEffects`].
pub struct GameplayEffectsSchedulePlugin;

impl bevy::prelude::Plugin for GameplayEffectsSchedulePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        use bevy::prelude::{IntoScheduleConfigs, Update};
        app.add_systems(
            Update,
            (
                bus::apply_flag_effects,
                bus::apply_quest_effects,
                bus::apply_switch_effects,
                ecs::apply_npc_stimuli,
                ecs::apply_actor_stimuli,
                bus::apply_gameplay_sfx_effects,
            )
                .chain()
                .in_set(crate::app::SandboxSet::GameplayEffects),
        );
    }
}

/// Schedules `WorldPrep`: LDtk hot-reload, feature-world overlay rebuild,
/// and per-frame hazard/actor/boss ticks before player simulation reads them.
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
                // Target selection refreshes each actor's `ActorTarget`
                // before actor / boss update systems consume it.
                select_actor_targets,
                update_ecs_actors,
                // NPCs tick in their own system (shared cluster
                // components prevent a unified actor query — see
                // `update_ecs_npcs`). Ordering relative to
                // `update_ecs_actors` is irrelevant: NPCs and enemies
                // touch disjoint entities.
                update_ecs_npcs,
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
                // Boss brain decides intent first; integration consumes
                // `desired_vel` after optional content-side steering.
                sync_boss_encounter_phase,
                tick_boss_brains_system,
                update_ecs_bosses,
                sync_boss_actor_components,
                sync_actor_poses_from_feature_aabbs,
            )
                .chain()
                .in_set(crate::app::SandboxSet::WorldPrep),
        );
        app.configure_sets(
            Update,
            crate::app::BossSteerSlot
                .after(tick_boss_brains_system)
                .before(update_ecs_bosses)
                .in_set(crate::app::SandboxSet::WorldPrep),
        );
        // The cut-rope steer system itself is registered by the content
        // plugin (`crate::content::bosses`), in `BossSteerSlot`.
    }
}

/// Schedules `FeatureCollection`: pickup collection followed by heal apply.
pub struct FeatureCollectionSchedulePlugin;

impl bevy::prelude::Plugin for FeatureCollectionSchedulePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        use bevy::prelude::{IntoScheduleConfigs, Update};
        app.add_systems(
            Update,
            (
                // Pull nearby loot toward the player, then collect on overlap.
                magnetize_pickups,
                collect_ecs_pickups,
                crate::player::apply_player_heal_requests,
            )
                .chain()
                .in_set(crate::app::SandboxSet::FeatureCollection),
        );
    }
}

/// Schedules `FeatureInteraction`: switches, chests, breakables, save sync,
/// and encounter switch-index rebuild.
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

/// Rebuilds [`FeatureViewIndex`] once per frame for presentation/HUD readers.
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
