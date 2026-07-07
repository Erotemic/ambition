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
//! - the reset pair (`apply_player_reset_input_system`,
//!   `apply_room_replay_request_system`) pins
//!   `.after(sync_live_player_dev_edits_system).before(input_timer_system)`
//!   in `SandboxSet::PlayerInput`;
//! - the home-reset/presentation pair (`apply_home_reset_policy`,
//!   `sync_player_presentation`) pins
//!   `.after(release_possession_if_target_lost).before(apply_player_hit_events)`
//!   in `SandboxSet::PlayerSimulation`.
//!
//! Both gaps are ordering SLOTS: a host that registers nothing there gets the
//! same engine chain with the slot collapsed.

use bevy::prelude::*;

use ambition_platformer_primitives::schedule::{gameplay_allowed, gameplay_suspended};
use ambition_actors::player::PlayerBodyFrameOutput;
use ambition_actors::schedule::SandboxSet;

/// Registers the engine-generic player frame (see module docs). Part of
/// [`crate::PlatformerEnginePlugins`]; headless/RL builds run every system
/// here (RL drives the same brain/input seams a human does).
pub struct PlayerSchedulePlugin;

impl Plugin for PlayerSchedulePlugin {
    fn build(&self, app: &mut App) {
        // Every player body carries the movement→presentation hand-off the
        // movement phase writes and the presentation phase reads (required so
        // both phase queries always match the player + any clone).
        app.register_required_components::<ambition_actors::actor::PlayerEntity, PlayerBodyFrameOutput>();
        // Every player body publishes the same gravity-oriented combat
        // footprint an actor does (fable review 2026-07-02 §A6);
        // integrate_home_body writes it.
        app.register_required_components_with::<ambition_actors::actor::PlayerEntity, ambition_engine_core::CenteredAabb>(
            || ambition_engine_core::CenteredAabb::new(ambition_engine_core::Vec2::ZERO, ambition_engine_core::Vec2::ZERO),
        );

        // ── PlayerInput, part A: the time-control pipeline ────────────────
        //
        // Ordering subtleties (ADR 0010 §"Suspended time"):
        // * `apply_suspended_time_scale_system` runs FIRST so when gameplay
        //   is suspended (pause / dialogue / cutscene / room transition) the
        //   sim_clock target and `SandboxSimState::time_scale` are zeroed
        //   BEFORE `refresh_world_time` snapshots them.
        // * The emit → apply → smooth trio is gated to `gameplay_allowed`
        //   so it doesn't immediately re-populate `RequestedClockScale` /
        //   `time_scale` back from the zero the suspended fallback just
        //   wrote. On the first re-resumed frame they run again and the
        //   smoother ramps back up from 0 to 1.0 at the authored rate.
        // * `refresh_world_time` then snapshots whichever path won this
        //   frame, so downstream systems always see a coherent `scaled_dt`.
        app.add_systems(
            Update,
            (
                ambition_actors::time::time_control::apply_suspended_time_scale_system
                    .run_if(gameplay_suspended),
                ambition_actors::time::time_control::emit_player_time_intent_system
                    .run_if(gameplay_allowed),
                ambition_actors::time::time_control::apply_clock_scale_requests
                    .run_if(gameplay_allowed),
                ambition_actors::time::time_control::smooth_sim_clock_toward_target_system
                    .run_if(gameplay_allowed),
                // Unconditional: snapshot whichever path (suspended-zero or
                // gameplay-smoothed) wrote `SandboxSimState::time_scale` this
                // frame into `WorldTime` for downstream readers.
                ambition_time::refresh_world_time,
                // Mirror the freshly-snapshotted `WorldTime::sim_dt()` into
                // the runtime crate's neutral `SimDt` so every downstream
                // runtime system (gravity / zones / orient-roll) reads scaled
                // dt without a sandbox dependency.
                ambition_actors::mirror_sim_dt_into_runtime,
                ambition_actors::dev::sync_live_player_dev_edits_system,
            )
                .chain()
                .in_set(SandboxSet::PlayerInput),
        );

        // ── PlayerInput, part B: input → controlled subject → brains ──────
        //
        // Ordered after part A's tail (`sync_live_player_dev_edits_system`).
        // The host's reset/replay pair slots into the A→B gap (module docs).
        app.add_systems(
            Update,
            (
                ambition_actors::player::input_timer_system
                    .run_if(gameplay_allowed)
                    .in_set(ambition_input::InputSet::Populate),
                ambition_actors::player::interaction_input_system.run_if(gameplay_allowed),
                // Portal-warped held movement input is registered by
                // `ambition_portal::PortalPlugin` so the portal
                // subsystem owns its input seam.
                // Controller-input setup, nested into one chained group:
                // 1. Resolve the CONTROLLED SUBJECT — the body carrying
                //    `Brain::Player(PRIMARY)` this frame (home avatar, or a
                //    possessed actor).
                // 2. Publish the local device frame into the slot-based
                //    controller model (`SlotControls[PRIMARY]`).
                // 3. Mirror each controlled body's slot frame onto its
                //    PlayerInputFrame (gated on brain ownership: a vacated
                //    avatar sees neutral input).
                (
                    ambition_actors::abilities::traversal::possession::resolve_controlled_subject,
                    ambition_actors::player::populate_slot_controls,
                    ambition_actors::player::sync_local_player_input_frame,
                )
                    .chain(),
                // Universal-brain seam: translate this frame's slot input into
                // each controlled body's ActorControl frame.
                ambition_actors::player::tick_player_brains,
                // Body-mode policy (crouch / morph / climb) consumes the
                // CONTROLLED body's freshly-produced ActorControl + its slot
                // gestures, so it runs AFTER `tick_player_brains` and before
                // WorldPrep movement consumes the resize/mode change.
                ambition_actors::body_mode::update_body_mode,
                ambition_actors::player::sync_player_actor_poses,
            )
                .chain()
                .in_set(SandboxSet::PlayerInput)
                .after(ambition_actors::dev::sync_live_player_dev_edits_system),
        );

        // The content dialogue-followup slot lives in PlayerInput; the HOST
        // adds the consumer-relative edge (`.before(its replay consumer)`) —
        // the engine only gives the slot its phase home.
        app.configure_sets(
            Update,
            ambition_actors::session::reset::ContentDialogueFollowupSet
                .in_set(SandboxSet::PlayerInput),
        );

        // Universal-brain effects resolver — AFTER `WorldPrep` so it observes
        // THIS frame's actor `ActorControl` (the actor/boss brain ticks run in
        // WorldPrep), and before `PlayerSimulation`/`Combat` where the
        // consumers spawn hitboxes/projectiles, same frame.
        app.add_systems(
            Update,
            (
                ambition_characters::brain::emit_brain_action_messages,
                ambition_characters::brain::emit_player_projectile_tick_messages,
                ambition_characters::brain::observe_brain_action_counter,
            )
                .chain()
                .after(SandboxSet::WorldPrep)
                .before(SandboxSet::PlayerSimulation),
        );

        // ── PlayerSimulation: possession + hit events ──────────────────────
        //
        // Possession is pure BRAIN TRANSFER: the vacated home avatar is inert
        // because it no longer carries a player brain, and the possessed
        // actor is driven through the actor tick by the transferred
        // `Brain::Player`. The host's home-reset/presentation pair slots
        // between `release_possession_if_target_lost` and
        // `apply_player_hit_events` (module docs).
        app.add_systems(
            Update,
            (
                ambition_actors::abilities::traversal::possession::possession_trigger_system
                    .run_if(gameplay_allowed),
                ambition_actors::abilities::traversal::possession::release_possession_if_target_lost,
                ambition_actors::features::ecs::damage_apply::apply_player_hit_events
                    .run_if(gameplay_allowed),
            )
                .chain()
                .in_set(SandboxSet::PlayerSimulation),
        );

        // ── PresentationSync: player ECS write-back + timer decay ──────────
        //
        // Runs unconditionally so paused / dialogue modes still wind down
        // flash and landing-pose timers.
        app.add_systems(
            Update,
            (
                ambition_actors::player::write_player_ecs_components,
                ambition_actors::player::cleanup_timers_system,
            )
                .chain()
                .in_set(SandboxSet::PresentationSync),
        );
    }
}
