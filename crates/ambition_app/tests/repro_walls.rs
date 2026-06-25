//! Reproduce the exact square_arena wall-cling teleport against engine
//! collision routines.

use ae::{AbilitySet, Block, InputState, PlayerClusterScratch, World, DEFAULT_TUNING};
use ambition_gameplay_core as sb;
use ambition_engine_core as ae;

fn scratch_at(spawn: ae::Vec2) -> PlayerClusterScratch {
    sb::player::primary_player_scratch(spawn, AbilitySet::sandbox_all())
}

/// Depenetration allowance: a single simulation step may legitimately
/// push the body out of an overlapping solid by at most about one
/// thin-wall thickness (16 px) plus integration slop. Anything beyond
/// `velocity * dt + this margin` is a position correction the integrator
/// must never make in one frame without a Reset / RoomTransition — i.e.
/// the wall-cling "teleport" class (TODO #96).
const DEPEN_MARGIN_PX: f32 = 16.0;

/// Principled replacement for the old `dy < 50.0` magic threshold:
/// assert one step's position change stays within the physically
/// justifiable budget (intended velocity displacement + bounded
/// depenetration). A snap to a far block blows past this by hundreds of
/// pixels. `pre_vel` is the velocity *before* the step (the intended
/// displacement); the 16 px margin absorbs the small gravity/​collision
/// velocity change across one frame.
fn assert_within_displacement_budget(
    label: &str,
    initial: ae::Vec2,
    after: ae::Vec2,
    pre_vel: ae::Vec2,
    dt: f32,
) {
    let budget = pre_vel.length() * dt + DEPEN_MARGIN_PX;
    let moved = (after - initial).length();
    assert!(
        moved <= budget,
        "{label}: step moved {moved:.2}px but the displacement budget is \
         {budget:.2}px (pre_vel={pre_vel:?}, dt={dt}, initial={initial:?}, after={after:?}) \
         — a correction this large in one frame is the wall-cling teleport class",
    );
}

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

    let mut player = scratch_at(world.spawn);
    player.kinematics.pos = ae::Vec2::new(62.0, 1567.91);
    player.kinematics.vel = ae::Vec2::new(0.0, 31.1);
    player.ground.on_ground = false;
    player.wall.on_wall = true;
    player.wall.wall_normal_x = 1.0;
    player.wall.wall_clinging = true;

    let initial = player.kinematics.pos;
    let _ = ae::update_player_simulation_with_tuning_scratch(
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
        player.kinematics.pos.x,
        player.kinematics.pos.y,
        player.kinematics.vel.x,
        player.kinematics.vel.y,
        player.ground.on_ground,
        player.wall.on_wall
    );
    assert!(
        player.kinematics.pos.y >= 0.0 && player.kinematics.pos.y <= world.size.y,
        "teleported out of world: pos={:?}",
        player.kinematics.pos
    );
    assert_within_displacement_budget(
        "square_arena_wall_cling",
        initial,
        player.kinematics.pos,
        ae::Vec2::new(0.0, 31.1),
        1.0 / 144.0,
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

    let mut player = scratch_at(world.spawn);
    // 0.01 px x penetration into the wall.
    player.kinematics.pos = ae::Vec2::new(62.0 - 0.01, 1567.91);
    player.kinematics.vel = ae::Vec2::new(0.0, 31.1);
    player.ground.on_ground = false;
    player.wall.on_wall = true;
    player.wall.wall_normal_x = 1.0;
    player.wall.wall_clinging = true;

    let initial = player.kinematics.pos;
    let _ = ae::update_player_simulation_with_tuning_scratch(
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
        player.kinematics.pos.x,
        player.kinematics.pos.y,
        player.kinematics.vel.x,
        player.kinematics.vel.y,
        player.ground.on_ground,
        player.wall.on_wall
    );
    assert!(
        player.kinematics.pos.y >= 0.0 && player.kinematics.pos.y <= world.size.y,
        "teleported out of world: pos={:?}",
        player.kinematics.pos
    );
    assert_within_displacement_budget(
        "square_arena_wall_cling_subpixel_penetration",
        initial,
        player.kinematics.pos,
        ae::Vec2::new(0.0, 31.1),
        1.0 / 144.0,
    );
}

/// Locate which specific block is the y-sweep teleport target by
/// running a downward sweep against each subset of square_arena blocks.
#[test]
fn locate_teleport_target_block() {
    let project =
        sb::ldtk_world::LdtkProject::load_default_for_dev().expect("sandbox LDtk should load");
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
        let mut player = scratch_at(world.spawn);
        player.kinematics.pos = ae::Vec2::new(62.0, 1567.9125);
        player.kinematics.vel = ae::Vec2::new(0.0, 31.148);
        player.ground.on_ground = false;
        player.wall.on_wall = true;
        player.wall.wall_normal_x = 1.0;
        player.wall.wall_clinging = true;
        player.kinematics.facing = -1.0;
        let initial = player.kinematics.pos;
        let _ = ae::update_player_simulation_with_tuning_scratch(
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
        let dy = (player.kinematics.pos.y - initial.y).abs();
        if dy > 50.0 {
            println!(
                "BLOCK {i} TRIGGERS teleport: {:?} {} aabb=({:.1},{:.1})→({:.1},{:.1})  dy={:.1}, after_pos.y={:.1}",
                candidate.kind, candidate.name,
                candidate.aabb.min.x, candidate.aabb.min.y,
                candidate.aabb.max.x, candidate.aabb.max.y,
                dy, player.kinematics.pos.y,
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
/// + room-authored moving platforms via `world_with_sandbox_solids`). Step at the
/// precise live `real_dt` from the trace.
#[test]
fn square_arena_wall_cling_full_world_does_not_teleport() {
    let project =
        sb::ldtk_world::LdtkProject::load_default_for_dev().expect("sandbox LDtk should load");
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
    let platforms = arena.moving_platforms.clone();
    let ecs_overlay = sb::features::FeatureEcsWorldOverlay::default();
    let augmented = sb::features::world_with_sandbox_solids(&world, &platforms, &ecs_overlay);

    let mut player = scratch_at(world.spawn);
    // EXACT live state from frame 1087 of trace 1777905256-095151097-000000.
    player.kinematics.pos = ae::Vec2::new(62.0, 1567.9125);
    player.kinematics.vel = ae::Vec2::new(0.0, 31.148);
    player.ground.on_ground = false;
    player.wall.on_wall = true;
    player.wall.wall_normal_x = 1.0;
    player.wall.wall_clinging = true;
    player.kinematics.facing = -1.0;

    let initial = player.kinematics.pos;
    let dt = 0.00677_f32; // live frame 1088 real_dt
    let _ = ae::update_player_simulation_with_tuning_scratch(
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
        player.kinematics.pos.x,
        player.kinematics.pos.y,
        player.kinematics.vel.x,
        player.kinematics.vel.y,
        player.ground.on_ground,
        player.wall.on_wall,
        player.wall.wall_clinging
    );
    assert!(
        player.kinematics.pos.y >= 0.0 && player.kinematics.pos.y <= world.size.y,
        "teleported out of world: pos={:?}",
        player.kinematics.pos
    );
    assert_within_displacement_budget(
        "square_arena_wall_cling_full_world",
        initial,
        player.kinematics.pos,
        ae::Vec2::new(0.0, 31.148),
        0.00677,
    );
}

/// Same pose, but step many times. Live trace had ~150 frames of
/// wall-cling before the teleport — maybe the bug needs accumulation.
#[test]
fn square_arena_wall_cling_full_world_steps_many_times() {
    let project =
        sb::ldtk_world::LdtkProject::load_default_for_dev().expect("sandbox LDtk should load");
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
    let platforms = arena.moving_platforms.clone();
    let ecs_overlay = sb::features::FeatureEcsWorldOverlay::default();
    let augmented = sb::features::world_with_sandbox_solids(&world, &platforms, &ecs_overlay);

    let mut player = scratch_at(world.spawn);
    player.kinematics.pos = ae::Vec2::new(62.0, 1567.9125);
    player.kinematics.vel = ae::Vec2::new(0.0, 31.148);
    player.ground.on_ground = false;
    player.wall.on_wall = true;
    player.wall.wall_normal_x = 1.0;
    player.wall.wall_clinging = true;
    player.kinematics.facing = -1.0;

    for i in 0..200 {
        let dt = 0.0069_f32;
        let _ = ae::update_player_simulation_with_tuning_scratch(
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
        if player.kinematics.pos.y < 100.0 || player.kinematics.pos.y > world.size.y - 32.0 {
            panic!(
                "TELEPORT at iter {i}: pos=({}, {}) vel=({}, {}) on_ground={} on_wall={}",
                player.kinematics.pos.x,
                player.kinematics.pos.y,
                player.kinematics.vel.x,
                player.kinematics.vel.y,
                player.ground.on_ground,
                player.wall.on_wall
            );
        }
    }
    println!(
        "after 200 steps: pos=({}, {}) vel=({}, {})",
        player.kinematics.pos.x,
        player.kinematics.pos.y,
        player.kinematics.vel.x,
        player.kinematics.vel.y
    );
}

/// Class-wide sweep of the wall-cling teleport: rather than four
/// hand-picked poses, drive a grid of cling positions along a wall span,
/// at a range of sub-pixel penetrations and downward speeds, and assert
/// the per-step displacement budget for every one. A teleport that only
/// triggers at a specific y / penetration / dt the four fixtures happen
/// to miss is caught here. Uses the minimal 3-block square-arena subset
/// so it stays fast and LDtk-independent.
#[test]
fn wall_cling_displacement_budget_holds_across_pose_sweep() {
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
            // Left wall: right edge at x=48.
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

    // Sweep y down the wall span, a few sub-pixel penetrations into the
    // wall, and a few downward speeds + frame times that mirror live
    // 144 Hz / 60 Hz play.
    let y_samples: Vec<f32> = (0..40).map(|i| 80.0 + i as f32 * 40.0).collect();
    let penetrations = [0.0_f32, -0.01, -0.1, -0.5];
    let speeds = [0.0_f32, 15.0, 31.148, 90.0, 240.0];
    let dts = [1.0 / 144.0_f32, 1.0 / 60.0_f32, 0.00677];

    for &y in &y_samples {
        for &pen in &penetrations {
            for &vy in &speeds {
                for &dt in &dts {
                    let mut player = scratch_at(world.spawn);
                    // Player half-width ~14; right edge of wall is x=48,
                    // so clinging body sits at x≈62 (+ penetration).
                    player.kinematics.pos = ae::Vec2::new(62.0 + pen, y);
                    player.kinematics.vel = ae::Vec2::new(0.0, vy);
                    player.ground.on_ground = false;
                    player.wall.on_wall = true;
                    player.wall.wall_normal_x = 1.0;
                    player.wall.wall_clinging = true;
                    player.kinematics.facing = -1.0;

                    let initial = player.kinematics.pos;
                    let pre_vel = player.kinematics.vel;
                    let _ = ae::update_player_simulation_with_tuning_scratch(
                        &world,
                        &mut player,
                        InputState {
                            axis_x: -1.0,
                            control_dt: dt,
                            ..Default::default()
                        },
                        dt,
                        DEFAULT_TUNING,
                    );
                    // Still inside the world.
                    assert!(
                        player.kinematics.pos.y >= 0.0
                            && player.kinematics.pos.y <= world.size.y
                            && player.kinematics.pos.x >= 0.0
                            && player.kinematics.pos.x <= world.size.x,
                        "pose sweep teleported OOB: start=({:.2},{:.2}) vy={vy} dt={dt} -> pos={:?}",
                        initial.x,
                        initial.y,
                        player.kinematics.pos,
                    );
                    assert_within_displacement_budget(
                        &format!("pose_sweep y={y} pen={pen} vy={vy} dt={dt}"),
                        initial,
                        player.kinematics.pos,
                        pre_vel,
                        dt,
                    );
                }
            }
        }
    }
}

/// Regression guard for the mob_lab lock-wall teleport documented in
/// `docs/planning/tech-debt-log.md` (HIGH).
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

    let mut player = scratch_at(world.spawn);
    // Wall-cling pose: right edge of the lock wall (x=704), player
    // sitting just outside at x=718, y=434.1 (per trace).
    player.kinematics.pos = ae::Vec2::new(718.0, 434.1);
    player.kinematics.vel = ae::Vec2::new(0.0, 31.1);
    player.ground.on_ground = false;
    player.wall.on_wall = true;
    player.wall.wall_normal_x = 1.0; // wall on the player's left → normal points right (+x)
    player.wall.wall_clinging = true;

    let initial = player.kinematics.pos;
    let _ = ae::update_player_simulation_with_tuning_scratch(
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
        player.kinematics.pos.x,
        player.kinematics.pos.y,
        player.kinematics.vel.x,
        player.kinematics.vel.y,
        player.ground.on_ground,
        player.wall.on_wall
    );
    assert!(
        player.kinematics.pos.y > 32.0 && player.kinematics.pos.y < world.size.y,
        "teleport detected: pos={:?} (initial y={}, world.size={:?})",
        player.kinematics.pos,
        initial.y,
        world.size
    );
    assert_within_displacement_budget(
        "mob_lab_lock_wall_cling",
        initial,
        player.kinematics.pos,
        ae::Vec2::new(0.0, 31.1),
        1.0 / 144.0,
    );
}

/// Full-world regression guard + reproduction attempt for the
/// goblin_encounter lock-wall teleport (tech-debt-log HIGH).
///
/// Unlike `mob_lab_lock_wall_cling_does_not_teleport` (a 3-block subset),
/// this loads the REAL goblin_encounter world — all its LDtk blocks in
/// production order — and APPENDS the runtime lock wall last, exactly as
/// `sync_lock_walls` does when the encounter goes Active. It then drives
/// the body off the lock-wall edge with a strong upward velocity (the
/// post-wall-jump state the trace blames) through the x=704..720 top-wall
/// corner.
///
/// **Result (2026-06-02):** this does NOT reproduce the production
/// teleport — the upward sweep correctly stops the body just below the
/// top wall (top≈401 vs wall bottom 400, vel.y zeroed). So the full block
/// set + append order is NOT sufficient; the production trigger needs the
/// exact trace state that synthetic fixtures don't capture (the
/// control-phase wall-jump velocity, sub-pixel x/penetration config, or
/// accumulated multi-frame history). Kept as a passing regression guard:
/// the budget assertion fires if a future change reintroduces the snap.
#[test]
fn goblin_encounter_full_world_lock_wall_cling_repro() {
    let project =
        sb::ldtk_world::LdtkProject::load_default_for_dev().expect("sandbox LDtk should load");
    let room_set = project.to_room_set().expect("room_set");
    let Some(room) = room_set.rooms.iter().find(|s| s.id == "goblin_encounter") else {
        eprintln!("no goblin_encounter room; known rooms:");
        for r in &room_set.rooms {
            eprintln!("  {}", r.id);
        }
        return;
    };
    let world = room.world.clone();
    let platforms = room.moving_platforms.clone();
    let ecs_overlay = sb::features::FeatureEcsWorldOverlay::default();
    let mut augmented = sb::features::world_with_sandbox_solids(&world, &platforms, &ecs_overlay);

    // Append the runtime lock wall last (matches sync_lock_walls insert
    // order). Trace coords: LDtk px (480,400) size (224,208).
    augmented.blocks.push(ae::Block::solid(
        "lockwall:goblin_encounter",
        ae::Vec2::new(480.0, 400.0),
        ae::Vec2::new(224.0, 208.0),
    ));

    println!(
        "goblin_encounter world: size={:?} spawn={:?} blocks={}",
        world.size,
        world.spawn,
        augmented.blocks.len()
    );
    for b in &augmented.blocks {
        if b.aabb.min.y < 64.0 || b.name.starts_with("lockwall") {
            println!(
                "  block {:?} aabb min={:?} max={:?}",
                b.name, b.aabb.min, b.aabb.max
            );
        }
    }

    let mut player = scratch_at(world.spawn);
    // EXACT live state from the trace: wall-clinging on the lock wall's
    // right edge (lock_wall.right=704), player just outside at x=718.
    player.kinematics.pos = ae::Vec2::new(718.0, 434.1);
    player.kinematics.vel = ae::Vec2::new(0.0, 31.1);
    player.ground.on_ground = false;
    player.wall.on_wall = true;
    player.wall.wall_normal_x = 1.0;
    player.wall.wall_clinging = true;
    player.kinematics.facing = -1.0;

    let dt = 1.0 / 144.0;
    // Simulate the post-wall-jump state directly (the simulation phase
    // doesn't process the jump press): a strong upward velocity, pushed
    // away from the wall. The body then rises from the lock-wall edge
    // into the x=704..720 top-wall corner — exactly the
    // edge-touching-then-vertical-sweep configuration the trace blames.
    player.kinematics.vel = ae::Vec2::new(180.0, -560.0);
    player.wall.wall_clinging = false;
    for frame in 0..40 {
        let pre = player.kinematics.pos;
        let pre_vel = player.kinematics.vel;
        let input = InputState {
            axis_x: 0.0,
            jump_held: frame < 8,
            control_dt: dt,
            ..Default::default()
        };
        let _ = ae::update_player_simulation_with_tuning_scratch(
            &augmented,
            &mut player,
            input,
            dt,
            DEFAULT_TUNING,
        );
        let moved = (player.kinematics.pos - pre).length();
        if frame < 4 || moved > 40.0 {
            println!(
                "f{frame:02}: pos=({:.1}, {:.1}) vel=({:.1}, {:.1}) moved={:.2}px",
                player.kinematics.pos.x,
                player.kinematics.pos.y,
                player.kinematics.vel.x,
                player.kinematics.vel.y,
                moved
            );
        }
        assert_within_displacement_budget(
            &format!("goblin_encounter_full_world_walljump_f{frame}"),
            pre,
            player.kinematics.pos,
            pre_vel,
            dt,
        );
    }
}

/// Faithful control+simulation wall-jump against the real goblin_encounter
/// world + appended lock wall. The previous repro only drove the
/// simulation phase, so `jump_pressed` was ignored and no real wall-jump
/// impulse fired. This one runs BOTH phases each frame (control then
/// simulation), pressing Jump while wall-clinging on the lock wall's right
/// edge — the exact "presses Jump while clinging" production trigger.
/// Budget-asserts every frame so a >budget snap (the teleport) fails.
#[test]
fn goblin_encounter_real_walljump_repro() {
    let project =
        sb::ldtk_world::LdtkProject::load_default_for_dev().expect("sandbox LDtk should load");
    let room_set = project.to_room_set().expect("room_set");
    let Some(room) = room_set.rooms.iter().find(|s| s.id == "goblin_encounter") else {
        return;
    };
    let world = room.world.clone();
    let platforms = room.moving_platforms.clone();
    let ecs_overlay = sb::features::FeatureEcsWorldOverlay::default();
    let mut augmented = sb::features::world_with_sandbox_solids(&world, &platforms, &ecs_overlay);
    augmented.blocks.push(ae::Block::solid(
        "lockwall:goblin_encounter",
        ae::Vec2::new(480.0, 400.0),
        ae::Vec2::new(224.0, 208.0),
    ));

    let mut player = scratch_at(world.spawn);
    player.kinematics.pos = ae::Vec2::new(718.0, 434.1);
    player.kinematics.vel = ae::Vec2::new(0.0, 31.1);
    player.ground.on_ground = false;
    player.wall.on_wall = true;
    player.wall.wall_normal_x = 1.0;
    player.wall.wall_clinging = true;
    player.kinematics.facing = -1.0;

    let dt = 1.0 / 144.0;
    for frame in 0..40 {
        let pre = player.kinematics.pos;
        let pre_vel = player.kinematics.vel;
        let input = InputState {
            axis_x: if frame == 0 { -1.0 } else { 0.0 },
            jump_pressed: frame == 0,
            jump_held: frame < 8,
            control_dt: dt,
            ..Default::default()
        };
        // Control phase first (fires the wall-jump impulse), then sim.
        let _ = ae::update_player_control_with_tuning_scratch(
            &augmented,
            &mut player,
            input,
            dt,
            DEFAULT_TUNING,
        );
        let _ = ae::update_player_simulation_with_tuning_scratch(
            &augmented,
            &mut player,
            input,
            dt,
            DEFAULT_TUNING,
        );
        let moved = (player.kinematics.pos - pre).length();
        if frame < 4 || moved > 40.0 {
            println!(
                "f{frame:02}: pos=({:.1}, {:.1}) vel=({:.1}, {:.1}) moved={:.2}px cling={}",
                player.kinematics.pos.x,
                player.kinematics.pos.y,
                player.kinematics.vel.x,
                player.kinematics.vel.y,
                moved,
                player.wall.wall_clinging
            );
        }
        assert_within_displacement_budget(
            &format!("goblin_encounter_real_walljump_f{frame}"),
            pre,
            player.kinematics.pos,
            pre_vel,
            dt,
        );
    }
}
