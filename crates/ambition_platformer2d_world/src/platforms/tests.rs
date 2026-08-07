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
    crate::rooms::RoomSpec {
        id: "test".into(),
        world,
        loading_zones: Vec::new(),
        metadata: crate::rooms::RoomMetadata::default(),
        camera_zones: Vec::new(),
        kinematic_paths: Vec::new(),
        moving_platforms: platforms,
        props: Vec::new(),
        ground_items: Vec::new(),
        portal_gun_spawns: Vec::new(),
        shrines: Vec::new(),
        gravity_zones: Vec::new(),
        enemy_spawns: Vec::new(),
        boss_spawns: Vec::new(),
        debug_labels: Vec::new(),
        mount_links: Vec::new(),
        placements: Vec::new(),
    }
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
fn moving_platform_matches_ledge_contact_on_its_edge() {
    let platform = MovingPlatformState::from_sweep(
        "ledge_platform",
        "Ledge Platform",
        ae::Vec2::new(100.0, 100.0),
        ae::Vec2::new(80.0, 20.0),
        120.0,
        60.0,
    );
    let player_size = ae::Vec2::new(28.0, 46.0);
    let half = player_size * 0.5;
    let wall_normal_x = -1.0;
    let left_edge = platform.aabb().left();
    let top = platform.aabb().top();
    let contact = ae::LedgeContact {
        wall_normal_x,
        anchor: ae::Vec2::new(
            left_edge + wall_normal_x * (half.x - 1.0),
            top + half.y - 4.0,
        ),
        climb_target: ae::Vec2::new(
            left_edge - wall_normal_x * (half.x + 4.0),
            top - half.y - 1.0,
        ),
    };

    assert!(
        platform.matches_ledge_contact(contact, player_size),
        "ledge contacts produced from the moving-platform block should match the platform"
    );
}

#[test]
fn moving_platform_rejects_unrelated_ledge_contact() {
    let platform = MovingPlatformState::from_sweep(
        "ledge_platform",
        "Ledge Platform",
        ae::Vec2::new(100.0, 100.0),
        ae::Vec2::new(80.0, 20.0),
        120.0,
        60.0,
    );
    let player_size = ae::Vec2::new(28.0, 46.0);
    let half = player_size * 0.5;
    let wall_normal_x = -1.0;
    let left_edge = platform.aabb().left();
    let other_top = platform.aabb().top() - 64.0;
    let contact = ae::LedgeContact {
        wall_normal_x,
        anchor: ae::Vec2::new(
            left_edge + wall_normal_x * (half.x - 1.0),
            other_top + half.y - 4.0,
        ),
        climb_target: ae::Vec2::new(
            left_edge - wall_normal_x * (half.x + 4.0),
            other_top - half.y - 1.0,
        ),
    };

    assert!(
        !platform.matches_ledge_contact(contact, player_size),
        "ledge contacts on unrelated blocks should not inherit this platform's motion"
    );
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
    let platform = MovingPlatformSpec::from_authored(
        "lift",
        "Lift",
        ae::Vec2::new(999.0, 999.0),
        ae::Vec2::new(80.0, 16.0),
        400.0,
        10.0,
        Some("intro_lift_path".into()),
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

#[test]
fn moving_platform_ledge_contact_matching_is_gravity_relative() {
    let platform = MovingPlatformState::from_sweep(
        "ledge_platform",
        "Ledge Platform",
        ae::Vec2::new(100.0, 100.0),
        ae::Vec2::new(80.0, 20.0),
        120.0,
        60.0,
    );
    let player_size = ae::Vec2::new(28.0, 46.0);
    for gravity_dir in [
        ae::Vec2::new(0.0, 1.0),
        ae::Vec2::new(1.0, 0.0),
        ae::Vec2::new(0.0, -1.0),
        ae::Vec2::new(-1.0, 0.0),
    ] {
        for side_normal in [-1.0, 1.0] {
            let contact =
                ledge_contact_for_platform(platform.aabb(), player_size, gravity_dir, side_normal);
            assert!(
                platform.matches_ledge_contact_in_frame(contact, player_size, gravity_dir),
                "ledge contact should match under gravity {gravity_dir:?} side {side_normal}"
            );
        }
    }
}

#[test]
fn moving_platform_ledge_contact_matches_previous_aabb_after_advance() {
    let mut platform = MovingPlatformState::from_sweep(
        "ledge_platform",
        "Ledge Platform",
        ae::Vec2::new(100.0, 100.0),
        ae::Vec2::new(80.0, 20.0),
        120.0,
        60.0,
    );
    let player_size = ae::Vec2::new(28.0, 46.0);
    let gravity_dir = ae::Vec2::new(-1.0, 0.0);
    let contact = ledge_contact_for_platform(platform.aabb(), player_size, gravity_dir, -1.0);
    let delta = platform.update(1.0 / 30.0);
    assert!(delta.length() > 0.0, "precondition: platform advanced");
    assert!(
        platform.matches_ledge_contact_in_frame(contact, player_size, gravity_dir),
        "a ledge contact stored before platform advance should still match so the hang can be carried"
    );
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

fn ledge_contact_for_platform(
    platform_box: ae::Aabb,
    player_size: ae::Vec2,
    gravity_dir: ae::Vec2,
    side_normal: f32,
) -> ae::LedgeContact {
    let frame = ae::AccelerationFrame::new(gravity_dir);
    let half = player_size * 0.5;
    let platform_center = platform_box.center();
    let platform_half = platform_box.half_size();
    let platform_side = platform_center.dot(frame.side);
    let platform_down = platform_center.dot(frame.down);
    let platform_side_half = projected_half_for_test(platform_half, frame.side);
    let platform_down_half = projected_half_for_test(platform_half, frame.down);
    let lip_down = platform_down - platform_down_half;
    let wall_side = platform_side + side_normal * platform_side_half;
    ae::LedgeContact {
        wall_normal_x: side_normal,
        anchor: frame.side * (wall_side + side_normal * (half.x - 1.0))
            + frame.down * (lip_down + half.y - 4.0),
        climb_target: frame.side * (wall_side - side_normal * (half.x + 4.0))
            + frame.down * (lip_down - half.y - 1.0),
    }
}

fn projected_half_for_test(half: ae::Vec2, axis: ae::Vec2) -> f32 {
    half.x * axis.x.abs() + half.y * axis.y.abs()
}

// ── Looping paths (queue D12) ──────────────────────────────────────────────

/// ⛔ **A TWO-POINT `Loop` PATH USED TO HANG THE SIMULATION, and this is how it
/// was found: the test binary never returned.**
///
/// `advance_path_segment` computed `last_segment` as `points.len() - 2`, which is
/// **0** for two points. Arriving at the second point "wrapped" the cursor to
/// segment 0 — whose target was the point the platform was already standing on.
/// Distance zero, cursor unchanged, `continue`, forever, consuming no `dt`.
///
/// ⭐ **this is the engine half of Jon's world 1-2** — *"vertical elevator
/// platforms that wrap when they leave the view"* — and a two-waypoint vertical
/// `Loop` is exactly how anyone would author it. No shipped content used `Loop`,
/// which is the only reason a hang sat here unnoticed.
///
/// Two guards now stand behind this: `advance_path_position` breaks when an
/// advance leaves the cursor unmoved (termination, whatever the mode does), and
/// `Loop` closes its circuit so the cursor genuinely moves.
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

/// ⭐ **`Loop` closes the CIRCUIT: `p0 → p1 → p2 → p0`, not a retrace.**
///
/// Before D12 the cursor wrapped to segment 0 while the POSITION was still at the
/// last point, so segment 0's target (`p1`) was reached by travelling BACKWARDS
/// over the final leg, and the first point was never revisited. A path of `n`
/// points has `n - 1` open segments and `n` closed ones; the closing leg is what
/// was missing.
///
/// ⚠ the tolerance is one FRAME of travel, not a hair: the platform moves in
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

/// **A wrapping platform must not fling whoever is standing on it.**
///
/// Jon asked for an infinite elevator: platforms that run one way and teleport
/// back to the far end rather than reversing. ⛔ **the teleport is a position
/// change that is NOT a movement**, and `last_delta` is exactly the quantity the
/// per-body tick adds to a rider (`body_integration.rs` reads it for
/// platform-ride and ledge-carry). Reporting `pos - old` across a wrap hands the
/// rider the whole span in one frame — the height of the shaft, in one tick, in
/// the direction opposite to travel.
///
/// ⭐ **the honest test of a wrap is the frame it happens on**, not the frames
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

/// **A looping platform never turns around.**
///
/// ⛔ the poison for the variant existing at all: if it reversed it would be a
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

/// **An authored `loop_dy` produces a platform that WRAPS.**
///
/// The authoring half of the elevator. ⛔ the tell that it is wired is not that
/// the platform moves vertically — a `Path` does that, and so would a sweep on
/// the wrong axis — it is that the platform **comes back to where it started
/// while still travelling the same way**. A reversing platform also returns to
/// its start, so the direction check is what separates the two.
#[test]
fn an_authored_vertical_loop_wraps_instead_of_reversing() {
    let spec = MovingPlatformSpec::from_authored(
        "shaft_lift",
        "Shaft Lift",
        ae::Vec2::new(0.0, 100.0),
        ae::Vec2::new(96.0, 16.0),
        // A sweep is authored too, and must LOSE to the loop.
        240.0,
        100.0,
        None,
    )
    .with_vertical_loop(Some(200.0));

    let mut platform = spec.resolve(&[]).expect("a loop spec resolves");
    assert!(platform.direction() > 0.0, "a positive loop_dy rises");

    // ⚠ **the wrap is a DROP in y on a platform that is rising**, not a return
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

/// **A staggered run of platforms shares ONE shaft.**
///
/// ⛔ this is the difference between a conveyor and three unrelated lifts, and
/// it is invisible on the first frame. Anchoring each platform's shaft at its own
/// position gives three platforms three shafts — they start looking evenly
/// spaced and slowly separate into their own bands, which is a bug you notice
/// late and hate diagnosing.
///
/// ⭐ **the assertion is that every platform stays inside the SHARED shaft**, not
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
            MovingPlatformSpec::from_authored(
                format!("lift_{i}"),
                format!("Lift {i}"),
                ae::Vec2::new(0.0, BASE + phase),
                ae::Vec2::new(96.0, 16.0),
                240.0,
                100.0,
                None,
            )
            .with_vertical_loop(Some(SPAN))
            .with_loop_anchor(Some(BASE))
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
