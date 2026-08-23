//! What a fighter does while it is being hit.
//!
//! Directional influence and smash DI are the two things a reeling body may
//! still ask for, and both are read off one field: the locomotion the body is
//! holding when the hit resolves and while the hitlag freeze runs. Neither
//! spends an APM token, because neither is a press — a person holding a
//! direction into a hit is not acting at machine speed.
//!
//! The choice is made against [`ae::hit_response::di_adjust`] itself rather than
//! against a second model of it: the two perpendicular candidates are each run
//! through the real rotation and priced by how long the resulting trajectory
//! stays inside the stage envelope. A brain that re-derived the rotation would
//! be free to disagree with the kernel about which way is survival.

use ambition_platformer2d_core::{self as ae, Vec2};

use crate::perception::{BodyPhase, Perceived};

/// Comparison probe, in radians. The match's real DI budget is a rule of the
/// match (`DeclaredCombatRules::di_max_angle`) and the brain does not read it:
/// both candidates are rotated by the SAME angle, so the argmax is the same for
/// every positive budget and this constant never leaks into the answer.
const PROBE_ANGLE: f32 = 0.3;

/// Below this the launch has no direction worth deflecting, and the stick is
/// worth holding only for the smash-DI shift.
const MIN_LAUNCH_SPEED: f32 = 1.0;

/// The stick a reeling body holds, in body-local axes — `None` when the body is
/// not reeling and the ordinary decision owns its movement.
///
/// One direction serves both mechanics: `di_adjust` rotates the launch by the
/// perpendicular part of this vector, and `smash_di_shift` displaces the frozen
/// body along it during hitlag. Survival wants the same answer from both.
pub fn survival_stick(view: Perceived<'_>) -> Option<ae::LocalAxes> {
    let me = &view.self_view;
    if me.phase != BodyPhase::Hitstun || !me.alive {
        return None;
    }
    // ONLY IN FLIGHT, AND THE GROUNDED CASE IS NOT A DETAIL.
    //
    // Measured 2026-08-22: a GROUNDED body in hitstun still resolves its
    // horizontal velocity from held locomotion, so a stick held against a
    // 4,800 px/s slide erased it inside twelve ticks. Influence is a rotation of
    // a launch the body is still flying; on the floor the same field is
    // ordinary walking, and a brain that held it would be walking out of
    // hitstun — the exact cheat this brain exists without. Whether the kernel
    // should scale grounded locomotion authority during hitstun at all is a
    // separate question, recorded for the combat lane.
    if me.on_ground {
        return None;
    }
    if !view.stage.is_known() {
        // No stage envelope means no blastzone to steer away from. Holding a
        // guessed direction would be worse than holding none: DI is a rotation,
        // and rotating a launch at random is as likely to shorten a life.
        return None;
    }
    let frame = ae::AccelerationFrame::new(me.gravity_down);
    let bounds = view.stage.bounds;
    let launch = me.vel;
    let speed = launch.length();

    if speed < MIN_LAUNCH_SPEED {
        // Frozen or barely moving: there is nothing to rotate, but the hitlag
        // shift still moves the body, so hold toward the middle of the stage.
        let centre = (bounds.min + bounds.max) * 0.5;
        let inward = centre - me.pos;
        return unit_local(frame, inward);
    }

    let launch_dir = launch / speed;
    let perpendicular = Vec2::new(-launch_dir.y, launch_dir.x);
    let survives = |candidate: Vec2| -> f32 {
        let Some(local) = unit_local(frame, candidate) else {
            return f32::NEG_INFINITY;
        };
        let steered = ae::hit_response::di_adjust(
            launch,
            Vec2::new(local.x, local.y),
            me.gravity_down,
            PROBE_ANGLE,
        );
        time_inside(bounds, me.pos, steered)
    };
    // Strictly greater, so an exact tie keeps the first candidate and the choice
    // does not depend on float comparison order.
    let best = if survives(-perpendicular) > survives(perpendicular) {
        -perpendicular
    } else {
        perpendicular
    };
    unit_local(frame, best)
}

/// How long a body leaving `from` at `vel` stays inside `bounds`, in seconds.
///
/// A straight ray, deliberately: gravity bends the real flight, but it bends
/// both candidates the same way, and the question here is only which of two
/// mirror-image deflections buys more room. `f32::INFINITY` for a velocity that
/// never leaves along an axis.
fn time_inside(bounds: ae::Aabb, from: Vec2, vel: Vec2) -> f32 {
    let axis = |p: f32, v: f32, lo: f32, hi: f32| -> f32 {
        if v > 0.0 {
            (hi - p) / v
        } else if v < 0.0 {
            (lo - p) / v
        } else {
            f32::INFINITY
        }
    };
    axis(from.x, vel.x, bounds.min.x, bounds.max.x)
        .min(axis(from.y, vel.y, bounds.min.y, bounds.max.y))
        .max(0.0)
}

/// A world direction as a full-deflection local stick, or `None` when it has no
/// direction at all.
fn unit_local(frame: ae::AccelerationFrame, world: Vec2) -> Option<ae::LocalAxes> {
    let local = frame.to_local(world);
    let length = local.length();
    if length < 1e-6 {
        return None;
    }
    let unit = local / length;
    Some(ae::LocalAxes::new(unit.x, unit.y))
}

#[cfg(test)]
mod tests;
