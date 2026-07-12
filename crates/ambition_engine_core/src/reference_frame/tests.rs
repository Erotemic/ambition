//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;

#[test]
fn motion_frame_keeps_acceleration_and_basis_canonical() {
    let frame = MotionFrame::from_acceleration(Vec2::new(300.0, 400.0)).expect("non-zero acceleration");
    assert_eq!(frame.acceleration(), Vec2::new(300.0, 400.0));
    assert!((frame.magnitude() - 500.0).abs() < 1e-5);
    assert!((frame.down() - Vec2::new(0.6, 0.8)).length() < 1e-6);
    assert!((frame.side() - Vec2::new(0.8, -0.6)).length() < 1e-6);

    let local = Vec2::new(17.0, -23.0);
    assert!((frame.to_local(frame.to_world(local)) - local).length() < 1e-5);
}

#[test]
fn zero_acceleration_can_retain_an_authored_reference_direction() {
    let frame = MotionFrame::from_direction(Vec2::X, 0.0);
    assert_eq!(frame.acceleration(), Vec2::ZERO);
    assert_eq!(frame.down(), Vec2::X);
    assert_eq!(frame.side(), Vec2::new(0.0, -1.0));
}

#[test]
fn changing_acceleration_magnitude_does_not_rotate_the_reference_frame() {
    let weak = MotionFrame::from_direction(Vec2::new(-2.0, 3.0), 10.0);
    let strong = MotionFrame::from_direction(Vec2::new(-2.0, 3.0), 900.0);
    assert!((weak.down() - strong.down()).length() < 1e-6);
    assert!((weak.side() - strong.side()).length() < 1e-6);
    assert!((weak.magnitude() - 10.0).abs() < 1e-5);
    assert!((strong.magnitude() - 900.0).abs() < 1e-4);
}

#[test]
fn normal_gravity_is_identity() {
    let f = AccelerationFrame::new(Vec2::new(0.0, 1.0));
    assert_eq!(f.down, Vec2::new(0.0, 1.0));
    assert_eq!(f.side, Vec2::new(1.0, 0.0));
    assert_eq!(f.descend(0.7), 0.7);
    assert_eq!(f.to_world(Vec2::new(3.0, 5.0)), Vec2::new(3.0, 5.0));
    assert_eq!(
        f.to_world_half(Vec2::new(26.0, 34.0)),
        Vec2::new(26.0, 34.0)
    );
}

#[test]
fn inverted_gravity_flips_descend_and_vertical() {
    let f = AccelerationFrame::new(Vec2::new(0.0, -1.0));
    // Holding screen-up (axis_y = -1) is "descend" (toward the up-feet).
    assert_eq!(f.descend(-1.0), 1.0);
    // A local-body "toward feet" offset (+y) maps to screen-up.
    assert_eq!(f.to_world(Vec2::new(0.0, 32.0)), Vec2::new(0.0, -32.0));
    // Vertical half-extent unchanged (still a 180° frame, no axis swap).
    assert_eq!(
        f.to_world_half(Vec2::new(26.0, 34.0)),
        Vec2::new(26.0, 34.0)
    );
}

#[test]
fn sideways_gravity_swaps_axes() {
    let f = AccelerationFrame::new(Vec2::new(1.0, 0.0)); // gravity points screen-right
    assert_eq!(f.down, Vec2::new(1.0, 0.0));
    // Toward-feet (+y player) maps to screen-right.
    assert_eq!(f.to_world(Vec2::new(0.0, 32.0)), Vec2::new(32.0, 0.0));
    // A wide-thin down-attack box becomes thin-wide in world.
    assert_eq!(
        f.to_world_half(Vec2::new(26.0, 34.0)),
        Vec2::new(34.0, 26.0)
    );
}

#[test]
fn off_axis_down_is_a_general_rotation() {
    // A 45° "down" (toward screen down-right) is not snapped — the frame is a
    // real rotation, so toward-feet maps along the diagonal.
    let f = AccelerationFrame::new(Vec2::new(1.0, 1.0));
    let inv_sqrt2 = 1.0 / 2.0_f32.sqrt();
    assert!((f.down - Vec2::new(inv_sqrt2, inv_sqrt2)).length() < 1e-6);
    let feet = f.to_world(Vec2::new(0.0, 10.0));
    assert!((feet - Vec2::new(10.0 * inv_sqrt2, 10.0 * inv_sqrt2)).length() < 1e-5);
}

#[test]
fn hybrid_control_frame_rotates_to_90_then_reverts() {
    // Right gravity (≤90°): the control frame follows the player, so "right"
    // on the stick maps to screen-up (the player's right).
    let right = AccelerationFrame::new(Vec2::new(1.0, 0.0));
    let cf = right.control_frame(InputFrameMode::BodyRelativeAssist);
    let world = cf.to_world(Vec2::new(1.0, 0.0));
    assert!((world - Vec2::new(0.0, -1.0)).length() < 1e-6, "{world:?}");
    // Up gravity (>90°): the control frame reverts to screen, so "right" maps
    // to screen-right (= the player's left — the accommodation).
    let up = AccelerationFrame::new(Vec2::new(0.0, -1.0));
    let cf = up.control_frame(InputFrameMode::BodyRelativeAssist);
    assert_eq!(cf.to_world(Vec2::new(1.0, 0.0)), Vec2::new(1.0, 0.0));
    // Player mode never reverts; Screen mode never rotates.
    assert_eq!(up.control_frame(InputFrameMode::BodyRelativeStrict), up);
    assert_eq!(
        up.control_frame(InputFrameMode::ScreenRelative).down,
        Vec2::new(0.0, 1.0)
    );
}

// The four cardinal gravities, as (name, down) pairs.
const CARDINALS: [(&str, Vec2); 4] = [
    ("down", Vec2::new(0.0, 1.0)),
    ("right", Vec2::new(1.0, 0.0)),
    ("up", Vec2::new(0.0, -1.0)),
    ("left", Vec2::new(-1.0, 0.0)),
];

#[test]
fn hybrid_resolve_input_matches_the_legacy_run_and_descend_at_every_orientation() {
    // Hybrid MUST stay byte-identical to the old seam (the replay guard only
    // covers normal gravity, so pin all four here). Old run: drive
    // `control_frame(Hybrid).side` by `axis_x`. Old descend: `descend(axis_y)`.
    for (name, down) in CARDINALS {
        let f = AccelerationFrame::new(down);
        for &(ax, ay) in &[
            (1.0, 0.0),
            (-0.4, 0.0),
            (0.0, 0.7),
            (0.0, -1.0),
            (0.6, -0.3),
        ] {
            let r = f.resolve_input(InputFrameMode::BodyRelativeAssist, ax, ay);
            // Run: world velocity direction must match the legacy basis * axis_x.
            let legacy_run = f.control_frame(InputFrameMode::BodyRelativeAssist).side * ax;
            let new_run = f.side * r.x;
            assert!(
                (legacy_run - new_run).length() < 1e-6,
                "{name}: run mismatch ax={ax} -> legacy {legacy_run:?} new {new_run:?}"
            );
            // Descend gate scalar must match `descend(axis_y)` exactly.
            assert!(
                (f.descend(ay) - r.y).abs() < 1e-6,
                "{name}: descend mismatch ay={ay}"
            );
        }
    }
}

#[test]
fn screen_mode_is_screen_relative_at_every_orientation() {
    // In Screen mode the body moves the way the stick points ON SCREEN: the
    // world movement direction equals the raw input vector regardless of
    // gravity (to_world ∘ resolve_input == identity on the screen vector).
    for (name, down) in CARDINALS {
        let f = AccelerationFrame::new(down);
        for &(ax, ay) in &[
            (1.0, 0.0),
            (0.0, 1.0),
            (-1.0, 0.0),
            (0.0, -1.0),
            (0.5, -0.5),
        ] {
            let world = f.to_world(f.resolve_input(InputFrameMode::ScreenRelative, ax, ay));
            assert!(
                (world - Vec2::new(ax, ay)).length() < 1e-6,
                "{name}: screen input ({ax},{ay}) should move screen-relative, got {world:?}"
            );
        }
    }
}

#[test]
fn screen_mode_matches_the_authored_quadrant_spec() {
    // The exact mapping Jon specified. Gravity RIGHT (player's feet point
    // screen-right): run = +side (screen-up), descend = +down (screen-right).
    let right = AccelerationFrame::new(Vec2::new(1.0, 0.0));
    let r = |ax, ay| right.resolve_input(InputFrameMode::ScreenRelative, ax, ay);
    assert_eq!(
        r(1.0, 0.0),
        Vec2::new(0.0, 1.0),
        "input-right -> player-down"
    );
    assert_eq!(
        r(0.0, -1.0),
        Vec2::new(1.0, 0.0),
        "input-up -> player-right"
    );
    assert_eq!(
        r(-1.0, 0.0),
        Vec2::new(0.0, -1.0),
        "input-left -> player-up"
    );
    assert_eq!(
        r(0.0, 1.0),
        Vec2::new(-1.0, 0.0),
        "input-down -> player-left"
    );

    // Gravity LEFT (feet point screen-left).
    let left = AccelerationFrame::new(Vec2::new(-1.0, 0.0));
    let l = |ax, ay| left.resolve_input(InputFrameMode::ScreenRelative, ax, ay);
    assert_eq!(
        l(-1.0, 0.0),
        Vec2::new(0.0, 1.0),
        "input-left -> player-down"
    );
    assert_eq!(
        l(0.0, 1.0),
        Vec2::new(1.0, 0.0),
        "input-down -> player-right"
    );
    assert_eq!(
        l(0.0, -1.0),
        Vec2::new(-1.0, 0.0),
        "input-up -> player-left"
    );
    assert_eq!(
        l(1.0, 0.0),
        Vec2::new(0.0, -1.0),
        "input-right -> player-up"
    );
}

#[test]
fn inverse_mapping_places_local_labels_on_raw_joystick_directions() {
    let right = AccelerationFrame::new(Vec2::new(1.0, 0.0));
    assert_eq!(
        right.raw_axis_for_resolved_input(InputFrameMode::ScreenRelative, Vec2::new(0.0, 1.0)),
        Vec2::new(1.0, 0.0),
        "screen-directed: local down labels raw right when feet point screen-right"
    );
    assert_eq!(
        right.raw_axis_for_resolved_input(InputFrameMode::BodyRelativeAssist, Vec2::new(0.0, 1.0)),
        Vec2::new(0.0, 1.0),
        "body-relative assist: local down stays on raw down for side gravity"
    );

    let up = AccelerationFrame::new(Vec2::new(0.0, -1.0));
    assert_eq!(
        up.raw_axis_for_resolved_input(InputFrameMode::BodyRelativeAssist, Vec2::new(0.0, 1.0)),
        Vec2::new(0.0, -1.0),
        "body-relative assist flips only when inverted"
    );
}

#[test]
fn local_edge_mapping_uses_the_same_inverse_mapping() {
    let right = AccelerationFrame::new(Vec2::new(1.0, 0.0));
    let edges = RawDirectionEdges::new(false, true, false, false); // raw right edge
    let resolved = right.resolve_control(InputFrameMode::ScreenRelative, 1.0, 0.0);
    assert!(resolved.local_down_pressed(edges));
    assert!(!resolved.local_up_pressed(edges));

    let hybrid = right.resolve_control(InputFrameMode::BodyRelativeAssist, 1.0, 0.0);
    assert!(!hybrid.local_down_pressed(edges));
}

#[test]
fn cardinalized_acceleration_uses_four_control_cones() {
    assert_eq!(
        AccelerationFrame::nearest_cardinal_down(Vec2::new(0.2, 0.9)),
        Vec2::new(0.0, 1.0)
    );
    assert_eq!(
        AccelerationFrame::nearest_cardinal_down(Vec2::new(0.9, 0.2)),
        Vec2::new(1.0, 0.0)
    );
    assert_eq!(
        AccelerationFrame::nearest_cardinal_down(Vec2::new(-0.8, 0.1)),
        Vec2::new(-1.0, 0.0)
    );
}

#[test]
fn player_mode_is_the_raw_stick_in_the_player_frame() {
    // Player mode never accommodates: the stick IS the local body frame.
    let up = AccelerationFrame::new(Vec2::new(0.0, -1.0));
    assert_eq!(
        up.resolve_input(InputFrameMode::BodyRelativeStrict, 0.3, -0.7),
        Vec2::new(0.3, -0.7)
    );
}

#[test]
fn resolve_aim_local_picks_frame_by_source_under_flipped_gravity() {
    // Upside-down gravity: feet point up (screen). The aim stick uses the AIM
    // policy, the movement stick uses the MOVEMENT policy, independently.
    let up = AccelerationFrame::new(Vec2::new(0.0, -1.0));
    let modes = ControlFrameModes {
        movement: InputFrameMode::BodyRelativeStrict, // strict body-relative locomotion
        aim: InputFrameMode::ScreenRelative,          // screen-directed precision aim
    };

    // Aim stick pushed screen-up (-y). Screen aim → world stays screen-up
    // regardless of gravity: to_world(resolve) == (0,-1).
    let aim_local = up.resolve_aim_local(modes, Vec2::new(0.0, -1.0), Vec2::ZERO, 1.0);
    assert_eq!(up.to_world(aim_local), Vec2::new(0.0, -1.0));

    // No aim, movement stick pushed screen-up (-y). Player movement → the
    // stick IS the body frame, so world = side*0 + down*(-1) = -down = (0,1).
    let move_local = up.resolve_aim_local(modes, Vec2::ZERO, Vec2::new(0.0, -1.0), 1.0);
    assert_eq!(up.to_world(move_local), Vec2::new(0.0, 1.0));

    // Neither stick → body-local facing (+x), gravity-independent in local frame.
    let facing_local = up.resolve_aim_local(modes, Vec2::ZERO, Vec2::ZERO, -1.0);
    assert_eq!(facing_local, Vec2::new(-1.0, 0.0));
}

#[test]
fn control_frame_modes_default_is_screen_relative_both() {
    let d = ControlFrameModes::default();
    assert_eq!(d.movement, InputFrameMode::ScreenRelative);
    assert_eq!(d.aim, InputFrameMode::ScreenRelative);
}

#[test]
fn launch_is_away_from_feet() {
    let f = AccelerationFrame::new(Vec2::new(0.0, -1.0)); // up gravity
    let mut v = Vec2::new(5.0, 0.0);
    f.launch(&mut v, 600.0);
    // Away from up-feet = screen-down (+y); perpendicular x preserved.
    assert_eq!(v, Vec2::new(5.0, 600.0));
}
