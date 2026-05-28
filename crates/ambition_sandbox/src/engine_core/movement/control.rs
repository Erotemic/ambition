use crate::engine_core::world::World;
use crate::engine_core::Vec2;

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
    handle_dodge(player, input, tuning, &mut events);
    handle_dash(player, input, tuning, &mut events);
    handle_shield(player, input, tuning, &mut events);
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


/// Cluster-ref variant of [`handle_blink`]. Mirrors the legacy
/// `&mut Player` version field-for-field.
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

/// Cluster-ref variant of [`handle_attacks`].
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
    // Pogo is an aerial verb: you bounce off something below you. On the
    // ground, `try_pogo`'s hitbox sits just under the player's feet and
    // any normal `Solid` floor counts as a valid target — so without
    // this gate, pressing down+attack (or the dedicated pogo button)
    // while grounded launches the player off the floor and flips
    // `on_ground` false, which then makes the sandbox `start_attack`
    // resolve the input as `AirDown` instead of the new grounded
    // kneeling poke (`AttackIntent::Down`). Gating both branches on
    // `!player.on_ground` keeps pogo airborne-only and lets the
    // grounded down-tilt fire as intended.
    let can_pogo = player.abilities.pogo && !player.on_ground;
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

fn handle_dodge(
    player: &mut Player,
    input: InputState,
    tuning: MovementTuning,
    events: &mut FrameEvents,
) {
    if player.dash_buffer_timer > 0.0
        && player.abilities.dodge
        && player.on_ground
        && player.dodge_roll_cooldown <= 0.0
    {
        let dir = if input.axis_x.abs() > 0.1 {
            input.axis_x.signum()
        } else {
            player.facing
        };
        player.vel.x = dir * tuning.dodge_roll_speed;
        player.vel.y = player.vel.y.min(0.0);
        player.dodge_roll_timer = tuning.dodge_roll_time;
        player.dodge_roll_cooldown = tuning.dodge_roll_cooldown;
        player.dash_buffer_timer = 0.0;
        events.op(player, MovementOp::DodgeRoll);
    }
}

fn handle_shield(
    player: &mut Player,
    input: InputState,
    tuning: MovementTuning,
    events: &mut FrameEvents,
) {
    if !player.abilities.shield {
        player.shield_active = false;
        player.parry_window_timer = 0.0;
        return;
    }
    // Shield cannot be held during an active dash; the dash always wins.
    let can_shield = player.dash_timer <= 0.0;
    let want_shield = input.shield_held && can_shield;
    if want_shield && !player.shield_active {
        // Fresh activation: start the parry window and record the op.
        player.parry_window_timer = tuning.parry_window_time;
        events.op(player, MovementOp::ShieldUp);
    }
    player.shield_active = want_shield;
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
