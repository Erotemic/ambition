//! Reproduce the exact square_arena wall-cling teleport against engine
//! collision routines.

use ae::{AbilitySet, Block, InputState, Player, World, DEFAULT_TUNING};
use ambition_engine as ae;
use ambition_sandbox as sb;

#[test]
fn square_arena_wall_cling_does_not_teleport() {
    // Exact subset of square_arena's wall geometry.
    let world = World::new(
        "square_arena_subset",
        ae::Vec2::new(1800.0, 1800.0),
        ae::Vec2::new(170.0, 1695.0),
        vec![
            // Ceiling.
            Block::solid(
                "ldtk solid",
                ae::Vec2::new(0.0, 0.0),
                ae::Vec2::new(1808.0, 32.0),
            ),
            // Left wall (top=32, bottom=1744).
            Block::solid(
                "ldtk solid",
                ae::Vec2::new(0.0, 32.0),
                ae::Vec2::new(48.0, 1712.0),
            ),
            // Floor.
            Block::solid(
                "ldtk solid",
                ae::Vec2::new(0.0, 1744.0),
                ae::Vec2::new(1808.0, 64.0),
            ),
        ],
    );

    let mut player = Player::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
    player.pos = ae::Vec2::new(62.0, 1567.91);
    player.vel = ae::Vec2::new(0.0, 31.1);
    player.on_ground = false;
    player.on_wall = true;
    player.wall_normal_x = 1.0;
    player.wall_clinging = true;

    let initial = player.pos;
    let _ = ae::update_player_simulation_with_tuning(
        &world,
        &mut player,
        InputState {
            axis_x: -1.0,
            control_dt: 1.0 / 144.0,
            ..Default::default()
        },
        1.0 / 144.0,
        DEFAULT_TUNING,
    );
    println!(
        "after step: pos=({},{}) vel=({},{}) on_ground={} on_wall={}",
        player.pos.x, player.pos.y, player.vel.x, player.vel.y, player.on_ground, player.on_wall
    );
    let dy_a = (player.pos.y - initial.y).abs();
    assert!(
        player.pos.y >= 0.0 && player.pos.y <= world.size.y,
        "teleported out of world: pos={:?}",
        player.pos
    );
    assert!(
        dy_a < 50.0,
        "dy={dy_a} (initial y={}, after y={})",
        initial.y,
        player.pos.y
    );
}

/// Same pose with sub-pixel penetration into the wall on x.
#[test]
fn square_arena_wall_cling_with_subpixel_penetration_does_not_teleport() {
    let world = World::new(
        "square_arena_subset",
        ae::Vec2::new(1800.0, 1800.0),
        ae::Vec2::new(170.0, 1695.0),
        vec![
            Block::solid(
                "ldtk solid",
                ae::Vec2::new(0.0, 0.0),
                ae::Vec2::new(1808.0, 32.0),
            ),
            Block::solid(
                "ldtk solid",
                ae::Vec2::new(0.0, 32.0),
                ae::Vec2::new(48.0, 1712.0),
            ),
            Block::solid(
                "ldtk solid",
                ae::Vec2::new(0.0, 1744.0),
                ae::Vec2::new(1808.0, 64.0),
            ),
        ],
    );

    let mut player = Player::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
    // 0.01 px x penetration into the wall.
    player.pos = ae::Vec2::new(62.0 - 0.01, 1567.91);
    player.vel = ae::Vec2::new(0.0, 31.1);
    player.on_ground = false;
    player.on_wall = true;
    player.wall_normal_x = 1.0;
    player.wall_clinging = true;

    let initial = player.pos;
    let _ = ae::update_player_simulation_with_tuning(
        &world,
        &mut player,
        InputState {
            axis_x: -1.0,
            control_dt: 1.0 / 144.0,
            ..Default::default()
        },
        1.0 / 144.0,
        DEFAULT_TUNING,
    );
    println!(
        "after step: pos=({},{}) vel=({},{}) on_ground={} on_wall={}",
        player.pos.x, player.pos.y, player.vel.x, player.vel.y, player.on_ground, player.on_wall
    );
    let dy_b = (player.pos.y - initial.y).abs();
    assert!(
        player.pos.y >= 0.0 && player.pos.y <= world.size.y,
        "teleported out of world: pos={:?}",
        player.pos
    );
    assert!(
        dy_b < 50.0,
        "dy={dy_b} (initial y={}, after y={})",
        initial.y,
        player.pos.y
    );
}

/// Locate which specific block is the y-sweep teleport target by
/// running a downward sweep against each subset of square_arena blocks.
#[test]
fn locate_teleport_target_block() {
    let project = sb::ldtk_world::LdtkProject::load_embedded();
    let report = project.validate();
    if !report.is_ok() {
        panic!("validation failed");
    }
    let room_set = project.to_room_set().expect("room_set");
    let arena = room_set
        .rooms
        .iter()
        .find(|s| s.id == "square_arena")
        .expect("arena");
    let world = arena.world.clone();

    // Scan: try the world with each block in turn (one block alone) to
    // see if it triggers the teleport.
    for (i, candidate) in world.blocks.iter().enumerate() {
        let single = World::new("single", world.size, world.spawn, vec![candidate.clone()]);
        let mut player = Player::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
        player.pos = ae::Vec2::new(62.0, 1567.9125);
        player.vel = ae::Vec2::new(0.0, 31.148);
        player.on_ground = false;
        player.on_wall = true;
        player.wall_normal_x = 1.0;
        player.wall_clinging = true;
        player.facing = -1.0;
        let initial = player.pos;
        let _ = ae::update_player_simulation_with_tuning(
            &single,
            &mut player,
            InputState {
                axis_x: -1.0,
                control_dt: 0.0069,
                ..Default::default()
            },
            0.0069,
            DEFAULT_TUNING,
        );
        let dy = (player.pos.y - initial.y).abs();
        if dy > 50.0 {
            println!(
                "BLOCK {i} TRIGGERS teleport: {:?} {} aabb=({:.1},{:.1})→({:.1},{:.1})  dy={:.1}, after_pos.y={:.1}",
                candidate.kind, candidate.name,
                candidate.aabb.min.x, candidate.aabb.min.y,
                candidate.aabb.max.x, candidate.aabb.max.y,
                dy, player.pos.y,
            );
        } else {
            println!(
                "block {i} ok: {:?} {} dy={:.3}",
                candidate.kind, candidate.name, dy,
            );
        }
    }
}

/// Replay against the FULL square_arena world (all 15 LDtk-derived blocks
/// + moving platform via `world_with_sandbox_solids`). Step at the
/// precise live `real_dt` from the trace.
#[test]
fn square_arena_wall_cling_full_world_does_not_teleport() {
    let project = sb::ldtk_world::LdtkProject::load_embedded();
    let report = project.validate();
    if !report.is_ok() {
        panic!("validation failed");
    }
    let room_set = project.to_room_set().expect("room_set");
    let arena = room_set
        .rooms
        .iter()
        .find(|s| s.id == "square_arena")
        .expect("arena");
    let world = arena.world.clone();
    let platform = sb::platforms::MovingPlatformState::time_reference(&world);
    let features = sb::features::FeatureRuntime::from_world(&world);
    let augmented = sb::features::world_with_sandbox_solids(&world, &platform, &features);

    let mut player = Player::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
    // EXACT live state from frame 1087 of trace 1777905256-095151097-000000.
    player.pos = ae::Vec2::new(62.0, 1567.9125);
    player.vel = ae::Vec2::new(0.0, 31.148);
    player.on_ground = false;
    player.on_wall = true;
    player.wall_normal_x = 1.0;
    player.wall_clinging = true;
    player.facing = -1.0;

    let initial = player.pos;
    let dt = 0.00677_f32; // live frame 1088 real_dt
    let _ = ae::update_player_simulation_with_tuning(
        &augmented,
        &mut player,
        InputState {
            axis_x: -1.0,
            control_dt: dt,
            ..Default::default()
        },
        dt,
        DEFAULT_TUNING,
    );
    println!(
        "FULL WORLD step: pos=({}, {}) vel=({}, {}) on_ground={} on_wall={} cling={}",
        player.pos.x,
        player.pos.y,
        player.vel.x,
        player.vel.y,
        player.on_ground,
        player.on_wall,
        player.wall_clinging
    );
    let dy_c = (player.pos.y - initial.y).abs();
    assert!(
        player.pos.y >= 0.0 && player.pos.y <= world.size.y,
        "teleported out of world: pos={:?}",
        player.pos
    );
    assert!(
        dy_c < 50.0,
        "dy={dy_c} (initial y={}, after y={})",
        initial.y,
        player.pos.y
    );
}

/// Same pose, but step many times. Live trace had ~150 frames of
/// wall-cling before the teleport — maybe the bug needs accumulation.
#[test]
fn square_arena_wall_cling_full_world_steps_many_times() {
    let project = sb::ldtk_world::LdtkProject::load_embedded();
    let report = project.validate();
    if !report.is_ok() {
        panic!("validation failed");
    }
    let room_set = project.to_room_set().expect("room_set");
    let arena = room_set
        .rooms
        .iter()
        .find(|s| s.id == "square_arena")
        .expect("arena");
    let world = arena.world.clone();
    let platform = sb::platforms::MovingPlatformState::time_reference(&world);
    let features = sb::features::FeatureRuntime::from_world(&world);
    let augmented = sb::features::world_with_sandbox_solids(&world, &platform, &features);

    let mut player = Player::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
    player.pos = ae::Vec2::new(62.0, 1567.9125);
    player.vel = ae::Vec2::new(0.0, 31.148);
    player.on_ground = false;
    player.on_wall = true;
    player.wall_normal_x = 1.0;
    player.wall_clinging = true;
    player.facing = -1.0;

    for i in 0..200 {
        let dt = 0.0069_f32;
        let _ = ae::update_player_simulation_with_tuning(
            &augmented,
            &mut player,
            InputState {
                axis_x: -1.0,
                control_dt: dt,
                ..Default::default()
            },
            dt,
            DEFAULT_TUNING,
        );
        if player.pos.y < 100.0 || player.pos.y > world.size.y - 32.0 {
            panic!(
                "TELEPORT at iter {i}: pos=({}, {}) vel=({}, {}) on_ground={} on_wall={}",
                player.pos.x,
                player.pos.y,
                player.vel.x,
                player.vel.y,
                player.on_ground,
                player.on_wall
            );
        }
    }
    println!(
        "after 200 steps: pos=({}, {}) vel=({}, {})",
        player.pos.x, player.pos.y, player.vel.x, player.vel.y
    );
}

/// Regression guard for the mob_lab lock-wall teleport documented in
/// `docs/tech_debt_log.md` (HIGH).
///
/// Geometry mirrors the runtime: the runtime-inserted
/// `lockwall:mob_lab` block sits at LDtk px (480, 400) size (224, 208),
/// with an arena ceiling above (top at y=0). Wall-clinging on the
/// lock wall's right edge previously snapped the player to the
/// arena_ceiling top (`y = ceiling_top - half_height = -23`).
///
/// In this minimal geometry the existing `body_is_side_contact`
/// predicate (added by the wall-jump OOB fix, commit 4002b4d) already
/// rejects the bogus far-block hit, so the test currently passes.
/// Keeping it here as a regression guard — if a future change to the
/// snap-direction logic re-introduces the teleport, this test fires.
/// The full production trigger may need additional context (encounter
/// running, lock wall hot-inserted) that this minimal fixture
/// deliberately omits; that's tracked separately under the parry
/// contact-normal fix (path_forward step D1).
#[test]
fn mob_lab_lock_wall_cling_does_not_teleport() {
    let world = World::new(
        "mob_lab_subset",
        ae::Vec2::new(1808.0, 1264.0),
        ae::Vec2::new(80.0, 1232.0),
        vec![
            // Arena ceiling: top at y=0, 32px tall, full width.
            Block::solid(
                "arena_ceiling",
                ae::Vec2::new(0.0, 0.0),
                ae::Vec2::new(1808.0, 32.0),
            ),
            // Lock wall (runtime-inserted in production): top at y=400,
            // height 208, x=480..704.
            Block::solid(
                "lockwall:mob_lab",
                ae::Vec2::new(480.0, 400.0),
                ae::Vec2::new(224.0, 208.0),
            ),
            // Floor below.
            Block::solid(
                "floor",
                ae::Vec2::new(0.0, 1232.0),
                ae::Vec2::new(1808.0, 32.0),
            ),
        ],
    );

    let mut player = Player::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
    // Wall-cling pose: right edge of the lock wall (x=704), player
    // sitting just outside at x=718, y=434.1 (per trace).
    player.pos = ae::Vec2::new(718.0, 434.1);
    player.vel = ae::Vec2::new(0.0, 31.1);
    player.on_ground = false;
    player.on_wall = true;
    player.wall_normal_x = 1.0; // wall on the player's left → normal points right (+x)
    player.wall_clinging = true;

    let initial = player.pos;
    let _ = ae::update_player_simulation_with_tuning(
        &world,
        &mut player,
        InputState {
            axis_x: -1.0, // pressing into the wall (cling)
            control_dt: 1.0 / 144.0,
            ..Default::default()
        },
        1.0 / 144.0,
        DEFAULT_TUNING,
    );
    println!(
        "after step: pos=({},{}) vel=({},{}) on_ground={} on_wall={}",
        player.pos.x, player.pos.y, player.vel.x, player.vel.y, player.on_ground, player.on_wall
    );
    let dy = (player.pos.y - initial.y).abs();
    assert!(
        player.pos.y > 32.0 && player.pos.y < world.size.y,
        "teleport detected: pos={:?} (initial y={}, world.size={:?})",
        player.pos,
        initial.y,
        world.size
    );
    assert!(
        dy < 50.0,
        "y-snap exceeded velocity budget: dy={dy} (initial y={}, after y={}, vel.y={})",
        initial.y,
        player.pos.y,
        player.vel.y
    );
}
