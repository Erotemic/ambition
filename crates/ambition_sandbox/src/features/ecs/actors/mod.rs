//! ECS actor types and the per-frame actor tick.
//!
//! `ActorRuntime` is the unified component that backs every authored
//! NPC, authored hostile enemy, and dynamic encounter-spawned mob.
//! Peaceful and hostile actors share the same entity identity so a
//! peaceful NPC can flip to hostile in place after enough strikes
//! rather than being moved between containers.

use super::*;

fn shark_charge_crashed(
    em: &super::enemy_clusters::EnemyMut<'_>,
    is_mounted: bool,
    charge_vec: ae::Vec2,
    previous_pos: ae::Vec2,
) -> bool {
    shark_charge_crashed_parts(
        em.caps,
        em.status.alive,
        em.kin.pos,
        em.kin.vel,
        em.config.tuning.chase_speed,
        is_mounted,
        charge_vec,
        previous_pos,
    )
}

#[allow(clippy::too_many_arguments)]
fn shark_charge_crashed_parts(
    caps: &crate::mechanics::combat::CombatCapabilities,
    alive: bool,
    pos: ae::Vec2,
    vel: ae::Vec2,
    chase_speed: f32,
    is_mounted: bool,
    charge_vec: ae::Vec2,
    previous_pos: ae::Vec2,
) -> bool {
    !is_mounted
        && caps.charge_crash_explodes
        && alive
        && shark_charge_crashed_geometry(charge_vec, pos, previous_pos, vel, chase_speed)
}

/// True when a fast shark charge along EITHER axis was stopped dead by a wall:
/// the charge speed was high on that axis, yet the body neither moved nor kept
/// any velocity on it. Per-axis so a shark that charges UP into a ceiling (or
/// down into a floor) explodes just like one that rams a side wall — the riderless
/// shark flies vertically + horizontally and crashes on any of them (#98).
fn shark_charge_crashed_geometry(
    charge_vec: ae::Vec2,
    pos: ae::Vec2,
    prev_pos: ae::Vec2,
    vel: ae::Vec2,
    chase_speed: f32,
) -> bool {
    let crashed = |cv: f32, p: f32, pp: f32, v: f32| {
        cv.abs() > chase_speed * 1.5 && (p - pp).abs() < 0.01 && v.abs() < 0.01
    };
    crashed(charge_vec.x, pos.x, prev_pos.x, vel.x)
        || crashed(charge_vec.y, pos.y, prev_pos.y, vel.y)
}

/// Marker for an actor entity. Both variants are payload-free: NPC and
/// enemy state live in ECS cluster components. The enum is only the
/// peaceful-vs-hostile disposition tag, so an NPC can flip to `Enemy`
/// in place when aggression provokes it.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActorRuntime {
    Npc,
    Enemy,
}

impl ActorRuntime {
    pub fn disposition(&self) -> ActorDisposition {
        match self {
            Self::Npc => ActorDisposition::Peaceful,
            Self::Enemy => ActorDisposition::Hostile,
        }
    }
}

mod conversion;
mod update;
pub use conversion::*;
pub use update::*;

#[cfg(test)]
mod tests;

