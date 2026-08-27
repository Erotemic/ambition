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
    put_bool, put_f32, put_i32, put_str, put_u32, put_u64, put_u8, put_vec2, Reader,
    SnapshotCursor, SnapshotState,
};

// ── A live MATCH's per-body state (AA2 / AC2) ────────────────────────────────
//
// Which seat a body is, which team it fights for, and who owns its death. All
// three are decided at match activation, all three are read by the rules every
// tick, and none of them was rollback state — because no swept population had
// a match in it until `every_component_in_a_live_match_is_registered_derived_or_waived`
// existed. A rewind across activation restored the fighters and left these
// behind, which is a body that comes back with no seat, no team, and the
// exploration death policy in the middle of a round.
impl SnapshotState for crate::character_runtime::MatchSeat {
    fn encode(&self, out: &mut Vec<u8>) {
        put_u64(out, self.0 as u64);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(crate::character_runtime::MatchSeat(r.u64()? as usize))
    }
}

/// The activation latch itself. Plain data with no identity in it — a seat count
/// and the frozen topology that decided it — which is what makes it snapshotable
/// at all: the BODIES are derived from `MatchSeat` and rewind on their own.
impl SnapshotState for crate::character_runtime::ActiveMatch {
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
        Some(crate::character_runtime::ActiveMatch::from_snapshot(
            seats,
            seat_topology,
            session,
            activated_on,
        ))
    }
}

/// WHICH MATCH is in sudden death — the same shape as the verdict below, and
/// registered for the same reason: a rewind that restored one and not the other
/// would restore a continuation belonging to a match that is not running.
impl SnapshotState for crate::features::stocks_match::SuddenDeathEntered {
    fn encode(&self, out: &mut Vec<u8>) {
        match self.entered_match() {
            None => put_bool(out, false),
            Some(instance) => {
                put_bool(out, true);
                let (session, activated_on) = instance.parts();
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
            Some(crate::character_runtime::MatchInstance::from_snapshot(
                session,
                activated_on,
            ))
        } else {
            None
        };
        Some(crate::features::stocks_match::SuddenDeathEntered::from_snapshot(entered))
    }
}

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
                let (session, activated_on) = instance.parts();
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
            Some(crate::character_runtime::MatchInstance::from_snapshot(
                session,
                activated_on,
            ))
        } else {
            None
        };
        Some(
            crate::character_runtime::live_match_clock::LiveMatchTicks::from_snapshot(of, r.u64()?),
        )
    }
}

/// The stocks ruleset's verdict, and WHICH MATCH it is about.
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

impl SnapshotState for crate::features::stocks_match::StocksMatchSettled {
    fn encode(&self, out: &mut Vec<u8>) {
        match self.decided_match() {
            None => put_bool(out, false),
            Some(instance) => {
                put_bool(out, true);
                let (session, activated_on) = instance.parts();
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
                // The VERDICT rides beside the match it is about, so a restore
                // hands presentation the same outcome it was showing.
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
            Some(crate::character_runtime::MatchInstance::from_snapshot(
                session,
                activated_on,
            ))
        } else {
            None
        };
        // The VERDICT rides beside the match it is about — presentation reads
        // it as state rather than as a message, so a speculative outcome cannot
        // reach the winner card. See `StocksMatchSettled::settle`.
        let decided = match decided {
            None => None,
            Some(instance) => Some((instance, decode_match_verdict(r)?)),
        };
        Some(crate::features::stocks_match::StocksMatchSettled::from_snapshot(decided))
    }
}

// The orphan rule forced it the moment the type moved down: `SnapshotState` is core's and the type
// is core's, so this crate may implement neither. Same shape as `BossEncounter` immediately below.

impl SnapshotCursor for crate::features::ActorMotionPath {
    fn encode_cursor(&self, out: &mut Vec<u8>) {
        match &self.0 {
            Some(motion) => {
                let (segment, dir) = motion.cursor();
                put_bool(out, true);
                put_u32(out, segment as u32);
                put_i32(out, dir);
            }
            // A body with no path is a state a body with a path can reach.
            None => put_bool(out, false),
        }
    }
}

/// `Omniscient` reads the global `ActorTarget`; `Sighted` carries its viewport. Not a
/// unit enum, so `snapshot_unit_enum!` cannot have it — but the discriminant is still
/// explicit for exactly the same reason.
impl SnapshotState for crate::features::ecs::perception::Perception {
    fn encode(&self, out: &mut Vec<u8>) {
        use crate::features::ecs::perception::Perception as P;
        match self {
            P::Omniscient => put_u8(out, 0),
            P::Sighted { viewport_half } => {
                put_u8(out, 1);
                put_vec2(out, *viewport_half);
            }
        }
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        use crate::features::ecs::perception::Perception as P;
        match r.u8()? {
            0 => Some(P::Omniscient),
            1 => Some(P::Sighted {
                viewport_half: r.vec2()?,
            }),
            _ => None,
        }
    }
}

/// The brain's memory of what it has seen — FB5's habit model reads it, and FB6's rollouts
/// cannot run until it rewinds.
impl SnapshotState for crate::features::ecs::perception::PerceptionMemory {
    fn encode(&self, out: &mut Vec<u8>) {
        let rows: Vec<_> = self.0.entries().collect();
        put_u32(out, rows.len() as u32);
        for (id, m) in rows {
            put_str(out, id);
            put_vec2(out, m.pos);
            put_vec2(out, m.vel);
            m.faction.encode(out);
            put_bool(out, m.hostile_to_self);
            put_f32(out, m.last_seen);
            put_f32(out, m.confidence);
        }
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        use ambition_characters::perception::{RememberedActor, WorldMemory};
        let n = r.u32()?;
        let mut rows = Vec::with_capacity(n as usize);
        for _ in 0..n {
            let id = r.str()?.to_string();
            rows.push((
                id,
                RememberedActor {
                    pos: r.vec2()?,
                    vel: r.vec2()?,
                    faction: ambition_characters::actor::ActorFaction::decode(r)?,
                    hostile_to_self: r.bool()?,
                    last_seen: r.f32()?,
                    confidence: r.f32()?,
                },
            ));
        }
        Some(crate::features::ecs::perception::PerceptionMemory(
            WorldMemory::from_snapshot(rows),
        ))
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

impl SnapshotState for crate::session::reset::NewGameResetRequested {
    fn encode(&self, out: &mut Vec<u8>) {
        put_bool(out, self.request);
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(Self { request: r.bool()? })
    }
}

impl SnapshotState for crate::session::lifecycle_commit::PendingLifecycleCommit {
    fn encode(&self, out: &mut Vec<u8>) {
        use crate::session::lifecycle_commit::LifecycleIntent;
        match &self.pending {
            None => put_bool(out, false),
            Some(intent) => {
                put_bool(out, true);
                put_i32(out, intent.frame);
                match &intent.kind {
                    LifecycleIntent::DeathReset => put_u8(out, 0),
                    LifecycleIntent::ManualReset => put_u8(out, 1),
                    LifecycleIntent::Replay => put_u8(out, 2),
                    LifecycleIntent::Transition(
                        crate::session::lifecycle_commit::RoomTransitionIntent {
                            subject,
                            target_room,
                            arrival,
                            edge_exit,
                            zone_sfx,
                        },
                    ) => {
                        put_u8(out, 3);
                        put_str(out, subject.as_str());
                        put_str(out, target_room);
                        put_vec2(out, *arrival);
                        put_bool(out, *edge_exit);
                        put_bool(out, zone_sfx.is_some());
                        put_str(out, zone_sfx.as_deref().unwrap_or(""));
                    }
                    LifecycleIntent::FullReset => put_u8(out, 4),
                }
            }
        }
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        use crate::session::lifecycle_commit::{
            LifecycleIntent, PendingIntent, PendingLifecycleCommit,
        };
        if !r.bool()? {
            return Some(PendingLifecycleCommit { pending: None });
        }
        let frame = r.i32()?;
        let kind = match r.u8()? {
            0 => LifecycleIntent::DeathReset,
            1 => LifecycleIntent::ManualReset,
            2 => LifecycleIntent::Replay,
            3 => LifecycleIntent::Transition(
                crate::session::lifecycle_commit::RoomTransitionIntent {
                    subject: ambition_platformer2d_shared_tangle::sim_id::SimId::from_snapshot(
                        r.str()?.to_string(),
                    ),
                    target_room: r.str()?.to_string(),
                    arrival: r.vec2()?,
                    edge_exit: r.bool()?,
                    zone_sfx: {
                        let present = r.bool()?;
                        let cue = r.str()?.to_string();
                        present.then_some(cue)
                    },
                },
            ),
            4 => LifecycleIntent::FullReset,
            _ => return None,
        };
        Some(PendingLifecycleCommit {
            pending: Some(PendingIntent { frame, kind }),
        })
    }
}
