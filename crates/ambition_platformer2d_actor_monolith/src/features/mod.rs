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

// ⛔⛔ FIFTY-EIGHT CROSS-CRATE RE-EXPORTS DELETED 2026-08-27 (D33). This module
// forwarded `ambition_combat`'s component vocabulary, its actor kit,
// `ambition_boss_encounter`'s attack geometry, `shared_tangle`'s feature kinds and
// a persistence message under the monolith's address. 515 sites named them
// through here, and every coupling census that counted those read the monolith as
// their owner — which is how `damage_apply` looked like ~70 outward references
// when the answer was zero.
//
// ⭐ Callers name the crate that owns the thing. What is left below is what this
// module actually declares.

use ambition_characters::actor::limb::fan_out_limb_intents;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::AabbExt;
use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;
use bevy::prelude::*;

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

pub mod brain_command;
pub mod combat_rules;
pub mod stocks_match;
// Stable facade for boss attack geometry.
pub mod ecs;
pub(crate) mod enemies;
mod npcs;

// Re-export the generic combat kit so existing feature-facing paths stay stable.
// None of them is player-only: `movement_fx` turns a frame's engine `FrameEvents` into Sfx/Vfx
// facts for whichever body produced them; `swim` and `ledge_grab` are thin shims over
// engine-owned water / ledge state and name no `crate::` type at all.
pub mod empowerment;
pub mod ledge_grab;
pub mod movement_fx;
pub mod swim;
pub mod transform_beat;
pub use movement_fx::{
    advance_body_anim_overlays, arm_ground_contact_anim_overlay, arm_movement_anim_overlays,
    emit_movement_fx, handle_player_events,
};

// ⛔⛔ SIX RE-EXPORT FACADES DELETED 2026-08-26, and FOUR of them had no
// consumer at all. `events`, `hazard_runtime as hazards`, `path_motion`, `util`,
// `collision as world_overlay` and `effect_bus as bus` all forwarded another
// crate's module under this one's address, which is how a coupling census counts
// `ambition_combat` and `ambition_platformer2d_world` as MONOLITH surface. The
// two live consumers named `ambition_combat::events` directly instead, and
// nothing else in the tree named the other four.
//
// ⭐ THE TELL, and it is the one that took the damage carve from an apparent ~70
// outward references to zero: one module spelling one crate two ways. Callers
// name the crate that owns the thing.

pub use ecs::effect_bus::{
    apply_flag_effects, apply_gameplay_sfx_effects, apply_quest_effects, apply_switch_effects,
};
pub use ecs::{actor_component_snapshot, boss_component_snapshot};
// Runtime minion/summon spawner, re-exported so non-feature modules (e.g. the
// puppy-slug gun) can summon actors without reaching into the private `ecs` tree.
pub(crate) use ecs::spawn_staged_actor_into;
pub use ecs::GiantHandPlan;
pub(crate) use ecs::{
    giant_hand_plans, is_limbed_host, spawn_boss_with_overrides_into, spawn_enemy_with_faction_into,
};
pub(crate) use ecs::{spawn_runtime_minion, spawn_runtime_minion_into};
// the CAST half of the conversation port: a bark line for a character in
// a situation. Named explicitly rather than opening the whole `npcs` module,
// because when the conversation module is carved out this single function is
// what answers its `ConversationCutBark`, and a `pub(crate) mod` would have
// hidden how small the remaining coupling is.
//
// Measured: `conversation/` contains zero non-doc `crate::` paths. The temporal half of the same
// port is `FeatureInteractionSet::CutBarkCast`, the phase this system is placed in below.
pub use npcs::speak_conversation_cut_barks;

// Switch machinery + the quest-advance message live with their owning domains
// (E2): the hub keeps the names importable until it dissolves (E7/E8).

pub use crate::world::rooms::LastConstructionVerification;
pub use brain_command::{
    apply_brain_commands, apply_release_provocations, BrainCommand, BrainCommandKind,
    BrainCommandPlugin, ReleaseProvocation,
};
pub use ecs::actor_bundles::{
    ChestBundle, EnemyActorBundle, FeatureBaseBundle, FeatureLifecycleBundle,
    FeatureRenderedBundle, PickupBundle,
};
pub use ecs::actor_clusters::{ActorClusterSeed, ActorMotionPath, ActorMut};
// ⭐ NAMED FROM `ambition_combat`, where the actor's kit vocabulary and its
// config now live (D33, 2026-08-27). Re-exported here only because the
// monolith's own module tree is a public surface many callers still walk.
pub use ecs::anim_helpers::{advance_actor_anim_overlays, ecs_breakable_state, ecs_chest_opened};
pub use ecs::{
    apply_actor_contact_damage, apply_actor_stimuli, apply_feature_hit_events,
    apply_gameplay_banner_requests, apply_hitbox_damage, apply_spawn_actor_requests,
    apply_summon_effects, arm_requested_challenges, boss_anim_state_for, boss_spawn_hurtboxes,
    can_damage, clear_encounter_reward_ecs, collect_ecs_pickups, damage_lands,
    derive_boss_sprite_metrics, derive_pogo_target_volumes, dissolve_settled_grudges,
    drive_boss_animators, ecs_boss_anim_state, ecs_boss_anim_state_and_entity,
    ecs_boss_animation_frame_sample, ecs_hit_event_hits_actor, ecs_hit_event_hits_boss,
    ecs_hit_event_hits_breakable, integrate_boss_bodies, integrate_sim_bodies,
    interact_ecs_actors_and_switches, magnetize_pickups, open_ecs_chests,
    project_boss_attack_state_from_move, rebuild_dismounted_rider_brains,
    rebuild_feature_ecs_world_overlay, refresh_body_damageable_volumes,
    refresh_boss_damageable_volumes, refresh_breakable_damageable_volumes,
    route_boss_strikes_to_limbs, select_actor_targets, snapshot_body_contact, spawn_encounter_mob,
    spawn_projectiles_from_brain_actions, spawn_room_feature_entities_from_plan,
    sync_actor_poses_from_feature_aabbs, sync_actor_read_model, sync_boss_actor_components,
    sync_boss_encounter_phase, sync_ecs_actors_with_save, sync_ecs_bosses_with_save,
    sync_ecs_switches_from_save, sync_encounter_reward_chests_ecs, tick_actor_brains,
    tick_and_despawn_hitboxes, tick_boss_brains_system, tick_gameplay_banner, tick_npc_idle_barks,
    tick_pending_challenges, trigger_boss_attack_moves, update_ecs_bosses, update_ecs_breakables,
    update_ecs_falling_chests, update_ecs_hazards, ActorConstructionContext, ActorSteering,
    ChallengeRequested, EncounterMobSeed, FactionRelations, FeatureWorldOverlaySet, FriendlyFire,
    HazardTickSet, HeldItem, Hitbox, HitboxAnchor, HitboxHits, HitboxKnockback, HitboxLifetime,
    OccurrenceContinuity, PendingChallenge, PickupArt, PickupCollect, PickupCollectLock,
    PickupMagnetize, RoomContentStagingError, RoomContentStagingRegistrationError,
    RoomContentStagingRegistry, RoomFeatureConstructionError, RoomFeatureConstructionPlan,
    RoomFeatureConstructionReceipt, SpawnActorKind, SpawnActorRequest, CHALLENGE_GRACE_S,
};
pub(crate) use ecs::{
    maintain_actor_pre_decision_state, observe_actor_decision_inputs,
    publish_actor_decision_frames, ActorDecisionFacts, ActorDecisionFrames,
};
// ⛔ `MotionModel`, `AxisSweptMotion` and `MomentumMotion` LEFT THIS FACADE,
// 2026-08-26. All three are `ambition_platformer2d_core::movement`'s, re-exported
// twice through this crate — and `MomentumMotion` also RENAMED
// `SurfaceMomentumMotion` on the way, so a caller could not grep the real type.
// Callers name `_core`; the SDK keeps its own one-hop alias.
pub use ambition_entity_catalog::placements::RespawnPolicy;
pub use ambition_platformer2d_core::body_clusters::ActorSurfaceState;
pub use enemies::ENEMY_DEAD_UNTIL_REST_SUFFIX;
// ⛔ THE COMBAT EVENT VOCABULARY LEFT THIS FACADE, 2026-08-26. All fifteen are
// `ambition_combat::events`', re-exported up to `features` beside a whole-module
// `pub use ambition_combat::events` — so 74 sites read as coupling to the actor
// crate for types it does not own, and `damage_apply`'s own tests reached the
// monolith for exactly two names, both of them these. Callers name the owner.
pub use ambition_characters::brain::state_machine::NPC_PATROL_SPEED;
pub use npcs::NPC_TALK_RADIUS;

use ambition_combat::util::*;
pub(super) use npcs::NPC_HOSTILE_STRIKE_THRESHOLD;

/// Schedules the gameplay-effect bus chain into
/// [`ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::GameplayEffects`].
pub struct GameplayEffectsSchedulePlugin;

impl bevy::prelude::Plugin for GameplayEffectsSchedulePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        let sim = app.sim_schedule();
        use ambition_platformer2d_shared_tangle::schedule::GameplayGated;
        use bevy::prelude::IntoScheduleConfigs;
        app.add_systems(
            sim,
            (
                // ARMING a challenge is first, so the grace that
                // `tick_pending_challenges` counts down starts on the tick the
                // narrative asked for it rather than the one after.
                ecs::arm_requested_challenges,
                crate::items::narrative::apply_item_grants,
                crate::items::narrative::apply_shop_transactions,
                ecs::effect_bus::apply_flag_effects,
                ecs::effect_bus::apply_quest_effects,
                ecs::effect_bus::apply_switch_effects,
                // Deferred-challenge grace runs only in `Playing` (after the dialog
                // box closes), then emits the `Challenged` stimulus the next system
                // consumes.
                ecs::tick_pending_challenges.in_set(GameplayGated),
                ecs::apply_actor_stimuli,
                ecs::effect_bus::apply_gameplay_sfx_effects,
            )
                .chain()
                .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::GameplayEffects),
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
/// Extracted from `WorldPrepSchedulePlugin` so the ordering is expressed exactly once, and so a
/// test can assert the production wiring instead of asserting a copy of it.
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
            .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::Combat)
            // Victim geometry is published between the move clock and the damage pass: AFTER
            // `Playback`, because a move's first active frame must not publish the previous frame's
            // volumes, and BEFORE `Resolve`, because that is what reads them.
            .after(ambition_platformer2d_shared_tangle::schedule::CombatSet::Playback)
            .before(ambition_platformer2d_shared_tangle::schedule::CombatSet::Resolve)
            // The one intra-crate edge that is genuinely between two systems: the
            // character runtime resolves the silhouette this reads.
            .after(crate::character_runtime::hurtbox::resolve_body_hurtboxes),
    );
}

/// Ordered authority boundaries for one autonomous actor decision.
///
/// These are deliberately coarser than individual systems. The contract is
/// semantic: targeting settles first, eligibility/projections are prepared,
/// observations are frozen, reaction clocks advance, decision produces plain
/// intent values, and only then does publication mutate `ActorControl`.
/// Movement begins after the whole chain through [`ambition_platformer2d_shared_tangle::schedule::WorldPrepSet`].
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub(crate) enum ActorDecisionSet {
    Targeting,
    Prepare,
    Observe,
    StateMaintenance,
    Decide,
    Publish,
}

fn configure_actor_decision_phases(app: &mut App) {
    let sim = app.sim_schedule();
    app.configure_sets(
        sim,
        (
            ActorDecisionSet::Targeting,
            ActorDecisionSet::Prepare,
            ActorDecisionSet::Observe,
            ActorDecisionSet::StateMaintenance,
            ActorDecisionSet::Decide,
            ActorDecisionSet::Publish,
        )
            .chain()
            .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::WorldPrep),
    );
    app.configure_sets(
        sim,
        ActorDecisionSet::Publish
            .before(ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::BeforeIntegrate),
    );
    // ⭐⭐ THE ONE RESTRICTION PHASE, AND IT IS AFTER BOTH PUBLICATIONS (D202).
    //
    // A possessed body's `ActorControl` is written in `PlayerInputSet::Brain`, a
    // whole phase earlier; an autonomous body's is written just above. Anything
    // that acts on FINISHED control therefore has exactly one legal home — after
    // the LATER of the two — and putting these sets in `PlayerInput` (where their
    // names still say they live) meant they ran before every AI frame in the
    // world. The answer was a SECOND copy of each restriction registered here,
    // and the pair was correct only by an invariant nothing enforced: the first
    // blank is what stopped the second sampler crediting the same human press.
    //
    // ⇒ one placement, one copy. `WorldPrepSet::BeforeIntegrate` still owns the
    // capture pair (sample-then-blank) because that chain has same-tick
    // dependencies of its own; these two sets sit between publication and it.
    app.configure_sets(
        sim,
        (
            ambition_platformer2d_shared_tangle::schedule::PlayerInputSet::ControlGate,
            ambition_platformer2d_shared_tangle::schedule::PlayerInputSet::BodyMode,
        )
            .chain()
            .after(ActorDecisionSet::Publish)
            .before(ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::BeforeIntegrate)
            .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::WorldPrep),
    );

    install_actor_decision_census_boundary(app);
}

/// Close the census bucket that owns everything this file schedules.
///
/// ⛔⛔ **WITHOUT THIS, ALL OF IT BILLED TO `WorldPrep.BeforeIntegrate`.** Every
/// set configured above is `in_set(WorldPrep)` and `before(BeforeIntegrate)`, and
/// the sim-phase census's first `WorldPrep` mark closes AFTER `BeforeIntegrate`.
/// A boundary instrument attributes "now minus the previous mark", so the six
/// decision sets plus `ControlGate` and `BodyMode` all landed in a bucket named
/// after a set none of them belong to. A windowed Hall capture on 2026-08-31
/// read 1.214 ms/tick there and it was taken for movement preparation.
///
/// ⭐ THE MARK IS INSTALLED HERE BECAUSE ONLY THIS CRATE CAN NAME THE SETS.
/// `ambition_dev_tools` depends on `shared_tangle`, not on the monolith, so it
/// cannot order a system after `ActorDecisionSet::Publish`; the dependency edge
/// runs the other way, and this crate already has it.
///
/// ⚠ THE SPAN IS `Targeting` THROUGH `BodyMode`, not `Targeting` through
/// `Publish` — the two gate sets sit between publication and `BeforeIntegrate`,
/// so a mark after `Publish` would have left them misattributed exactly as
/// before, just less of them.
///
/// ⭐ MEASURED 2026-09-01, and it is why the six sub-marks below exist. In
/// `hall_of_characters` at 130 bodies, headless and without Tracy, the span was
/// **0.958 ms/tick against `BeforeIntegrate`'s 0.037** — 96% of a bucket that had
/// been read as movement preparation. Splitting it further separates the two
/// candidates: `Targeting` (`select_actor_targets` documents itself O(n²)) from
/// `Decide` (`tick_actor_brains` builds a full `WorldView` for every actor,
/// including the ones authored `stand_still` that read none of it).
#[cfg(not(target_arch = "wasm32"))]
fn install_actor_decision_census_boundary(app: &mut App) {
    use ambition_dev_tools::runtime_census as census;
    use ambition_platformer2d_shared_tangle::schedule::{PlayerInputSet, WorldPrepSet};

    let sim = app.sim_schedule();

    // ⛔⛔ EACH MARK NEEDS BOTH EDGES. `.after(Targeting)` alone has no upper
    // bound — the sets are chained to each other, not to this system, so a mark
    // with only a lower bound may legally run after `Decide` and bill five
    // phases into one bucket. That is the same class of error as the boundary
    // this file already got wrong once, one level down.
    app.add_systems(
        sim,
        (
            census::mark_sim_phase(census::SIM_PHASE_DECISION_TARGETING)
                .after(ActorDecisionSet::Targeting)
                .before(ActorDecisionSet::Prepare),
            census::mark_sim_phase(census::SIM_PHASE_DECISION_PREPARE)
                .after(ActorDecisionSet::Prepare)
                .before(ActorDecisionSet::Observe),
            census::mark_sim_phase(census::SIM_PHASE_DECISION_OBSERVE)
                .after(ActorDecisionSet::Observe)
                .before(ActorDecisionSet::StateMaintenance),
            census::mark_sim_phase(census::SIM_PHASE_DECISION_STATE_MAINTENANCE)
                .after(ActorDecisionSet::StateMaintenance)
                .before(ActorDecisionSet::Decide),
            census::mark_sim_phase(census::SIM_PHASE_DECISION_DECIDE)
                .after(ActorDecisionSet::Decide)
                .before(ActorDecisionSet::Publish),
            census::mark_sim_phase(census::SIM_PHASE_DECISION_PUBLISH)
                .after(ActorDecisionSet::Publish)
                .before(PlayerInputSet::ControlGate),
            // The tail: `ControlGate` and `BodyMode`, which are chained after
            // `Publish` and before `BeforeIntegrate`.
            census::mark_sim_phase(census::SIM_PHASE_ACTOR_DECISION)
                .after(PlayerInputSet::BodyMode)
                .before(WorldPrepSet::BeforeIntegrate),
        ),
    );
}

#[cfg(target_arch = "wasm32")]
fn install_actor_decision_census_boundary(_app: &mut App) {}

#[cfg(test)]
mod actor_decision_phase_tests {
    use super::*;
    use bevy::ecs::schedule::{NodeId, ScheduleGraph, Schedules, SystemKey};

    const DECISION_PHASES: [ActorDecisionSet; 6] = [
        ActorDecisionSet::Targeting,
        ActorDecisionSet::Prepare,
        ActorDecisionSet::Observe,
        ActorDecisionSet::StateMaintenance,
        ActorDecisionSet::Decide,
        ActorDecisionSet::Publish,
    ];

    const DECISION_MEMBERSHIP: [(&str, ActorDecisionSet); 11] = [
        ("dissolve_settled_grudges", ActorDecisionSet::Targeting),
        ("select_actor_targets", ActorDecisionSet::Targeting),
        ("ensure_perception", ActorDecisionSet::Prepare),
        ("assess_dormancy", ActorDecisionSet::Prepare),
        ("project_authored_fighter_ladder", ActorDecisionSet::Prepare),
        ("collect_perception_peers", ActorDecisionSet::Observe),
        ("collect_perception_projectiles", ActorDecisionSet::Observe),
        ("observe_actor_decision_inputs", ActorDecisionSet::Observe),
        (
            "maintain_actor_pre_decision_state",
            ActorDecisionSet::StateMaintenance,
        ),
        ("tick_actor_brains", ActorDecisionSet::Decide),
        ("publish_actor_decision_frames", ActorDecisionSet::Publish),
    ];

    const MOVEMENT_MEMBERSHIP: [(
        &str,
        ambition_platformer2d_shared_tangle::schedule::WorldPrepSet,
    ); 8] = [
        (
            "tick_capture_holds",
            ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::BeforeIntegrate,
        ),
        (
            "steer_mount_from_rider",
            ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::BeforeIntegrate,
        ),
        (
            "advance_moving_platforms",
            ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::BeforeIntegrate,
        ),
        (
            "snapshot_body_contact",
            ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::BeforeIntegrate,
        ),
        (
            "integrate_sim_bodies",
            ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::Integrate,
        ),
        (
            "sync_actor_read_model",
            ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::AfterIntegrate,
        ),
        (
            "maintain_existing_capture_pose",
            ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::AfterIntegrate,
        ),
        (
            "apply_actor_contact_damage",
            ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::ContactDamage,
        ),
    ];

    fn composed_app() -> App {
        let mut app = App::new();
        crate::schedule::configure_platformer2d_simulation_phases(&mut app);
        app.add_plugins(WorldPrepSchedulePlugin);
        app
    }

    fn system_key(graph: &ScheduleGraph, leaf: &str) -> SystemKey {
        let mut found = None;
        for (key, system, _) in graph.systems.iter() {
            let name = format!("{}", system.name());
            if name.rsplit("::").next() == Some(leaf) {
                assert!(
                    found.is_none(),
                    "{leaf} resolved to more than one sim system; phase membership is ambiguous"
                );
                found = Some(key);
            }
        }
        found.unwrap_or_else(|| panic!("{leaf} must be scheduled by WorldPrepSchedulePlugin"))
    }

    fn assert_membership<S>(graph: &ScheduleGraph, leaf: &str, set: S)
    where
        S: SystemSet + Copy + std::fmt::Debug,
    {
        let system_key = system_key(graph, leaf);
        let set_key = graph
            .system_sets
            .get_key(set.intern())
            .unwrap_or_else(|| panic!("{set:?} must be a registered SystemSet"));
        assert!(
            graph
                .hierarchy()
                .graph()
                .contains_edge(NodeId::Set(set_key), NodeId::System(system_key)),
            "{leaf} must be a direct member of {set:?}"
        );
    }

    #[test]
    fn actor_decision_authority_phases_are_explicitly_ordered() {
        let mut app = composed_app();
        let sim = app.sim_schedule();
        let schedules = app.world().resource::<Schedules>();
        let schedule = schedules.get(sim).expect("sim schedule must exist");
        let graph = schedule.graph();

        let world_prep = graph
            .system_sets
            .get_key(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::WorldPrep.intern())
            .expect("WorldPrep must be registered");
        for phase in DECISION_PHASES {
            let phase_key = graph
                .system_sets
                .get_key(phase.intern())
                .unwrap_or_else(|| panic!("{phase:?} must be registered"));
            assert!(
                graph
                    .hierarchy()
                    .graph()
                    .contains_edge(NodeId::Set(world_prep), NodeId::Set(phase_key)),
                "{phase:?} must remain inside WorldPrep"
            );
        }

        for pair in DECISION_PHASES.windows(2) {
            let before = graph
                .system_sets
                .get_key(pair[0].intern())
                .unwrap_or_else(|| panic!("{:?} must be registered", pair[0]));
            let after = graph
                .system_sets
                .get_key(pair[1].intern())
                .unwrap_or_else(|| panic!("{:?} must be registered", pair[1]));
            assert!(
                graph
                    .dependency()
                    .graph()
                    .contains_edge(NodeId::Set(before), NodeId::Set(after)),
                "actor decision edge {:?} -> {:?} must be explicit",
                pair[0],
                pair[1]
            );
        }

        let publish = graph
            .system_sets
            .get_key(ActorDecisionSet::Publish.intern())
            .expect("Publish must be registered");
        let before_integrate = graph
            .system_sets
            .get_key(
                ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::BeforeIntegrate
                    .intern(),
            )
            .expect("BeforeIntegrate must be registered");
        assert!(
            graph
                .dependency()
                .graph()
                .contains_edge(NodeId::Set(publish), NodeId::Set(before_integrate)),
            "autonomous control publication must finish before movement preconditions begin"
        );
    }

    #[test]
    fn actor_decision_systems_are_members_of_their_authority_phases() {
        let mut app = composed_app();
        let sim = app.sim_schedule();
        let schedules = app.world().resource::<Schedules>();
        let graph = schedules.get(sim).expect("sim schedule must exist").graph();

        for (leaf, phase) in DECISION_MEMBERSHIP {
            assert_membership(graph, leaf, phase);
        }

        #[cfg(feature = "causal")]
        assert_membership(
            graph,
            "record_body_control_frame",
            ActorDecisionSet::Publish,
        );
    }

    #[test]
    fn actor_movement_systems_are_members_of_named_world_prep_phases() {
        let mut app = composed_app();
        let sim = app.sim_schedule();
        let schedules = app.world().resource::<Schedules>();
        let graph = schedules.get(sim).expect("sim schedule must exist").graph();

        for (leaf, phase) in MOVEMENT_MEMBERSHIP {
            assert_membership(graph, leaf, phase);
        }

        #[cfg(feature = "causal")]
        assert_membership(
            graph,
            "record_movement_operations",
            ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::AfterIntegrate,
        );
    }

    /// CONTACT DAMAGE RUNS AFTER THE POSES IT READS, AND BEFORE THE BARKS THAT
    /// REACT TO IT.
    ///
    /// ⛔ THE SUBJECT IS THE SET, NOT ONE SYSTEM, and deliberately: contact
    /// damage depends on POSES BEING SETTLED, which is a property of the whole
    /// phase — a system-to-system edge names one contributor to it and goes
    /// stale the moment another joins. So it is asserted in the two halves that
    /// compose it: membership, and the ordering between the sets.
    #[test]
    fn contact_damage_runs_after_settled_poses_and_before_the_barks() {
        let mut app = composed_app();
        let sim = app.sim_schedule();
        let schedules = app.world().resource::<Schedules>();
        let graph = schedules.get(sim).expect("sim schedule must exist").graph();

        // Half one: the captive constraint is inside the phase that settles poses.
        assert_membership(
            graph,
            "maintain_existing_capture_pose",
            ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::AfterIntegrate,
        );
        assert_membership(
            graph,
            "apply_actor_contact_damage",
            ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::ContactDamage,
        );

        // Half two: that phase is ordered ahead of contact damage's.
        let set_key = |set: ambition_platformer2d_shared_tangle::schedule::WorldPrepSet| {
            graph
                .system_sets
                .get_key(set.intern())
                .unwrap_or_else(|| panic!("{set:?} must be registered"))
        };
        let settled =
            set_key(ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::AfterIntegrate);
        let contact_set =
            set_key(ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::ContactDamage);
        assert!(
            graph
                .dependency()
                .graph()
                .contains_edge(NodeId::Set(settled), NodeId::Set(contact_set)),
            "contact damage must be ordered after the phase that settles poses"
        );
        //  the zero floor: the reverse edge must NOT exist, or the pair
        // would be ordered both ways and this would measure nothing.
        assert!(
            !graph
                .dependency()
                .graph()
                .contains_edge(NodeId::Set(contact_set), NodeId::Set(settled)),
            "the two phases are ordered in BOTH directions"
        );

        // And the tail edge, which is an ordinary singleton-to-singleton order.
        let contact = system_key(graph, "apply_actor_contact_damage");
        let idle_barks = system_key(graph, "tick_npc_idle_barks");
        //  the edge lands on `tick_npc_idle_barks`'s OWN `SystemTypeSet`,
        // not on its system node: that is what `.before(some_fn)` compiles to,
        // and it is why an assertion written as system-to-system cannot pass for
        // an ordering expressed that way. Asked as "is contact damage ordered
        // ahead of a set that idle barks belongs to", which is true for both
        // spellings.
        let ordered_ahead = graph
            .dependency()
            .graph()
            .neighbors_directed(
                NodeId::System(contact),
                bevy::ecs::schedule::graph::Direction::Outgoing,
            )
            .any(|node| match node {
                NodeId::System(system) => system == idle_barks,
                NodeId::Set(set) => graph
                    .hierarchy()
                    .graph()
                    .contains_edge(NodeId::Set(set), NodeId::System(idle_barks)),
            });
        assert!(
            ordered_ahead,
            "contact damage must be ordered ahead of the idle barks that read it"
        );
    }
}

pub struct WorldPrepSchedulePlugin;

impl bevy::prelude::Plugin for WorldPrepSchedulePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        let sim = app.sim_schedule();
        use ambition_platformer2d_world::placements::PlacementLoweringAppExt;

        // Autonomous decision is ordered by semantic phase sets. Systems may
        // join WorldPrep without acquiring unrelated leaf-system edges.
        configure_actor_decision_phases(app);
        // Relational targeting seam (default = today's behavior; stealth/bounty/
        // alliance systems mutate it). `select_actor_targets` reads it. Combat
        // owns these resources (rule 5); WorldPrep just invokes its registrar.
        ambition_combat::targeting::init_targeting_resources(app);
        // AE6: the rules a MATCH plays under, resolved from its declaration
        // folded over those baselines — so a stage never writes them. In
        // WorldPrep because every reader is later (PlayerSimulation/Combat),
        // and a resolution landing after them would hand the hit kernel last
        // tick's rules on the one tick they differ.
        app.init_resource::<ambition_combat::rules::ResolvedCombatTuning>();
        app.add_systems(
            sim,
            crate::features::combat_rules::project_combat_rules
                .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::WorldPrep),
        );
        // S4: spend a stock per KO. `CombatSet:Settle` is the phase for "everything that reads
        // this tick's damage outcome rather than producing it", which is exactly what this is —
        // the KO was decided in Resolve, and spending is bookkeeping over it.
        app.init_resource::<crate::features::stocks_match::StocksMatchSettled>();
        app.init_resource::<crate::features::stocks_match::SuddenDeathEntered>();
        app.add_message::<crate::features::stocks_match::SuddenDeathBegan>();
        // the same rule as `BodyHitResolved` below, applied to the clock.
        // `state_the_matchs_pace` writes a `ClockScaleRequest` and this plugin
        // is what schedules it, so this plugin registers it — a fixture that
        // composes the ruleset without the runtime's time-control assembly
        // would otherwise die on the first tick of a match. `add_message` is
        // idempotent, so the composition that already registers it is
        // unaffected (`transform_beat` does exactly this for the same reason).
        app.add_message::<ambition_time::time_control::ClockScaleRequest>();
        #[cfg(feature = "causal")]
        app.add_message::<ambition_damage::BodyHitResolved>();
        #[cfg(feature = "causal")]
        app.add_message::<crate::causal::BodyMovementOps>();
        // AN INSTRUMENT REGISTERS WHAT IT READS.
        //
        // found by running `ladder_probe --features causal`, which panicked on the first tick
        // with "Message not initialized". The inspector could not be enabled for the game it
        // was built to inspect.
        //
        // this is the READER half of the rule already learned on the writer
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
            app.add_message::<ambition_damage::BodyReactionApplied>();
        }
        // `decide_stocks_match` READS this, so a composition that installs the
        // stocks loop must have the channel or the system fails parameter
        // validation before it can run. Idempotent, so the host registering it
        // beside `StocksMatchDecided` costs nothing.
        app.add_message::<ambition_combat::stocks::FighterRespawnDue>();
        app.add_systems(
            sim,
            (
                ambition_combat::stocks::spend_fighter_stocks
                    .in_set(ambition_combat::stocks::FighterStocksSpent),
                // D192: the return is decided AFTER the spend and BEFORE any
                // ruleset places a body. An interval of zero still resolves on
                // this tick, so a mode that authored no beat is unchanged.
                //
                // D201: this no longer TICKS anything — the window it reads is
                // advanced by `tick_death_interlude`, which every death in the
                // process already shares. What is ordered here is the
                // CONSEQUENCE, and it still has to sit between the spend that
                // opens the window and the ruleset that places the body.
                ambition_combat::stocks::respawn_when_the_interlude_closes
                    .in_set(ambition_combat::stocks::FighterRespawnsDue)
                    .after(ambition_combat::stocks::FighterStocksSpent),
                crate::features::stocks_match::decide_stocks_match
                    .in_set(ambition_combat::stocks::MatchOutcomeDecided),
                // Reading the latch on the frame BEFORE the one that sets it would let the
                // winner play on for one tick, which is the window a final KO happens in.
                crate::features::stocks_match::state_the_matchs_pace,
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
                .in_set(ambition_platformer2d_shared_tangle::schedule::CombatSet::Settle),
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
        // ⭐ THE WORLD-SOURCE HOT-RELOAD WATCHER MOVED OUT, resource and system
        // together, to `ambition_dev_tools::DevToolsSimPlugin` — the crate that
        // owns it. A simulation package registering a developer facility is the
        // dependency-direction defect this campaign is about; the reasons the
        // watcher runs in `Update` rather than the sim moved with it.
        app.add_systems(
            sim,
            (
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
                // Actor targeting/decision, movement, read-model projection, and contact
                // damage are registered below on their owning phase sets.
                // Ambient NPC chatter (parrot squawks, etc.) on its own timer.
                tick_npc_idle_barks,
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
                .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::WorldPrep),
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
                .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::WorldPrep)
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
                .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::WorldPrep),
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
        // Same function, same rule, later phase. The `WorldPrep` copy above feeds the pogo
        // derivation and the feature-world collision overlay, which are rebuilt in that set;
        // this copy exists because DAMAGE resolves in `Combat`, and a body's `CenteredAabb` is
        // written by its own integrator — an actor's in `WorldPrep`, the PLAYER's in
        // `PlayerSimulation`.
        //
        // `in_set(CoreSimulation)` keeps it inside `GameplaySimulationRoot`, so the
        // session gate covers it like everything else.
        register_damage_facing_volume_publication(app);
        // Cross-phase order lives on named sets. Local chains remain only where
        // same-phase deferred commands or write-after-read order require them.
        app.init_resource::<ActorDecisionFacts>();
        app.init_resource::<ActorDecisionFrames>();
        app.init_resource::<ActorSteering>();
        // Every solid body's contact box, resampled before every movement
        // phase. Empty in every composition that grants no body the
        // capability, and an empty snapshot answers `BodyContactField::NONE`
        // for every body — so this resource existing changes nothing on its own.
        app.init_resource::<ambition_platformer2d_shared_tangle::body::BodyContactSnapshot>();
        app.init_resource::<crate::features::ecs::perception::PerceptionPeers>();
        app.init_resource::<crate::features::ecs::perception::PerceptionProjectiles>();

        app.add_systems(
            sim,
            (
                // Prepare same-tick eligibility before maintenance. The chain flushes
                // `assess_dormancy` commands before later phases filter on `Dormant`.
                crate::features::ecs::perception::ensure_perception,
                crate::features::ecs::dormancy::assess_dormancy,
                crate::features::ecs::project_authored_fighter_ladder,
            )
                .chain()
                .in_set(ActorDecisionSet::Prepare),
        );
        app.add_systems(
            sim,
            (
                crate::features::ecs::perception::collect_perception_peers,
                crate::features::ecs::perception::collect_perception_projectiles,
                observe_actor_decision_inputs,
            )
                .in_set(ActorDecisionSet::Observe),
        );
        app.add_systems(
            sim,
            maintain_actor_pre_decision_state.in_set(ActorDecisionSet::StateMaintenance),
        );
        app.add_systems(sim, tick_actor_brains.in_set(ActorDecisionSet::Decide));
        app.add_systems(
            sim,
            (
                publish_actor_decision_frames,
                // IMMEDIATELY after publication: this instrument must read this
                // tick's `ActorControl`, not the previous tick's frame.
                #[cfg(feature = "causal")]
                crate::causal::record_body_control_frame,
            )
                .chain()
                .in_set(ActorDecisionSet::Publish),
        );

        // Finished intent may now be gated/routed, then the common contact
        // snapshot is taken. These operations have real same-tick dependencies,
        // so their short chain is the contract inside `BeforeIntegrate`.
        app.add_systems(
            sim,
            (
                ambition_combat::capture::systems::tick_capture_holds,
                ambition_mount::steer_mount_from_rider,
                crate::avatar::advance_moving_platforms,
                snapshot_body_contact,
            )
                .chain()
                .in_set(
                    ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::BeforeIntegrate,
                ),
        );
        app.add_systems(
            sim,
            integrate_sim_bodies
                .in_set(ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::Integrate),
        );
        // ⛔⛔ THE SADDLE IS A POST-INTEGRATION CONSTRAINT, AND THE ORDER MUST BE
        // STATED. A chained tuple that does not CONTAIN the integrator states
        // nothing about it, so the two pose authorities for a ridden body are
        // ordered by scheduler topology — whether the rider is snapped to its
        // mount before or after the movement pass moved it becomes an accident.
        //
        // ⭐ `AfterIntegrate` IS THE PHASE FOR EXACTLY THIS, and capture's
        // equivalent external constraint already lives there. A constraint
        // that owns a body's final pose has to run after the thing that
        // proposes it, and now the schedule is what says so.
        //
        // ⚠ THIS DOES NOT YET STOP THE RIDER INTEGRATING ITS OWN
        // LOCOMOTION — it only fixes which authority speaks last. Making a
        // held body decline the movement pass is the other half, and
        // `PoseOwnedExternally` is the fact it will read.
        app.add_systems(
            sim,
            sync_actor_read_model.in_set(
                ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::AfterIntegrate,
            ),
        );
        // ⭐ AND IT FOLLOWS THE READ MODEL, exactly as the capture constraint
        // below does: the coarse-box mirror runs first so an external pose
        // authority gets the last word.
        app.add_systems(
            sim,
            ambition_mount::sync_riders_to_mounts
                .after(sync_actor_read_model)
                .in_set(
                    ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::AfterIntegrate,
                ),
        );
        // A body already in somebody's hands is put back after it moved. The
        // coarse-box mirror runs first so this external constraint is the last
        // word. (A body grabbed THIS tick is posed by the ruleset's own
        // `finalize_new_capture_pose`, later in the tick — two named phases, one
        // rule.)
        app.add_systems(
            sim,
            ambition_combat::capture::systems::maintain_existing_capture_pose
                .after(sync_actor_read_model)
                .in_set(
                    ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::AfterIntegrate,
                ),
        );
        #[cfg(feature = "causal")]
        app.add_systems(
            sim,
            // Movement operations are published during integration. This observer
            // has no ordering dependency on post-integration projection.
            crate::causal::record_movement_operations.in_set(
                ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::AfterIntegrate,
            ),
        );
        app.add_systems(
            sim,
            apply_actor_contact_damage
                .in_set(ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::ContactDamage)
                .before(tick_npc_idle_barks),
        );
        // Same set, same schedule, same guarantee that every game gets it — but the registration
        // lives with the system, so this crate does not depend on the crate that owns it.
        //
        // The body-orientation righting reflex: feet toward gravity — or, for a riding momentum
        // body, feet onto the ridden surface via the `SurfaceUpright` fact the integration just
        // published.
        app.add_systems(
            sim,
            (
                ambition_platformer2d_shared_tangle::orientation::ensure_actor_roll,
                ambition_platformer2d_shared_tangle::orientation::update_actor_roll,
            )
                .chain()
                .in_set(
                    ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::AfterIntegrate,
                ),
        );
        // TARGETING owns feud settlement and target/disposition selection. The
        // short chain is intentional: selection must see the grudge state produced
        // by settlement in the same tick. Later actor phases depend on the set, not
        // on either leaf system.
        app.add_systems(
            sim,
            (dissolve_settled_grudges, select_actor_targets)
                .chain()
                .in_set(ActorDecisionSet::Targeting),
        );
        // Q18 (G3): translate a rider-boss's live strike into per-limb intents on
        // its linked mount, then fan those out onto each limb body. `route_...`
        // bridges the `RidingOn`/`MountSlot` link (attack state on the RIDER, limbs
        // on the MOUNT) and writes `LimbIntents`; `fan_out_limb_intents` copies each
        // slot's frame onto its limb's `ActorControl`. Runs in the movement phase —
        // after the mount steer, before the bodies integrate — so each limb
        // EXECUTES its routed arc the same frame it's written.
        //
        // Frame contract: the router reads the rider's `BossAttackState`, a sim-owned
        // READ-MODEL projected from the live `MovePlayback` in the combat phase
        // (`project_boss_attack_state_from_move`), so it sees the PREVIOUS frame's projection —
        // the standard one-frame read-model lag every other consumer of that projection
        // accepts. Registered separately — the WorldPrep chain tuple is already at Bevy's
        // chain-length ceiling.
        app.add_systems(
            sim,
            (route_boss_strikes_to_limbs, fan_out_limb_intents)
                .chain()
                .after(ambition_mount::steer_mount_from_rider)
                .in_set(
                    ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::BeforeIntegrate,
                ),
        );
        app.configure_sets(
            sim,
            ambition_platformer2d_shared_tangle::schedule::BossSteerSlot
                .after(tick_boss_brains_system)
                .before(update_ecs_bosses)
                .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::WorldPrep),
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
                // it lived in the app's HUD chain in `Update` until
                // which made a rollback-registered component
                // (`body.mana`) move at render rate and never resimulate on a
                // rewind -- and left every non-app composition with mana that
                // does not refill.
                crate::avatar::regen_player_mana,
            )
                .chain()
                .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::FeatureCollection),
        );
    }
}

/// Schedules `FeatureInteraction`: switches, chests, breakables, save sync,
/// and encounter switch-index rebuild — and declares the cross-domain order
/// the phase runs in.
///
/// That is how `conversation` — 1,836 lines with zero `crate::` imports in either direction —
/// stayed pinned inside the monolith while every import measure said it was free.
///
/// the order is now
/// [`FeatureInteractionSet`](ambition_platformer2d_shared_tangle::schedule::FeatureInteractionSet), and each
/// prose rationale lives on the variant it explains rather than beside the
/// system it happened to precede. This plugin declares the total order ONCE and
/// each domain only says which phase it is in — `conversation` says so from
/// [`ambition_conversation::ConversationPlugin`], naming nothing in `features`.
pub struct FeatureInteractionSchedulePlugin;

impl bevy::prelude::Plugin for FeatureInteractionSchedulePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        let sim = app.sim_schedule();
        use ambition_platformer2d_shared_tangle::schedule::FeatureInteractionSet;

        // the conversation domain installs itself: the authority, the
        // cut-bark port channel, its own narrative payload, its presentation
        // pair, and its three sim systems placed by PHASE. Nothing about
        // conversation is registered here any more.
        app.add_plugins(ambition_conversation::ConversationPlugin);
        // the ledger payloads that are NOT conversation's. The ledger is
        // the record of what the narrative — which runs outside the simulation —
        // told the simulation, stamped with the tick it applies from. A rewind
        // restores what the simulation DECIDED; erasing what it was TOLD is how
        // the replay reaches a different answer.
        //
        // one per payload, and the list IS the classification — the
        // counterpart to the table in `crate::dialog::yarn_bindings`. A
        // gameplay-bearing Yarn command has a ledger here or it has no replay
        // story; a presentation-facing one must NOT be here, because deferring
        // a sound to a simulation tick would delay it for no reason. Content
        // registers its own vocabulary the same way, so this names no content.
        //
        // these stay here on purpose, and it is a seam rather than a
        // leftover. A payload belongs to whoever CONSUMES it: three of these
        // are `features` types that a carved-out conversation crate could not
        // name at all, and the other three are applied by `features::bus` and
        // `crate::items::narrative`. Conversation provides the ledger MECHANISM
        // and registers only `ConversationEnded`, the payload it both defines
        // and consumes.
        app.add_plugins((
            // Authored flag writes now ride `RunAuthoredCommand`'s ledger, installed once by
            // the conversation plugin. the CHANNEL is untouched — chests, pickups and
            // interactions still write it from inside the simulation, and `sim_core_resources`
            // registers it.
            ambition_conversation::NarrativeInputPlugin::<crate::features::ChallengeRequested>::default(),
            ambition_conversation::NarrativeInputPlugin::<crate::features::BrainCommand>::default(),
            ambition_conversation::NarrativeInputPlugin::<crate::features::ReleaseProvocation>::default(),
            ambition_conversation::NarrativeInputPlugin::<ambition_items::ItemGrantRequested>::default(),
            ambition_conversation::NarrativeInputPlugin::<ambition_items::shop::ShopTransactionRequested>::default(),
        ));

        // THE ORDER, SAID OUT LOUD. Every reason each boundary exists is
        // on the `FeatureInteractionSet` variant; the shape of the statement is
        // the part that belongs here.
        //
        // `.chain()` over the whole list, deliberately — `(A, B).before(C)`
        // would order both A and B before C and say nothing about A vs B, which
        // is exactly the gap that let ten systems look ordered while three of the
        // eleven pairwise contracts were unstated. A chain is a total order.
        //
        // the `ApplyDeferred` boundaries survive. The old per-system chain inserted a sync
        // point between every pair; Bevy inserts sync points on dependency edges after sets are
        // flattened to their members, so the set-level chain reproduces them at every phase
        // boundary.
        app.configure_sets(
            sim,
            (
                FeatureInteractionSet::NarrativeIntake,
                FeatureInteractionSet::Actuate,
                FeatureInteractionSet::Continuity,
                FeatureInteractionSet::CutBarkCast,
                FeatureInteractionSet::HoldProjection,
                FeatureInteractionSet::WorldObjects,
                FeatureInteractionSet::SwitchIndex,
            )
                .chain()
                .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhaseMonolith::FeatureInteraction),
        );

        app.add_systems(
            sim,
            interact_ecs_actors_and_switches.in_set(FeatureInteractionSet::Actuate),
        );
        // The cast half of the conversation break. It sits in a set the
        // conversation ordering vocabulary declares and this domain fills —
        // the temporal twin of the `ConversationCutBark` message port.
        app.add_systems(
            sim,
            npcs::speak_conversation_cut_barks.in_set(FeatureInteractionSet::CutBarkCast),
        );
        app.add_systems(
            sim,
            (
                open_ecs_chests,
                update_ecs_breakables,
                update_ecs_falling_chests,
                sync_ecs_switches_from_save,
            )
                .chain()
                .in_set(FeatureInteractionSet::WorldObjects),
        );
        app.add_systems(
            sim,
            crate::encounter::rebuild_encounter_switch_index
                .in_set(FeatureInteractionSet::SwitchIndex),
        );
    }
}

#[cfg(test)]
mod actor_movement_tests;

#[cfg(test)]
mod feature_interaction_order_tests;

#[cfg(test)]
mod sim_clock_tests {
    use super::{advance_gameplay_elapsed, GameplayElapsed};
    use bevy::prelude::*;

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
