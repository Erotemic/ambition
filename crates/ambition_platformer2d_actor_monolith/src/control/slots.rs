//! The publication boundary for local controller input.
//!
//! ⛔⛔ **`populate_slot_controls` USED TO LIVE HERE and it was the seat-zero
//! adapter.** It copied the finalized global `ControlFrame` into
//! `SlotControls[PRIMARY]`, which is the end of a pipeline only seat zero had:
//! every other seat went from its device straight into its own slot with no
//! shaping stage in between, so a gesture, a portal warp or a scripted
//! substitution could only ever apply to the primary (D175). `SeatRawFrames` is
//! that stage for everybody, and `commit_seat_raw_frames` is the one publication
//! for every seat — so this module keeps the BOUNDARY and has no function left.

use bevy::prelude::SystemSet;

/// The publication boundary for local controller input.
///
/// Any system that shapes a seat's frame for this tick — gesture derivation,
/// portal input shaping, a scripted substitution — must run **before** this set.
/// Systems that need the canonical slot value may order **after** it.
///
/// ⚠ **it is a semantic set rather than a named function on purpose**, so other
/// crates order against the boundary instead of against whichever leaf currently
/// performs the publish. That indirection is what let the leaf change from
/// `populate_slot_controls` to `commit_seat_raw_frames` without a single
/// consumer moving.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PrimarySlotInputCommit;
