//! The publication boundary for local controller input.
//!
//! `SeatRawFrames` is that stage for everybody, and `commit_seat_raw_frames` is the one publication
//! for every seat — so this module keeps the BOUNDARY and has no function left.

use bevy::prelude::SystemSet;

/// The publication boundary for local controller input.
///
/// Any system that shapes a seat's frame for this tick — gesture derivation,
/// portal input shaping, a scripted substitution — must run before this set.
/// Systems that need the canonical slot value may order after it.
///
/// it is a semantic set rather than a named function on purpose, so other
/// crates order against the boundary instead of against whichever leaf currently
/// performs the publish. That indirection is what let the leaf change from
/// `populate_slot_controls` to `commit_seat_raw_frames` without a single
/// consumer moving.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PrimarySlotInputCommit;
