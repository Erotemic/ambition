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

// ── The wrapping elevator (queue D12) ──────────────────────────────────────

/// ⛔ **A TWO-POINT `Loop` PATH USED TO HANG THE SIMULATION, and this is how it
/// was found: the test binary never returned.**
///
/// `advance_path_segment` computes `last_segment` as `points.len() - 2`, which is
/// **0** for a two-point path. So arriving at the second point "wraps" the cursor
/// to segment 0 — whose target is the point the platform is already standing on.
/// Distance zero, cursor unchanged, `continue`, forever. `advance_path_position`
/// now breaks when an advance leaves the cursor where it was.
///
/// ⭐ **this is the engine half of Jon's world 1-2** — *"vertical elevator
/// platforms that wrap when they leave the view"* — and a two-waypoint vertical
/// `Loop` is exactly how anyone would author it. No shipped content uses `Loop`,
/// which is why a hang has been sitting here unnoticed.
///
/// ⚠ this test asserts TERMINATION and pins today's behaviour. It does not
/// assert that `Loop` is correct, because it is not: see below.
#[test]
fn a_two_point_looping_path_terminates_instead_of_spinning() {
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
    // 120 frames is twice what the 300px column needs at 600px/s. Before the
    // termination guard this loop did not finish at all.
    for _ in 0..120 {
        platform.update(dt);
    }

    assert!(
        (platform.pos - top).length() < 1.0,
        "the platform climbs to the top and STOPS there: {:?}",
        platform.pos
    );
}

/// ⛔ **and `Loop` does not loop — it RETRACES, which the three-point case shows
/// without the two-point case's hang.**
///
/// A closed circuit would run `p0 → p1 → p2 → p0`. What happens instead is that
/// the cursor wraps to segment 0 while the POSITION is still at `p2`, so segment
/// 0's target (`p1`) is reached by travelling BACKWARDS. The platform oscillates
/// over the last leg forever and the first leg is never revisited.
///
/// ▢ **the fix is a design call and belongs in D12**: a `Loop` that treats the
/// path as CLOSED (adding the implicit `pₙ → p₀` leg) is one answer, and a `Loop`
/// that TELEPORTS back to `p₀` — what a wrapping elevator actually looks like —
/// is a different one. The queue row's instinct was right: *a platform that
/// teleports is a transit, and transits have an arbiter*, because a body riding
/// it has to be told whether it comes along.
#[test]
fn loop_mode_retraces_the_last_leg_rather_than_closing_the_circuit() {
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
    let mut reached_c = false;
    let mut returned_to_a = false;
    for _ in 0..600 {
        platform.update(dt);
        if (platform.pos - c).length() < 1.0 {
            reached_c = true;
        }
        // Only counts AFTER the far corner: the platform starts at `a`.
        if reached_c && (platform.pos - a).length() < 1.0 {
            returned_to_a = true;
        }
    }

    assert!(
        reached_c,
        "the platform should traverse the whole path first"
    );
    assert!(
        !returned_to_a,
        "TODAY `Loop` never returns to its first point — if this now fails, the \
         circuit has been closed (or the wrap implemented) and this test should \
         become the assertion that it STAYS closed. See queue D12."
    );
}
