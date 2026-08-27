//! Engine-generic per-frame player lifecycle and its host extension slots.
//! Hosts order against semantic sets here rather than runtime leaf systems.

use bevy::prelude::*;

use ambition_platformer2d_actor_monolith::avatar::PlayerBodyFrameOutput;
use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;
use ambition_platformer2d_shared_tangle::schedule::{gameplay_allowed, gameplay_suspended};
use ambition_platformer2d_shared_tangle::schedule::{
    Platformer2dSimulationPhaseMonolith, PlayerInputSet, PlayerSimulationSet,
};

/// Registers the engine-generic player frame for windowed and headless hosts.
pub struct PlayerSchedulePlugin;

impl Plugin for PlayerSchedulePlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        // Every player body carries the movement→presentation hand-off the
        // movement phase writes and the presentation phase reads (required so
        // both phase queries always match the player + any clone).
        app.register_required_components::<ambition_platformer2d_shared_tangle::markers::PlayerEntity, PlayerBodyFrameOutput>();
        app.register_required_components_with::<ambition_platformer2d_shared_tangle::markers::PlayerEntity, ambition_platformer2d_core::CenteredAabb>(
            || ambition_platformer2d_core::CenteredAabb::new(ambition_platformer2d_core::Vec2::ZERO, ambition_platformer2d_core::Vec2::ZERO),
        );
        // Every player body carries the same published silhouette used by other bodies.
        // The default is unpublished, not intangible: consumers fall back to the coarse
        // box until the publisher supplies this tick's authored volumes.
        app.register_required_components::<ambition_platformer2d_shared_tangle::markers::PlayerEntity, ambition_combat::components::DamageableVolumes>();

        // Snapshot time before input: suspension first zeros the clock target, then
        // `refresh_world_time` publishes one coherent `scaled_dt` for the frame.
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

        // Consume clock requests after every producer in the frame. `WorldTime` is still
        // refreshed at the next frame head, while requests produced this frame are not
        // left pending across the frame boundary. Suspended gameplay uses the zero-scale path.
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
                // Input timers and interaction consume the `WorldTime` snapshot refreshed at the
                // head of this schedule, then feed the normal input routing window.
                // THREE systems, one set. They share the refreshed `WorldTime` and
                // nothing else -- a room cooldown, the home body's reaction
                // timers, and each seat's direction gestures have different
                // owners. `InputTimersAdvanced` is what the sandbox reset orders
                // against, so it stays the dependency whatever it contains.
                (
                    ambition_platformer2d_actor_monolith::control::tick_room_transition_cooldown,
                    ambition_platformer2d_actor_monolith::control::tick_home_body_reaction_timers,
                    ambition_platformer2d_actor_monolith::control::derive_slot_direction_gestures,
                )
                    .in_set(ambition_platformer2d_actor_monolith::control::InputTimersAdvanced)
                    .run_if(gameplay_allowed)
                    .after(ambition_time::refresh_world_time)
                    .in_set(ambition_input::InputSet::Route),
                ambition_platformer2d_actor_monolith::control::interaction_input_system
                    .in_set(ambition_platformer2d_actor_monolith::control::InteractionInputBuffered)
                    .run_if(gameplay_allowed)
                    .after(ambition_time::refresh_world_time),
                // Portal input stays owned by `PortalPlugin`. Resolve the body driven by each
                // `DrivingParticipant`; the host has already committed `SlotControls` for all
                // seats. Mirror the primary slot only for consumers of the legacy `ControlFrame`
                // surface, and record the final frame the simulation consumes.
                (
                    ambition_platformer2d_actor_monolith::abilities::traversal::possession::resolve_controlled_subject,
                    ambition_platformer2d_actor_monolith::schedule::publish_seat_controls_when_nobody_else_does
                        .in_set(ambition_platformer2d_actor_monolith::control::PrimarySlotInputCommit),
                    //  and the MIRROR, once, for every host. `ControlFrame`
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

        app.add_systems(
            sim,
            // Derive the canonical persona before brain/effect consumers. Identity changes
            // refresh the full persona; live HostCode ability edits preserve authored movement state.
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
        // Causal recording runs in the simulation schedule after the frame stamp so
        // replay state and publisher timing refer to the same simulation frame.
        #[cfg(feature = "causal")]
        app.add_systems(
            sim,
            (
                crate::causal::stamp_causal_frame,
                (
                    crate::causal::record_execution_identity,
                    // Observe movement intent after the brain. The observer is read-only so
                    // instrumentation cannot perturb rollback state.
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
                // ⛔⛔ ONE SAMPLE-THEN-BLANK, AND THIS IS THE ONE (D202).
                //
                // Sampling reads the captive's mash out of the frame; blanking is
                // what takes it away. The pair used to exist TWICE — once here
                // and once in `WorldPrepSet::BeforeIntegrate` — because this set
                // ran BEFORE autonomous control was published and could not gate
                // it. That was correct only by an invariant nothing enforced: the
                // first blank was what stopped the second sampler crediting the
                // same human press, so deleting either blank doubled a held
                // human's escape rate in silence.
                //
                // ⇒ this whole SET now runs in `WorldPrep`, after BOTH
                // publications, so one copy gates every body in the world. The
                // pair moved back into it from `BeforeIntegrate` because a scripted
                // body must be blanked BEFORE the gate consumers below read its
                // frame — see `configure_actor_decision_phases`.
                ambition_combat::capture::systems::sample_capture_escape,
                // The captor's stick, read for the throw edge. Beside the
                // captive's escape sampling because both are the RULESET's
                // reading of a capture's inputs, and both are scoped by asking
                // for `SmashHoldState`.
                ambition_combat::capture::systems::arm_smash_throw_edge,
                ambition_platformer2d_actor_monolith::avatar::blank_scripted_control_frames,
                ambition_combat::capture::systems::restrict_captor_control,
                // ActionSet gates the generic resolver, but the body shield,
                // slash/recoil, and charge-projectile paths still read raw control.
                // Sanitize those direct verbs from the same worn kit before any
                // simulation/effects phase consumes them.
                ambition_platformer2d_actor_monolith::avatar::gate_worn_player_control
                    .in_set(ambition_platformer2d_actor_monolith::avatar::WornControlGateSet),
                // A folded `bubble_shield` special MOVE forces `shield_held` for
                // its duration, so pressing Special raises the ONE shield through
                // the same kernel path a held guard does. After the gate, which
                // keeps the persona's shield verb alive.
                ambition_platformer2d_actor_monolith::avatar::sustain_bubble_shield,
            )
                .chain()
                .in_set(PlayerInputSet::ControlGate),
        );
        app.add_systems(
            sim,
            (
                // Body-mode policy (crouch / morph / climb) consumes FINISHED
                // control + its slot gestures, so it runs after both publication
                // phases and the gate above, and before `WorldPrepSet::Integrate`
                // consumes the resize/mode change. ⭐ an autonomous body's mode
                // now follows THIS tick's decision rather than the last one — the
                // AI frame did not exist yet when this sat in `PlayerInput`.
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

        // Possession transfers the player brain to the driven body; the vacated home body
        // becomes inert until the brain returns.
        app.add_systems(
            sim,
            (
                ambition_platformer2d_actor_monolith::abilities::traversal::possession::possession_trigger_system
                    .run_if(gameplay_allowed),
                ambition_platformer2d_actor_monolith::abilities::traversal::possession::release_possession_if_target_lost,
                // Reproject body custody after possession settles. It remains ungated so room
                // transitions can read settled residency while gameplay is suspended.
                ambition_platformer2d_actor_monolith::body_custody::project_body_custody
                    //  the LABEL the item domain orders against, and it is
                    // on the system rather than on `Possession` because what the
                    // item chain depends on is body custody being SETTLED, not
                    // possession having happened. See `BodyCustodySettled`.
                    .in_set(ambition_platformer2d_shared_tangle::lifecycle::BodyCustodySettled),
                // Reproject `DrivingParticipant` from settled possession state so later systems
                // can query control authority without depending on the brain representation.
                ambition_platformer2d_actor_monolith::control::project_driving_participant,
            )
                .chain()
                .in_set(PlayerSimulationSet::Possession),
        );
        app.add_systems(
            sim,
            (
                ambition_damage::apply_player_hit_events
                    .in_set(
                        ambition_damage::PlayerHitResolutionSet,
                    )
                    .run_if(gameplay_allowed),
                // Kernel deaths bypass hit resolution, so publish their death fact here after
                // movement has flagged the reset. This remains ungated so suspended dialogue
                // cannot swallow a pit/drown/tile-hazard death.
                ambition_platformer2d_actor_monolith::avatar::body_integration::publish_kernel_reset_death,
                // Open the out-of-play window in the death frame so later simulation stops acting
                // on the body. Tick it in the same chain; a zero-length interlude closes immediately.
                ambition_platformer2d_actor_monolith::session::death::open_death_interlude,
                ambition_combat::death_rules::tick_death_interlude,
            )
                .chain()
                .in_set(PlayerSimulationSet::Outcome),
        );
        // Close the death interlude next frame immediately before replay consumption.
        // `RoomReplayRequested` is cleared on rollback, so request production and consumption
        // must share a resimulated frame and be derived only from snapshot state. The death
        // window still opens on the original frame because `ActorDiedMessage` is also cleared.
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

        // Presentation timers decay even while gameplay is suspended.
        app.add_systems(
            sim,
            ambition_platformer2d_actor_monolith::control::cleanup_timers_system
                .in_set(Platformer2dSimulationPhaseMonolith::PresentationSync),
        );
    }
}
