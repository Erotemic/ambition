//! Sandbox gameplay feature systems.
//!
//! Authored and dynamic feature families live as Bevy entities/components. This
//! facade re-exports the component types, messages, and systems used by the
//! simulation, presentation, encounter, and test layers while domain logic lives
//! in `features/*.rs`.

use ambition_engine as ae;
use ambition_engine::AabbExt;
use bevy::prelude::*;

use crate::world::platforms::MovingPlatformState;

const ENEMY_GRAVITY: f32 = 1450.0;
const ENEMY_MAX_FALL: f32 = 760.0;
const ENEMY_PATROL_SPEED: f32 = 105.0;
const ENEMY_CHASE_SPEED: f32 = 155.0;
const ENEMY_ATTACK_RANGE: f32 = 150.0;
const ENEMY_ATTACK_COOLDOWN: f32 = 1.05;
/// Velocity kick applied along the negative fire direction when a
/// non-pirate enemy discharges a projectile. Modest because most
/// enemies fire melee-range bolts and don't need the screen-shake
/// equivalent of a cannon.
const ENEMY_FIRE_RECOIL_DEFAULT: f32 = 60.0;
/// Recoil for `PirateOnShark`. Larger because the pirate's
/// gun-sword fires hurled laser-sword projectiles — the user spec
/// asks for "recoil that pushes the pirate (and its shark) back a
/// fair bit". The brain's velocity ramp will reel the shark back
/// toward its orbit slot over the next ~half second.
const ENEMY_FIRE_RECOIL_PIRATE: f32 = 380.0;
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
mod util;
mod world_overlay;

pub use bosses::{
    BossAttackProfile, BossBehaviorProfile, BossMovementProfile, BossRuntime, BossTickOutputs,
    GNU_TON_APPLE_OWNER_PREFIX,
};
pub use breakables::BreakableRuntime;
pub use bus::{
    apply_boss_damage_effects, apply_flag_effects, apply_gameplay_sfx_effects,
    apply_npc_strike_effects, apply_quest_effects, apply_switch_effects,
    GameplayEffectsSchedulePlugin,
};
pub use chests::ChestRuntime;
pub use components::{
    ActorCombatState, ActorCooldowns, ActorDisposition, ActorFaction, ActorHealth, ActorIdentity,
    ActorIntent, ActorTarget, BossPatternTimer, BossPhase, BossRewardChest, BreakableFeature,
    ChestBundle, ChestFeature, Collected, EncounterMob, EncounterRewardChest, EnemyActorBundle,
    FallingChest, FeatureAabb, FeatureBaseBundle, FeatureId, FeatureLifecycleBundle, FeatureName,
    FeatureRenderedBundle, Opened, PersistKey, PickupBundle, PickupFeature, PogoTargetContributor,
    RespawnTimer, SandboxSolidContributor, StandTimer, SwitchFeature, SwitchOn,
};
pub use ecs::{
    apply_feature_damage_events, apply_gameplay_banner_requests, clear_encounter_reward_ecs,
    collect_ecs_pickups, despawn_encounter_mobs, ecs_boss_anim_state, ecs_boss_name,
    ecs_breakable_state, ecs_chest_opened, ecs_damage_event_hits_actor, ecs_damage_event_hits_boss,
    ecs_damage_event_hits_breakable, ecs_enemy_anim_state, ecs_enemy_name,
    ecs_enemy_sprite_override, ecs_npc_anim_state, ecs_npc_name, interact_ecs_actors_and_switches,
    open_ecs_chests, rebuild_feature_ecs_world_overlay, rebuild_feature_view_index,
    reset_ecs_room_features, select_actor_targets, spawn_encounter_mob,
    spawn_room_feature_entities, sync_boss_reward_chests_ecs, sync_ecs_actors_with_save,
    sync_ecs_bosses_with_save, sync_ecs_switches_from_save, sync_encounter_reward_chests_ecs,
    tick_gameplay_banner, update_ecs_actors, update_ecs_bosses, update_ecs_breakables,
    update_ecs_falling_chests, update_ecs_hazards, ActorRuntime, BossFeature,
    FeatureEcsWorldOverlay, FeatureSimEntity, FeatureViewIndex, HazardFeature,
};
pub use enemies::{EnemyArchetype, EnemyRespawnPolicy, EnemyRuntime, ENEMY_DEAD_UNTIL_REST_SUFFIX};
pub use events::{
    DamageEvent, DamageSource, FeatureCombatTuning, FeatureView, FeatureVisualKind, GameplayBanner,
    GameplayBannerRequested, GameplayEffect, NpcDialogueRequest, PlayerDamageEvent,
    PlayerDamageMode, PlayerDamageSource, PogoBounceEvent, ResetRoomFeaturesEvent,
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
                rebuild_feature_ecs_world_overlay,
                update_ecs_hazards,
                // Target selection runs before actor / boss updates so
                // each non-player actor's per-frame "who am I looking
                // at" pointer is fresh by the time downstream ticks
                // consult `ActorTarget` (OVERNIGHT-TODO #17.8).
                select_actor_targets,
                update_ecs_actors,
                update_ecs_bosses,
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
