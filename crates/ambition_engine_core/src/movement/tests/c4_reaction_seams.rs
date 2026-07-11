//! C4 gravity-rotation conformance for the FULL body tick — the reaction seams.
//!
//! The `step_kinematic` primitive has axis-role conformance tests; the app has
//! `gravity_symmetry_room` for movement verbs. What was missing (fable review
//! 2026-07-02, `docs/archive/reviews/fable-review-2026-07-02.md` §B) is the same rig at
//! the `update_player_with_tuning_clusters` level for the *reaction/effect
//! epilogues* — slash recoil, blink fallback aim, post-blink velocity cleanup,
//! wall-slide ordering — where a verb correct in its main path historically kept
//! a screen-frame fallback.
//!
//! Method: author one scenario in the controlled body's LOCAL frame (side = +x,
//! down/toward-feet = +y), instantiate it under each cardinal gravity by rotating
//! the world + spawn through the arm's [`AccelerationFrame`], drive the SAME
//! local-frame input sequence, convert each world sample back to the arm's local
//! frame, and require all four traces to match the down-gravity baseline. If a
//! mechanic is frame-correct by construction the traces are identical up to
//! floating-point noise; any screen-frame epilogue shows up as a per-tick diff on
//! the rotated arms.

use super::*;
use crate::abilities::AbilitySet;
use crate::body_clusters::BodyClusterScratch;
use crate::world::Block;
use crate::{AccelerationFrame, Vec2, World};

/// Square rig world so a 90° rotation maps the world onto itself.
const WORLD_EXTENT: f32 = 1600.0;
const CENTER: Vec2 = Vec2::new(WORLD_EXTENT * 0.5, WORLD_EXTENT * 0.5);
const DT: f32 = 1.0 / 60.0;
/// Rotation by cardinal frames is exact component swaps; the tolerance only
/// absorbs `CENTER + x` vs `CENTER - x` float rounding.
const TOL: f32 = 0.02;

#[derive(Clone, Copy, Debug)]
struct Arm {
    name: &'static str,
    dir: Vec2,
}

fn arms() -> [Arm; 4] {
    [
        Arm {
            name: "down",
            dir: Vec2::new(0.0, 1.0),
        },
        Arm {
            name: "left",
            dir: Vec2::new(-1.0, 0.0),
        },
        Arm {
            name: "up",
            dir: Vec2::new(0.0, -1.0),
        },
        Arm {
            name: "right",
            dir: Vec2::new(1.0, 0.0),
        },
    ]
}

/// Local rig coordinate (side, down) -> world position.
fn world_from_local(f: &AccelerationFrame, local: Vec2) -> Vec2 {
    CENTER + f.side * local.x + f.down * local.y
}

/// World vector -> local (side, down) components.
fn local_vec(f: &AccelerationFrame, world: Vec2) -> Vec2 {
    Vec2::new(world.dot(f.side), world.dot(f.down))
}

/// A block authored in local rig coordinates, rotated into the arm's world.
fn local_block(
    f: &AccelerationFrame,
    name: &str,
    local_min: Vec2,
    local_size: Vec2,
    one_way: bool,
) -> Block {
    let local_center = local_min + local_size * 0.5;
    let world_center = world_from_local(f, local_center);
    let world_half = f.to_world_half(local_size * 0.5);
    let world_min = world_center - world_half;
    if one_way {
        Block::one_way(name, world_min, world_half * 2.0)
    } else {
        Block::solid(name, world_min, world_half * 2.0)
    }
}

/// The rig's authored furniture, in local coordinates: an open floor with a tall
/// wall rising from its right end. `(min, size, one_way)` tuples.
fn rig_blocks() -> Vec<(&'static str, Vec2, Vec2, bool)> {
    vec![
        // Floor: top face (local "ground") at down=300, spanning side [-400, 400].
        (
            "floor",
            Vec2::new(-400.0, 300.0),
            Vec2::new(800.0, 48.0),
            false,
        ),
        // Wall on the +side, face at side=120, spanning down [-320, 300].
        (
            "wall",
            Vec2::new(120.0, -320.0),
            Vec2::new(48.0, 620.0),
            false,
        ),
    ]
}

fn rig_world(f: &AccelerationFrame) -> World {
    World {
        name: "c4 reaction rig".to_string(),
        size: Vec2::new(WORLD_EXTENT, WORLD_EXTENT),
        spawn: CENTER,
        blocks: rig_blocks()
            .into_iter()
            .map(|(name, min, size, one_way)| local_block(f, name, min, size, one_way))
            .collect(),
        water_regions: Vec::new(),
        climbable_regions: Vec::new(),
        chains: Vec::new(),
    }
}

#[derive(Clone, Debug)]
struct LocalSample {
    pos: Vec2,
    vel: Vec2,
    on_ground: bool,
    on_wall: bool,
    facing: f32,
}

/// Per-tick local-frame input. Fields that are world-space at the engine seam
/// (`blink_quick_dir`, `blink_aim_step`) are authored local here and rotated
/// into the arm's world by the driver — mirroring what the input bridge does.
#[derive(Clone, Copy, Debug, Default)]
struct LocalInput {
    axis: Vec2,
    jump_pressed: bool,
    jump_held: bool,
    attack_pressed: bool,
    blink_pressed: bool,
    blink_held: bool,
    blink_released: bool,
    /// Local-frame quick-blink dir; rotated to world per arm. Zero = neutral.
    blink_quick_local: Vec2,
}

fn drive(arm: Arm, spawn_local: Vec2, script: &[LocalInput]) -> Vec<LocalSample> {
    let f = AccelerationFrame::new(arm.dir);
    let world = rig_world(&f);
    let mut tuning = DEFAULT_TUNING;
    tuning.gravity_dir = arm.dir;
    tuning.gravity_sign = if arm.dir.y != 0.0 { arm.dir.y } else { 1.0 };

    let spawn = world_from_local(&f, spawn_local);
    let mut scratch = BodyClusterScratch::new_with_abilities(spawn, AbilitySet::sandbox_all());

    script
        .iter()
        .map(|li| {
            let input = InputState {
                axis_x: li.axis.x,
                axis_y: li.axis.y,
                jump_pressed: li.jump_pressed,
                jump_held: li.jump_held,
                attack_pressed: li.attack_pressed,
                blink_pressed: li.blink_pressed,
                blink_held: li.blink_held,
                blink_released: li.blink_released,
                blink_quick_dir: f.to_world(li.blink_quick_local),
                ..InputState::default()
            };
            update_player_with_tuning_scratch(&world, &mut scratch, input, DT, tuning);
            LocalSample {
                pos: local_vec(&f, scratch.kinematics.pos - CENTER),
                vel: local_vec(&f, scratch.kinematics.vel),
                on_ground: scratch.ground.on_ground,
                on_wall: scratch.wall.on_wall,
                facing: scratch.kinematics.facing,
            }
        })
        .collect()
}

fn assert_c4(name: &str, spawn_local: Vec2, script: &[LocalInput]) {
    let all = arms();
    let reference = drive(all[0], spawn_local, script);
    for arm in all.into_iter().skip(1) {
        let actual = drive(arm, spawn_local, script);
        for (i, (a, b)) in reference.iter().zip(&actual).enumerate() {
            let dp = b.pos - a.pos;
            let dv = b.vel - a.vel;
            assert!(
                dp.x.abs() <= TOL && dp.y.abs() <= TOL,
                "{name} / {} arm tick {i} local pos: got ({:.3}, {:.3}), \
                 expected ({:.3}, {:.3})",
                arm.name,
                b.pos.x,
                b.pos.y,
                a.pos.x,
                a.pos.y,
            );
            assert!(
                dv.x.abs() <= TOL && dv.y.abs() <= TOL,
                "{name} / {} arm tick {i} local vel: got ({:.3}, {:.3}), \
                 expected ({:.3}, {:.3})",
                arm.name,
                b.vel.x,
                b.vel.y,
                a.vel.x,
                a.vel.y,
            );
            assert_eq!(
                b.on_ground, a.on_ground,
                "{name} / {} arm tick {i} on_ground",
                arm.name
            );
            assert_eq!(
                b.on_wall, a.on_wall,
                "{name} / {} arm tick {i} on_wall",
                arm.name
            );
            assert_eq!(
                b.facing, a.facing,
                "{name} / {} arm tick {i} facing",
                arm.name
            );
        }
    }
}

/// Feet on the floor's local ground face at the given side coordinate.
fn on_floor(side_x: f32) -> Vec2 {
    Vec2::new(side_x, 300.0 - DEFAULT_PLAYER_BODY_HEIGHT * 0.5)
}

fn ticks(n: usize, li: LocalInput) -> Vec<LocalInput> {
    vec![li; n]
}

/// Rig sanity: the main movement path (run, jump, land) is already known
/// frame-correct — if THIS fails, the rig itself leaks a frame.
#[test]
fn c4_run_jump_land_trace_matches() {
    let mut script = ticks(
        14,
        LocalInput {
            axis: Vec2::new(-1.0, 0.0),
            ..LocalInput::default()
        },
    );
    script.push(LocalInput {
        jump_pressed: true,
        jump_held: true,
        ..LocalInput::default()
    });
    script.extend(ticks(
        40,
        LocalInput {
            jump_held: true,
            ..LocalInput::default()
        },
    ));
    assert_c4("run+jump+land", on_floor(-104.0), &script);
}

/// B4 (fable review §B): slash recoil must kick along the body's LOCAL side
/// axis (opposite facing), not world X.
#[test]
fn c4_slash_recoil_kicks_along_local_side() {
    let mut script = vec![LocalInput {
        attack_pressed: true,
        ..LocalInput::default()
    }];
    script.extend(ticks(10, LocalInput::default()));
    assert_c4("slash recoil", on_floor(-104.0), &script);
}

/// B9 (fable review §B): a neutral-stick quick blink falls back to "forward
/// along facing" — the body's local side axis, not world X.
#[test]
fn c4_quick_blink_neutral_falls_back_to_local_facing() {
    let mut script = vec![LocalInput {
        blink_pressed: true,
        blink_held: true,
        ..LocalInput::default()
    }];
    script.push(LocalInput {
        blink_released: true,
        ..LocalInput::default()
    });
    script.extend(ticks(8, LocalInput::default()));
    assert_c4("neutral quick blink", on_floor(-260.0), &script);
}

/// B3 (fable review §B): the post-blink damp/clamp must damp the local SIDE
/// velocity and clamp the local FALL velocity — under sideways gravity the
/// fall axis is world X, which the world-axis version never clamps.
#[test]
fn c4_post_blink_fall_clamp_is_gravity_relative() {
    // Fall from high above the floor to build fall speed well past the clamp,
    // then quick-blink sideways (explicit local +side aim, away from the wall's
    // span is irrelevant here — the blink is short).
    let mut script = ticks(30, LocalInput::default());
    script.push(LocalInput {
        blink_pressed: true,
        blink_held: true,
        ..LocalInput::default()
    });
    script.push(LocalInput {
        blink_released: true,
        blink_quick_local: Vec2::new(-1.0, 0.0),
        ..LocalInput::default()
    });
    script.extend(ticks(6, LocalInput::default()));
    assert_c4("post-blink clamp", Vec2::new(-260.0, -140.0), &script);
}

/// B5/B6 (fable review §B): wall contact + wall slide must produce the same
/// steady-state local trace in every arm. Exercises the side-axis sweep's wall
/// guards AND the wall-ability ordering, which historically differed between
/// the vertical- and horizontal-gravity branches of the body tick.
#[test]
fn c4_wall_slide_steady_state_matches() {
    // Spawn airborne near the wall (face at side=120; body half-width 15),
    // hold into it: contact, cling, slide.
    let script = ticks(
        45,
        LocalInput {
            axis: Vec2::new(1.0, 0.0),
            ..LocalInput::default()
        },
    );
    assert_c4("wall slide", Vec2::new(95.0, -60.0), &script);
}

/// B7 (fable review §B): the "fell out of the world" reset must trigger past
/// the +gravity edge of the world in EVERY arm — the old check only watched
/// the world's bottom (`pos.y > size.y + 200`), so under up/sideways gravity a
/// body fell forever.
#[test]
fn c4_out_of_bounds_reset_is_gravity_relative() {
    for arm in arms() {
        let f = AccelerationFrame::new(arm.dir);
        let world = rig_world(&f);
        let mut tuning = DEFAULT_TUNING;
        tuning.gravity_dir = arm.dir;

        // 201px past the world AABB along the fall direction: must flag reset.
        let out = CENTER + f.down * (WORLD_EXTENT * 0.5 + 201.0);
        let mut scratch = BodyClusterScratch::new_with_abilities(out, AbilitySet::sandbox_all());
        let events = update_player_simulation_with_tuning_scratch(
            &world,
            &mut scratch,
            InputState::default(),
            DT,
            tuning,
        );
        assert!(
            events.reset && events.hazard,
            "{} arm: body 201px past the +gravity edge must flag reset",
            arm.name
        );

        // 100px past is inside the grace margin: no reset.
        let near = CENTER + f.down * (WORLD_EXTENT * 0.5 + 100.0);
        let mut scratch = BodyClusterScratch::new_with_abilities(near, AbilitySet::sandbox_all());
        let events = update_player_simulation_with_tuning_scratch(
            &world,
            &mut scratch,
            InputState::default(),
            DT,
            tuning,
        );
        assert!(
            !events.reset,
            "{} arm: body 100px past the edge is within the grace margin",
            arm.name
        );
    }
}
