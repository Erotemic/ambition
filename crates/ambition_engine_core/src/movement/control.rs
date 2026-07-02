use crate::world::World;

use super::events::FrameEvents;
use super::input::InputState;
use super::ops::MovementOp;
use super::tuning::MovementTuning;

/// Drive the blink hold / aim / release lifecycle: arm on press
/// when the cooldown has cleared, enter precision-aim after the hold
/// threshold (precision_blink ability required), update the aim
/// offset from stick input, and complete the blink on release.
#[allow(clippy::too_many_arguments)]
pub fn handle_blink_clusters(
    world: &World,
    kinematics: &mut crate::body_clusters::BodyKinematics,
    abilities: &crate::body_clusters::BodyAbilities,
    flight: &mut crate::body_clusters::BodyFlightState,
    wall: &mut crate::body_clusters::BodyWallState,
    dash: &mut crate::body_clusters::BodyDashState,
    blink: &mut crate::body_clusters::BodyBlinkState,
    combo_trace: &mut crate::body_clusters::BodyComboTrace,
    input: InputState,
    dt: f32,
    tuning: MovementTuning,
    events: &mut FrameEvents,
) {
    // "Forward along facing" is a LOCAL-side direction; every world-space
    // default derived from it must go through the body frame (fable review
    // 2026-07-02 §B9 — the world-X fallback broke sideways gravity).
    let frame = crate::AccelerationFrame::new(tuning.gravity_dir);
    let facing_aim_offset = frame.side * (tuning.blink_distance * kinematics.facing);

    if !abilities.abilities.blink {
        blink.hold_active = false;
        blink.aiming = false;
        blink.hold_timer = 0.0;
        blink.aim_offset = facing_aim_offset;
        return;
    }

    if (input.blink_pressed || (input.blink_held && !blink.hold_active)) && blink.cooldown <= 0.0 {
        blink.hold_active = true;
        blink.hold_timer = 0.0;
        blink.aiming = false;
        blink.aim_offset = facing_aim_offset;
    }

    if blink.hold_active && input.blink_held {
        let control_dt = dt.min(1.0 / 20.0);
        blink.hold_timer += control_dt;
        if abilities.abilities.precision_blink && blink.hold_timer >= tuning.blink_hold_threshold {
            blink.aiming = true;
        }
        if blink.aiming {
            // Precision steer in WORLD space, already resolved through the AIM
            // frame mode at the seam (screen-directed by default), so the cursor
            // moves the way the stick points ON SCREEN at any gravity.
            let aim_input = input.blink_aim_step;
            if aim_input.length_squared() > 0.01 {
                blink.aim_offset += aim_input * (tuning.precision_blink_aim_speed * control_dt);
                blink.aim_offset = blink
                    .aim_offset
                    .clamp_length_max(tuning.precision_blink_distance);
            }
        }
    }

    if blink.hold_active && input.blink_released {
        // Quick blink direction in WORLD space, resolved through the MOVEMENT
        // frame mode at the seam (locomotion-framed). Zero stick → forward
        // along facing, which lives on the body's local side axis.
        let fallback = frame.side * kinematics.facing;
        let aim = input.blink_quick_dir.normalize_or(fallback);
        let precision = blink.aiming && abilities.abilities.precision_blink;
        let from = kinematics.pos;
        let to = if precision {
            super::blink::blink_destination_to_point_clusters(
                world,
                kinematics,
                abilities,
                kinematics.pos + blink.aim_offset,
            )
        } else {
            super::blink::blink_destination_clusters(
                world,
                kinematics,
                abilities,
                aim,
                tuning.blink_distance,
            )
        };
        super::blink::complete_blink_clusters(
            kinematics,
            flight,
            wall,
            dash,
            blink,
            combo_trace,
            from,
            to,
            precision,
            tuning,
            events,
        );
    }

    if blink.hold_active && !input.blink_held && !input.blink_released {
        blink.hold_active = false;
        blink.aiming = false;
        blink.hold_timer = 0.0;
        blink.aim_offset = facing_aim_offset;
    }
}

/// Cluster-ref attack handler used by `update_player_control_with_clusters`.
///
/// Pogo (the dedicated button AND the air down-attack) is owned by the sandbox
/// attack system (`advance_attack`), which detects the target with the real
/// attack hitbox and bounces gravity-relatively. The engine here only applies the
/// slash recoil + records the combo op for a plain melee press. The old
/// probe-based `try_pogo_clusters` was a redundant second pogo mechanism (same
/// `world.blocks` check, same `set_jump_velocity`, same `PogoBounce` event) and
/// was removed (2026-06-16).
pub fn handle_attacks_clusters(
    kinematics: &mut crate::body_clusters::BodyKinematics,
    abilities: &crate::body_clusters::BodyAbilities,
    combo_trace: &mut crate::body_clusters::BodyComboTrace,
    input: InputState,
    tuning: MovementTuning,
    events: &mut FrameEvents,
) {
    if !abilities.abilities.attack {
        return;
    }
    if input.attack_pressed {
        // Recoil opposes facing along the body's LOCAL side axis (fable review
        // 2026-07-02 §B4 — the raw `vel.x` form shoved sideways-gravity bodies
        // along their gravity axis).
        let frame = crate::AccelerationFrame::new(tuning.gravity_dir);
        kinematics.vel -= frame.side * (kinematics.facing * tuning.slash_recoil);
        events.op_clusters(combo_trace, MovementOp::Slash);
    }
}
