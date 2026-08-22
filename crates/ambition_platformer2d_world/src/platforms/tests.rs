//! Unit tests for moving-platform sweep/path motion, riding, and ledge-carry.

use super::*;
use ambition_platformer2d_core::AabbExt;

fn test_world() -> ae::World {
    ae::World::new(
        "test",
        ae::Vec2::new(2000.0, 2000.0),
        ae::Vec2::new(100.0, 100.0),
        Vec::new(),
    )
}

fn sample_platform() -> MovingPlatformState {
    MovingPlatformState::from_authored(
        ae::Vec2::new(400.0, 800.0),
        ae::Vec2::new(155.0, 18.0),
        240.0,
        130.0,
    )
}

fn test_room_with_platforms(
    world: ae::World,
    platforms: Vec<MovingPlatformState>,
) -> crate::rooms::RoomSpec {
    //  built through the constructor, not spelled field by field. The
    // exhaustive literal this replaces had to be edited every time the room IR
    // grew a family, and it says nothing about platforms by listing fifteen
    // empty vectors — the ONE field this fixture cares about was buried among
    // them. `RoomSpec::new` is the room-with-nothing-authored, which is exactly
    // what a platform test wants underneath its platforms.
    let mut room = crate::rooms::RoomSpec::new("test", world);
    room.moving_platforms = platforms;
    room
}

#[test]
fn moving_platforms_for_room_returns_all_authored_ldtk_platforms() {
    let world = test_world();
    let authored = sample_platform();
    let second = MovingPlatformState::from_authored(
        ae::Vec2::new(700.0, 900.0),
        ae::Vec2::new(96.0, 16.0),
        -120.0,
        60.0,
    );
    let room = test_room_with_platforms(world, vec![authored.clone(), second.clone()]);
    let selected = moving_platforms_for_room(&room);
    assert_eq!(selected, vec![authored, second]);
}

#[test]
fn moving_platforms_for_room_empty_when_room_has_no_authored_platforms() {
    let world = test_world();
    let room = test_room_with_platforms(world, Vec::new());
    assert!(moving_platforms_for_room(&room).is_empty());
}

#[test]
fn moving_platform_update_swings_between_min_and_max() {
    let mut platform = sample_platform();
    let initial_x = platform.pos.x;
    // Many ticks at +x direction: platform reaches max_x and flips.
    for _ in 0..600 {
        let _ = platform.update(0.05);
        // Position must always stay within [min_x, max_x].
        assert!(platform.pos.x >= initial_x - 1.0);
    }
    // After enough time it must have flipped at least once.
    assert!(platform.direction() == 1.0 || platform.direction() == -1.0);
}

#[test]
fn moving_platform_update_returns_displacement() {
    let mut platform = sample_platform();
    let dt = 1.0 / 60.0;
    let delta = platform.update(dt);
    // Initial direction is +1, speed = 130 px/s, dt = 1/60.
    // So displacement.x ≈ 130 / 60 ≈ 2.17 px.
    assert!((delta.x - 130.0 * dt).abs() < 1e-3);
    assert_eq!(delta.y, 0.0);
}

#[test]
fn moving_platform_aabb_centered_on_pos() {
    let platform = sample_platform();
    let aabb = platform.aabb();
    assert_eq!(aabb.center(), platform.pos);
}

#[test]
fn moving_platform_as_collision_block_is_blink_wall_soft() {
    let platform = sample_platform();
    let block = platform.as_collision_block();
    // Soft blink wall — solid for collision but blink-passable
    // when soft-blink-through is unlocked.
    assert!(matches!(
        block.kind,
        ae::BlockKind::BlinkWall {
            tier: ae::BlinkWallTier::Soft,
        }
    ));
}

#[test]
fn world_with_moving_platforms_appends_all_blocks() {
    let world = test_world();
    let platform = sample_platform();
    let second = MovingPlatformState::from_authored(
        ae::Vec2::new(700.0, 900.0),
        ae::Vec2::new(96.0, 16.0),
        120.0,
        60.0,
    );
    let extended = world_with_moving_platforms(&world, &[platform, second]);
    assert_eq!(extended.blocks.len(), world.blocks.len() + 2);
}

#[test]
fn path_driven_platform_advances_along_authored_path() {
    let path = ambition_platformer2d_core::KinematicPath {
        points: vec![ae::Vec2::new(100.0, 200.0), ae::Vec2::new(180.0, 200.0)],
        speed: 80.0,
        mode: ambition_platformer2d_core::KinematicPathMode::PingPong,
        start_offset_seconds: 0.0,
    };
    let mut platform =
        MovingPlatformState::from_path("lift_a", "Lift A", ae::Vec2::new(64.0, 16.0), path);
    assert_eq!(platform.pos, ae::Vec2::new(100.0, 200.0));
    let delta = platform.update(0.5);
    assert_eq!(delta, ae::Vec2::new(40.0, 0.0));
    assert_eq!(platform.pos, ae::Vec2::new(140.0, 200.0));
}

#[test]
fn moving_platform_spec_resolves_path_id_against_room_paths() {
    let path = ambition_platformer2d_core::KinematicPath {
        points: vec![ae::Vec2::new(20.0, 30.0), ae::Vec2::new(120.0, 30.0)],
        speed: 50.0,
        mode: ambition_platformer2d_core::KinematicPathMode::PingPong,
        start_offset_seconds: 0.0,
    };
    let spec = KinematicPathSpec::new(
        "intro_lift_path",
        "Intro Lift Path",
        ae::Aabb::new(ae::Vec2::new(20.0, 30.0), ae::Vec2::new(8.0, 8.0)),
        path,
    );
    let platform = MovingPlatformSpec::new(
        "lift",
        "Lift",
        ae::Vec2::new(999.0, 999.0),
        ae::Vec2::new(80.0, 16.0),
        MovingPlatformMotionSpec::Path {
            path_id: "intro_lift_path".into(),
        },
    )
    .resolve(&[spec])
    .expect("path resolves");
    assert_eq!(platform.pos, ae::Vec2::new(20.0, 30.0));
}

#[test]
fn moving_platform_support_detection_is_gravity_relative() {
    let platform = MovingPlatformState::from_sweep(
        "support_platform",
        "Support Platform",
        ae::Vec2::new(100.0, 100.0),
        ae::Vec2::new(80.0, 20.0),
        120.0,
        60.0,
    );
    let body_size = ae::Vec2::new(30.0, 48.0);
    for gravity_dir in [
        ae::Vec2::new(0.0, 1.0),
        ae::Vec2::new(1.0, 0.0),
        ae::Vec2::new(0.0, -1.0),
        ae::Vec2::new(-1.0, 0.0),
    ] {
        let body = body_supported_by(platform.aabb(), body_size, gravity_dir, 0.0);
        assert!(
            platform.is_supporting_body(body, true, gravity_dir),
            "platform should support body under gravity {gravity_dir:?}"
        );
        let frame = ae::AccelerationFrame::new(gravity_dir);
        let shifted = body.translated(frame.side * 200.0);
        assert!(
            !platform.is_supporting_body(shifted, true, gravity_dir),
            "side-separated body should not be reported as riding under gravity {gravity_dir:?}"
        );
    }
}

fn body_supported_by(
    support: ae::Aabb,
    body_size: ae::Vec2,
    gravity_dir: ae::Vec2,
    side_offset: f32,
) -> ae::Aabb {
    let frame = ae::AccelerationFrame::new(gravity_dir);
    let body_half = body_size * 0.5;
    let support_center = support.center();
    let support_half = support.half_size();
    let support_side = support_center.dot(frame.side);
    let support_down = support_center.dot(frame.down);
    let support_down_half = projected_half_for_test(support_half, frame.down);
    let body_down_half = projected_half_for_test(body_half, frame.down);
    let support_head = support_down - support_down_half;
    let body_center_side = support_side + side_offset;
    let body_center_down = support_head - body_down_half;
    let body_center = frame.side * body_center_side + frame.down * body_center_down;
    ae::Aabb::new(body_center, body_half)
}

fn projected_half_for_test(half: ae::Vec2, axis: ae::Vec2) -> f32 {
    half.x * axis.x.abs() + half.y * axis.y.abs()
}

// ── Looping paths ─────────────────────────────────────────────────────────

/// A two-point `Loop` must keep circulating without stalling on a zero-distance
/// target. `advance_path_position` terminates if an advance leaves the cursor
/// unchanged, and loop indexing closes the circuit back to the first point.
#[test]
fn a_two_point_looping_path_circulates_instead_of_spinning() {
    let bottom = ae::Vec2::new(0.0, 400.0);
    let top = ae::Vec2::new(0.0, 100.0);
    let mut platform = MovingPlatformState::from_path(
        "elevator",
        "elevator",
        ae::Vec2::new(64.0, 16.0),
        ambition_platformer2d_core::KinematicPath {
            points: vec![bottom, top],
            speed: 600.0,
            mode: ambition_platformer2d_core::KinematicPathMode::Loop,
            start_offset_seconds: 0.0,
        },
    );

    let dt = 1.0 / 60.0;
    let step = 600.0 * dt;
    let mut near_top = false;
    let mut back_near_bottom = false;
    // 120 frames is four traverses of the 300px column.
    for _ in 0..120 {
        platform.update(dt);
        if (platform.pos - top).length() <= step {
            near_top = true;
        }
        if near_top && (platform.pos - bottom).length() <= step {
            back_near_bottom = true;
        }
    }

    assert!(near_top, "it should climb the column: {:?}", platform.pos);
    assert!(
        back_near_bottom,
        "and come back round — a `Loop` that stops at the far end is the bug this \
         test exists for: {:?}",
        platform.pos
    );
}

/// `Loop` closes the circuit as `p0 → p1 → p2 → p0`, rather than retracing the
/// last open segment.
///
///  the tolerance is one FRAME of travel, not a hair: the platform moves in
/// 10px steps and will step straight past a waypoint rather than land on it. An
/// earlier version of this test used `< 1.0` and reported "never returned" for a
/// platform that was passing through the corner every lap.
#[test]
fn loop_mode_closes_the_circuit_back_to_its_first_point() {
    let a = ae::Vec2::new(0.0, 0.0);
    let b = ae::Vec2::new(300.0, 0.0);
    let c = ae::Vec2::new(300.0, 300.0);
    let mut platform = MovingPlatformState::from_path(
        "circuit",
        "circuit",
        ae::Vec2::new(64.0, 16.0),
        ambition_platformer2d_core::KinematicPath {
            points: vec![a, b, c],
            speed: 600.0,
            mode: ambition_platformer2d_core::KinematicPathMode::Loop,
            start_offset_seconds: 0.0,
        },
    );

    let dt = 1.0 / 60.0;
    let step = 600.0 * dt;
    let mut reached_c = false;
    let mut returned_to_a = false;
    for _ in 0..600 {
        platform.update(dt);
        if (platform.pos - c).length() <= step {
            reached_c = true;
        }
        // Only counts AFTER the far corner: the platform starts at `a`.
        if reached_c && (platform.pos - a).length() <= step {
            returned_to_a = true;
        }
    }

    assert!(reached_c, "the platform should traverse the whole path");
    assert!(
        returned_to_a,
        "and close the circuit back to its first point: {:?}",
        platform.pos
    );
}

/// A wrapping platform must not fling whoever is standing on it.
///
/// The wrap teleport is a position change, not rider movement. `last_delta` is
/// exactly the quantity the
/// per-body tick adds to a rider (`body_integration.rs` reads it for
/// platform-ride and ledge-carry). Reporting `pos - old` across a wrap hands the
/// rider the whole span in one frame — the height of the shaft, in one tick, in
/// the direction opposite to travel.
///
///  the honest test of a wrap is the frame it happens on, not the frames
/// either side, and it is a frame the naive implementation gets wrong while
/// looking completely correct in a position trace: the platform IS where it
/// should be. Only the carried rider reveals it.
#[test]
fn a_wrapping_platform_carries_a_rider_by_its_travel_not_by_its_teleport() {
    // A shaft 300 tall, descending at 100/s. dt of 0.5 puts the wrap squarely
    // inside a step rather than exactly on the boundary.
    let mut platform = MovingPlatformState::from_vertical_loop(
        "lift",
        "Lift",
        ae::Vec2::new(0.0, 40.0),
        ae::Vec2::new(96.0, 16.0),
        0.0,
        300.0,
        100.0,
        false,
    );

    // One ordinary step: no wrap, and the delta is the travel.
    let delta = platform.update(0.2);
    assert!(
        (delta.y + 20.0).abs() < 1e-3,
        "an ordinary descending step carries the rider down 20px, got {delta:?}"
    );

    // The step that crosses the bottom and reappears at the top.
    let before = platform.pos.y;
    let delta = platform.update(0.5);
    assert!(
        platform.pos.y > before,
        "precondition: this step wrapped — the platform reappeared at the top \
         ({before} -> {})",
        platform.pos.y
    );
    assert!(
        (delta.y + 50.0).abs() < 1e-3,
        "the wrap frame reported {delta:?} of carry, but the platform only \
         TRAVELLED 50px down — the rest is a teleport, and handing it to a rider \
         throws them the length of the shaft in one tick"
    );
}

/// A looping platform never turns around.
///
///  reads as "rising" in the assertions below only in the +y sense; +y is DOWN
/// on screen. What is being pinned is that the sign never changes, not which way
/// the player sees it go.
///
///  the poison for the variant existing at all: if it reversed it would be a
/// `Sweep` on the other axis, and the elevator effect — step off the top, the
/// next one arrives from below — would not exist. Two full spans of travel must
/// leave the direction unchanged.
#[test]
fn a_looping_platform_keeps_going_the_same_way_forever() {
    let mut platform = MovingPlatformState::from_vertical_loop(
        "lift",
        "Lift",
        ae::Vec2::new(0.0, 0.0),
        ae::Vec2::new(96.0, 16.0),
        0.0,
        200.0,
        100.0,
        true,
    );
    for _ in 0..40 {
        let delta = platform.update(0.1);
        assert!(
            delta.y > 0.0,
            "a rising loop reported downward carry ({delta:?}) — either it \
             reversed, which would make it a lift rather than a paternoster, or a \
             wrap leaked into the carry"
        );
        assert!(
            (0.0..=200.0).contains(&platform.pos.y),
            "the platform left its shaft at {}",
            platform.pos.y
        );
    }
    assert!(
        platform.direction() > 0.0,
        "and it still reports the direction it was authored with"
    );
}

/// An authored `loop_dy` produces a platform that WRAPS.
///
/// The authoring half of the elevator.  the tell that it is wired is not that
/// the platform moves vertically — a `Path` does that, and so would a sweep on
/// the wrong axis — it is that the platform comes back to where it started
/// while still travelling the same way. A reversing platform also returns to
/// its start, so the direction check is what separates the two.
#[test]
fn an_authored_vertical_loop_wraps_instead_of_reversing() {
    let spec = MovingPlatformSpec::new(
        "shaft_lift",
        "Shaft Lift",
        ae::Vec2::new(0.0, 100.0),
        ae::Vec2::new(96.0, 16.0),
        MovingPlatformMotionSpec::VerticalLoop {
            dy: 200.0,
            anchor_y: None,
            speed: 100.0,
        },
    );

    let mut platform = spec.resolve(&[]).expect("a loop spec resolves");
    assert!(platform.direction() > 0.0, "a positive loop_dy rises");

    //  the wrap is a DROP in y on a platform that is rising, not a return
    // below the start: the shaft here begins at its own floor, so a wrap lands
    // just above where it began. Comparing against `start` would never fire, and
    // the test would pass a platform that ran off up the shaft forever.
    let mut previous = platform.pos.y;
    let mut wrapped = false;
    for _ in 0..40 {
        let delta = platform.update(0.1);
        assert!(
            delta.x.abs() < 1e-6,
            "the authored sweep_dx leaked into a loop platform ({delta:?}) — a \
             loop is not a sweep with an extra field"
        );
        assert!(
            delta.y > 0.0,
            "a rising loop never carries downward, so it never reversed: {delta:?}"
        );
        if platform.pos.y < previous {
            wrapped = true;
        }
        previous = platform.pos.y;
        assert!(
            (100.0..=300.0).contains(&platform.pos.y),
            "the platform left its authored shaft at {}",
            platform.pos.y
        );
    }
    assert!(
        wrapped,
        "the platform never dropped back down the shaft, so it never wrapped — \
         it is running off upward rather than looping"
    );
}

/// A staggered run of platforms shares ONE shaft.
///
///  this is the difference between a conveyor and three unrelated lifts, and it is invisible
/// on the first frame.
///
///  the assertion is that every platform stays inside the SHARED shaft, not
/// that they are evenly spaced. Even spacing is what the author wrote; staying
/// in one shaft is what makes it stay true.
#[test]
fn a_staggered_run_of_looping_platforms_shares_one_shaft() {
    const BASE: f32 = 100.0;
    const SPAN: f32 = 300.0;

    // Three platforms at thirds of the shaft — the conveyor an author writes.
    let mut run: Vec<MovingPlatformState> = [0.0, 100.0, 200.0]
        .into_iter()
        .enumerate()
        .map(|(i, phase)| {
            MovingPlatformSpec::new(
                format!("lift_{i}"),
                format!("Lift {i}"),
                ae::Vec2::new(0.0, BASE + phase),
                ae::Vec2::new(96.0, 16.0),
                MovingPlatformMotionSpec::VerticalLoop {
                    dy: SPAN,
                    anchor_y: Some(BASE),
                    speed: 100.0,
                },
            )
            .resolve(&[])
            .expect("a conveyor spec resolves")
        })
        .collect();

    for step in 0..60 {
        for platform in &mut run {
            platform.update(0.1);
            assert!(
                (BASE..=BASE + SPAN).contains(&platform.pos.y),
                "step {step}: '{}' left the shared shaft at {} (shaft is \
                 {BASE}..={}) — its shaft is anchored at itself, so the run is \
                 three lifts rather than one conveyor",
                platform.id,
                platform.pos.y,
                BASE + SPAN
            );
        }
    }

    // And they are still three distinct platforms, not a pile.
    let mut ys: Vec<f32> = run.iter().map(|p| p.pos.y).collect();
    ys.sort_by(|a, b| a.partial_cmp(b).unwrap());
    for pair in ys.windows(2) {
        assert!(
            (pair[1] - pair[0]) > 1.0,
            "two platforms converged to {pair:?} — the stagger the author wrote \
             has been lost, so the shaft has a gap somewhere else"
        );
    }
}

/// Every motion an author can write reaches exactly one variant.
///
/// The two shapes at the top are the ones content actually authors today (a
/// swept platform in the sandbox, an anchored conveyor in Mary-O 1-2); the rest
/// are the legal forms the editor allows. This is the non-vacuity half of the
/// refusal test below — a classifier that rejected everything would pass that
/// one and fail this one.
#[test]
fn authored_motion_fields_classify_to_exactly_one_motion() {
    let sweep = AuthoredPlatformMotion {
        sweep_dx: Some(320.0),
        speed: Some(90.0),
        ..Default::default()
    };
    assert_eq!(
        sweep.classify(),
        Ok(MovingPlatformMotionSpec::Sweep {
            dx: 320.0,
            speed: 90.0
        })
    );

    let conveyor = AuthoredPlatformMotion {
        speed: Some(60.0),
        loop_dy: Some(-300.0),
        loop_anchor_y: Some(100.0),
        ..Default::default()
    };
    assert_eq!(
        conveyor.classify(),
        Ok(MovingPlatformMotionSpec::VerticalLoop {
            dy: -300.0,
            anchor_y: Some(100.0),
            speed: 60.0
        })
    );

    let path = AuthoredPlatformMotion {
        path_id: Some("  lab_lift  ".into()),
        ..Default::default()
    };
    assert_eq!(
        path.classify(),
        Ok(MovingPlatformMotionSpec::Path {
            path_id: "lab_lift".into()
        }),
        "an editor field carries whatever whitespace the author typed"
    );

    // Placing a platform and stating nothing is legal, and means a platform that
    // moves. An empty `path_id` is the same as no `path_id`, because LDtk writes
    // one for a field the author never filled in.
    assert_eq!(
        AuthoredPlatformMotion::default().classify(),
        Ok(MovingPlatformMotionSpec::Sweep {
            dx: DEFAULT_SWEEP_DX,
            speed: DEFAULT_PLATFORM_SPEED
        })
    );
    assert_eq!(
        AuthoredPlatformMotion {
            path_id: Some("   ".into()),
            ..Default::default()
        }
        .classify(),
        Ok(MovingPlatformMotionSpec::Sweep {
            dx: DEFAULT_SWEEP_DX,
            speed: DEFAULT_PLATFORM_SPEED
        })
    );
}

/// Ambiguous motion authoring is refused rather than resolved by precedence;
/// diagnostics name the conflicting fields.
#[test]
fn authoring_two_motions_at_once_is_refused_rather_than_ranked() {
    let both = AuthoredPlatformMotion {
        sweep_dx: Some(240.0),
        loop_dy: Some(200.0),
        ..Default::default()
    }
    .classify()
    .expect_err("a sweep and a loop are two motions");
    assert!(
        both.contains("sweep_dx") && both.contains("loop_dy"),
        "the refusal must name both fields, or the author cannot act on it: {both}"
    );

    let anchor_alone = AuthoredPlatformMotion {
        loop_anchor_y: Some(100.0),
        ..Default::default()
    }
    .classify()
    .expect_err("an anchor describes no motion by itself");
    assert!(anchor_alone.contains("loop_min_y"), "{anchor_alone}");

    let flat_shaft = AuthoredPlatformMotion {
        loop_dy: Some(0.0),
        ..Default::default()
    }
    .classify()
    .expect_err("a shaft with no span never moves");
    assert!(flat_shaft.contains("loop_dy"), "{flat_shaft}");

    // A path carries its own speed, so a `speed` here does nothing at all.
    let path_speed = AuthoredPlatformMotion {
        path_id: Some("lab_lift".into()),
        speed: Some(90.0),
        ..Default::default()
    }
    .classify()
    .expect_err("the path owns the speed");
    assert!(path_speed.contains("speed"), "{path_speed}");
}
