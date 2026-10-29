//! THE SKID: dust at the feet when a fighter starts a dash and when it turns
//! around out of a run. Neither is a `MovementOp` (the initial dash and the
//! turnaround are phases of ordinary ground locomotion, not operations), so
//! the simulation emits nothing for them; this reads the published motion
//! facts and draws on their rising edge. Presentation only: it writes a
//! `VfxMessage` and nothing the simulation reads.

use bevy::prelude::*;

use ambition_platformer2d_core::BodyMotionFacts;
use ambition_platformer2d_shared_tangle::lifecycle::LiveRooms;
use ambition_sim_view::BodyPoseView;
use ambition_time::SimTick;
use ambition_vfx::vfx::{FxPose, VfxMessage, VfxWriter};

/// What each body did last tick, so an edge is one puff and not one a frame.
#[derive(Default)]
pub struct LocomotionDustMemory {
    seen: std::collections::HashMap<Entity, (bool, bool)>,
    tick: Option<u64>,
}

pub fn emit_locomotion_dust(
    tick: Res<SimTick>,
    mut memory: Local<LocomotionDustMemory>,
    rooms: LiveRooms,
    bodies: Query<(Entity, &BodyPoseView, &BodyMotionFacts)>,
    mut vfx: VfxWriter,
) {
    if memory.tick == Some(tick.0) {
        return;
    }
    memory.tick = Some(tick.0);
    let mut seen_now = std::collections::HashMap::with_capacity(memory.seen.len());
    for (entity, pose, facts) in &bodies {
        let now = (facts.initial_dashing, facts.turning_around);
        let was = memory.seen.get(&entity).copied().unwrap_or_default();
        seen_now.insert(entity, now);
        let dash_started = now.0 && !was.0;
        let turn_started = now.1 && !was.1;
        if !(dash_started || turn_started) {
            continue;
        }
        let feet = pose.pos + pose.gravity_dir * (pose.size.y * 0.5);
        vfx.for_room(rooms.of(entity)).write(VfxMessage::Effect {
            pos: feet,
            fx: crate::fx::SKID_PUFF_FX,
            scale: crate::fx::SKID_PUFF_FX_SCALE,
            // The skid trails the way the body came from: a dash start throws
            // dust behind the new facing, a turnaround behind the old one.
            pose: FxPose { mirror: if turn_started { pose.facing < 0.0 } else { pose.facing > 0.0 }, angle: 0.0 },
        });
    }
    memory.seen = seen_now;
}
