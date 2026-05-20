//! Module-local Bevy [`Plugin`] for the gameplay trace recorder.
//!
//! The trace runs in [`crate::app::SandboxSet::Trace`] (configured by
//! `app/schedule.rs`), which orders after `CoreSimulation` so the
//! per-frame snapshot captures the resolved player state. Both the
//! visible binary and the headless driver install this plugin via
//! `add_simulation_plugins`, so trace dumps work in either build.
//!
//! Carved out of `app/plugins.rs::register_trace_systems` per
//! OVERNIGHT-TODO #6 (module-local plugins). The systems registered
//! here are siblings in this module's `systems.rs`.

use bevy::prelude::*;

use crate::app::SandboxSet;

pub struct TraceSchedulePlugin;

impl Plugin for TraceSchedulePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                super::record_frame_system,
                super::flush_pending_dump.after(super::record_frame_system),
            )
                .in_set(SandboxSet::Trace),
        );
    }
}
