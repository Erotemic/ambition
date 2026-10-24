//! The tell of a boss whose hull turns blows: its guard glints.
//!
//! A boss with its guard up (`BossEncounter::guarded`, the Mockingbird out at
//! its side) turns every blow and shot (`apply_boss_hit`). The player sees it
//! by the glint that runs over its hull at a steady beat while the guard is up.
//! With the guard down (winded, diving, flying home) there is no glint: its
//! blows land.
//!
//! The beat is the sim's own tick ([`ambition_time::SimTick`]), so a rewind
//! that restores the tick draws the same glints, and the system keeps no state.

use bevy::prelude::*;

use ambition_combat::components::DamageableVolumes;
use ambition_combat::CenteredAabb;
use ambition_vfx::vfx::FxRequest;

use crate::{BossConfig, BossEncounter};

/// The guard's glint (an authored row of the `projectile_outcomes` sheet).
pub const GUARD_GLINT_FX: &str = "boss_guard_glint";

/// Ticks between two glints: one each 0.75 s at 60 ticks a second, a little
/// longer than the glint row, so the hull is not always lit.
const GLINT_EVERY_TICKS: u64 = 45;

/// Write a glint over the hull of each boss whose guard is up, once each beat.
pub fn boss_guard_tell(
    tick: Res<ambition_time::SimTick>,
    bosses: Query<(Entity, &BossConfig, &BossEncounter, &CenteredAabb, Option<&DamageableVolumes>)>,
    rooms: ambition_platformer2d_shared_tangle::lifecycle::LiveRooms,
    mut fx: MessageWriter<FxRequest>,
) {
    if tick.get() % GLINT_EVERY_TICKS != 0 {
        return;
    }
    for (entity, config, status, aabb, volumes) in &bosses {
        if !(status.guarded || config.behavior.environmental_kill_only) {
            continue;
        }
        // Over what a blow would strike: its published parts, else its box.
        let hull = volumes
            .and_then(|volumes| {
                volumes.volumes.iter().map(|part| part.bounds()).reduce(|a, b| ambition_platformer2d_core::Aabb {
                    min: a.min.min(b.min),
                    max: a.max.max(b.max),
                })
            })
            .unwrap_or_else(|| aabb.aabb());
        let center = (hull.min + hull.max) * 0.5;
        let size = (hull.max - hull.min).max_element();
        // The authored glint is drawn about 64 world units wide at scale 1.
        let request = FxRequest::new(rooms.of(entity), center, ambition_vfx::FxId::new(GUARD_GLINT_FX))
            .with_scale((size / 96.0).clamp(0.6, 2.5));
        fx.write(request);
    }
}
