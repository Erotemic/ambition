//! Accepted body control, decisions, movement, and action execution.

use ambition_characters::actor::limb::fan_out_limb_intents;
use ambition_persistence::save_data::npc_talked_flag;
use ambition_platformer2d_core as ae;
use ambition_platformer2d_core::AabbExt;
use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;
use bevy::prelude::*;

/// Mid-air jumps an enemy gets between landings. `1` = single
/// double-jump (matches the player's default). Resets when the
/// body transitions `on_ground: false → true` in `enemy.update()`.
pub(crate) const MAX_ENEMY_AIR_JUMPS: u8 = 1;

pub mod brain_command;
mod movement;
mod integration;
pub use integration::{BodyFrameOutput, BodyReset};
pub(crate) mod npcs;
/// The NPC talk flag and the provocation recorder; the provocation FLAG itself is
/// `ambition_persistence::save_data::npc_hostile_flag`, which construction also reads.
pub use npcs::npc_talk_dialogue_id;

pub mod empowerment;
pub mod ledge_grab;
pub mod movement_fx;
pub mod swim;
pub mod transform_beat;
pub mod wallet_shield;
pub use movement_fx::{
    arm_ground_contact_anim_overlay, arm_movement_anim_overlays, emit_movement_fx,
    handle_player_events,
};

pub use body_identity::{BodyIdentityCensus, install_body_identity_census};
pub use dismounted_rider::install_dismounted_rider_rebuild;
pub use npcs::speak_conversation_cut_barks;

use ambition_boss_encounter::ecs::tick_boss_brains_system;
use ambition_combat::targeting::dissolve_settled_grudges;
use ambition_combat::targeting::select_actor_targets;
pub use brain_command::{
    BrainCommand, BrainCommandKind, BrainCommandPlugin, ReleaseProvocation, apply_brain_commands,
    apply_release_provocations,
};
pub use anim_helpers::advance_body_anim_overlay_clocks;

use ambition_combat::util::*;
use ambition_characters::actor::BodyCombat;
use ambition_time::WorldTime;
use ambition_platformer2d_core::body_clusters::ActorSurfaceState;
pub(crate) use movement::ActorMutIntegrationExt;
#[cfg(test)]
pub(crate) use movement::SeedActorIntegrationTestExt;

pub(crate) mod actors;
mod aggression;
mod anim_helpers;
mod body_identity;
#[cfg(test)]
mod boss_scripted_pattern_tests;
/// The ladder projection, registered in the actor pipeline beside the brain tick.
/// The dismount reaction: mount announces, this rebuilds. See its own note.
pub(crate) mod autonomous_reconcile;
mod brain_effects;
pub(crate) mod character_policy;
mod damage;
pub mod death_beat;
mod death_effects;
mod dismounted_rider;
pub mod dormancy;
mod drawn_ledges;
pub mod errand;
mod fade_rows;
#[cfg(test)]
mod fighter_harness;
pub mod fighter_ladder;
mod interact;
pub mod navigation;
pub mod perception;
mod pet;
mod rest_row;
mod reactions;
pub use reactions::advance_body_reactions;
mod summon;
mod talk_spacing;
mod target_volumes;
mod wear;

pub(crate) use actors::{
    ActorDecisionFacts, ActorDecisionFrames,
    observe_actor_decision_inputs, publish_actor_decision_frames,
};

pub use actors::{
    ActorSteering, apply_actor_contact_damage, integrate_sim_bodies, route_boss_strikes_to_limbs,
    snapshot_body_contact, tick_actor_brains, tick_npc_idle_barks,
};
pub use aggression::{
    CHALLENGE_GRACE_S, ChallengeRequested, PendingChallenge, apply_actor_stimuli,
    arm_requested_challenges, tick_pending_challenges,
};
use ambition_platformer2d_core::movement::MotionModel;
pub use brain_effects::spawn_projectiles_from_brain_actions;
pub use damage::apply_feature_hit_events;
pub use drawn_ledges::follow_drawn_ledges;
pub use fade_rows::{FADE_ROW_WRITER, pin_fade_rows, withdraw_faded_contact};
pub use interact::{TalkableBodies, interact_ecs_actors_and_switches};
pub use pet::{
    PET_SECONDS, PET_WALK_SPEED, PetBeat, PetRequested, PetStage, advance_pet_beats,
    apply_pet_requests, project_pet_holds,
};
pub use rest_row::{REST_ROW_WRITER, pin_rest_rows};
pub use talk_spacing::{TALK_GAP, TalkSpacing, space_the_talkers};
pub use wear::{WearRequested, apply_wear_requests};
#[cfg(test)]
mod mount_pair_tests;

pub use summon::apply_summon_effects;
pub use target_volumes::{
    derive_pogo_target_volumes, refresh_body_damageable_volumes, refresh_boss_damageable_volumes,
};

/// Advance each body's diagnostic [`ae::BodyLifeStats`].
///
/// Reads the restart latch just before `announce_body_restarts` consumes it,
/// so every announced restart is counted once and restarts the clock.
pub fn track_body_life_stats(
    world_time: bevy::prelude::Res<ambition_time::WorldTime>,
    mut bodies: Query<(&ae::BodyRestartLatch, &mut ae::BodyLifeStats)>,
) {
    for (latch, mut stats) in &mut bodies {
        if latch.pending {
            stats.resets += 1;
            stats.time_alive = 0.0;
        } else {
            stats.time_alive += world_time.sim_dt();
        }
    }
}

/// The decision chain's boundaries now live in
/// [`ambition_platformer2d_shared_tangle::schedule::ActorDecisionSet`], with
/// the reason they had to leave. This crate still owns WHERE they sit — see
/// `configure_actor_decision_phases` directly below — and every use in this
/// file reads the same name through this import.
use ambition_platformer2d_shared_tangle::schedule::ActorDecisionSet;

fn configure_actor_decision_phases(app: &mut App) {
    let sim = app.sim_schedule();
    app.configure_sets(
        sim,
        (
            ActorDecisionSet::Targeting,
            ActorDecisionSet::Prepare,
            ActorDecisionSet::Observe,
            ActorDecisionSet::Decide,
            ActorDecisionSet::Publish,
        )
            .chain()
            .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhase::WorldPrep),
    );
    app.configure_sets(
        sim,
        ActorDecisionSet::Publish
            .before(ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::BeforeIntegrate),
    );
    // The collision overlay is complete before the first phase that reads it.
    // See `FeatureWorldOverlayContributions`.
    app.configure_sets(
        sim,
        ambition_platformer2d_shared_tangle::schedule::FeatureWorldOverlayContributions
            .after(ambition_platformer2d_shared_tangle::schedule::FeatureWorldOverlaySet)
            .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhase::WorldPrep),
    );
    app.configure_sets(
        sim,
        (
            ambition_platformer2d_shared_tangle::schedule::FeatureWorldOverlaySet,
            ambition_platformer2d_shared_tangle::schedule::FeatureWorldOverlayContributions,
        )
            .before(ActorDecisionSet::Targeting),
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
            .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhase::WorldPrep),
    );
}

/// ⭐ THE DECISION-PHASE CENSUS MARKS ARE NOT INSTALLED HERE ANY MORE, and the
/// reason this comment survives the systems is that the WHY was load-bearing
/// and easy to lose. Without a mark closing this file's span, all of it billed
/// to `WorldPrep.BeforeIntegrate` — a windowed Hall capture on 2026-08-31 read
/// 1.214 ms/tick there and it was taken for movement preparation, when 0.958 of
/// it was the decision chain against `BeforeIntegrate`'s 0.037.
///
/// ⛔ THE OLD REASON THE MARKS LIVED IN THIS CRATE WAS *"only this crate can
/// name the sets"*, and that stopped being true when `ActorDecisionSet` moved to
/// `shared_tangle`. `ambition_dev_tools::runtime_census::
/// install_actor_decision_census_boundary` owns them now — the crate whose
/// instrument they are — and it carries every ordering rule they depend on.

pub struct BodySimulationPlugin;

impl bevy::prelude::Plugin for BodySimulationPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        let sim = app.sim_schedule();

        // Autonomous decision is ordered by semantic phase sets. Systems may
        // join WorldPrep without acquiring unrelated leaf-system edges.
        configure_actor_decision_phases(app);
        // Relational targeting seam (default = today's behavior; stealth/bounty/
        // alliance systems mutate it). `select_actor_targets` reads it. Combat
        // owns these resources (rule 5); WorldPrep just invokes its registrar.
        ambition_combat::targeting::init_targeting_resources(app);

        #[cfg(feature = "causal")]
        app.add_message::<ambition_damage::BodyHitResolved>();
        #[cfg(feature = "causal")]
        app.add_message::<crate::causal::BodyMovementOps>();

        // Every body's death beat, LATE in the tick: a kill in any phase
        // before this starts its beat on the same tick, so the view rebuilt
        // after it never hides the body for a frame before its death row.
        app.add_systems(
            sim,
            death_beat::advance_death_beats
                .run_if(any_with_component::<ambition_combat::components::DeathBeat>)
                .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhase::Progression),
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
            (track_body_life_stats, ae::announce_body_restarts)
                .chain()
                .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhase::WorldPrep)
                .before(ambition_platformer2d_shared_tangle::schedule::FeatureWorldOverlaySet),
        );
        // Advance the accumulating sim clock before any actor brain reads its
        // perception snapshot, so reaction-latency lookback is live. Registered
        // separately (not in the chain above) only because that tuple is already
        // at Bevy's chain-length ceiling; the `.before` keeps the ordering exact.
        app.configure_sets(
            sim,
            ambition_platformer2d_shared_tangle::lifecycle::GameplayElapsedAdvanced
                .before(select_actor_targets),
        );

        // Cross-phase order lives on named sets. Local chains remain only where
        // same-phase deferred commands or write-after-read order require them.
        app.init_resource::<ActorDecisionFacts>();
        app.init_resource::<ActorDecisionFrames>();
        app.init_resource::<crate::body::navigation::NavigationAdvice>();
        app.init_resource::<crate::body::navigation::RoomNavigation>();
        app.init_resource::<ActorSteering>();
        // Every solid body's contact box, resampled before every movement
        // phase. Empty in every composition that grants no body the
        // capability, and an empty snapshot answers `BodyContactField::NONE`
        // for every body — so this resource existing changes nothing on its own.
        app.init_resource::<ambition_platformer2d_shared_tangle::body::BodyContactSnapshot>();
        app.init_resource::<crate::body::perception::PerceptionPeers>();
        app.init_resource::<crate::body::perception::PerceptionProjectiles>();

        app.add_systems(
            sim,
            (
                // Prepare same-tick eligibility before maintenance. The chain flushes
                // `assess_dormancy` commands before later phases filter on `Dormant`.
                crate::body::dormancy::assess_dormancy,
                crate::body::fighter_ladder::project_authored_fighter_ladder,
            )
                .chain()
                .in_set(ActorDecisionSet::Prepare),
        );
        app.add_systems(
            sim,
            (
                crate::body::perception::collect_perception_peers,
                // While a projectile is live, or last tick's rows remain to clear.
                crate::body::perception::collect_perception_projectiles.run_if(
                    |live: Query<(), With<ambition_projectiles::LiveProjectile>>,
                     rows: Res<crate::body::perception::PerceptionProjectiles>| {
                        !live.is_empty() || !rows.0.is_empty()
                    },
                ),
                observe_actor_decision_inputs,
                crate::body::navigation::advise_navigation,
            )
                .in_set(ActorDecisionSet::Observe),
        );
        // An errand ends where a press takes an item, and after it: a body that
        // took something by a press this tick has a full hand. Before the use
        // of a held item, as a press pickup is.
        app.add_systems(
            sim,
            crate::body::errand::settle_errands
                .run_if(any_with_component::<crate::body::errand::Errand>)
                .in_set(ambition_platformer2d_shared_tangle::schedule::GameplayGated)
                .after(ambition_platformer2d_shared_tangle::schedule::HeldItemStep::Pickup)
                .before(ambition_platformer2d_shared_tangle::schedule::HeldItemStep::Use),
        );
        // A body crosses to its item's room after the held items and after
        // this tick's custody is derived: it reads `InCustodyOf` (what goes
        // with it), which is derived and not rolled back, so before its
        // derivers a resimulated frame reads the latest forward frame's
        // custody. Its errand waits for the advice of the new room, on the
        // next tick.
        app.add_systems(
            sim,
            crate::body::errand::cross_on_errands
                .run_if(any_with_component::<crate::body::errand::Errand>)
                .in_set(ambition_platformer2d_shared_tangle::schedule::GameplayGated)
                .after(ambition_platformer2d_shared_tangle::schedule::ItemPickupSet::CoreHeldItems)
                .after(ambition_platformer2d_shared_tangle::lifecycle::BodyCustodySettled),
        );
        app.add_systems(sim, tick_actor_brains.in_set(ActorDecisionSet::Decide));
        app.add_systems(
            sim,
            (
                publish_actor_decision_frames,
                // The runtime's control-frame recorder runs IMMEDIATELY after
                // this (`causal::record_body_control_frame`).
            )
                .chain()
                .in_set(ActorDecisionSet::Publish),
        );

        // Finished intent may now be gated/routed, then the common contact
        // snapshot is taken. These operations have real same-tick dependencies,
        // so their short chain is the contract inside `BeforeIntegrate`.
        //
        // ⭐⭐ THE CONTRACT IS A CHAIN OF SETS NOW, NOT A CHAIN OF SYSTEMS, and
        // that is the whole difference the schedule-authority prerequisite asks
        // for. Chaining `tick_capture_holds` directly to `steer_mount_from_rider`
        // was this crate fixing the relative order of two OTHER crates' private
        // systems — the named form of private cross-domain ordering authority.
        // `ambition_combat` and `ambition_mount` publish `CaptureHoldsTicked` and
        // `MountsSteeredByRiders`; the order between them is stated here, where
        // the composition belongs, in vocabulary both crates own.
        //
        // ⛔ THE SAME-TICK DEPENDENCIES ARE UNCHANGED AND SO IS THE ORDER. Each
        // published set covers ONE system today, which is why the substitution is
        // exact rather than approximate — and each set's own doc says the chain
        // is the INSTALLER's contract, so widening one later is an argument
        // somebody has to make against that text.
        app.configure_sets(
            sim,
            (
                ambition_combat::capture::systems::CaptureHoldsTicked,
                ambition_mount::MountsSteeredByRiders,
            )
                .chain()
                .in_set(
                    ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::BeforeIntegrate,
                ),
        );
        // ⛔ AND THE FOREIGN PAIR IS NOT CHAINED HERE ANY MORE — the set chain
        // above is the only statement of their relative order. Leaving the
        // `.chain()` around them as well would say the same thing twice, in two
        // vocabularies, and the system-level one is the half that cannot be
        // reasoned about from either domain's crate.
        // ⛔⛔ AND `steer_mount_from_rider` IS NOT INSTALLED HERE ANY MORE — it
        // arrives with `install_mount_pose_systems` below, already in its own set.
        // Installing it in both places is not a duplicate DECLARATION that a
        // compiler would reject; Bevy adds a SECOND INSTANCE and the mount steers
        // twice a tick. ⇒ Moving an install is not additive: the old site has to go
        // in the same edit, and a census that reports the row as MOVED cannot tell
        // you it was COPIED — both spellings score identically.
        app.add_systems(
            sim,
            ambition_combat::capture::systems::tick_capture_holds
                .run_if(ambition_combat::capture::a_capture_stands)
                .in_set(ambition_combat::capture::systems::CaptureHoldsTicked)
                .in_set(
                    ambition_platformer2d_shared_tangle::capability::CapabilitySet(
                        ambition_platformer2d_shared_tangle::capability::Capability::Captures,
                    ),
                )
                .in_set(
                    ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::BeforeIntegrate,
                ),
        );
        // Contacts read bodies after platform movement and cache retirement.
        app.add_systems(
            sim,
            snapshot_body_contact
                .after(ambition_platformer2d_world::platforms::MovingPlatformStep)
                .after(ambition_mount::MountsSteeredByRiders)
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
        // ⭐⭐ THE MOUNT CRATE POSES ITSELF (prerequisite C2). This block used
        // to install `ambition_mount::sync_riders_to_mounts` by name and order it
        // after a function private to this module. Now the mount crate installs
        // its own pose stage and this layer says only WHICH PHASE each half
        // belongs to — two set names, no foreign function.
        ambition_mount::install_mount_pose_systems(app, sim);
        app.configure_sets(
            sim,
            (
                ambition_mount::MountsSteeredByRiders.in_set(
                    ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::BeforeIntegrate,
                ),
                ambition_mount::RidersSyncedToMounts.in_set(
                    ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::AfterIntegrate,
                ),
            ),
        );
        // A body already in somebody's hands is put back after it moved. The
        // coarse box is published by `integrate_sim_bodies`, so membership in
        // `AfterIntegrate` is what makes this external constraint the last word
        // — for the rider mirror above as well. (A body grabbed THIS tick is
        // posed by the ruleset's own `finalize_new_capture_pose`, later in the
        // tick — two named phases, one rule.)
        app.add_systems(
            sim,
            ambition_combat::capture::systems::maintain_existing_capture_pose
                .run_if(ambition_combat::capture::a_capture_stands)
                .in_set(ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::AfterIntegrate)
                .in_set(
                    ambition_platformer2d_shared_tangle::capability::CapabilitySet(
                        ambition_platformer2d_shared_tangle::capability::Capability::Captures,
                    ),
                ),
        );
        app.add_systems(
            sim,
            apply_actor_contact_damage
                .in_set(ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::ContactDamage)
                .before(tick_npc_idle_barks),
        );
        // A body that fades between its attacks does not hurt by touch while
        // it is faded: derived before the contact pass reads it.
        app.add_systems(
            sim,
            withdraw_faded_contact
                .in_set(ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::ContactDamage)
                .before(apply_actor_contact_damage),
        );

        // Same set, same schedule, same guarantee that every game gets it — but the registration
        // lives with the system, so this crate does not depend on the crate that owns it.
        //
        // The body-orientation righting reflex: feet toward gravity — or, for a riding momentum
        // body, feet onto the ridden surface via the `SurfaceUpright` fact the integration just
        // published.
        app.add_systems(
            sim,
            ambition_platformer2d_shared_tangle::orientation::update_actor_roll.in_set(
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
                // A boss's limbs exist only where named bosses do.
                .in_set(
                    ambition_platformer2d_shared_tangle::capability::CapabilitySet(
                        ambition_platformer2d_shared_tangle::capability::Capability::BossEncounters,
                    ),
                )
                .after(ambition_mount::MountsSteeredByRiders)
                .in_set(
                    ambition_platformer2d_shared_tangle::schedule::WorldPrepSet::BeforeIntegrate,
                ),
        );
        // ⭐ A4: THE BOSS BRAIN PUBLISHES WITH THE OTHER BRAINS, IN FRONT OF THE
        // CONTROL GATE. It wrote its frame after actor integration, so the gate
        // never saw a boss frame: a boss a script held flew its pattern
        // (`a_boss_a_script_holds_does_not_fly_its_pattern`).
        app.add_systems(
            sim,
            tick_boss_brains_system
                .run_if(any_with_component::<ambition_boss_encounter::BossConfig>)
                .in_set(ambition_platformer2d_shared_tangle::capability::CapabilitySet(ambition_platformer2d_shared_tangle::capability::Capability::BossEncounters))
                .after(ActorDecisionSet::Publish)
                .before(ambition_platformer2d_shared_tangle::schedule::PlayerInputSet::ControlGate)
                .in_set(ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhase::WorldPrep),
        );
        // Script steering of a boss (a conducted facing, content steering)
        // overrides the brain. It is in the gate after the blank and after
        // `drive_commanded_moves`, the one walk to a mark for every body, so a
        // held body is walked by the script and a more specific steer has the
        // last word.
        app.configure_sets(
            sim,
            ambition_platformer2d_shared_tangle::schedule::BossSteerSlot
                .after(tick_boss_brains_system)
                .after(crate::control::blank_scripted_control_frames)
                .after(crate::control::drive_commanded_moves)
                .in_set(ambition_platformer2d_shared_tangle::schedule::PlayerInputSet::ControlGate),
        );
    }
}

#[cfg(test)]
mod actor_movement_tests;

#[cfg(test)]
mod body_life_stats_tests {
    use super::track_body_life_stats;
    use ambition_platformer2d_core as ae;
    use bevy::prelude::*;

    /// The diagnostics count each announced restart once and restart their
    /// clock, reading the latch before the announcer consumes it.
    #[test]
    fn a_restart_is_counted_once_and_restarts_the_clock() {
        let mut app = App::new();
        app.insert_resource(ambition_time::WorldTime::new(0.5, 0.5));
        app.add_systems(
            Update,
            (track_body_life_stats, ae::announce_body_restarts).chain(),
        );
        let body = app
            .world_mut()
            .spawn((
                ae::BodyRestartLatch::default(),
                ae::BodyLifeStats::default(),
            ))
            .id();
        app.update();
        app.update();
        let stats = *app.world().get::<ae::BodyLifeStats>(body).unwrap();
        assert_eq!(
            stats,
            ae::BodyLifeStats {
                time_alive: 1.0,
                resets: 0
            }
        );

        app.world_mut()
            .get_mut::<ae::BodyRestartLatch>(body)
            .unwrap()
            .pending = true;
        app.update();
        app.update();
        let stats = *app.world().get::<ae::BodyLifeStats>(body).unwrap();
        assert_eq!(
            stats,
            ae::BodyLifeStats {
                time_alive: 0.5,
                resets: 1
            },
            "one pending restart is one reset, and the clock starts again from it"
        );
    }
}
