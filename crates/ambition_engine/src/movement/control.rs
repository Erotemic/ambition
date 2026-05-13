use crate::world::World;
use crate::Vec2;

use super::blink::{blink_destination, blink_destination_to_point, complete_blink};
use super::collision::try_pogo;
use super::events::FrameEvents;
use super::input::InputState;
use super::ops::MovementOp;
use super::player::Player;
use super::tuning::MovementTuning;

pub fn update_player_control_with_tuning(
    world: &World,
    player: &mut Player,
    input: InputState,
    control_dt: f32,
    tuning: MovementTuning,
) -> FrameEvents {
    let mut events = FrameEvents::default();

    if input.reset_pressed && player.abilities.reset {
        player.reset_to(world.spawn);
        events.reset = true;
        return events;
    }

    update_facing_and_control_intent(player, input, tuning);
    handle_mode_toggles(player, input, &mut events);
    handle_blink(world, player, input, control_dt, tuning, &mut events);
    handle_attacks(world, player, input, tuning, &mut events);
    handle_dash(player, input, tuning, &mut events);
    handle_jump_release(player, input);

    events
}

fn update_facing_and_control_intent(
    player: &mut Player,
    input: InputState,
    tuning: MovementTuning,
) {
    // Airborne players can still influence horizontal velocity but can't
    // turn around — the visual facing is locked at the direction they
    // left the ground (or last set on the ground). Flight mode is the
    // exception: in-flight movement is intentionally floaty and a free
    // pivot, so facing flips with input. This keeps directional aerial
    // attacks (forward / back) committed once the player leaves the
    // ground without sacrificing fly-mode steering.
    let can_turn = player.on_ground || player.fly_enabled;
    if can_turn && input.axis_x.abs() > 0.1 {
        player.facing = input.axis_x.signum();
    }

    if input.jump_pressed && player.abilities.jump {
        player.jump_buffer_timer = tuning.jump_buffer;
    }
    if input.dash_pressed && player.abilities.dash {
        player.dash_buffer_timer = tuning.dash_buffer;
    }
}

fn handle_mode_toggles(player: &mut Player, input: InputState, events: &mut FrameEvents) {
    if input.fly_toggle_pressed && player.abilities.fly {
        player.fly_enabled = !player.fly_enabled;
        if player.fly_enabled {
            player.fast_falling = false;
            player.wall_clinging = false;
            player.wall_climbing = false;
            player.dash_timer = 0.0;
            player.blink_grace_timer = 0.0;
        }
        events.op(player, MovementOp::FlyToggle);
    }
}

fn handle_blink(
    world: &World,
    player: &mut Player,
    input: InputState,
    dt: f32,
    tuning: MovementTuning,
    events: &mut FrameEvents,
) {
    if !player.abilities.blink {
        player.blink_hold_active = false;
        player.blink_aiming = false;
        player.blink_hold_timer = 0.0;
        player.blink_aim_offset = Vec2::new(tuning.blink_distance * player.facing, 0.0);
        return;
    }

    if (input.blink_pressed || (input.blink_held && !player.blink_hold_active))
        && player.blink_cooldown <= 0.0
    {
        // Permit a held blink button to arm as soon as cooldown clears. This
        // avoids a bad second-blink case where the user pressed slightly early,
        // the hold was ignored, and bullet-time never engaged.
        player.blink_hold_active = true;
        player.blink_hold_timer = 0.0;
        player.blink_aiming = false;
        player.blink_aim_offset = Vec2::new(tuning.blink_distance * player.facing, 0.0);
    }

    if player.blink_hold_active && input.blink_held {
        // Blink hold/aim uses unscaled control time. During precision blink,
        // physics can be nearly frozen, but the destination cursor should still
        // feel like a responsive UI control.
        let control_dt = dt.min(1.0 / 20.0);
        player.blink_hold_timer += control_dt;
        if player.abilities.precision_blink
            && player.blink_hold_timer >= tuning.blink_hold_threshold
        {
            player.blink_aiming = true;
        }
        if player.blink_aiming {
            let aim_input = Vec2::new(input.axis_x, input.axis_y);
            if aim_input.length_squared() > 0.01 {
                player.blink_aim_offset +=
                    aim_input * (tuning.precision_blink_aim_speed * control_dt);
                player.blink_aim_offset = player
                    .blink_aim_offset
                    .clamp_length_max(tuning.precision_blink_distance);
            }
        }
    }

    if player.blink_hold_active && input.blink_released {
        let fallback = Vec2::new(player.facing, 0.0);
        let aim = Vec2::new(input.axis_x, input.axis_y).normalize_or(fallback);
        let precision = player.blink_aiming && player.abilities.precision_blink;
        let from = player.pos;
        let to = if precision {
            blink_destination_to_point(world, player, player.pos + player.blink_aim_offset)
        } else {
            blink_destination(world, player, aim, tuning.blink_distance)
        };
        complete_blink(player, from, to, precision, tuning, events);
    }

    // Cancel a partially-started blink if the binding disappeared for any
    // reason without a release event. This avoids sticky bullet-time state when
    // focus changes or a future remapper swaps presets mid-hold.
    if player.blink_hold_active && !input.blink_held && !input.blink_released {
        player.blink_hold_active = false;
        player.blink_aiming = false;
        player.blink_hold_timer = 0.0;
        player.blink_aim_offset = Vec2::new(tuning.blink_distance * player.facing, 0.0);
    }
}

fn handle_attacks(
    world: &World,
    player: &mut Player,
    input: InputState,
    tuning: MovementTuning,
    events: &mut FrameEvents,
) {
    if !player.abilities.attack {
        return;
    }
    let can_pogo = player.abilities.pogo;
    if input.pogo_pressed && can_pogo {
        if let Some(orb_aabb) = try_pogo(world, player, tuning) {
            events.op(player, MovementOp::Pogo);
            events.pogo_hits.push(orb_aabb);
        } else {
            // Dedicated pogo whiff still gives a tiny correction so it can be
            // tested as a fourth face-button verb without requiring a target.
            player.vel.x -= player.facing * (tuning.slash_recoil * 0.45);
            events.op(player, MovementOp::Slash);
        }
    } else if input.attack_pressed {
        if can_pogo && input.axis_y > 0.25 {
            if let Some(orb_aabb) = try_pogo(world, player, tuning) {
                events.op(player, MovementOp::Pogo);
                events.pogo_hits.push(orb_aabb);
            } else {
                player.vel.x -= player.facing * tuning.slash_recoil;
                events.op(player, MovementOp::Slash);
            }
        } else {
            // A small generated recoil/correction action. It exists to test
            // cancellability and non-commutative feel.
            player.vel.x -= player.facing * tuning.slash_recoil;
            events.op(player, MovementOp::Slash);
        }
    }
}

fn handle_jump_release(player: &mut Player, input: InputState) {
    // Variable jump height is an input/control gesture. It should react even
    // during bullet-time rather than waiting for scaled simulation time.
    if player.abilities.variable_jump && input.jump_released && player.vel.y < -120.0 {
        player.vel.y *= 0.54;
    }
}

fn handle_dash(
    player: &mut Player,
    input: InputState,
    tuning: MovementTuning,
    events: &mut FrameEvents,
) {
    if player.dash_buffer_timer > 0.0
        && player.abilities.dash
        && player.dash_charges_available > 0
        && player.dash_cooldown <= 0.0
    {
        let fallback = Vec2::new(player.facing, 0.0);
        let aim = Vec2::new(input.axis_x, input.axis_y).normalize_or(fallback);
        player.vel = aim * tuning.dash_speed;
        player.dash_timer = tuning.dash_time;
        player.dash_cooldown = tuning.dash_cooldown;
        player.dash_buffer_timer = 0.0;
        let op = player.spend_dash_charge();
        events.op(player, op);
    }
}
