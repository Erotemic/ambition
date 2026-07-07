//! Gameplay flight recorder / OOB debug logging.
//!
//! A rolling ring buffer of per-frame player snapshots and discrete gameplay
//! events. The buffer is filled inside the player tick (simulation-side) so
//! the recorder works in the headless binary as well as the visible game.
//!
//! See `docs/systems/gameplay-trace-recorder.md` for the workflow and bug-reporting
//! checklist.

use ae::AabbExt;
use ambition_engine_core as ae;
use bevy::prelude::*;

use ambition_engine_core::RoomGeometry;
use ambition_input::ControlFrame;

const NEARBY_COLLISION_RADIUS: f32 = 220.0;
const MAX_NEARBY_COLLISION: usize = 32;
const ABSURD_VELOCITY_MAGNITUDE: f32 = 8000.0;
/// Margin (in active-area coords) beyond which a player is considered OOB.
/// This is intentionally generous so authored levels with intentional
/// camera-out-of-room moments do not auto-dump on every frame.
const OOB_MARGIN: f32 = 96.0;

/// If the per-frame position delta exceeds the maximum movement we'd
/// expect from the player's velocity (plus a small slack), the
/// recorder treats it as a teleport / collision correction and emits
/// a `CollisionCorrection` event. This catches cases where the player
/// teleports from a wall-cling position to an out-of-world ledge with
/// no input change.
const TELEPORT_DETECTION_SLACK_PX: f32 = 16.0;

mod actor_oob;
mod detect;
mod plugin;
mod systems;

#[cfg(test)]
mod tests;

// The reusable trace FORMAT (schema + buffer + dump writers) now lives in the
// `ambition_gameplay_trace` foundation crate; re-exported so existing `crate::trace::*`
// paths (and the headless replay harness) keep resolving. The recording SYSTEMS
// below (`detect` / `systems` / `plugin`) stay here, next to the live player +
// world state they sample.
pub use ambition_gameplay_trace::*;

pub use actor_oob::{body_snapshot, flush_actor_dump, record_actor_oob_frame_system};
pub use detect::{build_frame, detect_oob_from_kinematics, detect_oob_scratch, record_frame};
pub(crate) use detect::{synthesize_events_from_diff, update_previous_snapshot};
pub use plugin::TraceSchedulePlugin;
pub use systems::{
    flush_pending_dump, handle_trace_hotkey, record_frame_system, record_simulation_frame,
};
