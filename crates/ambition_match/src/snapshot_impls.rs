//! Rollback wire format for the match receipt and the per-body seat. The
//! orphan rule puts these beside their types: `SnapshotState` is core's.

use ambition_platformer2d_core::snapshot::{put_bool, put_u64, Reader, SnapshotState};

// ── A live MATCH's per-body state (AA2 / AC2) ────────────────────────────────
//
// Which seat a body is, which team it fights for, and who owns its death. All
// three are decided at match activation, all three are read by the rules every
// tick, and none of them was rollback state — because no swept population had
// a match in it until `every_component_in_a_live_match_is_registered_derived_or_waived`
// existed. A rewind across activation restored the fighters and left these
// behind, which is a body that comes back with no seat, no team, and the
// exploration death policy in the middle of a round.
impl SnapshotState for crate::MatchSeat {
    fn encode(&self, out: &mut Vec<u8>) {
        put_u64(out, self.0 as u64);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(crate::MatchSeat(r.u64()? as usize))
    }
}

/// The activation latch itself. Plain data with no identity in it — a seat count
/// and the frozen topology that decided it — which is what makes it snapshotable
/// at all: the BODIES are derived from `MatchSeat` and rewind on their own.
impl SnapshotState for crate::ActiveMatch {
    fn encode(&self, out: &mut Vec<u8>) {
        put_u64(out, self.seats() as u64);
        match self.seat_topology() {
            None => put_bool(out, false),
            Some(generation) => {
                put_bool(out, true);
                put_u64(out, generation);
            }
        }
        // The activation's IDENTITY travels with it. A rewind restores the
        // receipt, so it must restore WHICH match the receipt is for — the
        // whole point of the field is that activation compares it.
        match self.session() {
            None => put_bool(out, false),
            Some(session) => {
                put_bool(out, true);
                put_u64(out, session.0);
            }
        }
        match self.activated_on() {
            None => put_bool(out, false),
            Some(tick) => {
                put_bool(out, true);
                put_u64(out, tick);
            }
        }
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        let seats = r.u64()? as usize;
        let seat_topology = if r.bool()? { Some(r.u64()?) } else { None };
        let session = if r.bool()? {
            Some(ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId(r.u64()?))
        } else {
            None
        };
        let activated_on = if r.bool()? { Some(r.u64()?) } else { None };
        Some(crate::ActiveMatch::from_snapshot(
            seats,
            seat_topology,
            session,
            activated_on,
        ))
    }
}
