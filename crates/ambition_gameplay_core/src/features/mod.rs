//! The enemy / NPC / boss ECS ACTOR SIMULATION — NOT a feature-toggle layer.
//! Despite the name, "features" here means in-world entities (actors plus room
//! props: pickups, chests, switches, breakables, hazards), all as Bevy
//! components.
//!
//! This `mod.rs` is the facade + scheduling root: it re-exports the component
//! types, messages, and systems for the simulation/presentation/encounter/test
//! layers and registers the
//! `WorldPrep`/`GameplayEffects`/`FeatureCollection`/`FeatureInteraction`/
//! `FeatureViewSync` schedule plugins. (Non-grounded actors — including bosses
//! since AS4c — share the ONE flight limb; there is no bespoke float glue here.)
//!
//! Domain logic lives in siblings: `enemies/` (grounded + aerial enemy
//! integration onto the shared spine), `npcs` (per-NPC runtime glue + barks),
//! `bosses` (boss special-spec resolver + tuning), `banter` (ambient combat
//! chatter registry), and the private `ecs` tree (cluster components + the
//! per-actor tick/spawn/damage systems).

use ambition_engine_core as ae;
use ambition_engine_core::AabbExt;
use bevy::prelude::*;

// Movement physics (gravity / fall cap / run accel / jump / double-jump) used to
// be the hardcoded `ENEMY_*` constants here. They are now per-archetype DATA,
// composed hierarchically — see `crate::combat::BodyMovementTuning` (whose
// `BASELINE` carries these exact historical values) and the archetype `movement`
// patch + `inherits` resolution in `features/enemies/mod.rs`. The integrator reads
// `tuning.movement.*`.
/// Mid-air jumps an enemy gets between landings. `1` = single
/// double-jump (matches the player's default). Resets when the
/// body transitions `on_ground: false → true` in `enemy.update()`.
pub(crate) const MAX_ENEMY_AIR_JUMPS: u8 = 1;

// The former `step_floating_body` bespoke float is GONE (archetype swap AS4c):
// every non-grounded actor — aerial enemy, the parrot, and now bosses — flies
// through the ONE shared movement pipeline (`ActorMut::update` → the flight limb),
// so there is no parallel gravity-free integrator to keep in sync.

// Archetype data owns enemy speed/range tuning; keep only shared fallback
// clocks here.
pub(crate) const ENEMY_ATTACK_COOLDOWN: f32 = 1.05;
// Boss/profile and combat-kit data own their own cooldown/timing constants.

pub mod arena;
pub use arena::{
    duel_spawn_requests, stage_room_duel, DUEL_ARENA_ROOM_ID, DUEL_PCA_ID, DUEL_ROBOT_ID,
};
pub mod banter;
// Stable facade for boss attack geometry.
pub use crate::boss_encounter::attack_geometry as boss_attack_geometry;
pub mod bosses;
mod ecs;
pub use ecs::{rider_hand_world_pos, rider_hand_world_pos_in_frame};
mod enemies;
mod npcs;

// Re-export the generic combat kit so existing feature-facing paths stay stable.
pub use crate::combat::components;
pub use crate::combat::events;
pub use crate::combat::hazard_runtime as hazards;
pub use crate::combat::path_motion;
pub use crate::combat::world_overlay;
pub use crate::combat::{bus, util};

pub use boss_attack_geometry::{
    active_attack_volumes, body_damage_aabb, bounding_aabb, collision_aabb,
    damageable_volumes, volumes_for_profile, world_space_body_aabbs_from_metrics,
    world_space_body_aabbs_from_parts, AnimationSelection, BossAnimationFrameSample,
    BossVolumeContext, CombatGeometry, SimpleActorGeometry,
};
pub use bosses::{
    boss_special_for_profile, ActorSpriteMetrics, BossAttackProfile, BossBehaviorProfile,
    BossMovementProfile, BossRewardProfile, GNU_TON_APPLE_OWNER_PREFIX,
    GRADIENT_SENTINEL_ENCOUNTER_ID,
};
pub use bus::{
    apply_flag_effects, apply_gameplay_sfx_effects, apply_quest_effects, apply_switch_effects,
};
pub use ecs::actor_component_snapshot;
// Runtime minion/summon spawner, re-exported so non-feature modules (e.g. the
// puppy-slug gun) can summon actors without reaching into the private `ecs` tree.
pub(crate) use ecs::spawn_runtime_minion;

pub use components::{
    ActorAggression, ActorCooldowns, ActorDisposition, ActorFaction, ActorIdentity, ActorIntent,
    ActorInteraction, ActorPose, ActorRenderSize, ActorTarget, AggressionMode, AggressionTarget,
    BodyMelee, BossDeathAnimation, BossPatternTimer, BossPhase,
    BossRewardChest, BreakableFeature, CenteredAabb, ChestBundle, ChestFeature, Collected,
    CombatKit, DamageableVolumes, EncounterMob, EncounterRewardChest, EnemyActorBundle,
    FallingChest, FeatureBaseBundle, FeatureId, FeatureLifecycleBundle, FeatureName,
    FeatureRenderedBundle, MeleeSwing, Opened, PersistKey, PickupBundle, PickupFeature, PogoPolicy,
    PogoTargetContributor, PogoTargetVolumes, PostBossNpc, RespawnTimer, RuntimeStagedActor,
    SandboxSolidContributor, StandTimer, SwitchFeature, SwitchOn,
};
pub use ecs::actor_clusters::{
    ActorClusterSeed, ActorConfig, ActorMotionPath, ActorMut, ActorStatus, BodyKinematics,
};
pub use ecs::{
    apply_actor_contact_damage, apply_actor_stimuli, apply_feature_hit_events,
    apply_gameplay_banner_requests, apply_hitbox_damage, apply_spawn_actor_requests,
    apply_summon_effects, boss_is_cleared, boss_spawn_hurtboxes, can_damage,
    clear_encounter_reward_ecs, collect_ecs_pickups, damage_lands, derive_boss_sprite_metrics,
    derive_pogo_target_volumes, despawn_encounter_mobs, dissolve_settled_grudges,
    ecs_actor_anim_state, ecs_boss_anim_state,
    ecs_boss_anim_state_and_entity, ecs_boss_animation_frame_sample, ecs_boss_name,
    ecs_breakable_state, ecs_chest_opened,
    ecs_hit_event_hits_actor, ecs_hit_event_hits_boss, ecs_hit_event_hits_breakable,
    enforce_mount_rider_link, integrate_boss_bodies, integrate_sim_bodies,
    interact_ecs_actors_and_switches, sync_boss_strike_hitboxes,
    magnetize_pickups, open_ecs_chests, pirate_on_shark_rider_offset,
    rebuild_actor_render_index, rebuild_feature_ecs_world_overlay, rebuild_feature_view_index,
    refresh_actor_damageable_volumes, refresh_boss_damageable_volumes,
    refresh_breakable_damageable_volumes, reset_ecs_room_features, select_actor_targets,
    spawn_encounter_mob, spawn_enemy_projectiles_from_brain_actions, spawn_melee_hitbox,
    spawn_room_feature_entities, sync_actor_poses_from_feature_aabbs, sync_actor_read_model,
    sync_boss_actor_components, sync_boss_encounter_phase, sync_boss_reward_chests_ecs,
    sync_ecs_actors_with_save, sync_ecs_bosses_with_save, sync_ecs_switches_from_save,
    sync_encounter_reward_chests_ecs, sync_riders_to_mounts, tick_actor_brains,
    tick_and_despawn_hitboxes, tick_boss_brains_system, tick_gameplay_banner, tick_npc_idle_barks,
    tick_pending_challenges, update_ecs_bosses, update_ecs_breakables, update_ecs_falling_chests,
    update_ecs_hazards, ActorSteering, BossClusterQueryData, BossClusterRef, BossClusterScratch,
    ActorRenderIndex, ActorRenderView, BossConfig, BossMut, BossOverrides, BossRef, BossEncounter,
    FactionRelations,
    FeatureEcsWorldOverlay, FeatureSimEntity, FeatureViewIndex, FriendlyFire, HazardFeature,
    HeldItem, Hitbox, HitboxAnchor, HitboxHits, HitboxLifetime, MountSlot, Mountable, Mounted,
    MountedBrainCache, MountedSize, PendingChallenge, RidingOn, SpawnActorKind, SpawnActorRequest,
    CHALLENGE_GRACE_S,
};
pub use ecs::{ActorAnimFrame, ActorSpriteData};
pub use enemies::{
    composite_visual_plan, enemy_spawn_is_sandbag, install_enemy_roster, ActorSpawnState,
    ActorSurfaceState, CharacterRoster, CompositeVisualPlan, EnemyRespawnPolicy,
    ENEMY_DEAD_UNTIL_REST_SUFFIX,
};
pub use events::{
    ActorStimulus, FeatureCombatTuning, FeatureView, FeatureVisualKind, GameplayBanner,
    GameplayBannerRequested, GameplaySfxRequested, HitEvent, HitKnockback, HitMode, HitSource,
    HitTarget, NpcDialogueRequest, QuestAdvanceRequested, ResetRoomFeaturesEvent, RoomResetReason,
    SetFlagRequested, SwitchActivated,
};
pub use hazards::HazardRuntime;
pub use npcs::{NPC_PATROL_SPEED, NPC_TALK_RADIUS};
pub use path_motion::PathMotion;
pub use world_overlay::{
    world_with_gate_solids_and_carves, world_with_portal_carves, world_with_sandbox_solids,
    CollisionWorld,
};

pub(super) use npcs::NPC_HOSTILE_STRIKE_THRESHOLD;
use util::*;

/// Schedules the gameplay-effect bus chain into
/// [`crate::schedule::SandboxSet::GameplayEffects`].
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
                // Deferred-challenge grace runs only in `Playing` (after the dialog
                // box closes), then emits the `Challenged` stimulus the next system
                // consumes.
                ecs::tick_pending_challenges.run_if(crate::session::game_mode::gameplay_allowed),
                ecs::apply_actor_stimuli,
                bus::apply_gameplay_sfx_effects,
            )
                .chain()
                .in_set(crate::schedule::SandboxSet::GameplayEffects),
        );
    }
}

/// Accumulating sim-time (seconds), advanced by the gameplay clock so it slows
/// under bullet-time / freezes on pause alongside every other sim timer
/// (ADR 0010/0011 time-domains discipline). This is the monotone "now" the
/// per-actor brain perception reads: the Smash brain's reaction latency
/// (`obs_history` lookback by `reaction_delay_s`) is inert without it. Distinct
/// from `time_control::SimClock` (a time-*scale* request) — this is elapsed time.
#[derive(bevy::prelude::Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct GameplayElapsed(pub f32);

/// Advance [`GameplayElapsed`] by the scaled gameplay dt each frame. Runs at the
/// head of `WorldPrep`, before any actor brain reads the snapshot.
pub fn advance_gameplay_elapsed(
    mut elapsed: bevy::prelude::ResMut<GameplayElapsed>,
    world_time: bevy::prelude::Res<ambition_time::WorldTime>,
) {
    elapsed.0 += world_time.scaled_dt;
}

/// Schedules `WorldPrep`: LDtk hot-reload, feature-world overlay rebuild,
/// and per-frame hazard/actor/boss ticks before player simulation reads them.
pub struct WorldPrepSchedulePlugin;

impl bevy::prelude::Plugin for WorldPrepSchedulePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        use bevy::prelude::{IntoScheduleConfigs, Update};
        // Relational targeting seam (default = today's behavior; stealth/bounty/
        // alliance systems mutate it). `select_actor_targets` reads it.
        app.init_resource::<FactionRelations>();
        app.init_resource::<FriendlyFire>();
        // Accumulating sim-time for brain perception (reaction latency).
        app.init_resource::<GameplayElapsed>();
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
                // The per-actor pipeline (was the `update_ecs_actors` monolith) is
                // now four explicit phases — `tick_actor_brains` →
                // `integrate_actor_bodies` → `sync_actor_read_model` →
                // `apply_actor_contact_damage` — registered separately below (this
                // tuple is at Bevy's chain-length ceiling) so brain / movement /
                // read-model / contact are each their own scheduled system.
                // Ambient NPC chatter (parrot squawks, etc.) on its own timer.
                tick_npc_idle_barks,
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
                integrate_boss_bodies,
                update_ecs_bosses,
                sync_boss_strike_hitboxes,
                sync_boss_actor_components,
                sync_actor_poses_from_feature_aabbs,
            )
                .chain()
                .in_set(crate::schedule::SandboxSet::WorldPrep),
        );
        // Advance the accumulating sim clock before any actor brain reads its
        // perception snapshot, so reaction-latency lookback is live. Registered
        // separately (not in the chain above) only because that tuple is already
        // at Bevy's chain-length ceiling; the `.before` keeps the ordering exact.
        app.add_systems(
            Update,
            advance_gameplay_elapsed
                .before(select_actor_targets)
                .in_set(crate::schedule::SandboxSet::WorldPrep),
        );
        // The decomposed per-actor pipeline: brain → intent, movement integration,
        // read-model mirror, and contact-damage observer, as four explicit phases.
        // Chained (they share the actor cluster + `ActorControl`/`BodyCombat`) and
        // slotted where the old `update_ecs_actors` monolith ran (after target
        // selection, before the NPC bark ticker). Registered separately from the big
        // WorldPrep tuple, which is at Bevy's chain-length ceiling.
        app.init_resource::<ActorSteering>();
        app.add_systems(
            Update,
            (
                tick_actor_brains,
                // Advance moving platforms ONCE before any body integrates, so every
                // body (home + actors) rides THIS frame's platform positions — the
                // home body used to advance them in `PlayerSimulation`, after the
                // actors integrated, so actors read stale positions; unifying the
                // movement phase unifies this too.
                crate::player::advance_moving_platforms,
                // The ONE movement phase for every non-boss sim body: actor bodies
                // AND home/player bodies integrate here, through the same engine
                // entry. (`player_body_tick` in `PlayerSimulation` is gone.)
                integrate_sim_bodies,
                sync_actor_read_model,
                apply_actor_contact_damage,
            )
                .chain()
                .after(select_actor_targets)
                .before(tick_npc_idle_barks)
                .in_set(crate::schedule::SandboxSet::WorldPrep),
        );
        // Settle decided feuds before targeting reads grudges: a body forgets a slain
        // foe (won't re-aggro if it revives) and a defeated body forgets its own feud
        // (revives as a normal NPC). Registered separately — the WorldPrep chain tuple
        // is already at Bevy's chain-length ceiling — with `.before` to keep the order.
        app.add_systems(
            Update,
            dissolve_settled_grudges
                .before(select_actor_targets)
                .in_set(crate::schedule::SandboxSet::WorldPrep),
        );
        app.configure_sets(
            Update,
            crate::schedule::BossSteerSlot
                .after(tick_boss_brains_system)
                .before(update_ecs_bosses)
                .in_set(crate::schedule::SandboxSet::WorldPrep),
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
                .in_set(crate::schedule::SandboxSet::FeatureCollection),
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
                .in_set(crate::schedule::SandboxSet::FeatureInteraction),
        );
    }
}

/// Rebuilds the presentation read-models once per frame: [`FeatureViewIndex`]
/// (geometry/state for every feature) and [`ActorRenderIndex`] (the materialized
/// per-actor identity facts the renderer binds sprites from). Both let
/// presentation read a snapshot instead of live-querying the sim's ECS.
pub struct FeatureViewSyncSchedulePlugin;

impl bevy::prelude::Plugin for FeatureViewSyncSchedulePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        use bevy::prelude::{IntoScheduleConfigs, Update};
        app.add_systems(
            Update,
            (rebuild_feature_view_index, rebuild_actor_render_index)
                .in_set(crate::schedule::SandboxSet::FeatureViewSync),
        );
    }
}

#[cfg(test)]
mod conversion_tests;

#[cfg(test)]
mod sim_clock_tests {
    use super::{advance_gameplay_elapsed, GameplayElapsed};
    use bevy::prelude::*;

    /// `advance_gameplay_elapsed` accumulates the scaled gameplay dt: the brain's
    /// perception clock is no longer the inert `0.0` it used to read. Bullet-time
    /// scaling is honored because it sums `scaled_dt`, not wall-clock.
    #[test]
    fn gameplay_clock_accumulates_scaled_dt() {
        let mut app = App::new();
        app.insert_resource(ambition_time::WorldTime {
            raw_dt: 1.0 / 60.0,
            scaled_dt: 1.0 / 60.0,
        });
        app.init_resource::<GameplayElapsed>();
        app.add_systems(Update, advance_gameplay_elapsed);

        app.update();
        app.update();
        app.update();
        let elapsed = app.world().resource::<GameplayElapsed>().0;
        assert!(
            (elapsed - 3.0 / 60.0).abs() < 1e-6,
            "three ticks at 1/60 s must accumulate 3/60 s; got {elapsed}"
        );

        // Paused (scaled_dt == 0) the clock freezes — reaction latency, hitstun,
        // and every other sim timer that reads it stop together.
        app.insert_resource(ambition_time::WorldTime {
            raw_dt: 1.0 / 60.0,
            scaled_dt: 0.0,
        });
        app.update();
        let after_pause = app.world().resource::<GameplayElapsed>().0;
        assert_eq!(
            elapsed, after_pause,
            "a paused frame must not advance sim-time"
        );
    }
}
