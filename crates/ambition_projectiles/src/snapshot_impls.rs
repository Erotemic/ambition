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
    put_bool, put_f32, put_str, put_u32, put_u64, put_u8,
    Reader, SnapshotState,
};
use ambition_platformer2d_core::{snapshot_marker, snapshot_unit_enum};

snapshot_unit_enum!(crate::ProjectileKind {
    Fireball = 0,
    Hadouken = 1,
    HadoukenSuper = 2,
});

impl SnapshotState for crate::ProjectileSeq {
    fn encode(&self, out: &mut Vec<u8>) {
        put_u64(out, self.0);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(Self(r.u64()?))
    }
}

impl SnapshotState for crate::ProjectileVisualId {
    fn encode(&self, out: &mut Vec<u8>) {
        put_str(out, &self.0);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(Self(r.str()?.to_string()))
    }
}

snapshot_marker!(crate::LiveProjectile);


/// The global spawn-order stamp source. Two sims that stamped a different
/// number of projectiles are not in the same state; a restore that left the
/// counter at the abandoned future's value would stamp the replay's shots with
/// different orderings than the original run's.
impl SnapshotState for crate::ProjectileSeqCounter {
    fn encode(&self, out: &mut Vec<u8>) {
        put_u64(out, self.0);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(Self(r.u64()?))
    }
}

impl SnapshotState for crate::PlayerProjectileState {
    fn encode(&self, out: &mut Vec<u8>) {
        let meter = self.spawner.meter;
        put_f32(out, meter.current);
        put_f32(out, meter.max);
        put_f32(out, meter.regen_rate);
        put_f32(out, meter.decay_rate);
        put_f32(out, self.spawner.cooldown_remaining);

        put_u32(out, self.motion_buffer.samples.len() as u32);
        for sample in &self.motion_buffer.samples {
            put_motion_direction(out, sample.dir);
            put_f32(out, sample.time);
        }
        put_f32(out, self.motion_buffer.window);
        put_f32(out, self.clock);
        put_bool(out, self.unlocked.fireball);
        put_bool(out, self.unlocked.hadouken);
        put_bool(out, self.unlocked.hadouken_super);
        put_f32(out, self.charge_tuning.medium_after);
        put_f32(out, self.charge_tuning.heavy_after);
        put_bool(out, self.charging.is_some());
        if let Some(charging) = self.charging {
            put_f32(out, charging);
        }
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        use std::collections::VecDeque;

        let meter = ambition_platformer2d_core::ResourceMeter {
            current: r.f32()?,
            max: r.f32()?,
            regen_rate: r.f32()?,
            decay_rate: r.f32()?,
        };
        let cooldown_remaining = r.f32()?;
        let sample_count = r.u32()? as usize;
        let mut samples = VecDeque::with_capacity(sample_count);
        for _ in 0..sample_count {
            samples.push_back(crate::MotionSample {
                dir: read_motion_direction(r)?,
                time: r.f32()?,
            });
        }
        let window = r.f32()?;
        let clock = r.f32()?;
        let unlocked = crate::state::ProjectileUnlocks {
            fireball: r.bool()?,
            hadouken: r.bool()?,
            hadouken_super: r.bool()?,
        };
        let charge_tuning = crate::FireballChargeTuning {
            medium_after: r.f32()?,
            heavy_after: r.f32()?,
        };
        let charging = if r.bool()? { Some(r.f32()?) } else { None };

        Some(crate::PlayerProjectileState {
            spawner: crate::ProjectileSpawner {
                meter,
                cooldown_remaining,
            },
            motion_buffer: crate::MotionInputBuffer { samples, window },
            clock,
            unlocked,
            charge_tuning,
            charging,
        })
    }
}


fn put_motion_direction(out: &mut Vec<u8>, value: crate::MotionDirection) {
    use crate::MotionDirection::*;
    put_u8(
        out,
        match value {
            Neutral => 0,
            Up => 1,
            Down => 2,
            Left => 3,
            Right => 4,
            UpLeft => 5,
            UpRight => 6,
            DownLeft => 7,
            DownRight => 8,
        },
    );
}

fn read_motion_direction(r: &mut Reader<'_>) -> Option<crate::MotionDirection> {
    use crate::MotionDirection::*;
    Some(match r.u8()? {
        0 => Neutral,
        1 => Up,
        2 => Down,
        3 => Left,
        4 => Right,
        5 => UpLeft,
        6 => UpRight,
        7 => DownLeft,
        8 => DownRight,
        _ => return None,
    })
}
