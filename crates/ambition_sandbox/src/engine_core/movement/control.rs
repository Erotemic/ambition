use crate::engine_core::world::World;
use crate::engine_core::Vec2;

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
    kinematics: &mut crate::engine_core::player_clusters::PlayerKinematics,
    abilities: &crate::engine_core::player_clusters::PlayerAbilities,
    flight: &mut crate::engine_core::player_clusters::PlayerFlightState,
    wall: &mut crate::engine_core::player_clusters::PlayerWallState,
    dash: &mut crate::engine_core::player_clusters::PlayerDashState,
    blink: &mut crate::engine_core::player_clusters::PlayerBlinkState,
    combo_trace: &mut crate::engine_core::player_clusters::PlayerComboTrace,
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

    if (input.blink_pressed || (input.blink_held && !blink.hold_active)) && blink.cooldown <= 0.0
    {
        blink.hold_active = true;
        blink.hold_timer = 0.0;
        blink.aiming = false;
        blink.aim_offset = Vec2::new(tuning.blink_distance * kinematics.facing, 0.0);
    }

    if blink.hold_active && input.blink_held {
        let control_dt = dt.min(1.0 / 20.0);
        blink.hold_timer += control_dt;
        if abilities.abilities.precision_blink && blink.hold_timer >= tuning.blink_hold_threshold
        {
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
pub fn handle_attacks_clusters(
    world: &World,
    kinematics: &mut crate::engine_core::player_clusters::PlayerKinematics,
    abilities: &crate::engine_core::player_clusters::PlayerAbilities,
    ground: &mut crate::engine_core::player_clusters::PlayerGroundState,
    dash: &mut crate::engine_core::player_clusters::PlayerDashState,
    jump_state: &mut crate::engine_core::player_clusters::PlayerJumpState,
    combo_trace: &mut crate::engine_core::player_clusters::PlayerComboTrace,
    input: InputState,
    tuning: MovementTuning,
    events: &mut FrameEvents,
) {
    if !abilities.abilities.attack {
        return;
    }
    let can_pogo = abilities.abilities.pogo && !ground.on_ground;
    if input.pogo_pressed && can_pogo {
        if let Some(orb_aabb) = super::collision::try_pogo_clusters(
            world, kinematics, abilities, dash, jump_state, ground, tuning,
        ) {
            events.op_clusters(combo_trace, MovementOp::Pogo);
            events.pogo_hits.push(orb_aabb);
        } else {
            kinematics.vel.x -= kinematics.facing * (tuning.slash_recoil * 0.45);
            events.op_clusters(combo_trace, MovementOp::Slash);
        }
    } else if input.attack_pressed {
        if can_pogo && input.axis_y > 0.25 {
            if let Some(orb_aabb) = super::collision::try_pogo_clusters(
                world, kinematics, abilities, dash, jump_state, ground, tuning,
            ) {
                events.op_clusters(combo_trace, MovementOp::Pogo);
                events.pogo_hits.push(orb_aabb);
            } else {
                kinematics.vel.x -= kinematics.facing * tuning.slash_recoil;
                events.op_clusters(combo_trace, MovementOp::Slash);
            }
        } else {
            kinematics.vel.x -= kinematics.facing * tuning.slash_recoil;
            events.op_clusters(combo_trace, MovementOp::Slash);
        }
    }
}

