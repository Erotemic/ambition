use crate::world::World;

use super::events::FrameEvents;
use super::input::InputState;
use super::ops::MovementOp;
use super::tuning::{MovementTuning, ONE_WAY_DROP_THROUGH_GRACE};
use crate::player_state::BodyMode;

const LADDER_JUMP_BOOST_TIME: f32 = 0.10;

/// Consume the buffered jump (if any) and emit the right verb:
/// swim stroke while submerged + swim ability, drop-through gate
/// while standing on a one-way + drop_through_pressed, wall-jump,
/// regular jump, or double-jump. Each branch zeroes the buffer +
/// coyote timer so the same press can't re-fire.
#[allow(clippy::too_many_arguments)]
pub fn handle_jump_buffer_clusters(
    world: &World,
    action_buffer: &mut crate::body_clusters::BodyActionBuffer,
    env_contact: &crate::body_clusters::BodyEnvironmentContact,
    abilities: &crate::body_clusters::BodyAbilities,
    body_mode: BodyMode,
    // Movement-verb taxonomy: `jump`/`double-jump` are GROUNDED-mode verbs. A body
    // currently in FLIGHT mode steers vertically through the flight limb
    // (ascend/descend), so the buffered jump must NOT become a grounded leap — else
    // a possessed flyer "jumps straight up". Grounded/hybrid bodies (fly off) are
    // unaffected. Wall/ladder/swim keep their own context gates below.
    flying: bool,
    kinematics: &mut crate::body_clusters::BodyKinematics,
    ground: &mut crate::body_clusters::BodyGroundState,
    wall: &mut crate::body_clusters::BodyWallState,
    jump_state: &mut crate::body_clusters::BodyJumpState,
    combo_trace: &mut crate::body_clusters::BodyComboTrace,
    input: InputState,
    tuning: MovementTuning,
    events: &mut FrameEvents,
) {
    if action_buffer.jump <= 0.0 {
        return;
    }

    let frame = crate::AccelerationFrame::new(tuning.gravity_dir);

    if let Some(contact) = env_contact.water {
        if abilities.abilities.swim {
            let impulse = contact.spec.swim_up_impulse;
            let ascend_target = -impulse;
            let descend = kinematics.vel.dot(frame.down);
            if descend > ascend_target {
                kinematics.vel += frame.down * (ascend_target - descend);
            }
            action_buffer.jump = 0.0;
            ground.coyote_timer = 0.0;
            events.op_clusters(combo_trace, MovementOp::SwimStroke);
            return;
        }
    }

    let on_ladder = env_contact.climbable.is_some();

    if super::integration::wants_drop_through(tuning.stick(&input).y, input.jump_pressed)
        && on_ladder
    {
        jump_state.ladder_drop_through_timer = ONE_WAY_DROP_THROUGH_GRACE;
        jump_state.ladder_drop_through_hold_lock = true;
        jump_state.ladder_jump_boost = 0.0;
        let descend = kinematics.vel.dot(frame.down);
        if descend < 0.0 {
            kinematics.vel -= frame.down * descend;
        }
        action_buffer.jump = 0.0;
        ground.coyote_timer = 0.0;
        return;
    }

    if body_mode == BodyMode::Climbing && on_ladder {
        // "Press away from the feet + jump" boosts off the ladder (gravity- +
        // input-mode-relative via the resolved descend).
        if abilities.abilities.jump && tuning.stick(&input).y < -0.1 {
            jump_state.ladder_jump_boost = LADDER_JUMP_BOOST_TIME;
            events.op_clusters(combo_trace, MovementOp::Jump);
        }
        action_buffer.jump = 0.0;
        ground.coyote_timer = 0.0;
        return;
    }

    let can_ladder_jump = on_ladder && !ground.on_ground;
    if super::integration::wants_drop_through(tuning.stick(&input).y, input.jump_pressed)
        && ground.on_ground
        && crate::movement::collision::standing_on_one_way_aabb(
            world,
            kinematics.aabb_oriented(tuning.gravity_dir),
            tuning.gravity_dir,
        )
    {
        action_buffer.jump = 0.0;
        ground.on_ground = false;
        ground.coyote_timer = 0.0;
        ground.drop_through_timer = ONE_WAY_DROP_THROUGH_GRACE;
        return;
    }

    if abilities.abilities.wall_jump && wall.on_wall && !ground.on_ground {
        let frame = crate::AccelerationFrame::new(tuning.gravity_dir);
        let target_side = wall.wall_normal_x * tuning.wall_jump_x;
        let cur_side = kinematics.vel.dot(frame.side);
        kinematics.vel += frame.side * (target_side - cur_side);
        super::integration::set_jump_velocity(
            &mut kinematics.vel,
            tuning.gravity_dir,
            tuning.jump_speed * 0.94,
        );
        wall.on_wall = false;
        wall.wall_clinging = false;
        wall.wall_climbing = false;
        action_buffer.jump = 0.0;
        ground.coyote_timer = 0.0;
        events.op_clusters(combo_trace, MovementOp::WallJump);
    } else if abilities.abilities.jump
        && !flying
        && (ground.on_ground || ground.coyote_timer > 0.0 || can_ladder_jump)
    {
        super::integration::set_jump_velocity(
            &mut kinematics.vel,
            tuning.gravity_dir,
            tuning.jump_speed,
        );
        ground.on_ground = false;
        action_buffer.jump = 0.0;
        ground.coyote_timer = 0.0;
        jump_state.air_jumps_available = abilities.abilities.air_jump_count(tuning.air_jumps);
        events.op_clusters(combo_trace, MovementOp::Jump);
    } else if abilities.abilities.double_jump && !flying && jump_state.air_jumps_available > 0 {
        super::integration::set_jump_velocity(
            &mut kinematics.vel,
            tuning.gravity_dir,
            tuning.double_jump_speed,
        );
        ground.on_ground = false;
        wall.on_wall = false;
        wall.wall_clinging = false;
        wall.wall_climbing = false;
        action_buffer.jump = 0.0;
        jump_state.air_jumps_available -= 1;
        events.op_clusters(combo_trace, MovementOp::DoubleJump);
    }
}
