//! A move that takes its owner under the stage, and one that brings her back.
//!
//! ⭐⭐ THE OTHER HALF OF `ambition_characters::smash_trapdoor`. It sits beside
//! the authored teleport because it needs the same two things that file already
//! owns: a collision view of the stage, and
//! [`ledge_assisted_arrival`](super::teleport::ledge_assisted_arrival), which is
//! already the rule for *"find the surface above this point and stand the body
//! on it"*. Surfacing through a floor and a recovery catching a ledge are the
//! same question asked by two moves.
//!
//! ⛔ WHAT "UNDER THE STAGE" MEANS IS NOT HERE. No gravity, no geometry, no
//! hurtbox, and the stick still steers — every one of those is a property of
//! `BodyMode::Submerged`, stated once in the movement kernel and in the
//! invulnerability projection. This file only moves her between modes.

use bevy::prelude::*;

use ambition_characters::brain::{ActionRequest, ActorActionMessage, SpecialActionSpec};
use ambition_characters::smash_trapdoor::{TrapdoorParams, TRAPDOOR};
use ambition_platformer2d_core::{self as ae};

/// Recognise an authored trapdoor beat and move the body between modes.
///
/// ⛔ IT RUNS WHERE EVERY OTHER `ActorActionMessage` CONSUMER RUNS, so a beat
/// authored on a move's timeline lands on the frame the move says.
pub fn apply_authored_trapdoors(
    world: ambition_platformer2d_world::collision::CollisionWorld,
    mut actions: MessageReader<ActorActionMessage>,
    mut bodies: Query<(
        ae::BodyClusterQueryData,
        // ⭐ THE BODY'S OWN FRAME, because "which way is up" is not a constant.
        // The surfacing search looks for a floor along this body's gravity, so
        // a stage that rotates gravity rotates which side of her the boards are
        // on — the same reason the teleport's ledge assist takes it.
        &ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame,
        &mut ae::movement::MotionModel,
    )>,
    mut vfx: MessageWriter<ambition_vfx::vfx::VfxMessage>,
    mut sfx: ambition_sfx::BodySfxWriter,
) {
    let mut collision = None;
    for message in actions.read() {
        let ActionRequest::Special { spec, params } = &message.request else {
            continue;
        };
        let SpecialActionSpec::Special(key) = spec;
        if key.as_str() != TRAPDOOR {
            continue;
        }
        let params: TrapdoorParams = match params.hydrate() {
            Ok(params) => params,
            Err(err) => {
                warn!("trapdoor params did not hydrate: {err}");
                continue;
            }
        };
        let Ok((mut cluster_item, resolved_frame, mut motion_model)) =
            bodies.get_mut(message.actor)
        else {
            continue;
        };
        let gravity_dir = resolved_frame.down();
        let mut clusters = cluster_item.as_clusters_mut();
        // The door is drawn where the body is on the frame it opens: going
        // under, that is her feet on the boards; coming up, it is wherever she
        // steered to. Taken BEFORE the surfacing move so the exit door opens at
        // the floor she comes through and not at the point she came from.
        let mut door = clusters.kinematics.pos;

        if params.submerge {
            clusters.body_mode.body_mode = ae::player_state::BodyMode::Submerged;
            // ⛔⛔ THE FALL IS ENDED, NOT INHERITED. She may have pressed this
            // out of a run or a drop, and a submerged body integrates its own
            // velocity outright — a leftover fall would be overwritten on the
            // next tick anyway, but the tick BETWEEN this write and that one
            // would carry her down through the world with collision already
            // switched off.
            clusters.kinematics.vel = ae::Vec2::ZERO;
        } else {
            // ⭐ SHE COMES UP THROUGH A FLOOR. `ledge_assisted_arrival` finds
            // the surface above a point and stands the body on it, refusing a
            // placement that would embed — which is exactly the surfacing rule,
            // and is why this file borrows it rather than restating it.
            let half = clusters.kinematics.size * 0.5;
            let from = clusters.kinematics.pos;
            let solids = collision.get_or_insert_with(|| world.solids());
            let surfaced = match solids.as_ref() {
                Some(w) => super::teleport::ledge_assisted_arrival(
                    &**w,
                    from,
                    half,
                    params.surface_reach,
                    gravity_dir,
                ),
                // No collision world (a minimal test app): she comes up where
                // she is, which is what every other traversal does here.
                None => from,
            };
            door = surfaced;
            // ⛔ THE MODE FIRST, THEN THE PLACE. `transit_body` reconciles
            // departure contacts against the body's CURRENT mode, and a body
            // still marked submerged is one the contact pass believes nothing
            // touches.
            clusters.body_mode.body_mode = ae::player_state::BodyMode::Standing;
            ae::movement::transit_body(
                &mut motion_model,
                &mut clusters,
                surfaced,
                ae::movement::TransitVelocity::Zero,
            );
        }

        // The look and the sound are the MOVE's, not this system's — the same
        // rule the authored teleport follows, so a mole and a stagehand can use
        // one technique and share nothing else.
        vfx.write(ambition_vfx::vfx::VfxMessage::Effect {
            pos: door,
            fx: ambition_vfx::fx::FxId::new(&params.vfx),
            scale: 1.0,
            pose: ambition_vfx::FxPose::UPRIGHT,
        });
        sfx.write_for(
            message.actor,
            ambition_sfx::SfxMessage::Play {
                id: ambition_sfx::SfxId::new(&params.sfx),
                pos: door,
            },
        );
    }
}

#[cfg(test)]
mod tests;
