//! Module-local Bevy [`Plugin`] for the gameplay trace recorder.
//!
//! The trace runs in [`ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhase::Trace`] (configured by
//! `actor_monolith/src/schedule/schedule.rs`), which orders after
//! `CoreSimulation` so the per-frame snapshot captures the resolved player
//! state. Both the
//! visible binary and the headless driver install this plugin via
//! `add_simulation_plugins`, so trace dumps work in either build.

use bevy::prelude::*;

use ambition_platformer2d_shared_tangle::schedule::Platformer2dSimulationPhase;
use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;

pub struct TraceSchedulePlugin;

impl Plugin for TraceSchedulePlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        // `record_frame_system` reads the portal teleport fact when the
        // feature is compiled in; register the message here too (idempotent)
        // so the trace works in apps that compile portal support without
        // adding `PortalPlugin` (e.g. the demo shell).
        #[cfg(feature = "portal")]
        app.add_message::<ambition_portal2d::BodyTeleported>();
        use ambition_gameplay_trace::{ActorTraceBuffer, TraceDumpPolicy};
        let policy = TraceDumpPolicy::from_env();
        app.insert_resource(policy);
        // Install the recorders on hosts that support trace capture.
        if !cfg!(target_os = "android") {
            // Automatic dumps need the history. Tools can add the buffer to
            // record history without enabling automatic file writes.
            if policy.auto_dumps {
                app.init_resource::<ActorTraceBuffer>();
            }
            app.add_systems(
                sim,
                (
                    super::record_frame_system,
                    super::record_actor_oob_frame_system
                        .run_if(resource_exists::<ActorTraceBuffer>),
                )
                    .in_set(Platformer2dSimulationPhase::Trace),
            );
        }
        app
            // Disk writes are irreversible host effects, so they stay outside
            // the simulation schedule.
            //
            // A forensic record that quietly preserves the wrong version of history is worse than
            // one that lags.
            //
            // Rows and anomaly assessments are keyed by session generation +
            // simulation frame. A re-simulation REPLACES both. Automatic dump
            // arming waits for GGRS confirmation, so the irreversible file write
            // reflects corrected truth rather than whichever pass happened first.
            .add_systems(
                PostUpdate,
                (
                    super::flush_pending_dump,
                    super::flush_actor_dump.run_if(resource_exists::<ActorTraceBuffer>),
                ).chain(),
            );
    }
}
