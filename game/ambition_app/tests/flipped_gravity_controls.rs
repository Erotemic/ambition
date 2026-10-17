//! Left and right under flipped gravity (Jon, 2026-10-17: "something went
//! wrong with my left/right controls when gravity was flipped upside down", in
//! the C4 symmetry room's top arm).
//!
//! With the shipped input frame (`ScreenRelative`) right is screen-right at any
//! gravity: held right in the top arm, on the ceiling, the body goes +x.

use crate::common::{base, fixed_60hz_room_sim};
use ambition_app::AgentAction;
use ambition_platformer2d::engine_core::InputFrameMode;

/// The top arm (`grav_up`, x 448-832, y 32-400): its floor is the ceiling.
const TOP_ARM: (f32, f32) = (560.0, 120.0);

fn x_after_holding(move_x: f32, mode: Option<InputFrameMode>) -> (f32, f32, (f32, f32)) {
    let mut sim = fixed_60hz_room_sim("symmetry_room");
    if let Some(mode) = mode {
        sim.set_movement_frame_mode(mode);
    }
    sim.set_base_gravity_dir((0.0, -1.0));
    sim.teleport_player(TOP_ARM);
    for _ in 0..30 {
        sim.step(base());
    }
    let before = sim.observation();
    assert_eq!(
        before.gravity_dir,
        (0.0, -1.0),
        "in the top arm gravity is up: {before:?}"
    );
    assert!(before.on_ground, "standing on the ceiling: {before:?}");
    for _ in 0..40 {
        sim.step(AgentAction { move_x, ..base() });
    }
    let after = sim.observation();
    (before.player_pos.0, after.player_pos.0, after.gravity_dir)
}

#[test]
fn right_is_screen_right_upside_down() {
    for mode in [
        None,
        Some(InputFrameMode::ScreenRelative),
        Some(InputFrameMode::BodyRelativeAssist),
    ] {
        let (x0, x1, g) = x_after_holding(1.0, mode);
        assert!(
            x1 > x0 + 40.0,
            "{mode:?}: held right on the ceiling went from x {x0} to {x1} (gravity {g:?})"
        );
        let (x0, x1, g) = x_after_holding(-1.0, mode);
        assert!(
            x1 < x0 - 40.0,
            "{mode:?}: held left on the ceiling went from x {x0} to {x1} (gravity {g:?})"
        );
    }
}

/// Into the top arm through a portal: a wall portal in the down arm, its
/// partner on the wall of the up arm, walked into holding left. The held left
/// is warped through the crossing (it keeps carrying the body out); once
/// released, right and left are screen-right and screen-left on the ceiling.
#[cfg(feature = "portal")]
#[test]
fn right_is_screen_right_after_a_portal_into_up_gravity() {
    use ambition_platformer2d::engine_core::Vec2;
    use ambition_platformer2d::platformer::lifecycle::{InRoomInstance, LiveRoomInstance};
    use ambition_platformer2d::portal::{PlacedPortal, PortalChannel, PortalChannelColor};

    let mut sim = fixed_60hz_room_sim("symmetry_room");
    {
        let world = sim.world_mut();
        let room =
            ambition_platformer2d::session::sole_live_room_component::<LiveRoomInstance>(world)
                .copied()
                .expect("symmetry_room is live");
        let ends = [
            (PortalChannelColor::Teal, Vec2::new(448.0, 1206.0)),
            (PortalChannelColor::Red, Vec2::new(448.0, 80.0)),
        ];
        for (color, at) in ends {
            world.spawn((
                PlacedPortal::fixed(PortalChannel::Authored(color), at, Vec2::X, 46.0),
                InRoomInstance(room),
            ));
        }
    }
    sim.teleport_player((560.0, 1200.0));
    for _ in 0..30 {
        sim.step(base());
    }
    for _ in 0..60 {
        sim.step(AgentAction {
            move_x: -1.0,
            ..base()
        });
    }
    for _ in 0..30 {
        sim.step(base());
    }
    let before = sim.observation();
    assert_eq!(
        before.gravity_dir,
        (0.0, -1.0),
        "the walk went through to the up arm: {before:?}"
    );
    assert!(before.on_ground, "standing on the ceiling: {before:?}");
    for _ in 0..30 {
        sim.step(AgentAction {
            move_x: 1.0,
            ..base()
        });
    }
    let right = sim.observation();
    assert!(
        right.player_pos.0 > before.player_pos.0 + 40.0,
        "held right after the portal: {before:?} -> {right:?}"
    );
    for _ in 0..40 {
        sim.step(AgentAction {
            move_x: -1.0,
            ..base()
        });
    }
    let left = sim.observation();
    assert!(
        left.player_pos.0 < right.player_pos.0 - 40.0,
        "held left after the portal: {right:?} -> {left:?}"
    );
}
