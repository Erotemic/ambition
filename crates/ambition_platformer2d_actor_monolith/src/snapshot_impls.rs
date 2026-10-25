//! `SnapshotState` for this crate's own types — the rollback wire format.
//!
//! These impls live HERE, beside the types they encode, because
//! `ambition_platformer2d_core::snapshot` owns the trait and the orphan rule binds an impl to the
//! crate owning the trait OR the type. The orphan rule is what proves this file is in the right
//! crate: if a type moves, this stops compiling rather than drifting.
//!
//! A field added to an encoded type is a WIRE FORMAT change. Encode and
//! decode must stay in the same order, and `snapshot_unit_enum!` codes are
//! authored per variant so inserting one never renumbers the rest.

use ambition_platformer2d_core::snapshot::{
    put_bool, put_f32, put_str, put_u64, Reader, SnapshotState,
};

/// HOW LONG THIS MATCH HAS BEEN FOUGHT, and WHICH MATCH that is.
///
/// ⛔⛔ THIS ONE IS COUNTED, which is what makes it different from the two
/// beside it. `time_remaining` used to be a pure function of `(activated_on,
/// now)` — its own doc says a rewind RECOMPUTES the clock and a match clock
/// costs no wire format. Excluding pauses ends that: "how long was this stopped"
/// is written nowhere else, so the count is the only record and a rewind must
/// restore it. The bytes are the price of the mechanic.
impl SnapshotState for crate::character_runtime::live_match_clock::LiveMatchTicks {
    fn encode(&self, out: &mut Vec<u8>) {
        let (of, ticks) = self.parts();
        match of {
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
                // ⭐ THE PEER HALF TRAVELS TOO. A rewind that restored the local
                // stamp without it would restore a value that no longer names
                // which match of the agreed session it describes.
                match ordinal {
                    None => put_bool(out, false),
                    Some(ordinal) => {
                        put_bool(out, true);
                        put_u64(out, ordinal);
                    }
                }
            }
        }
        put_u64(out, ticks);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        let of = if r.bool()? {
            let session = if r.bool()? {
                Some(ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId(r.u64()?))
            } else {
                None
            };
            let activated_on = if r.bool()? { Some(r.u64()?) } else { None };
            let ordinal = if r.bool()? { Some(r.u64()?) } else { None };
            Some(ambition_match::MatchInstance::from_snapshot(
                session,
                activated_on,
                ordinal,
            ))
        } else {
            None
        };
        Some(
            crate::character_runtime::live_match_clock::LiveMatchTicks::from_snapshot(of, r.u64()?),
        )
    }
}

/// An accumulating sim clock, and netcode.md's N3.1 checklist names it: *"`WorldTime`
/// + every sim clock"*. A brain stamps `RememberedActor.last_seen` with it, so a rewind
/// that leaves it running makes every memory look older than it is — which is exactly
/// how `gnu_ton_arena` diverged on `perception_memory` and nothing else.
impl SnapshotState for crate::features::GameplayElapsed {
    fn encode(&self, out: &mut Vec<u8>) {
        put_f32(out, self.0);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(crate::features::GameplayElapsed(r.f32()?))
    }
}

/// A shot's own side of the fight.
///
/// The stamp is taken from the firer the first tick the bolt flies; after the firer is gone
/// there is nothing left to re-derive it from, so a rewind that dropped it would restore a shot
/// that had forgotten whose attack it is — indiscriminate, hitting its own team, which is the
/// state closed.
impl SnapshotState for crate::projectile::ProjectileAllegiance {
    fn encode(&self, out: &mut Vec<u8>) {
        self.faction.encode(out);
        match &self.team {
            None => put_bool(out, false),
            Some(team) => {
                put_bool(out, true);
                put_str(out, team.as_str());
            }
        }
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        let faction = ambition_characters::actor::ActorFaction::decode(r)?;
        let team = if r.bool()? {
            Some(ambition_combat::targeting::MatchTeam::new(r.str()?))
        } else {
            None
        };
        Some(Self { faction, team })
    }
}

