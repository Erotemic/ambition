//! `SnapshotState` for this crate's own types — the rollback wire format.
//!
//! ⚠ These impls live HERE, beside the types they encode, because
//! `ambition_platformer2d_core::snapshot` owns the trait and the orphan rule binds an
//! impl to the crate owning the trait OR the type. Until 2026-07-30 the trait
//! sat in `ambition_platformer2d_runtime`, above every domain crate, so the only place all
//! ~100 of them could compile was one 2688-line file in `ambition_platformer2d_runtime`. The
//! orphan rule is what proves this file is in the right crate: if a type moves,
//! this stops compiling rather than drifting.
//!
//! ⚠ A field added to an encoded type is a WIRE FORMAT change. Encode and
//! decode must stay in the same order, and `snapshot_unit_enum!` codes are
//! authored per variant so inserting one never renumbers the rest.

use ambition_platformer2d_core::snapshot::{
    put_bool, put_f32, put_i32, put_str, put_u32, put_u64, put_u8, put_vec2, Reader,
    SnapshotCursor, SnapshotState,
};
use ambition_platformer2d_core::{snapshot_pod, snapshot_unit_enum};

impl SnapshotState for crate::features::ActorStatus {
    fn encode(&self, out: &mut Vec<u8>) {
        put_f32(out, self.respawn_timer);
        self.ai_mode.encode(out);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(crate::features::ActorStatus {
            respawn_timer: r.f32()?,
            ai_mode: ambition_characters::actor::ai::CharacterAiMode::decode(r)?,
        })
    }
}

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
        // ⛔ **WHEN the cast was built travels too, and leaving it out would be
        // a rollback bug with a visible symptom.** The opening ceremony is
        // derived as `now - activated_on`, so a rewind that restored the receipt
        // without the stamp would restart the countdown from whatever tick the
        // rewind landed on — the cast held again, mid-match, for three beats.
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

/// **The stocks ruleset's verdict, and WHICH MATCH it is about** (D147).
///
/// ⚠ the stamp travels, and leaving it out would put the D140 defect back: a
/// rewind that restored "decided" without restoring which match it was decided
/// for would be restoring a process-global bool again.
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
        Some(crate::features::stocks_match::StocksMatchSettled::from_snapshot(decided))
    }
}

snapshot_pod!(crate::features::ActorSurfaceState {
    surface_normal: vec2,
    gravity_scale: f32,
});

/// **The boss's encounter phase**, and the `ActorPhaseState` it is forwarded from.
///
/// A cursor, because the rest of `BossEncounter` is sprite metrics derived from the
/// sheet registry, and because `ActorPhaseState.triggers` is authored data.
///
/// `encounter_phase` is the exposed MIRROR that `sync_boss_encounter_phase` copies out
/// of `encounter` every tick. Rewinding only the mirror is rewinding a thermometer:
/// `mockingbird_arena` telegraphed `wing_sweep` on the replay's tick 21 and stood still
/// on the original's, with every clock, seed, and cooldown identical, because the
/// replay's boss was already awake.
impl SnapshotCursor for crate::boss_encounter::BossEncounter {
    fn encode_cursor(&self, out: &mut Vec<u8>) {
        self.encounter_phase.encode(out);
        match &self.encounter {
            None => put_bool(out, false),
            Some(e) => {
                put_bool(out, true);
                e.phase.encode(out);
                put_f32(out, e.phase_elapsed);
                put_f32(out, e.transition_lock);
                e.start_phase.encode(out);
            }
        }
    }
}

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

/// The brain's memory of what it has seen — FB5's habit model reads it, and FB6's
/// rollouts cannot run until it rewinds. Ordered by actor id, because `WorldMemory`
/// is a `BTreeMap` (ADR 0023, and a real bug: see `last_known_hostile`).
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

/// **Temporary-control state**: whether an autonomous body is masked by a player
/// possession or a mount, by STABLE `SimId`. Registered so a rewind restores the
/// control MODE across time (not just avoids clobbering a live one): the `Brain`
/// cursor is a no-op for `Brain::Player`, and possession/mount relationships were
/// re-derived from live components, so without this a rollback across a
/// possess/release boundary left the body in the wrong mode. Reconciliation
/// rebuilds the live control (`Brain::Player` / `Mounted`) and its relationships
/// from the restored id.
impl SnapshotState for crate::features::TemporaryControl {
    fn encode(&self, out: &mut Vec<u8>) {
        use crate::features::TemporaryControl as T;
        match self {
            T::Autonomous => put_u8(out, 0),
            T::Player { controller } => {
                put_u8(out, 1);
                put_str(out, controller.as_str());
            }
            T::Mounted { mount } => {
                put_u8(out, 2);
                put_str(out, mount.as_str());
            }
        }
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        use crate::features::TemporaryControl as T;
        use ambition_platformer2d_shared_tangle::sim_id::SimId;
        Some(match r.u8()? {
            0 => T::Autonomous,
            1 => T::Player {
                controller: SimId::from_snapshot(r.str()?.to_string()),
            },
            2 => T::Mounted {
                mount: SimId::from_snapshot(r.str()?.to_string()),
            },
            _ => return None,
        })
    }
}

/// **An accumulating sim clock**, and netcode.md's N3.1 checklist names it: *"`WorldTime`
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

impl SnapshotState for crate::avatar::PlayerSafetyState {
    fn encode(&self, out: &mut Vec<u8>) {
        put_vec2(out, self.last_safe_pos);
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(Self::new(r.vec2()?))
    }
}

impl SnapshotState for crate::time::time_control::RequestedClockScale {
    fn encode(&self, out: &mut Vec<u8>) {
        put_f32(out, self.sim_clock);
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(Self {
            sim_clock: r.f32()?,
        })
    }
}

snapshot_unit_enum!(crate::time::time_control::Regime {
    Solo = 0,
    RLDeterministic = 1,
    Cinematic = 2,
});

impl SnapshotState for crate::time::time_control::RegimePolicy {
    fn encode(&self, out: &mut Vec<u8>) {
        self.regime.encode(out);
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(Self {
            regime: crate::time::time_control::Regime::decode(r)?,
        })
    }
}

impl SnapshotState for crate::control::SlotInteractionState {
    fn encode(&self, out: &mut Vec<u8>) {
        for index in 0..ambition_characters::brain::SlotControls::MAX_SLOTS {
            let gestures = self.get(ambition_characters::brain::PlayerSlot(index as u8));
            put_f32(out, gestures.down_tap_timer);
            put_f32(out, gestures.up_tap_timer);
            put_f32(out, gestures.interact_buffer_timer);
            put_bool(out, gestures.double_tap_down_pending);
            put_bool(out, gestures.double_tap_up_pending);
        }
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        let mut state = Self::default();
        for index in 0..ambition_characters::brain::SlotControls::MAX_SLOTS {
            *state.get_mut(ambition_characters::brain::PlayerSlot(index as u8)) =
                crate::control::SlotGestures {
                    down_tap_timer: r.f32()?,
                    up_tap_timer: r.f32()?,
                    interact_buffer_timer: r.f32()?,
                    double_tap_down_pending: r.bool()?,
                    double_tap_up_pending: r.bool()?,
                };
        }
        Some(state)
    }
}

/// **A shot's own side of the fight** (D150).
///
/// ⚠ it is state, not a memo, and the difference is exactly the bug it fixes.
/// The stamp is taken from the firer the first tick the bolt flies; after the
/// firer is gone there is nothing left to re-derive it from, so a rewind that
/// dropped it would restore a shot that had forgotten whose attack it is —
/// indiscriminate, hitting its own team, which is the state D150 closed.
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

impl SnapshotState for crate::RoomTransitionCooldown {
    fn encode(&self, out: &mut Vec<u8>) {
        put_f32(out, self.remaining);
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(Self {
            remaining: r.f32()?,
        })
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
