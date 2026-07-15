use crate::world::World;

use super::events::FrameEvents;
use super::input::InputState;
use super::ops::MovementOp;
use super::tuning::AxisSweptParams;
use crate::MotionFrame;

/// Drive the blink hold / aim / release lifecycle: arm on press
/// when the cooldown has cleared, enter precision-aim after the hold
/// threshold (precision_blink ability required), update the aim
/// offset from stick input, and complete the blink on release.
/// The hold/aim lifecycle is axis-policy maneuver state (`state.blink_*`);
/// only the recharge cooldown lives on the shared blink cluster.
#[allow(clippy::too_many_arguments)]
pub fn handle_blink_clusters(
    world: &World,
    kinematics: &mut crate::body_clusters::BodyKinematics,
    abilities: &crate::body_clusters::BodyAbilities,
    blink: &mut crate::body_clusters::BodyBlinkState,
    state: &mut crate::movement::AxisManeuverState,
    combo_trace: &mut crate::body_clusters::BodyComboTrace,
    input: InputState,
    dt: f32,
    frame: MotionFrame,
    tuning: AxisSweptParams,
    events: &mut FrameEvents,
) {
    // "Forward along facing" is a LOCAL-side direction; every world-space
    // default derived from it must go through the body frame (fable review
    // 2026-07-02 §B9 — the world-X fallback broke sideways gravity).
    let facing_aim_offset = frame.side() * (tuning.abilities.blink_distance * kinematics.facing);

    if !abilities.abilities.blink {
        state.blink_hold_active = false;
        state.blink_aiming = false;
        state.blink_hold_timer = 0.0;
        state.blink_aim_offset = facing_aim_offset;
        return;
    }

    if (input.blink_pressed || (input.blink_held && !state.blink_hold_active))
        && blink.cooldown <= 0.0
    {
        state.blink_hold_active = true;
        state.blink_hold_timer = 0.0;
        state.blink_aiming = false;
        state.blink_aim_offset = facing_aim_offset;
    }

    if state.blink_hold_active && input.blink_held {
        let control_dt = dt.min(1.0 / 20.0);
        state.blink_hold_timer += control_dt;
        if abilities.abilities.precision_blink
            && state.blink_hold_timer >= tuning.abilities.blink_hold_threshold
        {
            state.blink_aiming = true;
        }
        if state.blink_aiming {
            // Precision steer in WORLD space, already resolved through the AIM
            // frame mode at the seam (screen-directed by default), so the cursor
            // moves the way the stick points ON SCREEN at any gravity.
            let aim_input = input.blink_aim_step.vec();
            if aim_input.length_squared() > 0.01 {
                state.blink_aim_offset +=
                    aim_input * (tuning.abilities.precision_blink_aim_speed * control_dt);
                state.blink_aim_offset = state
                    .blink_aim_offset
                    .clamp_length_max(tuning.abilities.precision_blink_distance);
            }
        }
    }

    if state.blink_hold_active && input.blink_released {
        // Quick blink direction in WORLD space, resolved through the MOVEMENT
        // frame mode at the seam (locomotion-framed). Zero stick → forward
        // along facing, which lives on the body's local side axis.
        let fallback = frame.side() * kinematics.facing;
        let aim = input.blink_quick_dir.normalize_or(fallback);
        let precision = state.blink_aiming && abilities.abilities.precision_blink;
        let from = kinematics.pos;
        let to = if precision {
            super::blink::blink_destination_to_point_clusters(
                world,
                kinematics,
                abilities,
                kinematics.pos + state.blink_aim_offset,
            )
        } else {
            super::blink::blink_destination_clusters(
                world,
                kinematics,
                abilities,
                aim,
                tuning.abilities.blink_distance,
            )
        };
        super::blink::complete_blink_clusters(
            kinematics,
            blink,
            state,
            combo_trace,
            from,
            to,
            precision,
            frame,
            tuning,
            events,
        );
    }

    if state.blink_hold_active && !input.blink_held && !input.blink_released {
        state.blink_hold_active = false;
        state.blink_aiming = false;
        state.blink_hold_timer = 0.0;
        state.blink_aim_offset = facing_aim_offset;
    }
}

/// Cluster-ref attack handler used by `update_player_control_with_clusters`.
///
/// Pogo (the dedicated button AND the air down-attack) is owned by the moveset
/// down-air: the on-hit pogo technique (`dispatch_hitbox_on_hit`) for entity/
/// breakable victims and `pogo_moveset_off_world_orbs` for world `PogoOrb` blocks,
/// both bouncing gravity-relatively off the move's real hitbox. The engine here
/// only applies the slash recoil + records the combo op for a plain melee press. The old
/// probe-based `try_pogo_clusters` was a redundant second pogo mechanism (same
/// `world.blocks` check, same `set_jump_velocity`, same `PogoBounce` event) and
/// was removed (2026-06-16).
pub fn handle_attacks_clusters(
    kinematics: &mut crate::body_clusters::BodyKinematics,
    abilities: &crate::body_clusters::BodyAbilities,
    combo_trace: &mut crate::body_clusters::BodyComboTrace,
    input: InputState,
    frame: MotionFrame,
    tuning: AxisSweptParams,
    events: &mut FrameEvents,
) {
    if !abilities.abilities.attack {
        return;
    }
    if input.attack_pressed {
        // Recoil opposes facing along the body's LOCAL side axis (fable review
        // 2026-07-02 §B4 — the raw `vel.x` form shoved sideways-gravity bodies
        // along their gravity axis).
        kinematics.vel -= frame.side() * (kinematics.facing * tuning.abilities.slash_recoil);
        events.op_clusters(combo_trace, MovementOp::Slash);
    }
}
