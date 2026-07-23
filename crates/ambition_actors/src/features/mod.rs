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
//! `bosses` (boss moveset construction + tuning), `banter` (ambient combat
//! chatter registry), and the private `ecs` tree (cluster components + the
//! per-actor tick/spawn/damage systems).

use ambition_engine_core as ae;
use ambition_engine_core::AabbExt;
use ambition_platformer_primitives::schedule::SimScheduleExt;
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
// Boss/profile and combat-kit data own their own cooldown/timing constants.

pub mod banter;
pub mod brain_command;
pub mod temporary_control;
pub use temporary_control::TemporaryControl;
// Stable facade for boss attack geometry.
pub use crate::boss_encounter::attack_geometry as boss_attack_geometry;
pub mod bosses;
pub mod ecs;
pub use ecs::{rider_hand_world_pos, rider_hand_world_pos_in_frame};
pub(crate) mod enemies;
mod npcs;

// Re-export the generic combat kit so existing feature-facing paths stay stable.
pub use crate::combat::components;
// Body MECHANICS re-homed off `player/` in the S5/S6 fold (R6d). None of them is
// player-only: `movement_fx` turns a frame's engine `FrameEvents` into Sfx/Vfx
// facts for whichever body produced them; `swim` and `ledge_grab` are thin shims
// over engine-owned water / ledge state and name no `crate::` type at all.
pub mod ledge_grab;
pub mod movement_fx;
pub mod swim;
pub use movement_fx::{
    advance_body_anim_overlays, arm_ground_contact_anim_overlay, arm_movement_anim_overlays,
    emit_movement_fx, handle_player_events,
};

pub use crate::combat::events;
pub use crate::combat::hazard_runtime as hazards;
pub use crate::combat::path_motion;
pub use crate::combat::util;
pub use ambition_world::collision as world_overlay;
pub use ecs::effect_bus as bus;

pub use boss_attack_geometry::{
    active_attack_volumes, body_damage_aabb, bounding_aabb, collision_aabb, damageable_volumes,
    volumes_for_profile, world_space_body_aabbs_from_metrics, world_space_body_aabbs_from_parts,
    AnimationSelection, BossAnimationFrameSample, BossVolumeContext, CombatGeometry,
    SimpleActorGeometry,
};
pub use bosses::{
    boss_attack_moveset, ActorSpriteMetrics, BossAttackProfile, BossBehaviorProfile,
    BossMovementProfile, BossRewardProfile,
};
pub use bus::{
    apply_flag_effects, apply_gameplay_sfx_effects, apply_quest_effects, apply_switch_effects,
};
pub use ecs::{actor_component_snapshot, boss_component_snapshot};
// Runtime minion/summon spawner, re-exported so non-feature modules (e.g. the
// puppy-slug gun) can summon actors without reaching into the private `ecs` tree.
pub(crate) use ecs::spawn_staged_actor_into;
pub use ecs::GiantHandPlan;
pub(crate) use ecs::{
    giant_hand_plans, populate_giant_hand_into, populate_giant_host_into,
    spawn_boss_with_overrides_into, spawn_enemy_with_faction_into, spec_is_limbed_host,
};
pub(crate) use ecs::{spawn_runtime_minion, spawn_runtime_minion_into};

pub use components::{
    ActorAggression, ActorCooldowns, ActorDisposition, ActorFaction, ActorIdentity, ActorIntent,
    ActorInteraction, ActorPose, ActorRenderSize, ActorTarget, AggressionMode, AggressionTarget,
    BodyMelee, BossDeathAnimation, BossPatternTimer, BossPhase, BossRewardChest, BreakableFeature,
    CenteredAabb, ChestFeature, Collected, CombatKit, DamageableVolumes, EncounterMob,
    EncounterRewardChest, FallingChest, FeatureId, FeatureName, MeleeSwing, Opened, PersistKey,
    PickupFeature, PogoPolicy, PogoTargetContributor, PogoTargetVolumes, PostBossNpc, RespawnTimer,
    RuntimeStagedActor, SandboxSolidContributor, StandTimer,
};
// Switch machinery + the quest-advance message live with their owning domains
// (E2): the hub keeps the names importable until it dissolves (E7/E8).
pub use crate::encounter::{SwitchActivated, SwitchFeature, SwitchOn};
pub use crate::world::rooms::LastConstructionVerification;
pub use ambition_persistence::quest::QuestAdvanceRequested;
pub use ambition_platformer_primitives::feature_kind::{BoundFeatureKind, FeatureVisualKind};
pub use brain_command::{
    apply_brain_commands, apply_release_provocations, BrainCommand, BrainCommandKind,
    BrainCommandPlugin, ReleaseProvocation,
};
pub use ecs::actor_bundles::{
    ChestBundle, EnemyActorBundle, FeatureBaseBundle, FeatureLifecycleBundle,
    FeatureRenderedBundle, PickupBundle,
};
pub use ecs::actor_clusters::{
    ActorClusterSeed, ActorConfig, ActorMotionPath, ActorMut, ActorStatus, BodyKinematics,
};
pub use ecs::actor_tuning::{ActorTuning, CharacterBrainSpec, CharacterBrainTemplate};
pub use ecs::{
    advance_actor_anim_overlays, apply_actor_contact_damage, apply_actor_stimuli,
    apply_feature_hit_events, apply_gameplay_banner_requests, apply_hitbox_damage,
    apply_spawn_actor_requests, apply_summon_effects, boss_anim_state_for, boss_is_cleared,
    boss_spawn_hurtboxes, can_damage, clear_encounter_reward_ecs, collect_ecs_pickups,
    damage_lands, derive_boss_sprite_metrics, derive_pogo_target_volumes, dissolve_settled_grudges,
    drive_boss_animators, ecs_boss_anim_state, ecs_boss_anim_state_and_entity,
    ecs_boss_animation_frame_sample, ecs_breakable_state, ecs_chest_opened,
    ecs_hit_event_hits_actor, ecs_hit_event_hits_boss, ecs_hit_event_hits_breakable,
    enforce_mount_rider_link, fan_out_limb_intents, integrate_boss_bodies, integrate_sim_bodies,
    interact_ecs_actors_and_switches, magnetize_pickups, open_ecs_chests,
    project_boss_attack_state_from_move, rebuild_feature_ecs_world_overlay,
    reconcile_autonomous_actors, refresh_actor_damageable_volumes, refresh_boss_damageable_volumes,
    refresh_breakable_damageable_volumes, reset_ecs_room_features, route_boss_strikes_to_limbs,
    select_actor_targets, spawn_encounter_mob, spawn_enemy_projectiles_from_brain_actions,
    spawn_room_feature_entities_from_plan, steer_mount_from_rider,
    sync_actor_poses_from_feature_aabbs, sync_actor_read_model, sync_boss_actor_components,
    sync_boss_encounter_phase, sync_boss_reward_chests_ecs, sync_ecs_actors_with_save,
    sync_ecs_bosses_with_save, sync_ecs_switches_from_save, sync_encounter_reward_chests_ecs,
    sync_riders_to_mounts, tick_actor_brains, tick_and_despawn_hitboxes, tick_boss_brains_system,
    tick_gameplay_banner, tick_npc_idle_barks, tick_pending_challenges, trigger_boss_attack_moves,
    update_ecs_bosses, update_ecs_breakables, update_ecs_falling_chests, update_ecs_hazards,
    ActorConstructionContext, ActorSteering, BossClusterQueryData, BossClusterRef,
    BossClusterScratch, BossConfig, BossEncounter, BossMut, BossOverrides, BossRef, CanPilot,
    ControlGrant, FactionRelations, FeatureEcsWorldOverlay, FeatureSimEntity, FriendlyFire,
    HazardFeature, HeldItem, Hitbox, HitboxAnchor, HitboxHits, HitboxKnockback, HitboxLifetime,
    Limb, LimbIntents, LimbRig, LimbRouteState, LimbSlot, Mass, MountClass, MountDeathImpact,
    MountDied, MountSlot, Mountable, Mounted, MountedBrainCache, MountedSize, PendingChallenge,
    RidingOn, RoomContentStagingError, RoomContentStagingRegistrationError,
    RoomContentStagingRegistry, RoomFeatureConstructionError, RoomFeatureConstructionPlan,
    RoomFeatureConstructionReceipt, SpawnActorKind, SpawnActorRequest, CHALLENGE_GRACE_S,
};
pub use ecs::{AxisSweptMotion, MomentumMotion, MotionModel};
pub use enemies::{
    enemy_spawn_is_sandbag, ActorSpawnState, ActorSurfaceState, CharacterRoster,
    CharacterRosterAppExt, CharacterRosterAssemblyError, CharacterRosterFragment,
    CharacterRosterRegistry, RespawnPolicy, ENEMY_DEAD_UNTIL_REST_SUFFIX,
};
pub use events::{
    ActorStimulus, FeatureCombatTuning, GameplayBanner, GameplayBannerRequested,
    GameplaySfxRequested, HitEvent, HitKnockback, HitKnockbackMagnitude, HitMode, HitSource,
    HitTarget, NpcDialogueRequest, ResetRoomFeaturesEvent, RoomResetReason, SetFlagRequested,
};
pub use hazards::HazardRuntime;
pub use npcs::{NPC_PATROL_SPEED, NPC_TALK_RADIUS};
pub use path_motion::PathMotion;

pub(super) use npcs::NPC_HOSTILE_STRIKE_THRESHOLD;
use util::*;

/// Schedules the gameplay-effect bus chain into
/// [`crate::schedule::SandboxSet::GameplayEffects`].
pub struct GameplayEffectsSchedulePlugin;

impl bevy::prelude::Plugin for GameplayEffectsSchedulePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        let sim = app.sim_schedule();
        use ambition_platformer_primitives::schedule::gameplay_allowed;
        use bevy::prelude::IntoScheduleConfigs;
        app.add_systems(
            sim,
            (
                bus::apply_flag_effects,
                bus::apply_quest_effects,
                bus::apply_switch_effects,
                // Deferred-challenge grace runs only in `Playing` (after the dialog
                // box closes), then emits the `Challenged` stimulus the next system
                // consumes.
                ecs::tick_pending_challenges.run_if(gameplay_allowed),
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
        let sim = app.sim_schedule();
        use crate::world::placements::PlacementLoweringAppExt;
        use bevy::prelude::IntoScheduleConfigs;
        // Relational targeting seam (default = today's behavior; stealth/bounty/
        // alliance systems mutate it). `select_actor_targets` reads it. Combat
        // owns these resources (rule 5); WorldPrep just invokes its registrar.
        crate::combat::targeting::init_targeting_resources(app);
        app.register_placement_interpreter(
            ambition_entity_catalog::placements::PlacementKind::Hazard,
            "ambition_actors",
            "WorldPrepSchedulePlugin",
            "placement.hazard.v1",
            crate::features::ecs::spawn_static::lower_hazard_placement,
        );
        app.register_placement_interpreter(
            ambition_entity_catalog::placements::PlacementKind::Interactable,
            "ambition_actors",
            "WorldPrepSchedulePlugin",
            "placement.interactable.v1",
            crate::features::ecs::spawn_static::lower_interactable_placement,
        );
        app.register_placement_interpreter(
            ambition_entity_catalog::placements::PlacementKind::Pickup,
            "ambition_actors",
            "WorldPrepSchedulePlugin",
            "placement.pickup.v1",
            crate::features::ecs::spawn_static::lower_pickup_placement,
        );
        app.register_placement_interpreter(
            ambition_entity_catalog::placements::PlacementKind::Chest,
            "ambition_actors",
            "WorldPrepSchedulePlugin",
            "placement.chest.v1",
            crate::features::ecs::spawn_static::lower_chest_placement,
        );
        app.register_placement_interpreter(
            ambition_entity_catalog::placements::PlacementKind::Breakable,
            "ambition_actors",
            "WorldPrepSchedulePlugin",
            "placement.breakable.v1",
            crate::features::ecs::spawn_static::lower_breakable_placement,
        );
        #[cfg(feature = "portal")]
        app.register_placement_interpreter(
            ambition_entity_catalog::placements::PlacementKind::Portal,
            "ambition_actors",
            "WorldPrepSchedulePlugin",
            "placement.portal.v1",
            crate::features::ecs::spawn_static::lower_portal_placement,
        );
        // Accumulating sim-time for brain perception (reaction latency).
        app.init_resource::<GameplayElapsed>();
        // Hot-reload watcher state read by `poll_ldtk_file_changes` below.
        // Default = watcher disabled; the visible app pre-inserts its
        // `from_catalog` value before the engine group (init never clobbers).
        app.init_resource::<crate::ldtk_world::LdtkHotReloadState>();
        app.add_systems(
            sim,
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
            sim,
            advance_gameplay_elapsed
                .before(select_actor_targets)
                .in_set(crate::schedule::SandboxSet::WorldPrep),
        );
        // R1.3: the SIM owns the boss animation frame + writes the geometry sample
        // (retiring the render→sim write-back in `animate_bosses`). Runs after the
        // `BossAttackState` projection so it picks this frame's anim, and before the
        // renderer's presentation `animate_bosses` (a later schedule), which mirrors
        // the sim-driven frame into its draw-only animator.
        app.add_systems(
            sim,
            drive_boss_animators.after(project_boss_attack_state_from_move),
        );
        // The decomposed per-actor pipeline: brain → intent, movement integration,
        // read-model mirror, and contact-damage observer, as four explicit phases.
        // Chained (they share the actor cluster + `ActorControl`/`BodyCombat`) and
        // slotted where the old `update_ecs_actors` monolith ran (after target
        // selection, before the NPC bark ticker). Registered separately from the big
        // WorldPrep tuple, which is at Bevy's chain-length ceiling.
        app.init_resource::<ActorSteering>();
        app.init_resource::<crate::features::ecs::perception::PerceptionPeers>();
        app.init_resource::<crate::features::ecs::perception::PerceptionProjectiles>();
        app.add_systems(
            sim,
            (
                // §A7: grant every brained non-boss actor SIGHTED perception
                // (`Perception::Sighted` + a `PerceptionMemory` belief store) before the
                // brain tick reads it, so a foe that leaves its viewport is still pursued
                // from belief. Then snapshot every body's peer data + every live
                // projectile BEFORE the brain tick reads them, so a sighted body perceives
                // the surrounding world without a second borrow of the actor query.
                // (Bodies without a `Perception` — a boss, a fixture — default to the
                // basic `Omniscient` mode, reading the global `ActorTarget` directly.)
                crate::features::ecs::perception::ensure_perception,
                crate::features::ecs::perception::collect_perception_peers,
                crate::features::ecs::perception::collect_perception_projectiles,
                tick_actor_brains,
                // ADR 0020: a mount with a rider defers its locomotion to the
                // rider's brain (the orbit lives on the rider). Runs after the
                // brain tick (rider control frame fresh) and before the body
                // integrate (mount executes the routed intent).
                steer_mount_from_rider,
                // Advance moving platforms ONCE before any body integrates, so every
                // body (home + actors) rides THIS frame's platform positions — the
                // home body used to advance them in `PlayerSimulation`, after the
                // actors integrated, so actors read stale positions; unifying the
                // movement phase unifies this too.
                crate::avatar::advance_moving_platforms,
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
        // The body-orientation righting reflex: feet toward gravity — or, for a
        // riding momentum body, feet onto the ridden surface via the
        // `SurfaceUpright` fact the integration just published. Host-simulation
        // owned so EVERY game gets it (it used to ride inside the portal
        // plugin, which the demo hosts don't add); the portal transit systems
        // only ADD roll, and run later, in `PlayerSimulation`.
        app.add_systems(
            sim,
            (
                ambition_platformer_primitives::orientation::ensure_actor_roll,
                ambition_platformer_primitives::orientation::update_actor_roll,
            )
                .chain()
                .after(integrate_sim_bodies)
                .in_set(crate::schedule::SandboxSet::WorldPrep),
        );
        // Settle decided feuds before targeting reads grudges: a body forgets a slain
        // foe (won't re-aggro if it revives) and a defeated body forgets its own feud
        // (revives as a normal NPC). Registered separately — the WorldPrep chain tuple
        // is already at Bevy's chain-length ceiling — with `.before` to keep the order.
        app.add_systems(
            sim,
            dissolve_settled_grudges
                .before(select_actor_targets)
                .in_set(crate::schedule::SandboxSet::WorldPrep),
        );
        // Q18 (G3): translate a rider-boss's live strike into per-limb intents on
        // its linked mount, then fan those out onto each limb body. `route_...`
        // bridges the `RidingOn`/`MountSlot` link (attack state on the RIDER, limbs
        // on the MOUNT) and writes `LimbIntents`; `fan_out_limb_intents` copies each
        // slot's frame onto its limb's `ActorControl`. Runs in the movement phase —
        // after the mount steer, before the bodies integrate — so each limb
        // EXECUTES its routed arc the same frame it's written.
        //
        // Frame contract: the router reads the rider's `BossAttackState`, a
        // sim-owned READ-MODEL projected from the live `MovePlayback` in the combat
        // phase (`project_boss_attack_state_from_move`), so it sees the PREVIOUS
        // frame's projection — the standard one-frame read-model lag every other
        // consumer of that projection accepts. It must NOT be ordered
        // `.after(tick_boss_brains_system)`: the boss chain runs after
        // `integrate_sim_bodies` (the actor chain is `.before(tick_npc_idle_barks)`,
        // which precedes the boss tick in the WorldPrep chain), so demanding
        // boss-tick < router < integrate is an unsatisfiable before/after CYCLE —
        // it paniced the whole app schedule at startup (caught 2026-07-05; the
        // rl_sim headless app tests are the regression guard for this).
        // Registered separately — the WorldPrep chain tuple is already at Bevy's
        // chain-length ceiling.
        app.add_systems(
            sim,
            (route_boss_strikes_to_limbs, fan_out_limb_intents)
                .chain()
                .after(steer_mount_from_rider)
                .before(integrate_sim_bodies)
                .in_set(crate::schedule::SandboxSet::WorldPrep),
        );
        app.configure_sets(
            sim,
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
        let sim = app.sim_schedule();
        use bevy::prelude::IntoScheduleConfigs;
        app.add_systems(
            sim,
            (
                // Pull nearby loot toward the player, then collect on overlap.
                magnetize_pickups,
                collect_ecs_pickups,
                crate::avatar::apply_player_heal_requests,
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
        let sim = app.sim_schedule();
        use bevy::prelude::IntoScheduleConfigs;
        app.add_systems(
            sim,
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

#[cfg(test)]
mod actor_movement_tests;

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
