use crate::engine_core::world::World;

use super::events::FrameEvents;
use super::input::InputState;
use super::ops::MovementOp;
use super::tuning::{MovementTuning, ONE_WAY_DROP_THROUGH_GRACE};
use crate::engine_core::player_state::BodyMode;

const LADDER_JUMP_BOOST_TIME: f32 = 0.10;

/// Consume the buffered jump (if any) and emit the right verb:
/// swim stroke while submerged + swim ability, drop-through gate
/// while standing on a one-way + drop_through_pressed, wall-jump,
/// regular jump, or double-jump. Each branch zeroes the buffer +
/// coyote timer so the same press can't re-fire.
pub fn handle_jump_buffer_clusters(
    world: &World,
    action_buffer: &mut crate::engine_core::player_clusters::PlayerActionBuffer,
    env_contact: &crate::engine_core::player_clusters::PlayerEnvironmentContact,
    abilities: &crate::engine_core::player_clusters::PlayerAbilities,
    body_mode: BodyMode,
    kinematics: &mut crate::engine_core::player_clusters::BodyKinematics,
    ground: &mut crate::engine_core::player_clusters::PlayerGroundState,
    wall: &mut crate::engine_core::player_clusters::PlayerWallState,
    jump_state: &mut crate::engine_core::player_clusters::PlayerJumpState,
    combo_trace: &mut crate::engine_core::player_clusters::PlayerComboTrace,
    input: InputState,
    tuning: MovementTuning,
    events: &mut FrameEvents,
) {
    if action_buffer.jump <= 0.0 {
        return;
    }

    if let Some(contact) = env_contact.water {
        if abilities.abilities.swim {
            let impulse = contact.spec.swim_up_impulse;
            kinematics.vel.y = (kinematics.vel.y - impulse).min(-impulse);
            action_buffer.jump = 0.0;
            ground.coyote_timer = 0.0;
            events.op_clusters(combo_trace, MovementOp::SwimStroke);
            return;
        }
    }

    let on_ladder = env_contact.climbable.is_some();

    if input.drop_through_pressed && on_ladder {
        jump_state.ladder_drop_through_timer = ONE_WAY_DROP_THROUGH_GRACE;
        jump_state.ladder_drop_through_hold_lock = true;
        jump_state.ladder_jump_boost = 0.0;
        kinematics.vel.y = kinematics.vel.y.max(0.0);
        action_buffer.jump = 0.0;
        ground.coyote_timer = 0.0;
        return;
    }

    if body_mode == BodyMode::Climbing && on_ladder {
        if abilities.abilities.jump && input.axis_y < -0.1 {
            jump_state.ladder_jump_boost = LADDER_JUMP_BOOST_TIME;
            events.op_clusters(combo_trace, MovementOp::Jump);
        }
        action_buffer.jump = 0.0;
        ground.coyote_timer = 0.0;
        return;
    }

    let can_ladder_jump = on_ladder && !ground.on_ground;

    if input.drop_through_pressed
        && ground.on_ground
        && crate::engine_core::movement::collision::standing_on_one_way_aabb(
            world,
            kinematics.aabb(),
        )
    {
        action_buffer.jump = 0.0;
        ground.on_ground = false;
        ground.coyote_timer = 0.0;
        ground.drop_through_timer = ONE_WAY_DROP_THROUGH_GRACE;
        return;
    }

    if abilities.abilities.wall_jump && wall.on_wall && !ground.on_ground {
        kinematics.vel.x = wall.wall_normal_x * tuning.wall_jump_x;
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
    } else if abilities.abilities.double_jump && jump_state.air_jumps_available > 0 {
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
