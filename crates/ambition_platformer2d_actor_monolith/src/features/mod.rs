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

use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::AabbExt;
use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;
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
pub mod combat_rules;
pub mod stocks_match;
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
pub mod empowerment;
pub mod ledge_grab;
pub mod movement_fx;
pub mod swim;
pub mod transform_beat;
pub use movement_fx::{
    advance_body_anim_overlays, arm_ground_contact_anim_overlay, arm_movement_anim_overlays,
    emit_movement_fx, handle_player_events,
};

pub use crate::combat::events;
pub use crate::combat::hazard_runtime as hazards;
pub use crate::combat::path_motion;
pub use crate::combat::util;
pub use ambition_platformer2d_world::collision as world_overlay;
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
    giant_hand_plans, is_limbed_host, populate_giant_hand_into, populate_giant_host_into,
    spawn_boss_with_overrides_into, spawn_enemy_with_faction_into,
};
pub(crate) use ecs::{spawn_runtime_minion, spawn_runtime_minion_into};
// ⭐ **the ONE thing `crate::conversation` reaches back into `features` for**: a
// bark line for a character in a situation. Named explicitly rather than opening
// the whole `npcs` module, because when the conversation module is carved out
// this single function IS its port — and a `pub(crate) mod` would have hidden
// how small the remaining coupling is.
pub use npcs::speak_conversation_cut_barks;

pub use components::{
    ActorAggression, ActorDisposition, ActorFaction, ActorIdentity, ActorInteraction, ActorPose,
    ActorRenderSize, ActorSpriteOffset, ActorTarget, AggressionMode, AggressionTarget, BodyMelee,
    BossDeathAnimation, BossPatternTimer, BossPhase, BossRewardChest, BreakableFeature,
    CenteredAabb, ChestFeature, Collected, CombatKit, DamageableVolumes, EncounterMob,
    EncounterRewardChest, FallingChest, FeatureId, FeatureName, MeleeSwing, Opened, PersistKey,
    PickupFeature, PogoPolicy, PogoTargetContributor, PogoTargetVolumes, PostBossNpc, RespawnTimer,
    RuntimeStagedActor, StandTimer,
};
// Switch machinery + the quest-advance message live with their owning domains
// (E2): the hub keeps the names importable until it dissolves (E7/E8).
pub use crate::encounter::{SwitchActivated, SwitchFeature, SwitchOn};
pub use crate::world::rooms::LastConstructionVerification;
pub use ambition_persistence::quest::QuestAdvanceRequested;
pub use ambition_platformer2d_shared_tangle::feature_kind::{BoundFeatureKind, FeatureVisualKind};
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
pub use ecs::actor_tuning::{ActorTuning, BrainProfile, CharacterBrainTemplate};
pub use ecs::{
    advance_actor_anim_overlays, apply_actor_contact_damage, apply_actor_stimuli,
    apply_feature_hit_events, apply_gameplay_banner_requests, apply_hitbox_damage,
    apply_spawn_actor_requests, apply_summon_effects, arm_requested_challenges,
    boss_anim_state_for, boss_is_cleared, boss_spawn_hurtboxes, can_damage,
    clear_encounter_reward_ecs, collect_ecs_pickups, damage_lands, derive_boss_sprite_metrics,
    derive_pogo_target_volumes, dissolve_settled_grudges, drive_boss_animators,
    ecs_boss_anim_state, ecs_boss_anim_state_and_entity, ecs_boss_animation_frame_sample,
    ecs_breakable_state, ecs_chest_opened, ecs_hit_event_hits_actor, ecs_hit_event_hits_boss,
    ecs_hit_event_hits_breakable, enforce_mount_rider_link, fan_out_limb_intents,
    integrate_boss_bodies, integrate_sim_bodies, interact_ecs_actors_and_switches,
    magnetize_pickups, open_ecs_chests, project_boss_attack_state_from_move,
    rebuild_feature_ecs_world_overlay, refresh_body_damageable_volumes,
    refresh_boss_damageable_volumes, refresh_breakable_damageable_volumes, reset_ecs_room_features,
    route_boss_strikes_to_limbs, select_actor_targets, spawn_encounter_mob,
    spawn_enemy_projectiles_from_brain_actions, spawn_room_feature_entities_from_plan,
    steer_mount_from_rider, sync_actor_poses_from_feature_aabbs, sync_actor_read_model,
    sync_boss_actor_components, sync_boss_encounter_phase, sync_boss_reward_chests_ecs,
    sync_ecs_actors_with_save, sync_ecs_bosses_with_save, sync_ecs_switches_from_save,
    sync_encounter_reward_chests_ecs, sync_riders_to_mounts, tick_actor_brains,
    tick_and_despawn_hitboxes, tick_boss_brains_system, tick_gameplay_banner, tick_npc_idle_barks,
    tick_pending_challenges, trigger_boss_attack_moves, update_ecs_bosses, update_ecs_breakables,
    update_ecs_falling_chests, update_ecs_hazards, ActorConstructionContext, ActorSteering,
    BossClusterQueryData, BossClusterRef, BossClusterScratch, BossConfig, BossEncounter, BossMut,
    BossOverrides, BossRef, CanPilot, ChallengeRequested, ControlGrant, EncounterMobSeed,
    FactionRelations, FeatureEcsWorldOverlay, FeatureSimEntity, FeatureWorldOverlaySet,
    FriendlyFire, HazardFeature, HazardTickSet, HeldItem, Hitbox, HitboxAnchor, HitboxHits,
    HitboxKnockback, HitboxLifetime, Limb, LimbIntents, LimbRig, LimbRouteState, LimbSlot, Mass,
    MountClass, MountDeathImpact, MountDied, MountRiderLinkEnforced, MountSlot, Mountable, Mounted,
    MountedBrainCache, MountedSize, PendingChallenge, PickupArt, PickupCollect, PickupCollectLock,
    PickupMagnetize, RidingOn, RoomContentStagingError, RoomContentStagingRegistrationError,
    RoomContentStagingRegistry, RoomFeatureConstructionError, RoomFeatureConstructionPlan,
    RoomFeatureConstructionReceipt, SpawnActorKind, SpawnActorRequest, CHALLENGE_GRACE_S,
};
pub use ecs::{AxisSweptMotion, MomentumMotion, MotionModel};
pub use enemies::{
    ActorSpawnState, ActorSurfaceState, RespawnPolicy, ENEMY_DEAD_UNTIL_REST_SUFFIX,
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
/// [`crate::schedule::Platformer2dSimulationPhaseMonolith::GameplayEffects`].
pub struct GameplayEffectsSchedulePlugin;

impl bevy::prelude::Plugin for GameplayEffectsSchedulePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        let sim = app.sim_schedule();
        use ambition_platformer2d_shared_tangle::schedule::gameplay_allowed;
        use bevy::prelude::IntoScheduleConfigs;
        app.add_systems(
            sim,
            (
                // **What the narrative asked the simulation to do**, applied
                // here rather than by the Yarn command that asked. Each of these
                // used to be a presentation-side write into rollback state or
                // into a channel the host clears on rollback; the ledger hands
                // them over on the tick they were stamped for. See
                // `crate::conversation::ledger`.
                //
                // ⚠ ARMING a challenge is first, so the grace that
                // `tick_pending_challenges` counts down starts on the tick the
                // narrative asked for it rather than the one after.
                ecs::arm_requested_challenges,
                crate::items::narrative::apply_item_grants,
                crate::items::narrative::apply_shop_transactions,
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
                .in_set(crate::schedule::Platformer2dSimulationPhaseMonolith::GameplayEffects),
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
/// Register the DAMAGE-facing publication of every body's damageable volumes.
///
/// Extracted from `WorldPrepSchedulePlugin` so the ordering is expressed exactly
/// once, and so a test can assert the production wiring instead of asserting a
/// copy of it. A `.before`/`.after` naming an unregistered system is silently
/// ignored by Bevy, which makes a hand-rebuilt ordering in a test the easiest
/// possible way to prove nothing.
///
/// This is the SECOND invocation of `refresh_body_damageable_volumes`; the first
/// runs in `WorldPrep` for the pogo derivation and the collision overlay. Same
/// function, same rule, two consumers with different timing needs — see the
/// system's own documentation for why that is a refresh and not a clobber.
pub fn register_damage_facing_volume_publication(app: &mut bevy::prelude::App) {
    use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;
    let sim = app.sim_schedule();
    app.add_systems(
        sim,
        refresh_body_damageable_volumes
            .in_set(crate::schedule::Platformer2dSimulationPhaseMonolith::Combat)
            // Victim geometry is published between the move clock and the damage
            // pass: AFTER `Playback`, because a move's first active frame must not
            // publish the previous frame's volumes, and BEFORE `Resolve`, because
            // that is what reads them. Stated as PHASES — the leaf pair this used
            // to name (`advance_move_playback` / `apply_hitbox_damage`) is the same
            // constraint written in a way a caller cannot check.
            .after(crate::schedule::CombatSet::Playback)
            .before(crate::schedule::CombatSet::Resolve)
            // The one intra-crate edge that is genuinely between two systems: the
            // character runtime resolves the silhouette this reads.
            .after(crate::character_runtime::hurtbox::resolve_body_hurtboxes),
    );
}

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
        // AE6: the rules a MATCH plays under, resolved from its declaration
        // folded over those baselines — so a stage never writes them. In
        // WorldPrep because every reader is later (PlayerSimulation/Combat),
        // and a resolution landing after them would hand the hit kernel last
        // tick's rules on the one tick they differ.
        app.init_resource::<crate::combat::rules::ResolvedCombatTuning>();
        app.add_systems(
            sim,
            crate::features::combat_rules::project_combat_rules
                .in_set(crate::schedule::Platformer2dSimulationPhaseMonolith::WorldPrep),
        );
        // S4: spend a stock per KO. `CombatSet::Settle` is the phase for
        // "everything that reads this tick's damage outcome rather than
        // producing it", which is exactly what this is — the KO was decided in
        // Resolve, and spending is bookkeeping over it.
        app.init_resource::<ambition_combat::stocks::StocksMatchSettled>();
        // ⚠ REGISTERED WHERE THE WRITERS ARE SCHEDULED, not one crate up.
        // `apply_feature_hit_events` and the player hit path both write
        // `BodyHitResolved`, and registering it in `ambition_platformer2d_runtime` left every
        // `ambition_platformer2d_actor_monolith` fixture panicking "Message not initialized" the
        // moment a hit landed — the exact defect `BodyKnockedOut` already cost
        // this repo once. A writer whose message is registered by a different
        // plugin is a composition that works until somebody composes
        // differently, and a test fixture IS somebody composing differently.
        #[cfg(feature = "causal")]
        app.add_message::<crate::features::ecs::damage_apply::BodyHitResolved>();
        #[cfg(feature = "causal")]
        app.add_message::<crate::causal::BodyMovementOps>();
        // **AN INSTRUMENT REGISTERS WHAT IT READS.**
        //
        // ⛔ found by running `ladder_probe --features causal`, which panicked
        // on the first tick with "Message not initialized". The three causal
        // observers below read five message types and the composition only ever
        // registered one; every other registration came from whichever gameplay
        // plugin happened to WRITE that message, so turning the inspector on in
        // a composition lacking one of those plugins killed the app. The
        // inspector could not be enabled for the game it was built to inspect.
        //
        // ⚠ this is the READER half of the rule already learned on the writer
        // side: a knockout is acted on by the SIMULATION and must fail loud, but
        // an INSTRUMENT must degrade. `add_message` is idempotent, so a
        // composition that already registers these is unaffected and one that
        // does not gets an empty stream instead of a panic — the honest reading
        // of "nothing knocked anybody out this tick".
        #[cfg(feature = "causal")]
        {
            app.add_message::<ambition_combat::stocks::BodyKnockedOut>();
            app.add_message::<ambition_combat::stocks::FighterStockSpent>();
            app.add_message::<ambition_combat::stocks::StocksMatchDecided>();
            app.add_message::<crate::features::ecs::damage_apply::BodyReactionApplied>();
        }
        app.add_systems(
            sim,
            (
                ambition_combat::stocks::spend_fighter_stocks
                    .in_set(ambition_combat::stocks::FighterStocksSpent),
                // AFTER the spend, in the same phase: a match decided before this
                // tick's elimination lands would announce the previous frame's
                // answer on the frame the last fighter goes out.
                crate::features::stocks_match::decide_stocks_match,
                // The causal OBSERVER, last in the chain so it reads this tick's
                // decision rather than the previous one. It holds no authority
                // over any of the above — it reads their messages — which is why
                // it can sit inside the ruleset's own chain safely.
                #[cfg(feature = "causal")]
                ambition_combat::causal::record_stock_lifecycle,
                // The damage observer, in the same phase: `Settle` is "reads
                // this tick's damage outcome rather than producing it", which
                // is exactly what an explanation of that outcome is.
                #[cfg(feature = "causal")]
                crate::causal::record_hit_resolutions,
                #[cfg(feature = "causal")]
                crate::causal::record_hit_reactions,
            )
                .chain()
                .in_set(crate::schedule::CombatSet::Settle),
        );
        app.register_placement_interpreter(
            ambition_entity_catalog::placements::PlacementKind::Hazard,
            "ambition_platformer2d_actor_monolith",
            "WorldPrepSchedulePlugin",
            "placement.hazard.v1",
            crate::features::ecs::spawn_static::lower_hazard_placement,
        );
        app.register_placement_interpreter(
            ambition_entity_catalog::placements::PlacementKind::Interactable,
            "ambition_platformer2d_actor_monolith",
            "WorldPrepSchedulePlugin",
            "placement.interactable.v1",
            crate::features::ecs::spawn_static::lower_interactable_placement,
        );
        app.register_placement_interpreter(
            ambition_entity_catalog::placements::PlacementKind::Pickup,
            "ambition_platformer2d_actor_monolith",
            "WorldPrepSchedulePlugin",
            "placement.pickup.v1",
            crate::features::ecs::spawn_static::lower_pickup_placement,
        );
        app.register_placement_interpreter(
            ambition_entity_catalog::placements::PlacementKind::Chest,
            "ambition_platformer2d_actor_monolith",
            "WorldPrepSchedulePlugin",
            "placement.chest.v1",
            crate::features::ecs::spawn_static::lower_chest_placement,
        );
        app.register_placement_interpreter(
            ambition_entity_catalog::placements::PlacementKind::Breakable,
            "ambition_platformer2d_actor_monolith",
            "WorldPrepSchedulePlugin",
            "placement.breakable.v1",
            crate::features::ecs::spawn_static::lower_breakable_placement,
        );
        #[cfg(feature = "portal")]
        app.register_placement_interpreter(
            ambition_entity_catalog::placements::PlacementKind::Portal,
            "ambition_platformer2d_actor_monolith",
            "WorldPrepSchedulePlugin",
            "placement.portal.v1",
            crate::features::ecs::spawn_static::lower_portal_placement,
        );
        // Accumulating sim-time for brain perception (reaction latency).
        app.init_resource::<GameplayElapsed>();
        // Hot-reload watcher state read by `poll_ldtk_file_changes` below.
        // Default = watcher disabled; the visible app pre-inserts its
        // `from_catalog` value before the engine group (init never clobbers).
        app.init_resource::<ambition_platformer2d_ldtk::LdtkHotReloadState>();
        app.add_systems(
            sim,
            (
                ambition_platformer2d_ldtk::poll_ldtk_file_changes,
                // Sprite-driven boss metrics must be available before
                // boss damageable/pogo volumes are derived, otherwise
                // composite bosses such as GNU-ton would briefly fall
                // back to their coarse spawn envelope.
                derive_boss_sprite_metrics,
                refresh_body_damageable_volumes,
                refresh_boss_damageable_volumes,
                refresh_breakable_damageable_volumes,
                derive_pogo_target_volumes,
                rebuild_feature_ecs_world_overlay
                    .in_set(crate::world::overlay::FeatureWorldOverlaySet),
                update_ecs_hazards.in_set(ambition_combat::hazards::HazardTickSet),
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
                .in_set(crate::schedule::Platformer2dSimulationPhaseMonolith::WorldPrep),
        );
        // "This body starts again", announced to whoever authored it.
        //
        // At the FRONT of the tick on purpose. A body can be reset in almost any
        // phase — a death in `PlayerSimulation`, a room arrival, a sandbox reset
        // in `ResetProcessing`, a versus round in `Combat` — and by an app or a
        // provider the engine has never heard of. Announcing here means every
        // one of those is delivered before anything acts on that body again,
        // whichever phase performed it and whether or not it ran on this
        // schedule at all.
        app.add_systems(
            sim,
            ae::announce_body_restarts
                .in_set(crate::schedule::Platformer2dSimulationPhaseMonolith::WorldPrep)
                .before(rebuild_feature_ecs_world_overlay),
        );
        // Advance the accumulating sim clock before any actor brain reads its
        // perception snapshot, so reaction-latency lookback is live. Registered
        // separately (not in the chain above) only because that tuple is already
        // at Bevy's chain-length ceiling; the `.before` keeps the ordering exact.
        app.add_systems(
            sim,
            advance_gameplay_elapsed
                .before(select_actor_targets)
                .in_set(crate::schedule::Platformer2dSimulationPhaseMonolith::WorldPrep),
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
        // ── The SECOND publication of every body's damageable volumes ──
        //
        // Same function, same rule, later phase. The `WorldPrep` copy above feeds
        // the pogo derivation and the feature-world collision overlay, which are
        // rebuilt in that set; this copy exists because DAMAGE resolves in
        // `Combat`, and a body's `CenteredAabb` is written by its own integrator —
        // an actor's in `WorldPrep`, the PLAYER's in `PlayerSimulation`. Publishing
        // only in `WorldPrep` would therefore hand `apply_hitbox_damage` a player
        // silhouette one frame stale, which is the Mary-O contact defect again: a
        // hit classifier must read the positions the contact pass reads.
        //
        // `in_set(CoreSimulation)` keeps it inside `GameplaySimulationRoot`, so the
        // session gate covers it like everything else.
        register_damage_facing_volume_publication(app);
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
                // WHO IS AWAKE, decided before anybody decides anything. It is
                // recomputed from live positions every tick and inserts/removes
                // one marker, so the brain tick below can simply not match a
                // sleeping actor. Chained (not merely ordered) because the
                // marker is applied by `Commands` and must be flushed before the
                // query that filters on it runs.
                crate::features::ecs::dormancy::assess_dormancy,
                // ⭐ **THE GAME'S RUNGS, applied before the first decision.** A
                // fighter is constructed with the engine FLOOR because the
                // authored ladder lives in the content pack, above this crate and
                // out of reach of the spawn tree's many roots. This rewrites a
                // freshly-inserted fighter brain from the game's own rows.
                //
                // ⚠ before `tick_actor_brains` and immediately so: the projection
                // rebuilds `FighterState`, and doing that after a decision would
                // discard the habits that decision accumulated. Chained, so the
                // brain that ticks below is the one this wrote.
                crate::features::ecs::project_authored_fighter_ladder,
                tick_actor_brains,
                // **IMMEDIATELY after the writer, and that placement is the
                // whole correctness of the instrument.**
                //
                // ⛔ it was first registered in `PlayerInputSet::Brain`, after
                // `tick_controlled_brains` — a WHOLE PHASE EARLIER than this one. So
                // for every actor-brained body it read the PREVIOUS tick's
                // `ActorControl` and printed it beside THIS tick's decision. On
                // a level-9 ladder run that produced 378 apparent "the brain
                // asked left and the body went right" rows, every one of them
                // the instrument looking at a stale frame. The bug the trace
                // exists to find has exactly that shape, so the artifact was
                // indistinguishable from the finding — and this thread has been
                // fooled by a measurement artifact before. Correctly ordered,
                // `asked != holding` is 0 of 2279.
                #[cfg(feature = "causal")]
                crate::causal::record_body_control_frame,
                // The kernel's own operation list — `Dash`, `DodgeRoll`,
                // `WallJump`, `LedgeClimbStart`, … — which `FrameEvents` has
                // always carried and nothing published. One recorder covers
                // every velocity writer inside the movement kernel, which
                // cannot publish for itself: it has no `ambition_causal`
                // dependency and the floor contract allows it only
                // `ambition_geometry`.
                #[cfg(feature = "causal")]
                crate::causal::record_movement_operations,
                // The SECOND blanking position, and the one that makes
                // `ScriptedControl` mean the same thing for every body.
                //
                // The first is in `PlayerInputSet::ControlGate`, immediately
                // after `tick_controlled_brains` — "the only position where blanking
                // is observable", which was true of the writer it was placed
                // against and false of this one. Actor brains write
                // `ActorControl` HERE, in `WorldPrep`, a whole phase after that
                // gate, so a CPU-driven body under a scripted beat had its frame
                // blanked and then immediately refilled with the brain's own
                // decision. The versus KO card is where that surfaced: the
                // suspended fighter went on walking and swinging because the
                // marker only ever suppressed human input (GPT 5.6,
                // 2026-07-27).
                //
                // Before `steer_mount_from_rider` deliberately: a scripted rider
                // must not steer its mount either.
                crate::avatar::blank_scripted_control_frames,
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
                //
                // It carries `WorldPrepSet::Integrate` so a consumer — in this
                // crate or in a game — can say "before bodies move" or "after they
                // land" without naming this function.
                integrate_sim_bodies.in_set(crate::schedule::WorldPrepSet::Integrate),
                sync_actor_read_model,
                apply_actor_contact_damage.in_set(crate::schedule::WorldPrepSet::ContactDamage),
            )
                .chain()
                .after(select_actor_targets)
                .before(tick_npc_idle_barks)
                .in_set(crate::schedule::Platformer2dSimulationPhaseMonolith::WorldPrep),
        );
        // ⭐ **`sync_sprite_posed_bodies` used to be registered HERE** and is now
        // `ambition_character_sprites::SpritePosedBodyPlugin`, added beside this
        // plugin in `PlatformerEnginePlugins`. Same set, same schedule, same
        // guarantee that every game gets it — but the registration lives with
        // the system, so this crate does not depend on the crate that owns it.
        // That direction is the entire point of the 2026-08-09 carve: with the
        // line here, an edit to the posed-body derivation rebuilt this crate and
        // everything above it.
        //
        // The body-orientation righting reflex: feet toward gravity — or, for a
        // riding momentum body, feet onto the ridden surface via the
        // `SurfaceUpright` fact the integration just published. Host-simulation
        // owned so EVERY game gets it (it used to ride inside the portal
        // plugin, which the demo hosts don't add); the portal transit systems
        // only ADD roll, and run later, in `PlayerSimulation`.
        app.add_systems(
            sim,
            (
                ambition_platformer2d_shared_tangle::orientation::ensure_actor_roll,
                ambition_platformer2d_shared_tangle::orientation::update_actor_roll,
            )
                .chain()
                .in_set(crate::schedule::WorldPrepSet::AfterIntegrate),
        );
        // Settle decided feuds before targeting reads grudges: a body forgets a slain
        // foe (won't re-aggro if it revives) and a defeated body forgets its own feud
        // (revives as a normal NPC). Registered separately — the WorldPrep chain tuple
        // is already at Bevy's chain-length ceiling — with `.before` to keep the order.
        app.add_systems(
            sim,
            dissolve_settled_grudges
                .before(select_actor_targets)
                .in_set(crate::schedule::Platformer2dSimulationPhaseMonolith::WorldPrep),
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
                .in_set(crate::schedule::WorldPrepSet::BeforeIntegrate),
        );
        app.configure_sets(
            sim,
            crate::schedule::BossSteerSlot
                .after(tick_boss_brains_system)
                .before(update_ecs_bosses)
                .in_set(crate::schedule::Platformer2dSimulationPhaseMonolith::WorldPrep),
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
                magnetize_pickups.in_set(PickupMagnetize),
                collect_ecs_pickups.in_set(PickupCollect),
                crate::avatar::apply_player_heal_requests,
                // Beside the heal apply because it is the same kind of thing: a
                // METER MUTATOR on the controlled subject, scaled by sim dt.
                // ⛔ it lived in the app's HUD chain in `Update` until
                // 2026-08-02, which made a rollback-registered component
                // (`body.mana`) move at render rate and never resimulate on a
                // rewind -- and left every non-app composition with mana that
                // does not refill.
                crate::avatar::regen_player_mana,
            )
                .chain()
                .in_set(crate::schedule::Platformer2dSimulationPhaseMonolith::FeatureCollection),
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
        // The conversation authority is sim state and lives for the whole App;
        // the UI projection that follows it is presentation and runs outside the
        // simulation schedule, so a rewind cannot un-close a box the player
        // already watched close.
        app.init_resource::<crate::conversation::ActiveConversation>();
        // ⛔ **REGISTER THE CHANNEL THE PORT ASKS THROUGH.** The break rule
        // writes it and the cast answers it, both in this plugin's chain — so
        // this plugin owns the registration. Leaving it to whoever else wanted
        // the message is how the effect quarantine once worked in a shipped app
        // and nowhere else; here it failed parameter validation on frame one of
        // the sandbox harness.
        app.add_message::<crate::conversation::ConversationCutBark>();
        // ⚠ **the ledger is NOT rollback state, and that is the whole design.**
        // It is the record of what the narrative — which runs outside the
        // simulation — told the simulation, stamped with the tick it applies
        // from. A rewind restores what the simulation DECIDED; erasing what it
        // was TOLD is how the replay reaches a different answer. The plugin
        // installs the ledger, its release at the head of the sim frame, and the
        // prune that ages a record out once its tick can never be replayed.
        //
        // ⚠ **one per payload, and the list IS the classification** — the
        // counterpart to the table in `crate::dialog::yarn_bindings`. A
        // gameplay-bearing Yarn command has a ledger here or it has no replay
        // story; a presentation-facing one must NOT be here, because deferring
        // a sound to a simulation tick would delay it for no reason. Content
        // registers its own vocabulary the same way, so this names no content.
        app.add_plugins((
            crate::conversation::NarrativeInputPlugin::<crate::conversation::ConversationEnded>::default(),
            crate::conversation::NarrativeInputPlugin::<ambition_combat::events::SetFlagRequested>::default(),
            crate::conversation::NarrativeInputPlugin::<crate::features::ChallengeRequested>::default(),
            crate::conversation::NarrativeInputPlugin::<crate::features::BrainCommand>::default(),
            crate::conversation::NarrativeInputPlugin::<crate::features::ReleaseProvocation>::default(),
            crate::conversation::NarrativeInputPlugin::<ambition_items::ItemGrantRequested>::default(),
            crate::conversation::NarrativeInputPlugin::<ambition_items::shop::ShopTransactionRequested>::default(),
        ));
        // The TWO presentation halves of the seam: one projects the box from the
        // authority (and detaches from it), one observes the runner finishing and
        // records it for the simulation. Neither runs in the sim schedule, which
        // is what keeps a rewind from replaying a side effect onto state it does
        // not rewind.
        //
        // ⛔ **`.chain()`, and the order is load-bearing.** "The runner is not
        // active" is how the second one recognises a finished conversation — and
        // on the frame a conversation OPENS that is also true until the first one
        // has run. Unordered, a conversation could be recorded as finished on the
        // tick it began, and the simulation would close it before a line was
        // ever shown.
        app.add_systems(
            bevy::prelude::Update,
            (
                crate::conversation::project_the_dialog_ui_from_the_conversation,
                crate::conversation::publish_the_narrative_end,
            )
                .chain(),
        );
        app.add_systems(
            sim,
            (
                // The narrative running out of lines is an INPUT to the
                // simulation, and it lands before anything judges the
                // conversation for separation — otherwise a conversation that
                // ended this frame gets barked about on its way out.
                crate::conversation::close_conversation_on_narrative_end,
                interact_ecs_actors_and_switches,
                // ⚠ AFTER the interaction that starts a conversation, in the
                // same chain: a dialogue opened this frame must not be judged
                // for separation before the bodies that opened it have been
                // read. Both use the same `strict_intersects` reach, so a
                // conversation cannot begin and immediately break.
                crate::conversation::break_dialogue_on_hit_or_separation,
                // The CAST half of the break: continuity said who should speak,
                // this says what they say. Immediately after, so the bubble
                // lands on the same tick the conversation ended.
                npcs::speak_conversation_cut_barks,
                // The hold is PROJECTED after, in the same chain: whatever the
                // rule above decided — a break, a body that stopped existing, or
                // nothing at all — the world is made to match the authority on
                // the same frame. ⛔ it is not a "release": it both takes and
                // releases the hold, because a projection that only let go would
                // be a second rule about when to hold.
                crate::conversation::project_conversation_hold,
                open_ecs_chests,
                update_ecs_breakables,
                update_ecs_falling_chests,
                sync_ecs_switches_from_save,
                crate::encounter::rebuild_encounter_switch_index,
            )
                .chain()
                .in_set(crate::schedule::Platformer2dSimulationPhaseMonolith::FeatureInteraction),
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
