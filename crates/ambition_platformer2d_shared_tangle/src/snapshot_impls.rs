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
    put_f32, put_i32, put_str, put_u64, put_u8, put_vec2, Reader, SnapshotState,
};
use ambition_platformer2d_core::{snapshot_pod, snapshot_unit_enum};

impl SnapshotState for crate::lifecycle::RoomScopedEntity {
    fn encode(&self, _out: &mut Vec<u8>) {}

    fn decode(_r: &mut Reader<'_>) -> Option<Self> {
        Some(Self)
    }
}

impl SnapshotState for crate::lifecycle::SessionScopedEntity {
    fn encode(&self, out: &mut Vec<u8>) {
        put_u64(out, self.0 .0);
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(Self(crate::lifecycle::SessionScopeId(r.u64()?)))
    }
}

snapshot_pod!(crate::orientation::ActorRoll { angle: f32 });

impl SnapshotState for crate::sim_id::SimId {
    fn encode(&self, out: &mut Vec<u8>) {
        put_str(out, self.as_str());
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(crate::sim_id::SimId::from_snapshot(r.str()?.to_string()))
    }
}

/// Provenance is snapshot state, not derived state.
///
/// This is the durable fact that replaced splitting a `/`-delimited parent out of the entity's
/// own `SimId`.
impl SnapshotState for crate::construction::TransactionId {
    fn encode(&self, out: &mut Vec<u8>) {
        put_str(out, self.as_str());
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(Self::from_raw(r.str()?.to_owned()))
    }
}

impl SnapshotState for crate::construction::SpawnOrigin {
    fn encode(&self, out: &mut Vec<u8>) {
        use crate::construction::SpawnOrigin as O;
        match self {
            O::Authored { source, instance } => {
                put_u8(out, 0);
                put_str(out, source);
                put_str(out, instance);
            }
            O::ProviderStaged {
                provider,
                room,
                instance,
            } => {
                put_u8(out, 1);
                put_str(out, provider);
                put_str(out, room);
                put_str(out, instance);
            }
            O::Dynamic { parent, sequence } => {
                put_u8(out, 2);
                put_str(out, parent.as_str());
                put_u64(out, *sequence);
            }
        }
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        use crate::construction::SpawnOrigin as O;
        use crate::sim_id::SimId;
        Some(match r.u8()? {
            0 => O::Authored {
                source: r.str()?.to_string(),
                instance: r.str()?.to_string(),
            },
            1 => O::ProviderStaged {
                provider: r.str()?.to_string(),
                room: r.str()?.to_string(),
                instance: r.str()?.to_string(),
            },
            2 => O::Dynamic {
                parent: SimId::from_snapshot(r.str()?.to_string()),
                sequence: r.u64()?,
            },
            _ => return None,
        })
    }
}

impl SnapshotState for crate::sim_id::SimIdCounter {
    fn encode(&self, out: &mut Vec<u8>) {
        put_u64(out, self.0);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(crate::sim_id::SimIdCounter(r.u64()?))
    }
}

snapshot_unit_enum!(crate::projectile::WorldHitPolicy {
    Bouncing = 0,
    ExpireOnContact = 1,
});

impl SnapshotState for crate::projectile::ProjectileGameplay {
    fn encode(&self, out: &mut Vec<u8>) {
        put_f32(out, self.age);
        put_f32(out, self.max_lifetime);
        put_f32(out, self.gravity);
        put_i32(out, self.damage);
        put_u8(out, self.bounces_remaining);
        self.world_hit.encode(out);
        // A boomerang's whole trajectory is this vector; a peer that decoded a
        // shot without it would fly the outbound leg forever.
        put_vec2(out, self.accel);
        // Which leg the victim ledger belongs to. Without it a rewind across the
        // turnaround either re-arms the outbound leg's victims a second time or
        // never arms the return leg's at all.
        put_u8(out, self.hits_cleared_on_leg);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(Self {
            age: r.f32()?,
            max_lifetime: r.f32()?,
            gravity: r.f32()?,
            damage: r.i32()?,
            bounces_remaining: r.u8()?,
            world_hit: crate::projectile::WorldHitPolicy::decode(r)?,
            accel: r.vec2()?,
            hits_cleared_on_leg: r.u8()?,
        })
    }
}

impl SnapshotState for crate::time::SimDt {
    fn encode(&self, out: &mut Vec<u8>) {
        put_f32(out, self.dt);
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(Self { dt: r.f32()? })
    }
}

impl SnapshotState for crate::gravity::BaseGravity {
    fn encode(&self, out: &mut Vec<u8>) {
        put_vec2(out, self.dir);
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(Self { dir: r.vec2()? })
    }
}

impl SnapshotState for crate::gravity::GravityField {
    fn encode(&self, out: &mut Vec<u8>) {
        put_vec2(out, self.dir);
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(Self { dir: r.vec2()? })
    }
}

/// Temporary-control state: whether an autonomous body is masked by a player
/// possession or a mount, by STABLE `SimId`. Registered so a rewind restores the
/// control MODE across time (not just avoids clobbering a live one): the `Brain`
/// cursor is a no-op for a body nobody drives, and possession/mount relationships were
/// re-derived from live components, so without this a rollback across a
/// possess/release boundary left the body in the wrong mode. Reconciliation
/// rebuilds the live control (`DrivingParticipant` / `Mounted`) and its relationships
/// from the restored id.
impl SnapshotState for crate::temporary_control::TemporaryControl {
    fn encode(&self, out: &mut Vec<u8>) {
        use crate::temporary_control::TemporaryControl as T;
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
        use crate::sim_id::SimId;
        use crate::temporary_control::TemporaryControl as T;
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

// ⛔ THE ORPHAN RULE MOVED THESE TWO HERE, 2026-08-26, and that is the compiler
// doing the adjudication rather than a judgement call. `safe_position`'s types
// left the actor monolith; the `impl SnapshotState` blocks that stayed behind
// stopped compiling the moment they did, which is exactly what this file's own
// header promises.
impl SnapshotState for crate::safe_position::PlayerSafetyState {
    fn encode(&self, out: &mut Vec<u8>) {
        put_vec2(out, self.last_safe_pos);
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(Self::new(r.vec2()?))
    }
}

impl SnapshotState for crate::safe_position::RoomTransitionCooldown {
    fn encode(&self, out: &mut Vec<u8>) {
        put_f32(out, self.remaining);
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(Self {
            remaining: r.f32()?,
        })
    }
}
