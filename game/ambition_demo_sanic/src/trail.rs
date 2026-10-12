//! Dust where Sanic's speed shows: a puff at his feet while he runs, a spray
//! of it when he skids.
//!
//! Presentation only: it reads the facts the movement publishes
//! (`BodyMotionFacts::running`, `::skidding`) and writes bursts through the
//! engine's particle seam. It keeps no state of the simulation.

use bevy::prelude::*;

use ambition_platformer2d::engine_core as ae;

/// How often a running body leaves a puff, and a skidding one a spray (sim
/// seconds).
const RUN_DUST_PERIOD: f32 = 0.07;
const SKID_DUST_PERIOD: f32 = 0.04;

/// What a body leaves this tick: how many particles, how fast they go, their
/// colour. `None` when it leaves none.
fn dust_of(facts: &ae::BodyMotionFacts) -> Option<(f32, u32, f32, [f32; 4])> {
    if facts.skidding {
        Some((SKID_DUST_PERIOD, 3, 120.0, [0.93, 0.90, 0.80, 0.90]))
    } else if facts.running {
        Some((RUN_DUST_PERIOD, 1, 46.0, [0.95, 0.97, 1.00, 0.55]))
    } else {
        None
    }
}

/// Leave dust at the feet of the controlled body while it runs or skids.
pub fn emit_speed_dust(
    time: Res<ambition_platformer2d::time::WorldTime>,
    subject: Option<Res<ambition_platformer2d::platformer::markers::ControlledSubject>>,
    mut since: Local<f32>,
    bodies: Query<(&ae::BodyMotionFacts, &ae::BodyKinematics)>,
    rooms: ambition_platformer2d::platformer::lifecycle::LiveRooms,
    mut vfx: ambition_platformer2d::vfx::VfxWriter,
) {
    let body = subject.and_then(|subject| subject.0);
    let Some((body, (facts, kin))) =
        body.and_then(|body| bodies.get(body).ok().map(|found| (body, found)))
    else {
        *since = 0.0;
        return;
    };
    let Some((period, count, speed, color)) = dust_of(facts) else {
        *since = 0.0;
        return;
    };
    *since += time.sim_dt();
    if *since < period {
        return;
    }
    *since -= period;
    // At his feet, a little behind the way he goes.
    let behind = if kin.vel.x.abs() > 1.0 {
        -kin.vel.x.signum()
    } else {
        0.0
    };
    vfx.for_room(rooms.of(body))
        .write(ambition_platformer2d::vfx::VfxMessage::Burst {
            pos: ae::Vec2::new(
                kin.pos.x + behind * kin.size.x * 0.4,
                kin.pos.y + kin.size.y * 0.45,
            ),
            count,
            speed,
            color,
            kind: ambition_platformer2d::vfx::ParticleKind::Dust,
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A skid is a spray and a run is a puff; a body that does neither
    /// leaves nothing, and a skid wins over a run.
    #[test]
    fn a_skid_sprays_a_run_puffs_and_a_walk_leaves_nothing() {
        let facts = |running, skidding| ae::BodyMotionFacts {
            running,
            skidding,
            ..Default::default()
        };
        assert_eq!(dust_of(&facts(false, false)), None);
        let run = dust_of(&facts(true, false)).expect("a run leaves dust");
        let skid = dust_of(&facts(true, true)).expect("a skid leaves dust");
        assert!(
            skid.1 > run.1 && skid.0 < run.0,
            "a skid is more dust, more often: {skid:?} against {run:?}"
        );
    }
}
