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

use ambition_characters::perception::{BodyPhase, Perceived};

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
    // HOW LONG THIS BODY NEEDS TO SURVIVE: until it can act again, and not one
    // second longer. `phase_remaining` is the hitstun it still owes, read off
    // the same view the rest of this decision uses.
    //
    // ⭐ THIS IS WHAT MAKES THE TWO OBJECTIVES BELOW COMMENSURABLE WITHOUT A
    // FITTED THRESHOLD. Surviving past the moment control returns buys nothing —
    // a body that will still be inside the stage when it can move again has
    // already solved survival — so the survival term SATURATES there instead of
    // rewarding ever-larger numbers.
    let needed = me.phase_remaining.max(0.0);
    let steer = |candidate: Vec2| -> Option<Vec2> {
        let local = unit_local(frame, candidate)?;
        Some(ae::hit_response::di_adjust(
            launch,
            Vec2::new(local.x, local.y),
            me.gravity_down,
            PROBE_ANGLE,
        ))
    };
    // SURVIVAL, capped at what is needed. Uncapped this was the whole decision,
    // and at centre stage both deflections keep a body inside for so long that
    // the argmax between them was noise.
    let survives = |candidate: Vec2| -> f32 {
        steer(candidate).map_or(f32::NEG_INFINITY, |steered| {
            time_inside(bounds, me.pos, steered).min(needed)
        })
    };
    // ⭐⭐ ESCAPE, which is the OTHER thing a real player does with this stick
    // and the half this brain did not have. The genre uses one mechanic for two
    // purposes: survival DI rotates a launch away from the blast zone at kill
    // percent, and escape DI rotates it away from the OPPONENT to break a
    // juggle. Only the objective differs, so nothing in the kernel changes here.
    //
    // Priced as where this body will BE when it can act again — `needed` seconds
    // along the steered launch — measured from the foe. A juggle is a loop
    // because an upward launch returns the victim to the attacker; the way out
    // is to not come down in the same place.
    let escapes = |candidate: Vec2| -> f32 {
        let Some(foe) = view.nearest_hostile() else {
            return 0.0;
        };
        steer(candidate).map_or(f32::NEG_INFINITY, |steered| {
            (me.pos + steered * needed).distance(foe.pos)
        })
    };
    // ⛔ SURVIVAL FIRST, AND ONLY WHERE IT IS ACTUALLY AT STAKE. Because the
    // survival term saturates at `needed`, two deflections that both carry the
    // body safely past the moment it regains control score EQUAL — and the
    // decision falls through to escape. Near a blast zone they do not tie, and
    // survival decides outright. No percent test, no distance-to-blastzone
    // constant, and nothing that has to be re-fitted when a stage changes size.
    let (a, b) = (-perpendicular, perpendicular);
    let (survive_a, survive_b) = (survives(a), survives(b));
    // Strictly greater, so an exact tie keeps the first candidate and the choice
    // does not depend on float comparison order.
    let best = if survive_a > survive_b {
        a
    } else if survive_b > survive_a {
        b
    } else if escapes(a) > escapes(b) {
        a
    } else {
        b
    };
    unit_local(frame, best)
}

/// Should this body press the evade button to tech its landing?
///
/// A tech is the genre's most basic defensive read and no CPU has ever thrown
/// one: the mechanic ships — the press arms a ~20-frame window and a landing
/// inside it keeps the body's feet — and the brain had no verb that reaches it.
/// It is the SAME press that means dodge everywhere else; the body resolves
/// which, so nothing here decides that.
///
/// The read is the time to the floor, taken against the FULL window rather than
/// a safety margin inside it. Measured 2026-08-23: with half the window a body
/// falling at 1,500 px/s never pressed at all — this view is delayed by the
/// fighter's own reaction time, so the position it reports is stale, and a
/// margin meant to stop an early press made every press late instead. One window
/// before the PERCEIVED contact lands inside the real one, and the press stays
/// live for the whole window either way.
///
/// ⭐ THE PRESS REACHES THE BODY. It did not when this was written:
/// `apply_post_hit_input_gates` stripped `MovementAction::Burst` for the whole of
/// hitstun — precisely the state a tech exists to escape — so a tumbling body's
/// press was deleted before `tick_knockdown` could read it, and teching was
/// unreachable for every body in the game, a human one included. That gate now
/// exempts the Burst EDGE while tumbling.
///
/// ⛔ Exempt while TUMBLING only, and that distinction is load-bearing for
/// anything reading this to escape a juggle: hitstun with Burst open is a body
/// that air-dodges out of being hit, so a victim in hitstun and NOT tumbling has
/// no escape press by design. Its only agency is the stick — see
/// [`survival_stick`].
pub fn tech_press(view: Perceived<'_>) -> bool {
    let me = &view.self_view;
    if !me.alive || !me.tumbling || me.on_ground {
        return false;
    }
    let Some(floor_top) = view.ground_below() else {
        return false;
    };
    // `ground_below` answers in the view's own +y-down sense, which is also the
    // sense its terrain is published in, so the gap and the closing speed are
    // read in the same frame rather than converted through gravity twice.
    let feet = me.pos.y + me.half_extent.y;
    let gap = floor_top - feet;
    let closing = me.vel.y;
    if gap < 0.0 || closing <= 0.0 {
        return false;
    }
    gap / closing <= ae::movement::knockdown::TECH_WINDOW
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
