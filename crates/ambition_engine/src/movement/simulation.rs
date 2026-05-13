use crate::world::World;

use super::collision::{standing_on_one_way, touching_hazard};
use super::dec;
use super::events::FrameEvents;
use super::input::InputState;
use super::integration::integrate_velocity;
use super::ops::MovementOp;
use super::player::Player;
use super::tuning::{MovementTuning, ONE_WAY_DROP_THROUGH_GRACE};

pub fn update_player_simulation_with_tuning(
    world: &World,
    player: &mut Player,
    input: InputState,
    raw_dt: f32,
    tuning: MovementTuning,
) -> FrameEvents {
    let mut events = FrameEvents::default();
    if raw_dt <= 0.0 {
        return events;
    }
    let dt = raw_dt.min(1.0 / 30.0);

    // Water contact is queried once per tick and cached on the
    // player so jump-buffer handling, gravity integration, and the
    // post-step reset gate all see the same answer. Source-agnostic:
    // `water_at` covers both IntGrid `Water` cells and entity
    // `WaterVolume` regions.
    player.water_contact = world.water_at(player.aabb());

    // Climbable contact: same one-query-per-tick discipline as
    // `water_contact`. Movement consumes this when BodyMode::Climbing
    // is active, while HUD / trace / adapters can read the cached answer.
    player.climbable_contact = world.climbable_at(player.aabb());
    if !player.abilities.ledge_grab {
        player.ledge_grab = None;
    }

    // Drowning gate: water without the swim ability is a death zone,
    // not a slow-down. Trigger the same reset path the hazard tile
    // uses so the existing flash/sfx/respawn pipeline applies.
    if player.water_contact.is_some() && !player.abilities.swim {
        player.reset_to(world.spawn);
        events.hazard = true;
        events.reset = true;
        return events;
    }

    age_player(player, dt);
    update_simulation_timers(player, dt, tuning);
    if crate::ledge_grab::tick_active_ledge_grab(player, input, dt, &mut events) {
        return events;
    }
    handle_jump_buffer(world, player, input, tuning, &mut events);
    integrate_velocity(world, player, input, dt, tuning, &mut events);
    crate::ledge_grab::try_start_ledge_grab(world, player, input, &mut events);

    if touching_hazard(world, player) || player.pos.y > world.size.y + 200.0 {
        player.reset_to(world.spawn);
        events.hazard = true;
        events.reset = true;
    }

    events
}

fn age_player(player: &mut Player, dt: f32) {
    player.time_alive += dt;
    player.max_speed = player.max_speed.max(player.vel.length());
    for mark in &mut player.combo {
        mark.age += dt;
    }
    player
        .combo
        .retain(|m| m.age < 4.0 || m.op == MovementOp::Reset);
}

fn update_simulation_timers(player: &mut Player, dt: f32, tuning: MovementTuning) {
    player.jump_buffer_timer = dec(player.jump_buffer_timer, dt);
    player.dash_buffer_timer = dec(player.dash_buffer_timer, dt);
    player.coyote_timer = dec(player.coyote_timer, dt);
    player.drop_through_timer = dec(player.drop_through_timer, dt);
    player.dash_cooldown = dec(player.dash_cooldown, dt);
    player.blink_cooldown = dec(player.blink_cooldown, dt);
    player.blink_grace_timer = dec(player.blink_grace_timer, dt);
    player.rebound_cooldown = dec(player.rebound_cooldown, dt);

    if player.on_ground {
        player.coyote_timer = tuning.coyote_time;
        player.refresh_movement_resources(tuning);
    }
}

fn handle_jump_buffer(
    world: &World,
    player: &mut Player,
    input: InputState,
    tuning: MovementTuning,
    events: &mut FrameEvents,
) {
    if player.jump_buffer_timer > 0.0 {
        // Underwater swimming wins over every other jump path: while
        // submerged with the swim ability, a buffered jump becomes
        // exactly one upward swim stroke and nothing else. This keeps
        // "underwater jump != normal jump" true on a single press,
        // and the `min(-impulse)` floor makes repeated taps reliably
        // rise even if the previous stroke is still climbing.
        if let Some(contact) = player.water_contact {
            if player.abilities.swim {
                let impulse = contact.spec.swim_up_impulse;
                player.vel.y = (player.vel.y - impulse).min(-impulse);
                player.jump_buffer_timer = 0.0;
                player.coyote_timer = 0.0;
                events.op(player, MovementOp::SwimStroke);
                return;
            }
        }

        // Down + jump while standing on a one-way platform means "drop through",
        // not "jump". Cancel the buffered jump so the vertical sweep can take
        // the player past the platform on the next integration step.
        if input.drop_through_pressed && player.on_ground && standing_on_one_way(world, player) {
            player.jump_buffer_timer = 0.0;
            player.on_ground = false;
            player.coyote_timer = 0.0;
            // Latch the drop-through so subsequent frames keep ignoring the
            // one-way until the player has cleared the landing tolerance band.
            // Without this, the gesture only frees the player for a single
            // frame and the resolve-up step snaps them back onto the platform.
            player.drop_through_timer = ONE_WAY_DROP_THROUGH_GRACE;
            return;
        }
        if player.abilities.wall_jump && player.on_wall && !player.on_ground {
            player.vel.x = player.wall_normal_x * tuning.wall_jump_x;
            player.vel.y = -tuning.jump_speed * 0.94;
            player.on_wall = false;
            player.wall_clinging = false;
            player.wall_climbing = false;
            player.jump_buffer_timer = 0.0;
            player.coyote_timer = 0.0;
            events.op(player, MovementOp::WallJump);
        } else if player.abilities.jump && (player.on_ground || player.coyote_timer > 0.0) {
            player.vel.y = -tuning.jump_speed;
            player.on_ground = false;
            player.jump_buffer_timer = 0.0;
            player.coyote_timer = 0.0;
            player.air_jumps_available = player.abilities.air_jump_count(tuning.air_jumps);
            events.op(player, MovementOp::Jump);
        } else if player.abilities.double_jump && player.air_jumps_available > 0 {
            player.vel.y = -tuning.double_jump_speed;
            player.on_ground = false;
            player.on_wall = false;
            player.wall_clinging = false;
            player.wall_climbing = false;
            player.jump_buffer_timer = 0.0;
            player.air_jumps_available -= 1;
            events.op(player, MovementOp::DoubleJump);
        }
    }
}
