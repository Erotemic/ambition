//! Unit tests for the parent module, extracted from an inline
//! `#[cfg(test)] mod tests` (test-organization campaign, 2026-07-10). Pure move:
//! same test names + logic, now an adjacent child module with private access via
//! `use super::*;`.

use super::*;
use crate::geometry::AabbExt;
use crate::movement::default_player_body_size;
use crate::world::{Block, BlockKind, World};

fn scratch_at(pos: Vec2) -> crate::BodyClusterScratch {
    crate::BodyClusterScratch::new_with_abilities(pos, crate::AbilitySet::sandbox_all())
}

fn locomotion(s: &crate::BodyClusterScratch) -> LocomotionState {
    LocomotionState::from_body(&s.model, &s.ground, &s.wall, &s.flight)
}

#[test]
fn locomotion_default_grounded_when_player_on_ground() {
    let mut s = scratch_at(Vec2::new(0.0, 0.0));
    s.ground.on_ground = true;
    assert_eq!(locomotion(&s), LocomotionState::Grounded);
}

#[test]
fn locomotion_airborne_when_off_ground() {
    let s = scratch_at(Vec2::new(0.0, 0.0));
    assert_eq!(locomotion(&s), LocomotionState::Airborne);
}

#[test]
fn locomotion_dashing_overrides_other_states() {
    let mut s = scratch_at(Vec2::new(0.0, 0.0));
    s.ground.on_ground = true;
    s.axis_mut().dash_timer = 0.10;
    assert_eq!(locomotion(&s), LocomotionState::Dashing);
}

#[test]
fn locomotion_blink_aiming_recognized() {
    let mut s = scratch_at(Vec2::new(0.0, 0.0));
    s.axis_mut().blink_aiming = true;
    assert_eq!(locomotion(&s), LocomotionState::BlinkAiming);
}

#[test]
fn body_shape_smaller_for_crouch_and_morph() {
    let base = Vec2::new(28.0, 46.0);
    let standing = BodyMode::Standing.shape(base);
    let crouch = BodyMode::Crouching.shape(base);
    let morph = BodyMode::MorphBall.shape(base);
    assert_eq!(standing.size, base);
    assert!(crouch.size.y < standing.size.y);
    assert!(morph.size.x < standing.size.x);
    assert!(morph.size.y < standing.size.y);
}

#[test]
fn body_fits_at_open_space() {
    let world = World::new(
        "test",
        Vec2::new(200.0, 200.0),
        Vec2::new(50.0, 50.0),
        Vec::new(),
    );
    let shape = BodyMode::Standing.shape(Vec2::new(28.0, 46.0));
    assert!(shape.fits_at(Vec2::new(50.0, 50.0), &world, |_| true));
}

#[test]
fn body_does_not_fit_inside_solid_block() {
    let world = World::new(
        "test",
        Vec2::new(200.0, 200.0),
        Vec2::new(50.0, 50.0),
        vec![Block::solid(
            "ceiling",
            Vec2::new(40.0, 40.0),
            Vec2::new(60.0, 30.0),
        )],
    );
    // Standing fits below the ceiling but not under it; check the
    // collision-safe stand-up case directly.
    let standing = BodyMode::Standing.shape(Vec2::new(28.0, 46.0));
    assert!(!standing.fits_at(Vec2::new(70.0, 65.0), &world, |b| {
        matches!(b.kind, crate::world::BlockKind::Solid)
    }));
}

#[test]
fn resource_meter_try_spend_succeeds_and_reduces() {
    let mut m = ResourceMeter::new(10.0, 0.0, 0.0);
    assert!(m.try_spend(3.0));
    assert!((m.current - 7.0).abs() < 1e-4);
}

#[test]
fn resource_meter_try_spend_fails_when_insufficient() {
    let mut m = ResourceMeter::new(2.0, 0.0, 0.0);
    assert!(!m.try_spend(5.0));
    assert_eq!(m.current, 2.0);
}

#[test]
fn resource_meter_regen_clamps_to_max() {
    let mut m = ResourceMeter::new(10.0, 5.0, 0.0);
    m.current = 8.0;
    m.tick_regen(1.0);
    assert!((m.current - 10.0).abs() < 1e-4);
}

#[test]
fn resource_meter_decay_clamps_at_zero() {
    let mut m = ResourceMeter::new(10.0, 0.0, 100.0);
    m.current = 1.0;
    m.tick_decay(1.0);
    assert_eq!(m.current, 0.0);
}

#[test]
fn try_change_body_mode_to_crouching_keeps_feet_planted_and_shrinks() {
    let world = World::new(
        "test",
        Vec2::new(400.0, 400.0),
        Vec2::new(50.0, 50.0),
        Vec::new(),
    );
    let mut s = scratch_at(Vec2::new(100.0, 100.0));
    let original_size = s.kinematics.size;
    let original_feet = s.kinematics.pos.y + s.kinematics.size.y * 0.5;

    let ok = try_change_body_mode_clusters(
        &mut s.kinematics,
        &s.base_size,
        &mut s.body_mode,
        BodyMode::Crouching,
        &world,
        Vec2::new(0.0, 1.0),
        |_| true,
    );
    assert!(ok);
    assert_eq!(s.body_mode.body_mode, BodyMode::Crouching);
    assert!(s.kinematics.size.y < original_size.y);
    assert_eq!(s.kinematics.size.x, original_size.x);
    let new_feet = s.kinematics.pos.y + s.kinematics.size.y * 0.5;
    assert!((new_feet - original_feet).abs() < 1e-3);
}

#[test]
fn shrinking_is_allowed_when_surface_contact_slightly_overlaps_the_floor() {
    let floor_y = 140.0;
    let world = World::new(
        "contact tolerance",
        Vec2::new(400.0, 400.0),
        Vec2::new(100.0, 100.0),
        vec![crate::Block::solid(
            "floor",
            Vec2::new(0.0, floor_y),
            Vec2::new(400.0, 20.0),
        )],
    );
    let mut s = scratch_at(Vec2::new(100.0, floor_y - 24.0 + 0.25));
    assert!(
        world.body_overlaps_any(s.kinematics.aabb(), |_| true),
        "the fixture reproduces a tiny inherited support overlap"
    );

    let ok = try_change_body_mode_clusters(
        &mut s.kinematics,
        &s.base_size,
        &mut s.body_mode,
        BodyMode::Crouching,
        &world,
        Vec2::Y,
        |_| true,
    );
    assert!(
        ok,
        "a feet-anchored shrink is a subset of the occupied space and cannot create a new collision"
    );
    assert_eq!(s.body_mode.body_mode, BodyMode::Crouching);
}

#[test]
fn try_change_body_mode_to_crouching_keeps_feet_planted_under_sideways_gravity() {
    // Under wall-walking gravity the FEET are the AABB's gravity-facing side
    // edge. Crouching must keep that side edge planted; otherwise the body
    // pulls away from the wall, loses ground contact, and flickers between
    // standing/crouching on successive frames.
    let world = World::new(
        "test",
        Vec2::new(400.0, 400.0),
        Vec2::new(200.0, 200.0),
        Vec::new(),
    );
    let gravity = Vec2::new(1.0, 0.0);
    let mut s = scratch_at(Vec2::new(100.0, 100.0));
    let original_size = s.kinematics.size;
    let original_feet = s.kinematics.aabb_oriented(gravity).feet_coord(gravity);

    let ok = try_change_body_mode_clusters(
        &mut s.kinematics,
        &s.base_size,
        &mut s.body_mode,
        BodyMode::Crouching,
        &world,
        gravity,
        |_| true,
    );
    assert!(ok);
    assert!(
        s.kinematics.size.y < original_size.y,
        "local height should shrink"
    );
    let new_feet = s.kinematics.aabb_oriented(gravity).feet_coord(gravity);
    assert!(
        (new_feet - original_feet).abs() < 1e-3,
        "side-edge feet must stay planted under sideways gravity (moved {})",
        new_feet - original_feet
    );
}

#[test]
fn try_change_body_mode_to_crouching_keeps_feet_planted_under_inverted_gravity() {
    // Under inverted gravity the FEET are the AABB's TOP edge. Crouching must
    // keep that top edge planted (against the ceiling the player stands on),
    // not the world-bottom — otherwise the body floats off the surface and the
    // crouch flickers / loses ground contact.
    let world = World::new(
        "test",
        Vec2::new(400.0, 400.0),
        Vec2::new(200.0, 200.0),
        Vec::new(),
    );
    let mut s = scratch_at(Vec2::new(100.0, 100.0));
    let original_size = s.kinematics.size;
    let original_feet = s.kinematics.pos.y - s.kinematics.size.y * 0.5; // TOP edge

    let ok = try_change_body_mode_clusters(
        &mut s.kinematics,
        &s.base_size,
        &mut s.body_mode,
        BodyMode::Crouching,
        &world,
        Vec2::new(0.0, -1.0), // inverted gravity
        |_| true,
    );
    assert!(ok);
    assert!(s.kinematics.size.y < original_size.y, "body should shrink");
    let new_feet = s.kinematics.pos.y - s.kinematics.size.y * 0.5;
    assert!(
        (new_feet - original_feet).abs() < 1e-3,
        "top-edge feet must stay planted under inverted gravity (moved {})",
        new_feet - original_feet
    );
}

#[test]
fn try_change_body_mode_back_to_standing_uses_base_size() {
    let world = World::new(
        "test",
        Vec2::new(400.0, 400.0),
        Vec2::new(50.0, 50.0),
        Vec::new(),
    );
    let mut s = scratch_at(Vec2::new(100.0, 100.0));
    let base = s.base_size.base_size;
    try_change_body_mode_clusters(
        &mut s.kinematics,
        &s.base_size,
        &mut s.body_mode,
        BodyMode::Crouching,
        &world,
        Vec2::new(0.0, 1.0),
        |_| true,
    );
    try_change_body_mode_clusters(
        &mut s.kinematics,
        &s.base_size,
        &mut s.body_mode,
        BodyMode::Crouching,
        &world,
        Vec2::new(0.0, 1.0),
        |_| true,
    );
    let ok = try_change_body_mode_clusters(
        &mut s.kinematics,
        &s.base_size,
        &mut s.body_mode,
        BodyMode::Standing,
        &world,
        Vec2::new(0.0, 1.0),
        |_| true,
    );
    assert!(ok);
    assert_eq!(s.body_mode.body_mode, BodyMode::Standing);
    assert_eq!(s.kinematics.size, base);
}

#[test]
fn try_change_body_mode_blocked_stand_up_under_low_ceiling() {
    let player_spawn = Vec2::new(100.0, 100.0);
    let mut s = scratch_at(player_spawn);
    let standing_top = s.kinematics.pos.y - s.kinematics.size.y * 0.5;
    let ceiling_bottom = standing_top + 5.0;
    let ceiling_top = ceiling_bottom - 30.0;
    let world = World::new(
        "test",
        Vec2::new(400.0, 400.0),
        Vec2::new(50.0, 50.0),
        vec![Block::solid(
            "ceiling",
            Vec2::new(s.kinematics.pos.x - 50.0, ceiling_top),
            Vec2::new(100.0, 30.0),
        )],
    );

    let ok = try_change_body_mode_clusters(
        &mut s.kinematics,
        &s.base_size,
        &mut s.body_mode,
        BodyMode::Crouching,
        &world,
        Vec2::new(0.0, 1.0),
        |b| matches!(b.kind, crate::world::BlockKind::Solid),
    );
    assert!(ok);

    let stand_attempt = try_change_body_mode_clusters(
        &mut s.kinematics,
        &s.base_size,
        &mut s.body_mode,
        BodyMode::Standing,
        &world,
        Vec2::new(0.0, 1.0),
        |b| matches!(b.kind, crate::world::BlockKind::Solid),
    );
    assert!(!stand_attempt);
    assert_eq!(s.body_mode.body_mode, BodyMode::Crouching);
}

#[test]
fn try_change_body_mode_to_same_mode_is_no_op_success() {
    let world = World::new(
        "test",
        Vec2::new(400.0, 400.0),
        Vec2::new(50.0, 50.0),
        Vec::new(),
    );
    let mut s = scratch_at(Vec2::new(100.0, 100.0));
    let pos_before = s.kinematics.pos;
    let size_before = s.kinematics.size;
    let ok = try_change_body_mode_clusters(
        &mut s.kinematics,
        &s.base_size,
        &mut s.body_mode,
        BodyMode::Standing,
        &world,
        Vec2::new(0.0, 1.0),
        |_| true,
    );
    assert!(ok);
    assert_eq!(s.kinematics.pos, pos_before);
    assert_eq!(s.kinematics.size, size_before);
}

#[test]
fn resource_meter_fraction_handles_zero_max() {
    let m = ResourceMeter {
        current: 5.0,
        max: 0.0,
        regen_rate: 0.0,
        decay_rate: 0.0,
    };
    assert_eq!(m.fraction(), 0.0);
}

/// Regression test for the morph_lab tunnel: a one-grid-cell
/// (16 px) gap between a ceiling at y=336 and a floor at y=352
/// must allow MorphBall through and block every other body
/// mode (Standing, Crouching, Crawling, Sliding). The morph-ball
/// shape is decoupled from `base_size` precisely so the
/// discriminator survives changes to the player's standing
/// dimensions — earlier the multiplier-based morph ball became
/// 16.5 px when `base_size.x` grew to 30, snagging on the
/// 16-px tunnel even though the player had transitioned into
/// morph mode.
#[test]
fn morphball_fits_one_grid_cell_tunnel() {
    // A "tunnel" sandwich: floor at y=352, low ceiling at y=336.
    // 16-px gap between them. Player center at y=344 (midway).
    let world = World::new(
        "morph_tunnel",
        Vec2::new(200.0, 500.0),
        Vec2::ZERO,
        vec![
            Block::solid("floor", Vec2::new(0.0, 352.0), Vec2::new(200.0, 40.0)),
            Block::solid("ceiling", Vec2::new(0.0, 200.0), Vec2::new(200.0, 136.0)),
        ],
    );
    let base = default_player_body_size();
    let center = Vec2::new(100.0, 344.0);
    let solid_predicate = |b: &Block| matches!(b.kind, BlockKind::Solid);
    assert!(
        BodyMode::MorphBall
            .shape(base)
            .fits_at(center, &world, solid_predicate),
        "MorphBall must fit a 16-px tunnel — the morph_lab tunnel is sized exactly this way",
    );
    for non_fit in [
        BodyMode::Standing,
        BodyMode::Crouching,
        BodyMode::Crawling,
        BodyMode::Sliding,
    ] {
        assert!(
            !non_fit.shape(base).fits_at(center, &world, solid_predicate),
            "{:?} must NOT fit the 16-px morph-lab tunnel (discriminator broken)",
            non_fit,
        );
    }
}

/// Priority-order contract for the body-native `LocomotionState::from_body`:
/// grounded at rest, the axis policy's private dash maneuver overrides it,
/// and a non-axis policy projects to grounded/airborne from the shared
/// support fact (it owns no axis maneuver verbs).
#[test]
fn locomotion_from_body_priority_order_and_non_axis_projection() {
    use crate::body_clusters::{BodyFlightState, BodyGroundState, BodyWallState};
    use crate::movement::{AxisSweptParams, MomentumParams, MotionModel};
    let ground = BodyGroundState {
        on_ground: true,
        ..Default::default()
    };
    let wall = BodyWallState::default();
    let flight = BodyFlightState::default();
    let model = MotionModel::axis_swept(AxisSweptParams::default());
    assert_eq!(
        LocomotionState::from_body(&model, &ground, &wall, &flight),
        LocomotionState::Grounded
    );

    let ground = BodyGroundState::default();
    let mut model = MotionModel::axis_swept(AxisSweptParams::default());
    let MotionModel::AxisSwept(axis) = &mut model else {
        unreachable!();
    };
    axis.state.dash_timer = 0.1;
    assert_eq!(
        LocomotionState::from_body(&model, &ground, &wall, &flight),
        LocomotionState::Dashing,
        "the model-private dash timer overrides ground state"
    );

    let momentum = MotionModel::surface_momentum(MomentumParams::default());
    assert_eq!(
        LocomotionState::from_body(&momentum, &ground, &wall, &flight),
        LocomotionState::Airborne,
        "non-axis policies project grounded/airborne from the support fact"
    );
    let ground = BodyGroundState {
        on_ground: true,
        ..Default::default()
    };
    assert_eq!(
        LocomotionState::from_body(&momentum, &ground, &wall, &flight),
        LocomotionState::Grounded
    );
}

#[test]
fn body_mode_from_clusters_reads_authoritative_field() {
    use crate::body_clusters::BodyModeState;
    let bm = BodyModeState {
        body_mode: BodyMode::Crouching,
    };
    assert_eq!(BodyMode::from_clusters(&bm), BodyMode::Crouching);
}
