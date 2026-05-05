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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::ControlFrame;
    use crate::{GameWorld, LedgeGrabState, SandboxRuntime};
    use ambition_engine::{self as ae};

    fn empty_world() -> ae::World {
        ae::World::new(
            "ledge_test",
            ae::Vec2::new(2000.0, 2000.0),
            ae::Vec2::new(200.0, 1000.0),
            Vec::new(),
        )
    }

    fn ledge_app(ledge_grab_ability: bool) -> App {
        let mut app = App::new();
        app.insert_resource(Time::<()>::default());
        app.insert_resource(GameWorld(empty_world()));
        let world = empty_world();
        let mut abilities = ae::AbilitySet::sandbox_all();
        abilities.ledge_grab = ledge_grab_ability;
        let runtime = SandboxRuntime::new(
            &world,
            abilities,
            ae::DEFAULT_TUNING,
            crate::physics::PhysicsSandboxSettings::default(),
        );
        app.insert_resource(runtime);
        app.insert_resource(ControlFrame::default());
        app.add_systems(Update, update_ledge_grab);
        app
    }

    /// Disabling the ledge_grab ability must clear any latched
    /// `SandboxRuntime::ledge_grab` state and must not move the
    /// player. This is the contract the tech-debt note relies on:
    /// turning the ability off cleanly disengages the post-update
    /// mechanic.
    #[test]
    fn ability_off_clears_state_and_does_not_move_player() {
        let mut app = ledge_app(false);
        // Pre-populate a latched ledge state and remember the player
        // position so we can assert it didn't move.
        let pos_before;
        let vel_before;
        {
            let mut runtime = app.world_mut().resource_mut::<SandboxRuntime>();
            pos_before = runtime.player.pos;
            vel_before = runtime.player.vel;
            runtime.ledge_grab = Some(LedgeGrabState {
                contact: ae::LedgeContact {
                    wall_normal_x: 1.0,
                    anchor: ae::Vec2::new(123.0, 456.0),
                    climb_target: ae::Vec2::new(150.0, 432.0),
                },
                elapsed: 0.5,
                climbing: false,
            });
        }
        app.update();
        let runtime = app.world().resource::<SandboxRuntime>();
        assert!(
            runtime.ledge_grab.is_none(),
            "ability off must drop the latched ledge state"
        );
        assert_eq!(
            runtime.player.pos, pos_before,
            "ability off must not move the player"
        );
        assert_eq!(
            runtime.player.vel, vel_before,
            "ability off must not modify player velocity"
        );
    }

    /// Sanity check: with the ability off and no latched state, the
    /// system is a no-op even if the player happens to be wall-
    /// clinging (the early return must short-circuit before the
    /// probe runs).
    #[test]
    fn ability_off_short_circuits_even_when_wall_clinging() {
        let mut app = ledge_app(false);
        {
            let mut runtime = app.world_mut().resource_mut::<SandboxRuntime>();
            runtime.player.wall_clinging = true;
            runtime.player.on_wall = true;
            runtime.player.wall_normal_x = 1.0;
        }
        app.update();
        let runtime = app.world().resource::<SandboxRuntime>();
        assert!(runtime.ledge_grab.is_none());
        // Wall-cling flags are not cleared by the ledge_grab system —
        // it's a no-op on the player when the ability is off.
        assert!(runtime.player.wall_clinging);
        assert!(runtime.player.on_wall);
    }
}
