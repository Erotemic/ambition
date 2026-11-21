//! Reproduce the exact square_arena wall-cling teleport against engine
//! collision routines.

use ae::{AbilitySet, Block, BodyClusterScratch, InputState, World, DEFAULT_TUNING};
use ambition_platformer2d::engine_core as ae;

fn scratch_at(spawn: ae::Vec2) -> BodyClusterScratch {
    BodyClusterScratch::new_with_abilities(spawn, AbilitySet::sandbox_all())
}

/// Depenetration allowance: a single simulation step may legitimately
/// push the body out of an overlapping solid by at most about one
/// thin-wall thickness (16 px) plus integration slop. Anything beyond
/// `velocity * dt + this margin` is a position correction the integrator
/// must never make in one frame without a Reset / RoomTransition — i.e.
/// the wall-cling "teleport" class (TODO #96).
const DEPEN_MARGIN_PX: f32 = 16.0;

fn step_axis_for_test(
    world: &World,
    scratch: &mut BodyClusterScratch,
    input: InputState,
    dt: f32,
    tuning: ae::MovementTuning,
) -> ae::FrameEvents {
    let frame = ae::MotionFrame::from_direction(ae::DEFAULT_GRAVITY_DIR, tuning.gravity);
    // Thread the scratch body's OWN persistent model (a same-variant param
    // refresh preserves maneuver state, ADR 0024) so arranged cling state and
    // multi-step tests behave like a live entity.
    let (model, mut clusters) = scratch.parts();
    ae::switch_motion_model(
        model,
        ae::MotionModelSpec::AxisSwept(tuning.axis_swept_params()),
    );
    ae::step_motion(
        model,
        &mut clusters,
        ae::MotionStepContext {
            admission: None,
            world,
            input,
            frame,
            facing_intent: input.axes.x,
            dt,
            contact: ae::BodyContactField::NONE,
            pose_owned_externally: false,
            recovery_commitment_outstanding: false,
        },
    )
    .events
}

/// A snap to a far block blows past this by hundreds of pixels.
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

/// The three blocks of square_arena's walls a cling on the left wall meets:
/// the ceiling, the left wall (right edge x=48, top 32, bottom 1744) and the
/// floor.
fn square_arena_subset() -> World {
    World::new(
        "square_arena_subset",
        ae::Vec2::new(1800.0, 1800.0),
        ae::Vec2::new(170.0, 1695.0),
        vec![
            Block::solid("ldtk solid", ae::Vec2::new(0.0, 0.0), ae::Vec2::new(1808.0, 32.0)),
            Block::solid("ldtk solid", ae::Vec2::new(0.0, 32.0), ae::Vec2::new(48.0, 1712.0)),
            Block::solid("ldtk solid", ae::Vec2::new(0.0, 1744.0), ae::Vec2::new(1808.0, 64.0)),
        ],
    )
}

/// The goblin encounter's lock wall at the trace's LDtk px (480,400), size
/// (224,208); production inserts it at runtime (`sync_lock_walls`).
fn lock_wall(name: &str) -> Block {
    Block::solid(name, ae::Vec2::new(480.0, 400.0), ae::Vec2::new(224.0, 208.0))
}

/// The sandbox room `id` as the LDtk project builds it, and the same world
/// with its room-authored moving platforms added (`world_with_sandbox_solids`).
/// `None` when the project has no such room. The project is validated first.
fn sandbox_room(id: &str) -> Option<(World, World)> {
    let project = load_project_for_test().expect("sandbox LDtk should load");
    let report = project.validate(&ambition_app::composed_ldtk_vocabulary());
    assert!(report.is_ok(), "validation failed");
    let room_set = project
        .to_room_set(&ambition_content::worlds::world_manifest(), &ambition_app::composed_ldtk_vocabulary())
        .expect("room_set");
    let Some(room) = room_set.rooms.iter().find(|s| s.id == id) else {
        eprintln!("no {id} room; known rooms:");
        for r in &room_set.rooms {
            eprintln!("  {}", r.id);
        }
        return None;
    };
    let augmented = ambition_platformer2d::world::collision::world_with_sandbox_solids(
        &room.world,
        &room.moving_platforms,
        &ambition_platformer2d::world::FeatureEcsWorldOverlay::default(),
    );
    Some((room.world.clone(), augmented))
}

/// A body clinging to a wall on its left (the normal points +x), in the air,
/// at `pos` with `vel`.
fn clinging_to_a_left_wall(world: &World, pos: ae::Vec2, vel: ae::Vec2) -> BodyClusterScratch {
    let mut player = scratch_at(world.spawn);
    player.kinematics.pos = pos;
    player.kinematics.vel = vel;
    player.ground.on_ground = false;
    player.wall.on_wall = true;
    player.wall.wall_normal_x = 1.0;
    player.axis_mut().wall_clinging = true;
    player
}

/// One step of `dt` pressing into the wall (left).
fn step_pressing_left(world: &World, player: &mut BodyClusterScratch, dt: f32) {
    let input = InputState { axes: ae::LocalAxes::new(-1.0, 0.0), control_dt: dt, ..Default::default() };
    let _ = step_axis_for_test(world, player, input, dt, DEFAULT_TUNING);
}

fn print_body(label: &str, player: &BodyClusterScratch) {
    println!(
        "{label}: pos=({},{}) vel=({},{}) on_ground={} on_wall={} cling={}",
        player.kinematics.pos.x,
        player.kinematics.pos.y,
        player.kinematics.vel.x,
        player.kinematics.vel.y,
        player.ground.on_ground,
        player.wall.on_wall,
        player.axis().wall_clinging
    );
}

fn assert_inside_vertically(world: &World, player: &BodyClusterScratch) {
    assert!(
        player.kinematics.pos.y >= 0.0 && player.kinematics.pos.y <= world.size.y,
        "teleported out of world: pos={:?}",
        player.kinematics.pos
    );
}

/// Steps `player` 40 frames of 1/144 s with `input_at(frame)`, asserting the
/// displacement budget on every frame.
fn drive_within_budget(
    world: &World,
    player: &mut BodyClusterScratch,
    label: &str,
    input_at: impl Fn(usize, f32) -> InputState,
) {
    let dt = 1.0 / 144.0;
    for frame in 0..40 {
        let pre = player.kinematics.pos;
        let pre_vel = player.kinematics.vel;
        // The unified kernel owns intent and simulation in one deterministic tick.
        let _ = step_axis_for_test(world, player, input_at(frame, dt), dt, DEFAULT_TUNING);
        let moved = (player.kinematics.pos - pre).length();
        if frame < 4 || moved > 40.0 {
            println!(
                "f{frame:02}: pos=({:.1}, {:.1}) vel=({:.1}, {:.1}) moved={:.2}px cling={}",
                player.kinematics.pos.x,
                player.kinematics.pos.y,
                player.kinematics.vel.x,
                player.kinematics.vel.y,
                moved,
                player.axis().wall_clinging
            );
        }
        assert_within_displacement_budget(&format!("{label}_f{frame}"), pre, player.kinematics.pos, pre_vel, dt);
    }
}

/// Jump held for the first 8 frames, pressed on frame 0 when `press`.
fn jump_input(frame: usize, dt: f32, press: bool, axis_x: f32) -> InputState {
    InputState {
        movement: ae::ActionEdges::EMPTY.with(
            ae::MovementAction::Jump,
            ae::Edge { pressed: press && frame == 0, held: frame < 8, released: false },
        ),
        axes: ae::LocalAxes::new(axis_x, 0.0),
        control_dt: dt,
        ..Default::default()
    }
}

#[test]
fn square_arena_wall_cling_does_not_teleport() {
    let world = square_arena_subset();
    let mut player = clinging_to_a_left_wall(&world, ae::Vec2::new(62.0, 1567.91), ae::Vec2::new(0.0, 31.1));
    let initial = player.kinematics.pos;
    step_pressing_left(&world, &mut player, 1.0 / 144.0);
    print_body("after step", &player);
    assert_inside_vertically(&world, &player);
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
    let world = square_arena_subset();
    // 0.01 px x penetration into the wall.
    let mut player = clinging_to_a_left_wall(&world, ae::Vec2::new(62.0 - 0.01, 1567.91), ae::Vec2::new(0.0, 31.1));
    let initial = player.kinematics.pos;
    step_pressing_left(&world, &mut player, 1.0 / 144.0);
    print_body("after step", &player);
    assert_inside_vertically(&world, &player);
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
    let (world, _) = sandbox_room("square_arena").expect("arena");
    // Scan: try the world with each block in turn (one block alone) to
    // see if it triggers the teleport.
    for (i, candidate) in world.blocks.iter().enumerate() {
        let single = World::new("single", world.size, world.spawn, vec![candidate.clone()]);
        let mut player = clinging_to_a_left_wall(&world, ae::Vec2::new(62.0, 1567.9125), ae::Vec2::new(0.0, 31.148));
        player.kinematics.facing = -1.0;
        let initial = player.kinematics.pos;
        step_pressing_left(&single, &mut player, 0.0069);
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
    let (world, augmented) = sandbox_room("square_arena").expect("arena");
    // EXACT live state from frame 1087 of trace 1777905256-095151097-000000.
    let mut player = clinging_to_a_left_wall(&world, ae::Vec2::new(62.0, 1567.9125), ae::Vec2::new(0.0, 31.148));
    player.kinematics.facing = -1.0;
    let initial = player.kinematics.pos;
    let dt = 0.00677_f32; // live frame 1088 real_dt
    step_pressing_left(&augmented, &mut player, dt);
    print_body("FULL WORLD step", &player);
    assert_inside_vertically(&world, &player);
    assert_within_displacement_budget(
        "square_arena_wall_cling_full_world",
        initial,
        player.kinematics.pos,
        ae::Vec2::new(0.0, 31.148),
        dt,
    );
}

#[test]
fn square_arena_wall_cling_full_world_steps_many_times() {
    let (world, augmented) = sandbox_room("square_arena").expect("arena");
    let mut player = clinging_to_a_left_wall(&world, ae::Vec2::new(62.0, 1567.9125), ae::Vec2::new(0.0, 31.148));
    player.kinematics.facing = -1.0;
    for i in 0..200 {
        step_pressing_left(&augmented, &mut player, 0.0069);
        if player.kinematics.pos.y < 100.0 || player.kinematics.pos.y > world.size.y - 32.0 {
            print_body(&format!("TELEPORT at iter {i}"), &player);
            panic!("TELEPORT at iter {i}: pos={:?}", player.kinematics.pos);
        }
    }
    print_body("after 200 steps", &player);
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
    let world = square_arena_subset();
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
                    // Player half-width ~14; right edge of wall is x=48,
                    // so clinging body sits at x≈62 (+ penetration).
                    let mut player = clinging_to_a_left_wall(&world, ae::Vec2::new(62.0 + pen, y), ae::Vec2::new(0.0, vy));
                    player.kinematics.facing = -1.0;
                    let initial = player.kinematics.pos;
                    let pre_vel = player.kinematics.vel;
                    step_pressing_left(&world, &mut player, dt);
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

/// The full production trigger may need additional context (encounter running, lock wall
/// hot-inserted) that this minimal fixture deliberately omits; that's tracked separately under
/// the parry contact-normal fix (path_forward step ).
#[test]
fn mob_lab_lock_wall_cling_does_not_teleport() {
    let world = World::new(
        "mob_lab_subset",
        ae::Vec2::new(1808.0, 1264.0),
        ae::Vec2::new(80.0, 1232.0),
        vec![
            // Arena ceiling: top at y=0, 32px tall, full width.
            Block::solid("arena_ceiling", ae::Vec2::new(0.0, 0.0), ae::Vec2::new(1808.0, 32.0)),
            // Lock wall (runtime-inserted in production): x=480..704, y=400..608.
            lock_wall("lockwall:mob_lab"),
            // Floor below.
            Block::solid("floor", ae::Vec2::new(0.0, 1232.0), ae::Vec2::new(1808.0, 32.0)),
        ],
    );
    // Wall-cling pose: right edge of the lock wall (x=704), player
    // sitting just outside at x=718, y=434.1 (per trace).
    let mut player = clinging_to_a_left_wall(&world, ae::Vec2::new(718.0, 434.1), ae::Vec2::new(0.0, 31.1));
    let initial = player.kinematics.pos;
    step_pressing_left(&world, &mut player, 1.0 / 144.0);
    print_body("after step", &player);
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

/// Unlike `mob_lab_lock_wall_cling_does_not_teleport` (a 3-block subset),
/// this loads the REAL goblin_encounter world — all its LDtk blocks in
/// production order — and APPENDS the runtime lock wall last, exactly as
/// `sync_lock_walls` does when the encounter goes Active. It then drives
/// the body off the lock-wall edge with a strong upward velocity (the
/// post-wall-jump state the trace blames) through the x=704..720 top-wall
/// corner.
#[test]
fn goblin_encounter_full_world_lock_wall_cling_repro() {
    let Some((world, mut augmented)) = sandbox_room("goblin_encounter") else {
        return;
    };
    // Appended last, as `sync_lock_walls` inserts it.
    augmented.blocks.push(lock_wall("lockwall:goblin_encounter"));
    println!(
        "goblin_encounter world: size={:?} spawn={:?} blocks={}",
        world.size,
        world.spawn,
        augmented.blocks.len()
    );
    for b in &augmented.blocks {
        if b.aabb.min.y < 64.0 || b.name.starts_with("lockwall") {
            println!("  block {:?} aabb min={:?} max={:?}", b.name, b.aabb.min, b.aabb.max);
        }
    }
    // EXACT live state from the trace: wall-clinging on the lock wall's
    // right edge (lock_wall.right=704), player just outside at x=718.
    let mut player = clinging_to_a_left_wall(&world, ae::Vec2::new(718.0, 434.1), ae::Vec2::new(0.0, 31.1));
    player.kinematics.facing = -1.0;
    // Simulate the post-wall-jump state directly (the simulation phase
    // doesn't process the jump press): a strong upward velocity, pushed
    // away from the wall. The body then rises from the lock-wall edge
    // into the x=704..720 top-wall corner — exactly the
    // edge-touching-then-vertical-sweep configuration the trace blames.
    player.kinematics.vel = ae::Vec2::new(180.0, -560.0);
    player.axis_mut().wall_clinging = false;
    drive_within_budget(&augmented, &mut player, "goblin_encounter_full_world_walljump", |frame, dt| {
        jump_input(frame, dt, false, 0.0)
    });
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
    let Some((world, mut augmented)) = sandbox_room("goblin_encounter") else {
        return;
    };
    augmented.blocks.push(lock_wall("lockwall:goblin_encounter"));
    let mut player = clinging_to_a_left_wall(&world, ae::Vec2::new(718.0, 434.1), ae::Vec2::new(0.0, 31.1));
    player.kinematics.facing = -1.0;
    drive_within_budget(&augmented, &mut player, "goblin_encounter_real_walljump", |frame, dt| {
        jump_input(frame, dt, true, if frame == 0 { -1.0 } else { 0.0 })
    });
}

/// Load the game's merged LDtk project the way a sim entry point does:
/// install the world manifest first — post-R3.2 the engine ships no worlds
/// and panics without a provider-owned manifest.
fn load_project_for_test() -> Result<ambition_platformer2d::ldtk_map::LdtkProject, String> {
    ambition_platformer2d::ldtk_map::LdtkProject::load_default_for_dev(
        &ambition_content::worlds::world_manifest(),
    )
}
