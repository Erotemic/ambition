//! Sandbox-side ledge grab driver.
//!
//! Wires `ambition_engine::probe_ledge_grab` into the live runtime.
//! Runs after `sandbox_update`. The model:
//!
//! 1. While the player has the `ledge_grab` ability and is wall-
//!    clinging without a ledge already grabbed, probe for a grabbable
//!    ledge each frame.
//! 2. On a hit, snap the player to `LedgeContact::anchor`, suppress
//!    velocity, and stash the contact in `SandboxRuntime::ledge_grab`.
//! 3. While grabbed, gravity is held to zero by the system (we just
//!    keep snapping the player to the anchor each frame). The HUD
//!    shows "ledge".
//! 4. Up + Jump pulls up: snap the player to `climb_target`, clear
//!    the state, restore normal movement.
//! 5. Down (or letting go of the wall) drops back into wall-slide.
//!
//! This deliberately runs as a separate, narrow Bevy system so we
//! avoid threading a new state into the dense `movement.rs` simulator.
//! When the broader character-state-machine refactor lands, this code
//! relocates to the engine alongside `LedgeContact`.

use ambition_engine as ae;
use bevy::prelude::*;

pub fn update_ledge_grab(
    world: Res<crate::GameWorld>,
    mut runtime: ResMut<crate::SandboxRuntime>,
    controls: Res<crate::input::ControlFrame>,
    time: Res<Time>,
) {
    if !runtime.player.abilities.ledge_grab {
        runtime.ledge_grab = None;
        return;
    }
    let dt = time.delta_secs();

    // Already on a ledge: handle climb / drop input.
    if let Some(mut state) = runtime.ledge_grab {
        state.elapsed += dt;
        let want_climb = (controls.axis_y < -0.4 && controls.jump_pressed)
            || (controls.axis_y < -0.4 && controls.interact_pressed);
        let want_drop = controls.axis_y > 0.4 && !controls.jump_pressed;
        if want_climb {
            runtime.player.pos = ae::Vec2::new(
                state.contact.climb_target.x,
                state.contact.climb_target.y,
            );
            runtime.player.vel = ae::Vec2::ZERO;
            runtime.player.on_ground = true;
            runtime.player.wall_clinging = false;
            runtime.player.on_wall = false;
            runtime.ledge_grab = None;
            return;
        }
        if want_drop {
            runtime.player.wall_clinging = false;
            runtime.player.on_wall = false;
            runtime.ledge_grab = None;
            return;
        }
        // Hold position. Suppress gravity by re-anchoring each frame.
        runtime.player.pos = state.contact.anchor;
        runtime.player.vel = ae::Vec2::ZERO;
        runtime.player.wall_clinging = true;
        runtime.player.on_wall = true;
        runtime.ledge_grab = Some(state);
        return;
    }

    // Not yet grabbed: probe while wall-clinging.
    if !runtime.player.wall_clinging {
        return;
    }
    let player_pos = runtime.player.pos;
    let player_size = runtime.player.size;
    let wall_normal = runtime.player.wall_normal_x;
    let Some(contact) = ae::probe_ledge_grab(player_pos, player_size, wall_normal, &world.0)
    else {
        return;
    };
    runtime.player.pos = contact.anchor;
    runtime.player.vel = ae::Vec2::ZERO;
    runtime.ledge_grab = Some(crate::LedgeGrabState {
        contact,
        elapsed: 0.0,
        climbing: false,
    });
}
