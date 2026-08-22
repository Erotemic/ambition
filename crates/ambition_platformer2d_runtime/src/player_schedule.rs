//! The per-frame PLAYER schedule wiring (E5 step 5) — the engine-generic
//! player-frame lifecycle every platformer built on this engine runs, headless
//! or windowed: time control → input → controlled-subject resolution → brains
//! → body mode → possession → hit events → presentation write-back.
//!
//! Moved from `ambition_app::app::plugins::register_player_input_systems` /
//! `register_player_simulation_systems` / `register_presentation_sync_systems`.
//! The app-LOCAL residue stays app-side and pins itself into the gaps by
//! naming these engine systems (the ordering contract below documents the
//! gaps):
//!
//! - the host's own reset-INPUT system (Ambition's
//!   `apply_player_reset_input_system`) pins
//!   `.after(DevEditApplySet).before(input_timer_system)`
//!   in `Platformer2dSimulationPhaseMonolith::PlayerInput`. Its former chain partner, the
//!   `RoomReplayRequested` consumer, is engine-side as of 2026-07-21 —
//!   see [`crate::sandbox_reset`];
//! - the home-reset/presentation pair (`apply_home_reset_policy`,
//!   `sync_player_presentation`) joins `PlayerSimulationSet::PostPossession` —
//!   the slot between control settling and damage landing. It used to pin itself
//!   with `.after(release_possession_if_target_lost).before(apply_player_hit_events)`,
//!   which is the same position stated as two leaf names a host had to trust.
//!
//! Both gaps are ordering SLOTS: a host that registers nothing there gets the
//! same engine chain with the slot collapsed.

use bevy::prelude::*;

use ambition_platformer2d_actor_monolith::avatar::PlayerBodyFrameOutput;
use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;
use ambition_platformer2d_shared_tangle::schedule::{gameplay_allowed, gameplay_suspended};
use ambition_platformer2d_shared_tangle::schedule::{
    Platformer2dSimulationPhaseMonolith, PlayerInputSet, PlayerSimulationSet,
};

/// Registers the engine-generic player frame (see module docs). Part of
/// [`crate::PlatformerEnginePlugins`]; headless/RL builds run every system
/// here (RL drives the same brain/input seams a human does).
pub struct PlayerSchedulePlugin;

impl Plugin for PlayerSchedulePlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        // Every player body carries the movement→presentation hand-off the
        // movement phase writes and the presentation phase reads (required so
        // both phase queries always match the player + any clone).
        app.register_required_components::<ambition_platformer2d_actor_monolith::actor::PlayerEntity, PlayerBodyFrameOutput>();
        // Every player body publishes the same gravity-oriented combat
        // footprint an actor does (fable review 2026-07-02 §A6);
        // integrate_home_body writes it.
        app.register_required_components_with::<ambition_platformer2d_actor_monolith::actor::PlayerEntity, ambition_platformer2d_core::CenteredAabb>(
            || ambition_platformer2d_core::CenteredAabb::new(ambition_platformer2d_core::Vec2::ZERO, ambition_platformer2d_core::Vec2::ZERO),
        );
        // ...and the same PUBLISHED silhouette every other body has.
        //
        // Carrying `DamageableVolumes` is what makes a body a damage target, and
        // the player did not carry it — so `refresh_body_damageable_volumes` had
        // nothing to write into and `apply_hitbox_damage` fell back to the coarse
        // box for the player alone. A player could author a hurtbox timeline and be
        // hit on a rectangle instead. Jon's ruling: a thing that works for an enemy
        // and not a player is a smell, not a special case.
        //
        // The default is UNPUBLISHED, not intangible — the distinction the
        // component exists to make. `DamageableVolumes::intangible()` is
        // `published && volumes.is_empty()`, so a body that has never run the
        // publisher reads as "no answer yet" and every consumer falls back to the
        // coarse box. A freshly required component cannot make the player a ghost.
        //
        // The publisher's ordering still matters, for PRECISION rather than for
        // existence: `register_damage_facing_volume_publication` pins it after
        // `CombatSet::Playback` and before `CombatSet::Resolve`, so damage resolves
        // against this tick's authored silhouette instead of the coarse box.
        app.register_required_components::<ambition_platformer2d_actor_monolith::actor::PlayerEntity, ambition_combat::components::DamageableVolumes>();

        // ── PlayerInput, part A: the frame's time snapshot ────────────────
        //
        // Ordering subtleties (ADR 0010 §"Suspended time"):
        // * `apply_suspended_time_scale_system` runs FIRST so when gameplay
        //   is suspended (pause / dialogue / cutscene / room transition) the
        //   sim_clock target and `RoomTransitionCooldown::time_scale` are zeroed
        //   BEFORE `refresh_world_time` snapshots them.
        // * `refresh_world_time` snapshots whichever path won — the
        //   suspended-zero fallback this frame, or the clock-request tail of
        //   the PREVIOUS frame (below) — so downstream systems always see a
        //   coherent `scaled_dt`.
        app.add_systems(
            sim,
            (
                // THE TIMELINE, first thing in the step: every system below
                // this line — and every hash or recorded input frame — belongs
                // to the tick this names. Unconditional: a suspended world
                // still advances its timeline, it just moves zero sim seconds.
                ambition_time::advance_sim_tick,
                ambition_time::time_control::apply_suspended_time_scale_system
                    .run_if(gameplay_suspended),
                ambition_time::refresh_world_time,
                // Mirror the freshly-snapshotted `WorldTime::sim_dt()` into
                // the runtime crate's neutral `SimDt` so every downstream
                // runtime system (gravity / zones / orient-roll) reads scaled
                // dt without a sandbox dependency.
                ambition_platformer2d_actor_monolith::mirror_sim_dt_into_runtime
                    .in_set(ambition_platformer2d_actor_monolith::SimDtMirrored),
            )
                .chain()
                .in_set(Platformer2dSimulationPhaseMonolith::PlayerInput),
        );

        // ── Frame tail: the time-control pipeline ─────────────────────────
        //
        // The clock-request consumers run AFTER every producer in the frame
        // (hit resolver in PlayerSimulation, room commit in RoomTransition,
        // session reset in ResetProcessing, plus the intent emitter here), so
        // a `ClockScaleRequest`/`ClockResetRequest` is consumed the same sim
        // frame it is written. That is the rollback doctrine (deep review
        // §2.2): message buffers are cleared on GGRS `LoadWorld`, so a request
        // crossing a frame boundary silently dies during re-simulation; the
        // truth that DOES cross the boundary here is `RequestedClockScale` +
        // `ClockState` — registered resources GGRS restores exactly.
        //
        // Observable timing is unchanged from the historical frame-start
        // placement: `refresh_world_time` (part A above) still snapshots at
        // the top of the NEXT frame, which is exactly when a frame-start
        // consumer would first have exposed the effect of a request produced
        // mid-frame. The one deliberate delta: the emitter now reads THIS
        // frame's hitstop/blink facts instead of last frame's, so the tail
        // consumes its request in-frame too.
        //
        // Gating mirrors part A: while gameplay is suspended the trio idles
        // and the suspended-zero path keeps pause/dialogue/room-transition
        // frames frozen; on resume the smoother ramps back up from 0 at the
        // authored rate.
        app.add_systems(
            sim,
            (
                ambition_platformer2d_actor_monolith::time::time_control::emit_player_time_intent_system
                    .run_if(gameplay_allowed),
                ambition_time::time_control::apply_clock_scale_requests
                    .run_if(gameplay_allowed),
                ambition_platformer2d_actor_monolith::time::time_control::smooth_sim_clock_toward_target_system
                    .run_if(gameplay_allowed),
                ambition_time::time_control::apply_clock_reset_requests
                    .run_if(gameplay_allowed),
            )
                .chain()
                .in_set(ambition_platformer2d_shared_tangle::schedule::GameplaySimulationRoot)
                .after(Platformer2dSimulationPhaseMonolith::ResetProcessing)
                .before(Platformer2dSimulationPhaseMonolith::FeatureViewSync),
        );

        // The dev-tools DOMAIN set (its systems live in `DevToolsSimPlugin`;
        // decision #9: the assembly orders sets, never dev leaf systems).
        // Positioned at part A's tail so live tuning edits apply before the
        // input→brain chain consumes them.
        app.configure_sets(
            sim,
            ambition_dev_tools::DevEditApplySet
                .after(ambition_platformer2d_actor_monolith::SimDtMirrored)
                .in_set(Platformer2dSimulationPhaseMonolith::PlayerInput),
        );

        // ── PlayerInput, part B: input → controlled subject → brains ──────
        //
        // Ordered after part A's tail (the dev-tools `DevEditApplySet`).
        // The host's reset/replay pair slots into the A→B gap (module docs).
        app.add_systems(
            sim,
            (
                // `InputSet::Route` supplies the ordering window: interaction is
                // buffered before portal warping, and both happen before
                // `PrimarySlotInputCommit`. Confirmed-input derivations run in the sim schedule.
                // ⛔ both read `WorldTime`, which is a SNAPSHOT taken by
                // `refresh_world_time` at the head of this same schedule. Bevy's
                // `Time` needed no such edge; a snapshot does, or these advance
                // on last tick's dt.
                ambition_platformer2d_actor_monolith::control::input_timer_system
                    .in_set(ambition_platformer2d_actor_monolith::control::InputTimersAdvanced)
                    .run_if(gameplay_allowed)
                    .after(ambition_time::refresh_world_time)
                    .in_set(ambition_input::InputSet::Route),
                ambition_platformer2d_actor_monolith::control::interaction_input_system
                    .in_set(ambition_platformer2d_actor_monolith::control::InteractionInputBuffered)
                    .run_if(gameplay_allowed)
                    .after(ambition_time::refresh_world_time),
                // Portal-warped held movement input is registered by
                // `ambition_portal2d::PortalPlugin` so the portal
                // subsystem owns its input seam.
                // Controller-input setup, nested into one chained group:
                // 1. Resolve the CONTROLLED SUBJECT — the body carrying
                //    `DrivingParticipant(PRIMARY)` this frame (home avatar, or a
                //    possessed actor).
                // 2. ⭐ **the publish used to be here and it was seat zero's
                //    alone** — `populate_slot_controls`, copying the global
                //    `ControlFrame` into `SlotControls[PRIMARY]`. Every seat is
                //    committed together now, by `commit_seat_raw_frames` in the
                //    host's `PrimarySlotInputCommit`; bodies still read their
                //    slot through `DrivingParticipant(slot)` and no input
                //    component is copied.
                (
                    ambition_platformer2d_actor_monolith::abilities::traversal::possession::resolve_controlled_subject,
                    // ⭐ **THE PUBLISH, for a composition with no latch** — where
                    // `populate_slot_controls` used to stand, and registered here
                    // for the same reason it was: every composition has a sim
                    // schedule, and not every one installs the device host.
                    ambition_platformer2d_actor_monolith::schedule::publish_seat_controls_when_nobody_else_does
                        .in_set(ambition_platformer2d_actor_monolith::control::PrimarySlotInputCommit),
                    // ⭐ **and the MIRROR, once, for every host.** `ControlFrame`
                    // is seat zero's output now; the forensic trace codec and the
                    // harness's action encoder read it, and they must be able to
                    // whatever clock the composition runs on.
                    ambition_platformer2d_actor_monolith::schedule::mirror_primary_slot_to_control_frame,
                    // N0.2: capture the input the SIM consumes, which is not the
                    // input the device produced — gestures, portal warp, and the
                    // fixed-tick latch all rewrite the frame on the way here.
                    crate::input_stream::record_input_stream
                        .run_if(crate::input_stream::input_stream_recording),
                )
                    .chain(),
            )
                .chain()
                .in_set(PlayerInputSet::Device)
                .after(ambition_dev_tools::DevEditApplySet),
        );

        // The rest of what used to be one long chain, now placed by PHASE.
        //
        // The systems and their order are unchanged; what changed is that each
        // one states which phase it belongs to, so a caller elsewhere can order
        // against the phase instead of against a name. `PlayerInputSet` carries
        // the argument, including the schedule cycle the leaf-naming style cost
        // on 2026-07-27.
        app.add_systems(
            sim,
            // Canonical persona derive. Identity changes refresh the full
            // persona; live BodyAbilities edits refresh only HostCode-derived
            // kit state, preserving authored movement state. Its own phase gives
            // deferred capability-marker commands an apply-deferred seam before
            // the brain/effects consumers run.
            ambition_platformer2d_actor_monolith::avatar::apply_worn_character_gameplay
                .in_set(PlayerInputSet::Persona),
        );
        app.add_systems(
            sim,
            // Universal-brain seam: translate this frame's slot input into
            // each controlled body's ActorControl frame.
            ambition_platformer2d_actor_monolith::avatar::tick_controlled_brains
                .in_set(ambition_platformer2d_actor_monolith::avatar::ControlledBrainTick)
                .in_set(PlayerInputSet::Brain),
        );
        // Causal recording, in the SIM schedule because that is where its
        // publishers live and where `SimulationReplayState` means anything.
        //
        // ⚠ the stamp runs FIRST and everything else is `.after` it. The first
        // version of this plugin stamped in `Last`, and the parallel-schedule
        // proof caught it immediately: every fact published during the frame
        // carried the PREVIOUS frame's tick, and none of them knew the host was
        // resimulating. A publisher cannot know either of those; the host is the
        // only thing that does.
        #[cfg(feature = "causal")]
        app.add_systems(
            sim,
            (
                crate::causal::stamp_causal_frame,
                (
                    crate::causal::record_execution_identity,
                    // The movement-intent OBSERVER, strictly after the brain: it
                    // reads the frame the brain just wrote. It takes every
                    // component immutably, so it cannot be the thing that broke
                    // the tick — a property of its signature, not a promise,
                    // which matters because a rollback host resimulates and an
                    // instrument that nudged state would desync exactly when
                    // somebody was using it to find out why.
                    crate::causal::record_player_movement_intent
                        .after(ambition_platformer2d_actor_monolith::avatar::ControlledBrainTick),
                )
                    .in_set(crate::causal::RecordingSet::Publish),
            )
                .chain()
                .in_set(PlayerInputSet::Brain),
        );
        app.add_systems(
            sim,
            (
                // A body a scripted sequence is driving (a death beat, a
                // flagpole slide, an act clear) stops answering input HERE —
                // after the brain wrote the frame, before anything reads it.
                // That position is the only one where blanking is observable,
                // which is why the sequences that blanked from their own phase
                // suppressed nothing.
                // **A CAPTIVE'S STRUGGLE, read while the frame is still the
                // person's.** Human input is blanked on the line below, so this
                // is the only position where a held player's mash exists at all.
                // Its twin sits before the WorldPrep blanking, where an actor
                // brain's frame is the live one — the same reason the blanking
                // itself is in two places.
                ambition_combat::capture::systems::sample_capture_escape,
                ambition_platformer2d_actor_monolith::avatar::blank_scripted_control_frames,
                // ActionSet gates the generic resolver, but the body shield,
                // slash/recoil, and charge-projectile paths still read raw control.
                // Sanitize those direct verbs from the same worn kit before any
                // simulation/effects phase consumes them.
                ambition_platformer2d_actor_monolith::avatar::gate_worn_player_control
                    .in_set(ambition_platformer2d_actor_monolith::avatar::WornControlGateSet),
                // A folded `bubble_shield` special MOVE forces `shield_held` for
                // its duration, so pressing Special raises the ONE shield through
                // the same kernel path a held guard does. After the gate (which
                // keeps the persona's shield verb alive), before WorldPrep.
                ambition_platformer2d_actor_monolith::avatar::sustain_bubble_shield,
            )
                .chain()
                .in_set(PlayerInputSet::ControlGate),
        );
        app.add_systems(
            sim,
            (
                // Body-mode policy (crouch / morph / climb) consumes the
                // CONTROLLED body's freshly-produced ActorControl + its slot
                // gestures, so it runs AFTER the brain phase and before
                // WorldPrep movement consumes the resize/mode change.
                ambition_platformer2d_actor_monolith::body_mode::update_body_mode,
                ambition_platformer2d_actor_monolith::avatar::sync_player_actor_poses,
            )
                .chain()
                .in_set(PlayerInputSet::BodyMode),
        );

        // The content dialogue-followup slot lives in PlayerInput; the HOST
        // adds the consumer-relative edge (`.before(its replay consumer)`) —
        // the engine only gives the slot its phase home.
        app.configure_sets(
            sim,
            ambition_platformer2d_actor_monolith::session::reset::ContentDialogueFollowupSet
                .in_set(Platformer2dSimulationPhaseMonolith::PlayerInput),
        );

        // Universal-brain effects resolver — AFTER `WorldPrep` so it observes
        // THIS frame's actor `ActorControl` (the actor/boss brain ticks run in
        // WorldPrep), and before `PlayerSimulation`/`Combat` where the
        // consumers spawn hitboxes/projectiles, same frame.
        app.add_systems(
            sim,
            (
                ambition_characters::brain::emit_brain_action_messages,
                ambition_characters::brain::emit_player_projectile_tick_messages,
                ambition_characters::brain::observe_brain_action_counter,
            )
                .chain()
                .in_set(ambition_platformer2d_shared_tangle::schedule::GameplaySimulationRoot)
                .after(Platformer2dSimulationPhaseMonolith::WorldPrep)
                .before(Platformer2dSimulationPhaseMonolith::PlayerSimulation),
        );

        // ── PlayerSimulation: possession + hit events ──────────────────────
        //
        // Possession is pure BRAIN TRANSFER: the vacated home avatar is inert
        // because it no longer carries a player brain, and the possessed
        // actor is driven through the actor tick by the transferred
        // the primary seat. The host's home-reset/presentation pair slots
        // between `release_possession_if_target_lost` and
        // `apply_player_hit_events` (module docs).
        app.add_systems(
            sim,
            (
                ambition_platformer2d_actor_monolith::abilities::traversal::possession::possession_trigger_system
                    .run_if(gameplay_allowed),
                ambition_platformer2d_actor_monolith::abilities::traversal::possession::release_possession_if_target_lost,
                // ⭐ **THE DRIVEN BODY'S CUSTODY MARKER, reprojected every tick.**
                // Last in the chain so it sees the possession this tick settled
                // on, and deliberately UNGATED for the same reason its item
                // sibling is: a room transition suspends gameplay between the
                // crossing and the commit, and that window is exactly when the
                // room sweep reads residency.
                ambition_platformer2d_actor_monolith::body_custody::project_body_custody
                    // ⭐ **the LABEL the item domain orders against**, and it is
                    // on the system rather than on `Possession` because what the
                    // item chain depends on is body custody being SETTLED, not
                    // possession having happened. See `BodyCustodySettled`.
                    .in_set(ambition_platformer2d_shared_tangle::lifecycle::BodyCustodySettled),
                // ⭐⭐ **WHO DRIVES WHICH BODY, reprojected beside WHERE IT IS.**
                // The same shape as its neighbour above and for the same reason:
                // a fact derived from state that is already in the snapshot needs
                // no snapshot entry, and deriving it here — after the possession
                // this tick settled on — is what lets a reader ask *who is
                // driving* without knowing that today's answer is spelled inside
                // an AI-policy enum. `FeatureInteraction` is a later phase than
                // `PlayerSimulation` in the top-level chain, so the interaction
                // systems read a component this frame's possession wrote.
                ambition_platformer2d_actor_monolith::control::project_driving_participant,
            )
                .chain()
                .in_set(PlayerSimulationSet::Possession),
        );
        app.add_systems(
            sim,
            (
                ambition_platformer2d_actor_monolith::features::ecs::damage_apply::apply_player_hit_events
                    .in_set(
                        ambition_platformer2d_actor_monolith::features::ecs::damage_apply::PlayerHitResolutionSet,
                    )
                    .run_if(gameplay_allowed),
                // The kernel's own death path (pit / drown / tile hazard) never
                // reaches the hit resolver, so it publishes its death fact here
                // — the movement phase in `WorldPrep` has already flagged the
                // reset this reads. Deliberately NOT gated on
                // `gameplay_allowed`: falling out of the world while a dialogue
                // is open is still a death, and dropping it would leave the
                // body respawned with no consumer ever told why.
                ambition_platformer2d_actor_monolith::features::ecs::damage_apply::publish_kernel_reset_death,
                // ADR 0033 — the death fact is published, and then the ruleset
                // owns everything after it. Opening the window in the SAME
                // frame as the fact is what stops the world acting on the body:
                // from here the blast-zone gate skips it, so a corpse in a pit
                // cannot re-flag a reset (192 times per death, measured).
                //
                // Ticked and closed in the same chain so an interlude of zero —
                // the default for a composition that states no rules — opens
                // and closes on the death frame rather than leaking a window
                // nobody will ever close.
                ambition_platformer2d_actor_monolith::session::death::open_death_interlude,
                ambition_platformer2d_actor_monolith::session::death::tick_death_interlude,
            )
                .chain()
                .in_set(PlayerSimulationSet::Outcome),
        );
        // ⛔ **CLOSING runs NEXT FRAME, immediately before the replay consumer,
        // and the split is a ROLLBACK requirement rather than taste.**
        //
        // The consequence it requests is `RoomReplayRequested`, and that channel
        // is `clear_message_on_rollback`. Written here in `Outcome` it would be
        // consumed by `apply_room_replay_request_system` in the NEXT frame's
        // `PlayerInput` — so a rewind across that boundary wipes the message,
        // the resimulated branch never resets the level, and the two runs
        // diverge. Measured: a GGRS sync-test checksum mismatch at the first
        // death, on a route that was green before.
        //
        // Everything this system reads — the window and the roster — is SNAPSHOT
        // state, so running it in the same frame as the consumer makes the whole
        // request re-derivable during a resimulation. Opening still happens in
        // `Outcome`, in the same frame as the death fact, for the mirror-image
        // reason: `ActorDiedMessage` is cleared on rollback too, so it must be
        // read in the frame it was written.
        app.add_systems(
            sim,
            ambition_platformer2d_actor_monolith::session::death::close_death_interlude
                .in_set(Platformer2dSimulationPhaseMonolith::PlayerInput)
                .before(crate::sandbox_reset::RoomReplayApplied),
        );
        // Every respawn in the workspace announces itself through the derived
        // `BodyRestarted`, so returning a body to play needs no line at any of
        // the call sites that bring one back.
        app.add_observer(
            ambition_platformer2d_actor_monolith::session::death::clear_out_of_play_on_restart,
        );

        // ── PresentationSync: presentation timer decay ─────────────────────
        //
        // Runs unconditionally so paused / dialogue modes still wind down
        // flash and landing-pose timers.
        //
        // ⭐ **`write_player_ecs_components` is gone** (AC3.1.A/B). It existed to
        // maintain two `BodyCombat` mirrors — `attacking` from `BodyMelee` and
        // `alive` from `BodyHealth` — and when both were deleted it had no work
        // left to do at all.
        app.add_systems(
            sim,
            ambition_platformer2d_actor_monolith::control::cleanup_timers_system
                .in_set(Platformer2dSimulationPhaseMonolith::PresentationSync),
        );
    }
}
