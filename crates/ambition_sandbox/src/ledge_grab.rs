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
//! 4. Up, Interact, Jump, or pressing into the platform starts a short
//!    climb-up transition instead of teleporting to the top instantly.
//! 5. Down or pressing away from the platform drops back into normal
//!    airborne movement.
//!
//! This deliberately runs as a separate, narrow Bevy system so we
//! avoid threading a new state into the dense `movement.rs` simulator.
//! When the broader character-state-machine refactor lands, this code
//! relocates to the engine alongside `LedgeContact`.

use ambition_engine as ae;
use bevy::prelude::*;

/// Duration of the ledge pull-up transition.
///
/// This is deliberately short: long enough to stop the climb from reading as a
/// position snap, but not long enough to feel like a canned cutscene in a fast
/// platformer.
pub const LEDGE_CLIMB_TIME: f32 = 0.24;

/// Require a tiny hang beat before held horizontal input into the platform
/// auto-starts the climb. Without this, any held wall-cling input can collapse
/// the ledge hang into an effectively invisible state on the same visual beat.
pub const LEDGE_TOWARD_CLIMB_DELAY: f32 = 0.045;

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn climb_position(contact: ae::LedgeContact, progress: f32) -> ae::Vec2 {
    let t = smoothstep(progress);
    contact.anchor + (contact.climb_target - contact.anchor) * t
}

fn into_platform_axis(contact: ae::LedgeContact) -> f32 {
    -contact.wall_normal_x
}

fn away_from_platform_axis(contact: ae::LedgeContact) -> f32 {
    contact.wall_normal_x
}

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
        // Face into the wall so the overhead-reaching ledge-grab
        // sprite (drawn for facing-right) flips to match the cling
        // side. wall_normal_x = -1 (wall on player's right) -> face
        // right; +1 (wall on left) -> face left.
        runtime.player.facing = into_platform_axis(state.contact);

        if state.climbing {
            state.climb_elapsed += dt;
            let progress = (state.climb_elapsed / LEDGE_CLIMB_TIME).clamp(0.0, 1.0);
            runtime.player.pos = climb_position(state.contact, progress);
            runtime.player.vel = ae::Vec2::ZERO;
            runtime.player.on_ground = false;
            runtime.player.wall_clinging = false;
            runtime.player.on_wall = false;

            if progress >= 1.0 {
                runtime.player.pos = state.contact.climb_target;
                runtime.player.vel = ae::Vec2::ZERO;
                runtime.player.on_ground = true;
                runtime.player.wall_clinging = false;
                runtime.player.on_wall = false;
                runtime.ledge_grab = None;
            } else {
                runtime.ledge_grab = Some(state);
            }
            return;
        }

        let input_up = controls.axis_y < -0.4 || controls.up_pressed;
        let input_down = controls.axis_y > 0.4 || controls.down_pressed;
        let input_into_platform = controls.axis_x * into_platform_axis(state.contact) > 0.4;
        let input_away_from_platform = controls.axis_x * away_from_platform_axis(state.contact) > 0.4;
        let want_climb = input_up
            || controls.interact_pressed
            || controls.jump_pressed
            || (state.elapsed >= LEDGE_TOWARD_CLIMB_DELAY && input_into_platform);
        let want_drop = input_down || input_away_from_platform;

        if want_drop && !want_climb {
            runtime.player.wall_clinging = false;
            runtime.player.on_wall = false;
            runtime.ledge_grab = None;
            return;
        }
        if want_climb {
            state.climbing = true;
            state.climb_elapsed = 0.0;
            runtime.player.pos = state.contact.anchor;
            runtime.player.vel = ae::Vec2::ZERO;
            runtime.player.on_ground = false;
            runtime.player.wall_clinging = false;
            runtime.player.on_wall = false;
            runtime.ledge_grab = Some(state);
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
    let Some(contact) = ae::probe_ledge_grab(player_pos, player_size, wall_normal, &world.0) else {
        return;
    };
    runtime.player.pos = contact.anchor;
    runtime.player.vel = ae::Vec2::ZERO;
    runtime.player.facing = into_platform_axis(contact);
    runtime.ledge_grab = Some(crate::LedgeGrabState {
        contact,
        elapsed: 0.0,
        climbing: false,
        climb_elapsed: 0.0,
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

    fn ledge_state(wall_normal_x: f32, anchor: ae::Vec2, climb_target: ae::Vec2) -> LedgeGrabState {
        LedgeGrabState {
            contact: ae::LedgeContact {
                wall_normal_x,
                anchor,
                climb_target,
            },
            elapsed: 0.0,
            climbing: false,
            climb_elapsed: 0.0,
        }
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
            runtime.ledge_grab = Some(ledge_state(
                1.0,
                ae::Vec2::new(123.0, 456.0),
                ae::Vec2::new(150.0, 432.0),
            ));
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

    /// With ability on, a latched ledge state, and Down held with no
    /// jump, the player drops back into wall-slide: state cleared,
    /// `wall_clinging` and `on_wall` cleared.
    #[test]
    fn down_input_drops_off_a_latched_ledge() {
        let mut app = ledge_app(true);
        {
            let mut runtime = app.world_mut().resource_mut::<SandboxRuntime>();
            runtime.ledge_grab = Some(ledge_state(
                1.0,
                ae::Vec2::new(120.0, 400.0),
                ae::Vec2::new(140.0, 380.0),
            ));
            runtime.player.wall_clinging = true;
            runtime.player.on_wall = true;
        }
        // axis_y > 0.4 is "Down", jump_pressed false.
        *app.world_mut().resource_mut::<ControlFrame>() = ControlFrame {
            axis_y: 1.0,
            jump_pressed: false,
            ..ControlFrame::default()
        };
        app.update();
        let runtime = app.world().resource::<SandboxRuntime>();
        assert!(runtime.ledge_grab.is_none(), "down should drop the ledge");
        assert!(!runtime.player.wall_clinging);
        assert!(!runtime.player.on_wall);
    }

    #[test]
    fn away_input_drops_off_a_latched_ledge() {
        let mut app = ledge_app(true);
        {
            let mut runtime = app.world_mut().resource_mut::<SandboxRuntime>();
            // wall_normal_x = -1 means the wall/platform is on the player's
            // right; pressing left is away from it.
            runtime.ledge_grab = Some(ledge_state(
                -1.0,
                ae::Vec2::new(120.0, 400.0),
                ae::Vec2::new(140.0, 380.0),
            ));
            runtime.player.wall_clinging = true;
            runtime.player.on_wall = true;
        }
        *app.world_mut().resource_mut::<ControlFrame>() = ControlFrame {
            axis_x: -1.0,
            ..ControlFrame::default()
        };
        app.update();
        let runtime = app.world().resource::<SandboxRuntime>();
        assert!(runtime.ledge_grab.is_none(), "away should release the ledge");
        assert!(!runtime.player.wall_clinging);
        assert!(!runtime.player.on_wall);
    }

    /// With ability on and a latched ledge, Up starts the climb-up
    /// transition. The transition, not the initial input frame, is
    /// responsible for placing the player at `climb_target`.
    #[test]
    fn up_input_starts_climb_transition() {
        let mut app = ledge_app(true);
        let anchor = ae::Vec2::new(120.0, 400.0);
        let target = ae::Vec2::new(150.0, 380.0);
        {
            let mut runtime = app.world_mut().resource_mut::<SandboxRuntime>();
            runtime.ledge_grab = Some(ledge_state(1.0, anchor, target));
        }
        *app.world_mut().resource_mut::<ControlFrame>() = ControlFrame {
            axis_y: -1.0,
            ..ControlFrame::default()
        };
        app.update();
        let runtime = app.world().resource::<SandboxRuntime>();
        let ledge = runtime
            .ledge_grab
            .expect("climb-up should start a transition before clearing");
        assert!(ledge.climbing);
        assert_eq!(runtime.player.pos, anchor, "first climb frame stays anchored");
        assert!(!runtime.player.on_ground, "climb transition is not grounded yet");
    }

    #[test]
    fn toward_platform_input_starts_climb_after_short_hang_delay() {
        let mut app = ledge_app(true);
        let anchor = ae::Vec2::new(120.0, 400.0);
        let target = ae::Vec2::new(150.0, 380.0);
        {
            let mut runtime = app.world_mut().resource_mut::<SandboxRuntime>();
            let mut state = ledge_state(-1.0, anchor, target);
            state.elapsed = LEDGE_TOWARD_CLIMB_DELAY;
            runtime.ledge_grab = Some(state);
        }
        *app.world_mut().resource_mut::<ControlFrame>() = ControlFrame {
            axis_x: 1.0,
            ..ControlFrame::default()
        };
        app.update();
        let runtime = app.world().resource::<SandboxRuntime>();
        assert!(
            runtime.ledge_grab.expect("state should still exist").climbing,
            "holding toward the platform should start the pull-up"
        );
    }

    #[test]
    fn climb_transition_completes_at_target() {
        let mut app = ledge_app(true);
        let anchor = ae::Vec2::new(120.0, 400.0);
        let target = ae::Vec2::new(150.0, 380.0);
        {
            let mut runtime = app.world_mut().resource_mut::<SandboxRuntime>();
            let mut state = ledge_state(1.0, anchor, target);
            state.climbing = true;
            state.climb_elapsed = LEDGE_CLIMB_TIME;
            runtime.ledge_grab = Some(state);
        }
        app.update();
        let runtime = app.world().resource::<SandboxRuntime>();
        assert!(runtime.ledge_grab.is_none(), "completed climb should clear state");
        assert_eq!(runtime.player.pos, target, "completed climb lands at target");
        assert!(runtime.player.on_ground, "completed climb sets on_ground");
        assert!(!runtime.player.wall_clinging);
        assert!(!runtime.player.on_wall);
    }
}
