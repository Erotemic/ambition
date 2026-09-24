//! Rollback wire format for the match receipt and the per-body seat. The
//! orphan rule puts these beside their types: `SnapshotState` is core's.

use ambition_platformer2d_core::snapshot::{put_bool, put_str, put_u64, put_u8, Reader, SnapshotState};

// ── A live match's per-body state (AA2 / AC2) ────────────────────────────────
//
// Which seat a body is, its team, and who owns its death. All three are set at
// match activation and read by the rules every tick. A rewind across
// activation must restore them with the fighters. Guarded by
// `every_component_in_a_live_match_is_registered_derived_or_waived`.
impl SnapshotState for crate::MatchSeat {
    fn encode(&self, out: &mut Vec<u8>) {
        put_u64(out, self.0 as u64);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(crate::MatchSeat(r.u64()? as usize))
    }
}

/// The activation latch. Plain data with no identity (a seat count and the
/// frozen topology), so it can be snapshotted. The bodies derive from
/// `MatchSeat` and rewind on their own.
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
        // The activation's identity travels with it, so a rewind restores
        // which match the receipt is for.
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
        // The ordinal keys the draws. Without it, a rewind would restart the
        // match's item table.
        match self.ordinal() {
            None => put_bool(out, false),
            Some(ordinal) => {
                put_bool(out, true);
                put_u64(out, ordinal);
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
        let ordinal = if r.bool()? { Some(r.u64()?) } else { None };
        Some(crate::ActiveMatch::from_snapshot(
            seats,
            seat_topology,
            session,
            activated_on,
            ordinal,
        ))
    }
}

/// The ordinal mint rewinds because it is a counter: a resimulated
/// activation must draw the same ordinal, or the match re-rolls its item table
/// on every rollback.
impl SnapshotState for crate::seating::SessionMatchOrdinal {
    fn encode(&self, out: &mut Vec<u8>) {
        let (session, next) = self.parts();
        match session {
            None => put_bool(out, false),
            Some(session) => {
                put_bool(out, true);
                put_u64(out, session.0);
            }
        }
        put_u64(out, next);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        let session = if r.bool()? {
            Some(ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId(r.u64()?))
        } else {
            None
        };
        Some(crate::seating::SessionMatchOrdinal::from_snapshot(
            session,
            r.u64()?,
        ))
    }
}

/// Which match is in sudden death. Same shape and reason as the verdict below:
/// restoring one without the other would restore a state for a match that is
/// not running.
impl SnapshotState for crate::SuddenDeathEntered {
    fn encode(&self, out: &mut Vec<u8>) {
        match self.entered_match() {
            None => put_bool(out, false),
            Some(instance) => {
                put_bool(out, true);
                let (session, activated_on, ordinal) = instance.parts();
                match session {
                    None => put_bool(out, false),
                    Some(session) => {
                        put_bool(out, true);
                        put_u64(out, session.0);
                    }
                }
                match activated_on {
                    None => put_bool(out, false),
                    Some(tick) => {
                        put_bool(out, true);
                        put_u64(out, tick);
                    }
                }
                // The peer half travels too, so the value still names which
                // match of the agreed session it describes.
                match ordinal {
                    None => put_bool(out, false),
                    Some(ordinal) => {
                        put_bool(out, true);
                        put_u64(out, ordinal);
                    }
                }
            }
        }
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        let entered = if r.bool()? {
            let session = if r.bool()? {
                Some(ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId(r.u64()?))
            } else {
                None
            };
            let activated_on = if r.bool()? { Some(r.u64()?) } else { None };
            let ordinal = if r.bool()? { Some(r.u64()?) } else { None };
            Some(crate::MatchInstance::from_snapshot(
                session,
                activated_on,
                ordinal,
            ))
        } else {
            None
        };
        Some(crate::SuddenDeathEntered::from_snapshot(entered))
    }
}


/// The stocks ruleset's verdict, and which match it is about.
/// One byte of tag plus the winning side's label when there is one.
fn encode_match_verdict(out: &mut Vec<u8>, verdict: &ambition_combat::stocks::MatchVerdict) {
    use ambition_combat::stocks::MatchVerdict;
    match verdict {
        MatchVerdict::Winner(side) => {
            put_u8(out, 0);
            put_str(out, side);
        }
        MatchVerdict::Draw => put_u8(out, 1),
        MatchVerdict::NoContest => put_u8(out, 2),
    }
}

fn decode_match_verdict(r: &mut Reader<'_>) -> Option<ambition_combat::stocks::MatchVerdict> {
    use ambition_combat::stocks::MatchVerdict;
    match r.u8()? {
        0 => Some(MatchVerdict::Winner(r.str()?.to_string())),
        1 => Some(MatchVerdict::Draw),
        2 => Some(MatchVerdict::NoContest),
        _ => None,
    }
}

impl SnapshotState for crate::StocksMatchSettled {
    fn encode(&self, out: &mut Vec<u8>) {
        match self.decided_match() {
            None => put_bool(out, false),
            Some(instance) => {
                put_bool(out, true);
                let (session, activated_on, ordinal) = instance.parts();
                match session {
                    None => put_bool(out, false),
                    Some(session) => {
                        put_bool(out, true);
                        put_u64(out, session.0);
                    }
                }
                match activated_on {
                    None => put_bool(out, false),
                    Some(tick) => {
                        put_bool(out, true);
                        put_u64(out, tick);
                    }
                }
                // The peer half travels too, so the value still names which
                // match of the agreed session it describes.
                match ordinal {
                    None => put_bool(out, false),
                    Some(ordinal) => {
                        put_bool(out, true);
                        put_u64(out, ordinal);
                    }
                }
                // The verdict travels with its match, so a restore gives
                // presentation the same outcome.
                encode_match_verdict(
                    out,
                    self.decided_verdict()
                        .expect("a stamped match carries its verdict"),
                );
            }
        }
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        let decided = if r.bool()? {
            let session = if r.bool()? {
                Some(ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId(r.u64()?))
            } else {
                None
            };
            let activated_on = if r.bool()? { Some(r.u64()?) } else { None };
            let ordinal = if r.bool()? { Some(r.u64()?) } else { None };
            Some(crate::MatchInstance::from_snapshot(
                session,
                activated_on,
                ordinal,
            ))
        } else {
            None
        };
        // The verdict travels with its match. Presentation reads it as state,
        // not a message, so a speculative outcome cannot reach the winner
        // card. See `StocksMatchSettled::settle`.
        let decided = match decided {
            None => None,
            Some(instance) => Some((instance, decode_match_verdict(r)?)),
        };
        Some(crate::StocksMatchSettled::from_snapshot(decided))
    }
}
