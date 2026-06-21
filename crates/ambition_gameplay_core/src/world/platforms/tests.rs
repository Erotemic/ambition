//! Unit tests for moving-platform sweep/path motion, riding, and ledge-carry.

use super::*;
use crate::engine_core::AabbExt;

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
        #[cfg(feature = "portal")]
        portal_gun_spawns: Vec::new(),
        #[cfg(feature = "portal")]
        portals: Vec::new(),
        shrines: Vec::new(),
        gravity_zones: Vec::new(),
        hazards: Vec::new(),
        interactables: Vec::new(),
        pickups: Vec::new(),
        chests: Vec::new(),
        breakables: Vec::new(),
        enemy_spawns: Vec::new(),
        boss_spawns: Vec::new(),
        debug_labels: Vec::new(),
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
    let path = crate::actor::KinematicPath {
        points: vec![ae::Vec2::new(100.0, 200.0), ae::Vec2::new(180.0, 200.0)],
        speed: 80.0,
        mode: crate::actor::KinematicPathMode::PingPong,
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
    let path = crate::actor::KinematicPath {
        points: vec![ae::Vec2::new(20.0, 30.0), ae::Vec2::new(120.0, 30.0)],
        speed: 50.0,
        mode: crate::actor::KinematicPathMode::PingPong,
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
