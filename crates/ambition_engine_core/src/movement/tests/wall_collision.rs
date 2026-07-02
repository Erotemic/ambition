//! One-way platforms, wall-jump catapult guards, wall-cling y-sweep
//! teleport guards, top-corner landing, and the `body_is_side_contact`
//! predicate that the y-sweep / vertical resolver share.

use super::super::*;
use super::{step_scratch, test_world};
use crate::body_clusters::BodyClusterScratch;
use crate::geometry::AabbExt;
use crate::world::Block;
use crate::{Aabb, AbilitySet, Vec2, World};

fn scratch_with(abilities: AbilitySet, spawn: Vec2) -> BodyClusterScratch {
    BodyClusterScratch::new_with_abilities(spawn, abilities)
}

#[test]
fn one_way_platform_requires_down_plus_jump_to_drop_through() {
    let mut world = test_world();
    // One-way platform suspended above the floor. Player will land on it
    // from above and we expect plain "down" alone to keep them resting.
    let plat_top_y = 600.0;
    world.blocks.push(Block::one_way(
        "drop test platform",
        Vec2::new(360.0, plat_top_y),
        Vec2::new(180.0, 12.0),
    ));

    let mut scratch = scratch_with(AbilitySet::sandbox_all(), world.spawn);
    scratch.kinematics.pos = Vec2::new(450.0, plat_top_y - scratch.kinematics.size.y * 0.5);
    scratch.kinematics.vel = Vec2::ZERO;
    scratch.ground.on_ground = false;

    // Settle onto the platform.
    for _ in 0..6 {
        step_scratch(&world, &mut scratch, InputState::default());
    }
    assert!(
        scratch.ground.on_ground,
        "player should land on the one-way"
    );
    let resting_y = scratch.kinematics.pos.y;

    // Holding down alone must NOT drop through anymore.
    for _ in 0..6 {
        step_scratch(
            &world,
            &mut scratch,
            InputState {
                axis_y: 1.0,
                ..Default::default()
            },
        );
    }
    assert!(
        (scratch.kinematics.pos.y - resting_y).abs() < 1.0,
        "down-alone must not drop through one-way (moved {} px)",
        scratch.kinematics.pos.y - resting_y
    );

    // Down + jump (with the explicit drop_through_pressed gesture) drops.
    // Critically the gesture only fires for one frame: the presentation
    // layer recomputes drop_through_pressed each frame from
    // `axis_y > 0.35 && jump_pressed`, and `jump_pressed` is just-pressed,
    // so subsequent frames see drop_through_pressed=false. The engine must
    // latch the drop-through internally for long enough to clear the
    // landing-tolerance band.
    step_scratch(
        &world,
        &mut scratch,
        InputState {
            axis_y: 1.0,
            jump_pressed: true,
            ..Default::default()
        },
    );
    for _ in 0..10 {
        step_scratch(
            &world,
            &mut scratch,
            InputState {
                axis_y: 1.0,
                // jump_pressed is NOT held: this
                // is exactly the input shape the sandbox produces after
                // the initial press.
                ..Default::default()
            },
        );
    }
    assert!(
        scratch.kinematics.pos.y > resting_y + 12.0,
        "down+jump should drop the player below the one-way (delta {})",
        scratch.kinematics.pos.y - resting_y
    );
}

/// Wall-jumping off the left wall while the player's body slightly
/// overlaps a wide horizontal block (floor/ceiling) must not catapult
/// the player out the opposite side of the room.
#[test]
fn wall_jump_does_not_catapult_through_left_wall() {
    let world = test_world();
    let mut scratch = scratch_with(AbilitySet::sandbox_all(), world.spawn);

    // Park the player against the left wall with a tiny overlap into the
    // floor (1 pixel deep) — the kind of residual penetration the engine
    // tolerates between sweeps.
    let body = scratch.kinematics.aabb();
    let left_wall_right = 36.0;
    let floor_top = world.size.y - 48.0;
    scratch.kinematics.pos.x = left_wall_right + body.half_size().x; // touching wall on its right edge
    scratch.kinematics.pos.y = floor_top - body.half_size().y + 1.0; // bottom 1 px below floor top
    scratch.kinematics.vel = Vec2::ZERO;
    scratch.ground.on_ground = false;
    scratch.wall.on_wall = true;
    scratch.wall.wall_normal_x = 1.0;
    scratch.ground.coyote_timer = 0.0;

    let initial_x = scratch.kinematics.pos.x;
    let _ = update_player_with_tuning_scratch(
        &world,
        &mut scratch,
        InputState {
            axis_x: -1.0,
            axis_y: 0.0,
            jump_pressed: true,
            jump_held: true,
            control_dt: 1.0 / 60.0,
            ..Default::default()
        },
        1.0 / 60.0,
        DEFAULT_TUNING,
    );

    assert!(
            scratch.kinematics.pos.x >= initial_x - 1.0,
            "wall jump pushed player to x={} from x={} — expected to stay near or right of starting position",
            scratch.kinematics.pos.x,
            initial_x,
        );
    assert!(
        scratch.kinematics.pos.x - body.half_size().x >= 0.0,
        "wall jump punched the player through the left wall (body left = {})",
        scratch.kinematics.pos.x - body.half_size().x,
    );
}

#[test]
fn wall_jump_uses_local_side_axis_under_sideways_gravity() {
    let world = test_world();
    let mut scratch = scratch_with(AbilitySet::sandbox_all(), world.spawn);
    let mut tuning = DEFAULT_TUNING;
    tuning.gravity_dir = Vec2::new(1.0, 0.0);

    scratch.ground.on_ground = false;
    scratch.ground.coyote_timer = 0.0;
    scratch.wall.on_wall = true;
    scratch.wall.wall_normal_x = 1.0;
    scratch.action_buffer.jump = tuning.jump_buffer;
    scratch.kinematics.vel = Vec2::ZERO;

    let mut events = FrameEvents::default();
    {
        let mut clusters = scratch.as_mut();
        super::super::simulation::handle_jump_buffer_clusters(
            &world,
            clusters.action_buffer,
            clusters.env_contact,
            clusters.abilities,
            clusters.body_mode.body_mode,
            clusters.flight.fly_enabled,
            clusters.kinematics,
            clusters.ground,
            clusters.wall,
            clusters.jump,
            clusters.combo_trace,
            InputState::default(),
            tuning,
            &mut events,
        );
    }

    let frame = crate::AccelerationFrame::new(tuning.gravity_dir);
    assert!(
        scratch.kinematics.vel.dot(frame.side) > tuning.wall_jump_x * 0.9,
        "wall jump should kick along local side; vel={:?} side={:?}",
        scratch.kinematics.vel,
        frame.side,
    );
    assert!(
        scratch.kinematics.vel.dot(frame.down) < -tuning.jump_speed * 0.8,
        "wall jump should launch away from feet; vel={:?} down={:?}",
        scratch.kinematics.vel,
        frame.down,
    );
}

/// Closer match to the actual reported bug: the player has a tiny
/// residual penetration into the left wall (sub-pixel rounding from
/// the previous frame's snap) and is moving away from it on
/// wall-jump.
#[test]
fn wall_jump_does_not_catapult_player_off_wall_overlap() {
    let world = test_world();
    let mut scratch = scratch_with(AbilitySet::sandbox_all(), world.spawn);
    let body = scratch.kinematics.aabb();
    let left_wall_right = 36.0;
    scratch.kinematics.pos.x = left_wall_right + body.half_size().x - 1.0;
    scratch.kinematics.pos.y = world.size.y * 0.5;
    scratch.kinematics.vel = Vec2::new(500.0, -650.0); // wall-jump initial velocities
    scratch.ground.on_ground = false;
    scratch.wall.on_wall = false;
    scratch.wall.wall_normal_x = 0.0;

    let initial_x = scratch.kinematics.pos.x;
    let _ = update_player_simulation_with_tuning_scratch(
        &world,
        &mut scratch,
        InputState {
            axis_x: -1.0,
            control_dt: 1.0 / 60.0,
            ..Default::default()
        },
        1.0 / 60.0,
        DEFAULT_TUNING,
    );

    let dx = (scratch.kinematics.pos.x - initial_x).abs();
    assert!(
        dx < 30.0,
        "wall overlap caused horizontal teleport: dx={dx}, pos.x went from {initial_x} to {}",
        scratch.kinematics.pos.x,
    );
    assert!(
        scratch.kinematics.pos.x - body.half_size().x >= 0.0 - 0.5,
        "player was punched through the left wall: body left = {}",
        scratch.kinematics.pos.x - body.half_size().x,
    );
}

/// Regression: reproduces the wall-cling → Grounded teleport.
#[test]
fn wall_cling_does_not_teleport_to_wall_top_on_y_sweep() {
    let world = test_world();
    let mut scratch = scratch_with(AbilitySet::sandbox_all(), world.spawn);
    let half = scratch.kinematics.size * 0.5;
    let wall_right = 36.0;
    scratch.kinematics.pos.x = wall_right + half.x - 0.05;
    scratch.kinematics.pos.y = world.size.y * 0.5; // ~450, well inside the room
    scratch.kinematics.vel = Vec2::new(0.0, DEFAULT_TUNING.wall_slide_speed);
    scratch.ground.on_ground = false;
    scratch.wall.on_wall = true;
    scratch.wall.wall_normal_x = 1.0;
    scratch.wall.wall_clinging = true;

    let initial_y = scratch.kinematics.pos.y;
    let _ = update_player_simulation_with_tuning_scratch(
        &world,
        &mut scratch,
        InputState {
            axis_x: -1.0,
            control_dt: 1.0 / 60.0,
            ..Default::default()
        },
        1.0 / 60.0,
        DEFAULT_TUNING,
    );

    assert!(
            scratch.kinematics.pos.y >= 0.0 && scratch.kinematics.pos.y <= world.size.y,
            "wall-cling y-sweep teleported player out of the world envelope: pos.y = {} (world.size.y = {})",
            scratch.kinematics.pos.y,
            world.size.y,
        );
    let dy = (scratch.kinematics.pos.y - initial_y).abs();
    assert!(
            dy < 50.0,
            "wall-cling y-sweep moved player by {dy} px in one frame; expected at most a few pixels of slide",
        );
    assert!(
        !scratch.ground.on_ground,
        "wall-cling y-sweep falsely set on_ground; player was supposedly grounded at y={}",
        scratch.kinematics.pos.y,
    );
}

/// Regression: player wall-clinging on a tall column whose top
/// is far above the player must NOT teleport upward when their
/// body partially overlaps the column on its bottom edge.
#[test]
fn partial_wall_cling_overlap_does_not_teleport_upward() {
    let world = World {
        name: "column".into(),
        size: Vec2::new(1600.0, 768.0),
        spawn: Vec2::new(50.0, 50.0),
        // Column matching the trace: x=[704, 720], y=[16, 400].
        // Center=(712, 208), size=(16, 384).
        blocks: vec![Block::solid(
            "column",
            Vec2::new(712.0, 208.0),
            Vec2::new(16.0, 384.0),
        )],
        water_regions: Vec::new(),
        climbable_regions: Vec::new(),
    };
    let mut scratch = scratch_with(AbilitySet::sandbox_all(), world.spawn);
    // Reproduce the exact pre-OOB state from the trace.
    scratch.kinematics.pos = Vec2::new(718.0, 419.0);
    scratch.kinematics.vel = Vec2::new(0.0, 15.0); // gravity-decelerated tiny downward
    scratch.ground.on_ground = false;
    scratch.wall.on_wall = true;
    scratch.wall.wall_clinging = true;
    scratch.wall.wall_normal_x = -1.0;

    let start_y = scratch.kinematics.pos.y;
    let _ = update_player_simulation_with_tuning_scratch(
        &world,
        &mut scratch,
        InputState {
            control_dt: 1.0 / 60.0,
            axis_x: -1.0, // pressing toward wall
            ..Default::default()
        },
        1.0 / 60.0,
        DEFAULT_TUNING,
    );

    let dy = (scratch.kinematics.pos.y - start_y).abs();
    assert!(
        dy < 50.0,
        "y-sweep teleported player by {} px; expected ~tiny gravity-driven motion (start_y={}, end_y={})",
        dy, start_y, scratch.kinematics.pos.y,
    );
    assert!(
        scratch.kinematics.pos.y > 0.0 && scratch.kinematics.pos.y < world.size.y,
        "player ended OOB at y={}",
        scratch.kinematics.pos.y,
    );
}

/// Descending onto the top corner of a tall solid (a pillar) with
/// slight x overlap should still resolve as a normal landing.
#[test]
fn descending_onto_top_corner_of_tall_block_lands_normally() {
    let world = World {
        name: "pillar".into(),
        size: Vec2::new(800.0, 600.0),
        spawn: Vec2::new(50.0, 50.0),
        blocks: vec![Block::solid(
            "pillar",
            Vec2::new(380.0, 200.0),
            Vec2::new(40.0, 400.0),
        )],
        water_regions: Vec::new(),
        climbable_regions: Vec::new(),
    };
    let mut scratch = scratch_with(AbilitySet::sandbox_all(), world.spawn);
    scratch.kinematics.pos = Vec2::new(391.0, 200.0 - 23.0 - 0.5);
    scratch.kinematics.vel = Vec2::new(0.0, 200.0);
    scratch.ground.on_ground = false;
    scratch.wall.on_wall = false;

    let _ = update_player_simulation_with_tuning_scratch(
        &world,
        &mut scratch,
        InputState {
            control_dt: 1.0 / 60.0,
            ..Default::default()
        },
        1.0 / 60.0,
        DEFAULT_TUNING,
    );

    let body = scratch.kinematics.aabb();
    assert!(
        scratch.ground.on_ground,
        "descending onto pillar top should land (on_ground = true); got pos={:?}",
        scratch.kinematics.pos
    );
    assert!(
        (body.bottom() - 200.0).abs() < 1.0,
        "body.bottom should snap to pillar.top = 200; got {} (pos.y = {})",
        body.bottom(),
        scratch.kinematics.pos.y,
    );
}

/// Direct unit test of `body_is_side_contact`. Both `sweep_player_y`
/// and `resolve_vertical` consult it to avoid the wall-cling teleport
/// class.
#[test]
fn body_is_side_contact_classifies_walls_vs_floors() {
    // Player about to land on a wide floor.
    let body = Aabb::new(Vec2::new(50.0, 100.0), Vec2::new(14.0, 23.0));
    let floor = Aabb::new(Vec2::new(80.0, 125.0), Vec2::new(60.0, 6.0));
    assert!(
        !body_is_side_contact(body, floor),
        "player about to land on a wide floor must NOT be classified as a side contact"
    );

    // Tall left wall, body fully alongside it.
    let wall = Aabb::new(Vec2::new(18.0, 450.0), Vec2::new(18.0, 450.0));
    let body_alongside_edge = Aabb::new(Vec2::new(36.0 + 14.0, 450.0), Vec2::new(14.0, 23.0));
    assert!(
        body_is_side_contact(body_alongside_edge, wall),
        "body alongside a tall wall (edge-touching on x) must be a side contact"
    );

    // Same wall, body penetrating by 1 px on x.
    let body_inside_wall = Aabb::new(Vec2::new(36.0 + 14.0 - 1.0, 450.0), Vec2::new(14.0, 23.0));
    assert!(
        body_is_side_contact(body_inside_wall, wall),
        "body penetrating a tall wall on x is still a side contact"
    );

    // Player landing on the top corner of a tall block (small x overlap).
    let pillar = Aabb::new(Vec2::new(900.0, 800.0), Vec2::new(40.0, 200.0));
    let body_landing_on_pillar = Aabb::new(
        Vec2::new(900.0 - 40.0 + 5.0, 600.0 - 23.0 + 1.0),
        Vec2::new(14.0, 23.0),
    );
    assert!(
            !body_is_side_contact(body_landing_on_pillar, pillar),
            "descending onto the top edge of a tall block (slight x overlap, body.top above block.top) must NOT be classified as a side contact"
        );

    // Player jumping up into a thick ceiling block.
    let ceiling = Aabb::new(Vec2::new(900.0, 200.0), Vec2::new(400.0, 100.0));
    let body_under_ceiling = Aabb::new(Vec2::new(900.0, 300.0 + 23.0 - 1.0), Vec2::new(14.0, 23.0));
    assert!(
            !body_is_side_contact(body_under_ceiling, ceiling),
            "rising into a thick ceiling (body.bottom poking past block.bottom) must NOT be classified as a side contact"
        );
}

/// Regression: flying deep into the wide, thin ceiling must push the body DOWN
/// the short way, never shove it out the ceiling block's far X edge. The old
/// overlap-depth de-penetration heuristic ejected a deeply-penetrating body
/// hundreds of px in X -- out the ceiling's far edge, into the border wall /
/// past the world -- which the OOB detector flagged as "inside solid" /
/// "outside world (y)" in the fly-into-ceiling traces.
#[test]
fn deep_ceiling_penetration_resolves_down_not_out_the_far_x_edge() {
    let world = test_world(); // ceiling (0,0)-(1600,24); right wall x:1564-1600
    let mut scratch = scratch_with(AbilitySet::sandbox_all(), world.spawn);
    // Place the 40-tall body fully spanning the 24-tall ceiling (top at the world
    // top), mid-span so its near X edge is ~800px away -- the exact shape the old
    // code mis-read as an X penetration and shoved out the far edge.
    scratch.kinematics.pos = Vec2::new(800.0, 20.0);
    scratch.kinematics.vel = Vec2::ZERO;
    scratch.ground.on_ground = false;

    step_scratch(&world, &mut scratch, InputState::default());

    let pos = scratch.kinematics.pos;
    let half = scratch.kinematics.size * 0.5;
    assert!(
        (pos.x - 800.0).abs() < 2.0,
        "deep ceiling de-pen shoved the body sideways out the far edge: x={}",
        pos.x,
    );
    assert!(
        pos.y - half.y >= 24.0 - 1.0e-3,
        "body still embedded in the ceiling: top={}, ceiling bottom=24",
        pos.y - half.y,
    );
    let body = scratch.kinematics.aabb();
    assert!(
        body.left() >= 0.0
            && body.right() <= world.size.x
            && body.top() >= 0.0
            && body.bottom() <= world.size.y,
        "body left the world envelope: {body:?}",
    );

    // Gravity carries it down over time; it never re-enters a solid or leaves.
    for _ in 0..120 {
        step_scratch(&world, &mut scratch, InputState::default());
        let b = scratch.kinematics.aabb();
        assert!(
            b.left() >= 0.0 && b.right() <= world.size.x && b.top() >= 0.0,
            "body left the world while falling: {b:?}",
        );
    }
}

/// Realistic repro: hold up+right into the top-right corner (where the wide thin
/// ceiling meets the right wall) under flight for a long time. The body settles
/// in the corner and is never ejected out the ceiling's far X edge / past the
/// world.
#[test]
fn flying_into_the_ceiling_corner_never_ejects_the_body_from_the_world() {
    let world = test_world();
    let mut scratch = scratch_with(AbilitySet::sandbox_all(), world.spawn);
    step_scratch(
        &world,
        &mut scratch,
        InputState {
            fly_toggle_pressed: true,
            ..Default::default()
        },
    );
    assert!(scratch.flight.fly_enabled);
    scratch.kinematics.pos = Vec2::new(1400.0, 200.0);
    scratch.kinematics.vel = Vec2::ZERO;
    scratch.ground.on_ground = false;

    for _ in 0..240 {
        step_scratch(
            &world,
            &mut scratch,
            InputState {
                axis_x: 1.0,
                axis_y: -1.0,
                ..Default::default()
            },
        );
        let b = scratch.kinematics.aabb();
        assert!(
            b.left() >= 0.0
                && b.right() <= world.size.x
                && b.top() >= 0.0
                && b.bottom() <= world.size.y,
            "flying into the corner ejected the body from the world: {b:?}",
        );
    }
    // Pressed into the corner: under the ceiling (bottom y=24), left of the right
    // wall (left edge x=1564).
    let b = scratch.kinematics.aabb();
    assert!(
        b.top() >= 24.0 - 1.0,
        "should rest under the ceiling, got top={}",
        b.top()
    );
    assert!(
        b.right() <= 1564.0 + 1.0,
        "should rest left of the right wall, got right={}",
        b.right(),
    );
}

#[test]
fn sliding_along_the_ceiling_edge_does_not_teleport_across_the_room() {
    // Repro of the fly-along-the-ceiling OOB (trace 1780544963): the 30x48 body
    // slides LEFT just under a wide thin ceiling, its top grazing the ceiling's
    // bottom edge, at high speed. The swept cast reports a *non-immediate*
    // grazing contact with the ceiling; the de-pen must NOT shove the body out
    // the ceiling's far X edge (it was teleporting ~900px right, to the world's
    // right wall / out of the world).
    let world = World {
        name: "hub ceiling".to_string(),
        size: Vec2::new(1900.0, 2004.0),
        spawn: Vec2::new(950.0, 883.0),
        blocks: vec![
            Block::solid("ceiling", Vec2::new(0.0, 0.0), Vec2::new(1904.0, 32.0)),
            Block::solid(
                "right wall",
                Vec2::new(1856.0, 32.0),
                Vec2::new(48.0, 224.0),
            ),
            Block::solid("floor", Vec2::new(0.0, 1980.0), Vec2::new(1904.0, 24.0)),
        ],
        water_regions: Vec::new(),
        climbable_regions: Vec::new(),
    };
    let mut scratch = scratch_with(AbilitySet::sandbox_all(), world.spawn);
    scratch.kinematics.size = Vec2::new(30.0, 48.0);
    scratch.base_size.base_size = Vec2::new(30.0, 48.0);
    scratch.kinematics.pos = Vec2::new(1000.0, 56.01); // top ~= ceiling bottom (32)
    scratch.kinematics.vel = Vec2::new(-760.0, 0.0); // flying left, fast
    scratch.ground.on_ground = false;
    scratch.flight.fly_enabled = true;

    for i in 0..40 {
        step_scratch(
            &world,
            &mut scratch,
            InputState {
                axis_x: -1.0,
                ..Default::default()
            },
        );
        let b = scratch.kinematics.aabb();
        assert!(
            b.right() <= world.size.x && b.left() >= 0.0,
            "frame {i}: body teleported out of the world: pos={:?} aabb={b:?}",
            scratch.kinematics.pos,
        );
    }
}

/// Exact reproduction of trace 1780544963 tick 644 (fly left along the hub
/// ceiling): the body's top grazes the ceiling's bottom edge while sweeping
/// LEFT, the swept cast returns a spurious *non-immediate* grazing hit, and the
/// de-pen pushed the body out the ceiling's far X edge -- a ~918px teleport
/// RIGHT, past the world. Calls the X-sweep directly with the captured state.
#[test]
fn ceiling_graze_x_sweep_does_not_teleport_body_to_the_far_edge() {
    // Exact trace 1780544963 tick 644 inputs, calling the X-sweep directly.
    let world = World {
        name: "hub ceiling".to_string(),
        size: Vec2::new(1900.0, 2004.0),
        spawn: Vec2::new(950.0, 883.0),
        blocks: vec![
            Block::solid("ceiling", Vec2::new(0.0, 0.0), Vec2::new(1904.0, 32.0)),
            Block::solid(
                "right wall",
                Vec2::new(1856.0, 32.0),
                Vec2::new(48.0, 224.0),
            ),
        ],
        water_regions: Vec::new(),
        climbable_regions: Vec::new(),
    };
    let mut scratch = scratch_with(AbilitySet::sandbox_all(), world.spawn);
    scratch.kinematics.size = Vec2::new(30.0, 48.0);
    scratch.base_size.base_size = Vec2::new(30.0, 48.0);
    scratch.kinematics.pos = Vec2::new(1000.9404, 56.01338);
    scratch.kinematics.vel = Vec2::new(-760.0, 1.6108618);
    let dt = 0.006874742_f32;
    let delta_x = scratch.kinematics.vel.x * dt;
    let gravity_dir = crate::Vec2::new(0.0, 1.0);
    let prev_feet_coord = scratch
        .kinematics
        .aabb_oriented(gravity_dir)
        .feet_coord(gravity_dir);
    super::super::collision::sweep_player_axis_clusters(
        &world,
        &mut scratch.kinematics,
        &mut scratch.ground,
        &mut scratch.wall,
        &scratch.body_mode,
        &scratch.env_contact,
        crate::collision_semantics::Axis::X,
        delta_x,
        prev_feet_coord,
        false,
        gravity_dir,
    );
    let after = scratch.kinematics.pos;
    assert!(
        after.x < 1100.0,
        "X-sweep teleported the body out the ceiling's far edge: pos={after:?} (delta_x={delta_x})",
    );
}

#[test]
fn one_way_drop_through_works_under_inverted_gravity() {
    // The reported bug: with gravity inverted (pointing up), you couldn't drop
    // through a one-way platform. Deterministic mirror of the down-gravity test:
    // the player rests on the platform's BOTTOM face and local "down + jump"
    // drops them through, toward gravity (-Y). Raw screen-UP is mapped into this
    // local intent before the movement engine sees `InputState`.
    use crate::movement::tuning::DEFAULT_TUNING;
    let g = Vec2::new(0.0, -1.0);
    let tuning = MovementTuning {
        gravity_dir: g,
        gravity_sign: -1.0,
        ..DEFAULT_TUNING
    };
    let step = |world: &World, scratch: &mut BodyClusterScratch, input: InputState| {
        update_player_with_tuning_scratch(world, scratch, input, 1.0 / 60.0, tuning);
    };

    let mut world = test_world();
    // one-way platform; its BOTTOM face is the gravity-up (rest) side under inversion.
    let plat_min_y = 400.0;
    world.blocks.push(Block::one_way(
        "inv drop platform",
        Vec2::new(360.0, plat_min_y),
        Vec2::new(180.0, 12.0),
    ));
    let plat_bottom = plat_min_y + 12.0;

    let mut scratch = scratch_with(AbilitySet::sandbox_all(), world.spawn);
    scratch.kinematics.pos = Vec2::new(450.0, plat_bottom + scratch.kinematics.size.y * 0.5);
    scratch.kinematics.vel = Vec2::ZERO;
    scratch.ground.on_ground = false;

    for _ in 0..6 {
        step(&world, &mut scratch, InputState::default());
    }
    let resting_y = scratch.kinematics.pos.y;
    assert!(
        scratch.ground.on_ground,
        "player should rest on the one-way's bottom face under inverted gravity"
    );

    // In engine `InputState`, axis_y is already controlled-body-local.
    // Local down alone (toward gravity) must NOT drop through.
    for _ in 0..6 {
        step(
            &world,
            &mut scratch,
            InputState {
                axis_y: 1.0,
                ..Default::default()
            },
        );
    }
    assert!(
        (scratch.kinematics.pos.y - resting_y).abs() < 1.0,
        "descend-alone must not drop through (moved {} px)",
        scratch.kinematics.pos.y - resting_y
    );

    // Local down + jump drops through, toward gravity (-Y, so pos.y decreases).
    step(
        &world,
        &mut scratch,
        InputState {
            axis_y: 1.0,
            jump_pressed: true,
            ..Default::default()
        },
    );
    for _ in 0..10 {
        step(
            &world,
            &mut scratch,
            InputState {
                axis_y: 1.0,
                ..Default::default()
            },
        );
    }
    assert!(
        scratch.kinematics.pos.y < resting_y - 12.0,
        "descend+jump should drop the player through toward gravity under inverted gravity (delta {})",
        scratch.kinematics.pos.y - resting_y
    );
}

#[test]
fn sideways_gravity_blink_wall_is_ground_support() {
    // Blink walls are still authored surfaces for contact: blink pathing may pass
    // through them with upgrades, but a controlled body standing on their
    // gravity-facing face should be grounded just like on Solid.
    use crate::movement::tuning::DEFAULT_TUNING;
    use crate::world::{BlinkWallTier, Block, World};

    let world = World {
        name: "side blink support".to_string(),
        size: Vec2::new(800.0, 600.0),
        spawn: Vec2::new(500.0, 300.0),
        blocks: vec![Block::blink_wall(
            "blink support",
            Vec2::new(300.0, 120.0),
            Vec2::new(16.0, 360.0),
            BlinkWallTier::Soft,
        )],
        water_regions: Vec::new(),
        climbable_regions: Vec::new(),
    };
    let mut scratch = scratch_with(AbilitySet::sandbox_all(), world.spawn);
    scratch.kinematics.vel = Vec2::ZERO;
    scratch.ground.on_ground = false;

    let g = Vec2::new(-1.0, 0.0); // feet point screen-left
    let tuning = MovementTuning {
        gravity_dir: g,
        gravity_sign: 1.0,
        ..DEFAULT_TUNING
    };
    for _ in 0..90 {
        update_player_with_tuning_scratch(
            &world,
            &mut scratch,
            InputState::default(),
            1.0 / 60.0,
            tuning,
        );
    }

    let body = scratch.kinematics.aabb_oriented(g);
    assert!(
        scratch.ground.on_ground,
        "blink-wall side support must ground"
    );
    assert!(
        (body.left() - 316.0).abs() < 6.0,
        "feet edge should rest on the blink wall's right face; left={}",
        body.left()
    );
}

#[test]
fn one_way_platform_works_under_sideways_gravity() {
    // One-way passability is authored against the acceleration frame: under
    // gravity-left the platform's right face is its anti-gravity/top face.
    use crate::movement::tuning::DEFAULT_TUNING;
    use crate::world::{Block, World};

    let world = World {
        name: "side one-way".to_string(),
        size: Vec2::new(800.0, 600.0),
        spawn: Vec2::new(500.0, 300.0),
        blocks: vec![Block::one_way(
            "side oneway",
            Vec2::new(300.0, 120.0),
            Vec2::new(16.0, 360.0),
        )],
        water_regions: Vec::new(),
        climbable_regions: Vec::new(),
    };
    let mut scratch = scratch_with(AbilitySet::sandbox_all(), world.spawn);
    scratch.kinematics.vel = Vec2::ZERO;
    scratch.ground.on_ground = false;

    let g = Vec2::new(-1.0, 0.0);
    let tuning = MovementTuning {
        gravity_dir: g,
        gravity_sign: 1.0,
        ..DEFAULT_TUNING
    };
    for _ in 0..90 {
        update_player_with_tuning_scratch(
            &world,
            &mut scratch,
            InputState::default(),
            1.0 / 60.0,
            tuning,
        );
    }
    let resting_x = scratch.kinematics.pos.x;
    let body = scratch.kinematics.aabb_oriented(g);
    assert!(
        scratch.ground.on_ground,
        "sideways one-way must be standable"
    );
    assert!(
        (body.left() - 316.0).abs() < 6.0,
        "feet edge should rest on the one-way's right face; left={}",
        body.left()
    );

    // Local down alone does not drop through.
    for _ in 0..6 {
        update_player_with_tuning_scratch(
            &world,
            &mut scratch,
            InputState {
                axis_y: 1.0,
                ..Default::default()
            },
            1.0 / 60.0,
            tuning,
        );
    }
    assert!(
        (scratch.kinematics.pos.x - resting_x).abs() < 1.0,
        "descend-alone must not drop through sideways one-way"
    );

    update_player_with_tuning_scratch(
        &world,
        &mut scratch,
        InputState {
            axis_y: 1.0,
            jump_pressed: true,
            ..Default::default()
        },
        1.0 / 60.0,
        tuning,
    );
    for _ in 0..12 {
        update_player_with_tuning_scratch(
            &world,
            &mut scratch,
            InputState {
                axis_y: 1.0,
                ..Default::default()
            },
            1.0 / 60.0,
            tuning,
        );
    }
    assert!(
        scratch.kinematics.pos.x < resting_x - 12.0,
        "descend+jump should drop through toward local down/gravity; delta {}",
        scratch.kinematics.pos.x - resting_x
    );
}

/// Regression for the central-hub OOB clip caught by the actor OOB trace
/// (2026-06-25): under sideways gravity the player walked/fell into a wide hub
/// solid and was pushout-teleported hundreds of pixels — once 310px on the side
/// axis, once 163px clear out of the world's left edge — in a single tick. The
/// shared kinematic primitive already forbids this
/// (`is_contact_range_snap`); the authored player collision path did not, so
/// every penetration-resolution snap/push here must stay bounded by the body's
/// own half-extent. This pins the player path to the same no-pushout invariant.
#[test]
fn deeply_embedded_player_is_not_pushout_teleported_under_sideways_gravity() {
    use crate::world::Block;
    use crate::AbilitySet;

    // A big solid the player is jammed inside, far from every face — like the
    // hub's wide floor/ceiling slabs. With gravity along +X, X is the gravity
    // axis and Y the side axis, exercising both resolution branches.
    let world = World {
        name: "embed-hub".into(),
        size: Vec2::new(1900.0, 2004.0),
        spawn: Vec2::new(950.0, 1000.0),
        blocks: vec![
            Block::solid("slab", Vec2::new(300.0, 300.0), Vec2::new(700.0, 700.0)),
            // Containing walls (like the hub's perimeter) so a body falling
            // sideways out of the slab is CAUGHT, never flung out of the world.
            Block::solid("left wall", Vec2::new(0.0, 0.0), Vec2::new(48.0, 2004.0)),
            Block::solid(
                "right wall",
                Vec2::new(1852.0, 0.0),
                Vec2::new(48.0, 2004.0),
            ),
        ],
        water_regions: Vec::new(),
        climbable_regions: Vec::new(),
    };
    let start = Vec2::new(650.0, 650.0); // deep inside the 700x700 slab

    // (1) A SINGLE resolution step must stay bounded in every cardinal gravity.
    // Gravity magnitude is zeroed so the only motion is penetration resolution
    // (no free-fall to confound the per-tick budget) — the body's own velocity,
    // not a pushout teleport, is what carries it out the near face over later
    // frames. This mirrors the kinematic primitive's `deeply_embedded` guard.
    for dir in [
        Vec2::new(0.0, 1.0),
        Vec2::new(0.0, -1.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(-1.0, 0.0),
    ] {
        let tuning = MovementTuning {
            gravity: 0.0,
            gravity_dir: dir,
            ..DEFAULT_TUNING
        };
        let mut scratch =
            BodyClusterScratch::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
        scratch.kinematics.pos = start;
        scratch.kinematics.vel = Vec2::ZERO;
        let cap = scratch.kinematics.aabb_oriented(dir).half_size().length() + 1.0;
        update_player_with_tuning_scratch(
            &world,
            &mut scratch,
            InputState::default(),
            1.0 / 60.0,
            tuning,
        );
        let moved = (scratch.kinematics.pos - start).length();
        assert!(
            moved <= cap,
            "gravity {dir:?}: penetration resolution pushout-teleported the player \
             {moved:.1}px (cap {cap:.1}px) to {:?}",
            scratch.kinematics.pos
        );
    }

    // (2) Under real sideways gravity the body falls THROUGH the slab and out
    // its near face, but must never be flung outside the world envelope — the
    // actual OOB the actor trace recorded (player at x=-81, outside x).
    let tuning = MovementTuning {
        gravity_dir: Vec2::new(-1.0, 0.0),
        ..DEFAULT_TUNING
    };
    let mut scratch =
        BodyClusterScratch::new_with_abilities(world.spawn, AbilitySet::sandbox_all());
    scratch.kinematics.pos = start;
    scratch.kinematics.vel = Vec2::ZERO;
    for tick in 0..240 {
        update_player_with_tuning_scratch(
            &world,
            &mut scratch,
            InputState::default(),
            1.0 / 60.0,
            tuning,
        );
        let p = scratch.kinematics.pos;
        assert!(
            p.x > 0.0 && p.x < world.size.x && p.y > 0.0 && p.y < world.size.y,
            "tick {tick}: player was flung out of the world to {p:?}",
        );
    }
}
