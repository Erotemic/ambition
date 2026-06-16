use crate::world::World;
use crate::Vec2;

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
    kinematics: &mut crate::player_clusters::BodyKinematics,
    abilities: &crate::player_clusters::PlayerAbilities,
    flight: &mut crate::player_clusters::PlayerFlightState,
    wall: &mut crate::player_clusters::PlayerWallState,
    dash: &mut crate::player_clusters::PlayerDashState,
    blink: &mut crate::player_clusters::PlayerBlinkState,
    combo_trace: &mut crate::player_clusters::PlayerComboTrace,
    input: InputState,
    dt: f32,
    tuning: MovementTuning,
    events: &mut FrameEvents,
) {
    if !abilities.abilities.blink {
        blink.hold_active = false;
        blink.aiming = false;
        blink.hold_timer = 0.0;
        blink.aim_offset = Vec2::new(tuning.blink_distance * kinematics.facing, 0.0);
        return;
    }

    if (input.blink_pressed || (input.blink_held && !blink.hold_active)) && blink.cooldown <= 0.0 {
        blink.hold_active = true;
        blink.hold_timer = 0.0;
        blink.aiming = false;
        blink.aim_offset = Vec2::new(tuning.blink_distance * kinematics.facing, 0.0);
    }

    if blink.hold_active && input.blink_held {
        let control_dt = dt.min(1.0 / 20.0);
        blink.hold_timer += control_dt;
        if abilities.abilities.precision_blink && blink.hold_timer >= tuning.blink_hold_threshold {
            blink.aiming = true;
        }
        if blink.aiming {
            let aim_input = Vec2::new(input.axis_x, input.axis_y);
            if aim_input.length_squared() > 0.01 {
                blink.aim_offset += aim_input * (tuning.precision_blink_aim_speed * control_dt);
                blink.aim_offset = blink
                    .aim_offset
                    .clamp_length_max(tuning.precision_blink_distance);
            }
        }
    }

    if blink.hold_active && input.blink_released {
        let fallback = Vec2::new(kinematics.facing, 0.0);
        let aim = Vec2::new(input.axis_x, input.axis_y).normalize_or(fallback);
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
        blink.aim_offset = Vec2::new(tuning.blink_distance * kinematics.facing, 0.0);
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
    kinematics: &mut crate::player_clusters::BodyKinematics,
    abilities: &crate::player_clusters::PlayerAbilities,
    combo_trace: &mut crate::player_clusters::PlayerComboTrace,
    input: InputState,
    tuning: MovementTuning,
    events: &mut FrameEvents,
) {
    if !abilities.abilities.attack {
        return;
    }
    if input.attack_pressed {
        kinematics.vel.x -= kinematics.facing * tuning.slash_recoil;
        events.op_clusters(combo_trace, MovementOp::Slash);
    }
}
