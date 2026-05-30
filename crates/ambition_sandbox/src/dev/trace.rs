//! Gameplay flight recorder / OOB debug logging.
//!
//! A rolling ring buffer of per-frame player snapshots and discrete gameplay
//! events. The buffer is filled inside the player tick (simulation-side) so
//! the recorder works in the headless binary as well as the visible game.
//!
//! See `docs/systems/gameplay-trace-recorder.md` for the workflow and bug-reporting
//! checklist.

use std::collections::VecDeque;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::engine_core as ae;
use ae::AabbExt;
use bevy::prelude::*;
use serde::Serialize;

use crate::input::ControlFrame;
use crate::GameWorld;

const DEFAULT_FRAME_CAPACITY: usize = 240;
const DEFAULT_EVENT_CAPACITY: usize = 240;
const MARKDOWN_FRAME_SUMMARY_TAIL: usize = 120;
const MARKDOWN_EVENT_TAIL: usize = 100;
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

mod buffer;
mod detect;
mod dump;
mod model;
mod plugin;
mod systems;

#[cfg(test)]
mod tests;

pub use buffer::GameplayTraceBuffer;
pub use detect::{build_frame, detect_oob_from_kinematics, detect_oob_scratch, record_frame};
pub(crate) use detect::{synthesize_events_from_diff, update_previous_snapshot};
pub use dump::{default_dump_dir, dump_paths, write_dump};
pub use model::{
    CollisionTraceShape, ControlFrameTrace, DumpReason, GameplayTraceEvent, GameplayTraceFrame,
    MovingPlatformTraceState, OobReason, PlayerTraceState, TraceAabb, TracePoint,
};
pub use plugin::TraceSchedulePlugin;
pub use systems::{
    flush_pending_dump, handle_trace_hotkey, record_frame_system, record_simulation_frame,
};

use model::PreviousFrameSnapshot;

#[cfg(test)]
use dump::{timestamp_label, timestamp_label_with_seq};
