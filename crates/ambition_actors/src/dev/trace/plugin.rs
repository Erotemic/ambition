//! Module-local Bevy [`Plugin`] for the gameplay trace recorder.
//!
//! The trace runs in [`crate::schedule::SandboxSet::Trace`] (configured by
//! `app/schedule.rs`), which orders after `CoreSimulation` so the
//! per-frame snapshot captures the resolved player state. Both the
//! visible binary and the headless driver install this plugin via
//! `add_simulation_plugins`, so trace dumps work in either build.

use bevy::prelude::*;

use crate::schedule::SandboxSet;
use ambition_platformer_primitives::schedule::SimScheduleExt;

pub struct TraceSchedulePlugin;

impl Plugin for TraceSchedulePlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        // `record_frame_system` reads the portal teleport fact when the
        // feature is compiled in; register the message here too (idempotent)
        // so the trace works in apps that compile portal support without
        // adding `PortalPlugin` (e.g. the demo shell).
        #[cfg(feature = "portal")]
        app.add_message::<ambition_portal::BodyTeleported>();
        app.init_resource::<ambition_gameplay_trace::ActorTraceBuffer>()
            .add_systems(
                sim,
                (
                    super::record_frame_system,
                    // Non-player-centric OOB recorder: samples every body and
                    // requests a dump when any character leaves the world.
                    super::record_actor_oob_frame_system,
                )
                    .in_set(SandboxSet::Trace),
            )
            // Disk writes are irreversible host effects, so they stay outside
            // the simulation schedule.
            //
            // The recorders above used to be gated on
            // `simulation_pass_is_authoritative`, and that was the wrong policy
            // rather than a conservative one. "Authoritative" there meant
            // FIRST-PASS, which is not the same as confirmed: a first pass may
            // be a prediction that a rewind later corrects, and skipping the
            // corrected pass meant the trace kept the guess forever. A forensic
            // record that quietly preserves the wrong version of history is
            // worse than one that lags.
            //
            // Rows and anomaly assessments are keyed by session generation +
            // simulation frame. A re-simulation REPLACES both. Automatic dump
            // arming waits for GGRS confirmation, so the irreversible file write
            // reflects corrected truth rather than whichever pass happened first.
            .add_systems(
                PostUpdate,
                (super::flush_pending_dump, super::flush_actor_dump).chain(),
            );
    }
}
