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

use ambition_platformer2d_core::snapshot::{put_f32, put_u64, Reader, SnapshotState};
use ambition_platformer2d_core::snapshot_unit_enum;

impl SnapshotState for crate::SimTick {
    fn encode(&self, out: &mut Vec<u8>) {
        put_u64(out, self.0);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(crate::SimTick(r.u64()?))
    }
}

impl SnapshotState for crate::WorldTime {
    fn encode(&self, out: &mut Vec<u8>) {
        put_f32(out, self.raw_dt);
        put_f32(out, self.scaled_dt);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(crate::WorldTime {
            raw_dt: r.f32()?,
            scaled_dt: r.f32()?,
        })
    }
}

/// A body's proper-time dilation (ADR 0011): hitstop, bullet-time, a boss's slow.
/// Every move clock and every brain timer advances on `world_time.entity_dt(scale)`, so
/// a stale scale makes a rewound body live in a differently-paced universe.
impl SnapshotState for crate::ProperTimeScale {
    fn encode(&self, out: &mut Vec<u8>) {
        put_f32(out, self.0);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(crate::ProperTimeScale(r.f32()?))
    }
}

impl SnapshotState for crate::ClockState {
    fn encode(&self, out: &mut Vec<u8>) {
        put_f32(out, self.time_scale);
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(Self {
            time_scale: r.f32()?,
        })
    }
}
impl SnapshotState for crate::time_control::RequestedClockScale {
    fn encode(&self, out: &mut Vec<u8>) {
        put_f32(out, self.sim_clock);
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(Self {
            sim_clock: r.f32()?,
        })
    }
}

snapshot_unit_enum!(crate::time_control::Regime {
    Solo = 0,
    RLDeterministic = 1,
    Cinematic = 2,
});

impl SnapshotState for crate::time_control::RegimePolicy {
    fn encode(&self, out: &mut Vec<u8>) {
        self.regime.encode(out);
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(Self {
            regime: crate::time_control::Regime::decode(r)?,
        })
    }
}
