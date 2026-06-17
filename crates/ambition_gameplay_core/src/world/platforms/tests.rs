//! Unit tests for moving-platform sweep/path motion, riding, and ledge-carry.

use super::*;

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
