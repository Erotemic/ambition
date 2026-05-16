use super::*;
use crate::input::ControlFrame;
use crate::GameWorld;
use ambition_engine as ae;
use bevy::prelude::{App, Time, Update, With};

fn empty_world() -> ae::World {
    ae::World::new(
        "body_mode_test",
        ae::Vec2::new(2000.0, 2000.0),
        ae::Vec2::new(200.0, 1000.0),
        Vec::new(),
    )
}

fn ceiling_world(ceiling_top: f32, ceiling_h: f32) -> ae::World {
    ae::World::new(
        "body_mode_ceiling",
        ae::Vec2::new(2000.0, 2000.0),
        ae::Vec2::new(200.0, 1000.0),
        vec![ae::Block::solid(
            "ceiling",
            ae::Vec2::new(0.0, ceiling_top),
            ae::Vec2::new(2000.0, ceiling_h),
        )],
    )
}

fn body_app(world: ae::World) -> App {
    let mut app = App::new();
    app.insert_resource(Time::<()>::default());
    app.insert_resource(GameWorld(world.clone()));
    let mut initial_player =
        ae::Player::new_with_abilities(world.spawn, ae::AbilitySet::sandbox_all());
    initial_player.refresh_movement_resources(ae::DEFAULT_TUNING);
    app.insert_resource(ControlFrame::default());
    app.world_mut().spawn((
        crate::player::PlayerEntity,
        crate::player::PlayerMovementAuthority::new(initial_player),
        crate::player::PlayerInteractionState::default(),
    ));
    app.add_systems(Update, update_body_mode);
    app
}

/// Clone the player from the authoritative ECS component for assertions.
fn player(app: &mut App) -> ae::Player {
    let mut q = app.world_mut().query_filtered::<
        &crate::player::PlayerMovementAuthority,
        With<crate::player::PlayerEntity>,
    >();
    q.single(app.world())
        .map(|a| a.player.clone())
        .expect("no PlayerMovementAuthority")
}

/// Set player state directly on `PlayerMovementAuthority`.
fn set_grounded_at(app: &mut App, pos: ae::Vec2) {
    let mut q = app.world_mut().query_filtered::<
        &mut crate::player::PlayerMovementAuthority,
        With<crate::player::PlayerEntity>,
    >();
    for mut authority in q.iter_mut(app.world_mut()) {
        authority.player.pos = pos;
        authority.player.vel = ae::Vec2::ZERO;
        authority.player.on_ground = true;
        authority.player.on_wall = false;
        authority.player.wall_clinging = false;
        authority.player.wall_climbing = false;
        authority.player.dash_timer = 0.0;
        authority.player.blink_aiming = false;
        authority.player.water_contact = None;
    }
}

fn set_axis_y(app: &mut App, axis_y: f32) {
    let mut controls = app.world_mut().resource_mut::<ControlFrame>();
    controls.axis_y = axis_y;
}

/// Mark the double-tap-down edge on `PlayerInteractionState` exactly as
/// `input_timer_system` does in the live build. The driver consumes
/// via `mem::take`, so the test only needs to arm it before the tick
/// under test.
fn arm_double_tap_down(app: &mut App) {
    let mut q = app.world_mut().query_filtered::<
        &mut crate::player::PlayerInteractionState,
        With<crate::player::PlayerEntity>,
    >();
    for mut interaction in q.iter_mut(app.world_mut()) {
        interaction.double_tap_down_pending = true;
    }
}

/// Holding Down on the ground transitions Standing → Crouching and
/// shrinks `player.size.y`.
#[test]
fn down_held_grounded_enters_crouch() {
    let mut app = body_app(empty_world());
    set_grounded_at(&mut app, ae::Vec2::new(200.0, 500.0));
    set_axis_y(&mut app, 1.0);

    app.update();

    let p = player(&mut app);
    assert_eq!(p.body_mode, ae::BodyMode::Crouching);
    assert!(p.size.y < p.base_size.y);
}

/// Releasing Down with overhead clearance returns to Standing.
#[test]
fn down_released_returns_to_standing_when_clear() {
    let mut app = body_app(empty_world());
    set_grounded_at(&mut app, ae::Vec2::new(200.0, 500.0));

    // Crouch first.
    set_axis_y(&mut app, 1.0);
    app.update();
    assert_eq!(player(&mut app).body_mode, ae::BodyMode::Crouching);

    // Release down.
    set_axis_y(&mut app, 0.0);
    app.update();

    let p = player(&mut app);
    assert_eq!(p.body_mode, ae::BodyMode::Standing);
    assert_eq!(p.size, p.base_size);
}

/// A low ceiling above the crouched body must reject the stand-up
/// transition — the player stays crouched.
#[test]
fn stand_up_blocked_under_low_ceiling() {
    // Player at pos.y = 600, base_size.y = 46. AABB convention:
    // pos = center, +Y is downward, so:
    //   * Standing y range = [600 - 23, 600 + 23] = [577, 623].
    //   * Crouching size = 46 * 0.55 = 25.3, dy = (46-25.3)/2 = 10.35,
    //     so pos.y = 610.35 and crouched y range = [597.7, 623].
    //   * Stand-up restores pos.y = 600 and standing y range = [577, 623].
    //
    // Ceiling y range [560, 590]:
    //   * Crouched [597.7, 623] vs [560, 590]: 597.7 > 590 → no overlap.
    //   * Standing [577, 623] vs [560, 590]: 577 < 590 → overlap.
    // Initial standing also overlaps; the helper doesn't reject pre-
    // existing penetration — it only checks the *target* shape, so
    // the crouch transition still succeeds and the stand-up correctly
    // fails.
    let world = ceiling_world(560.0, 30.0);
    let mut app = body_app(world);
    set_grounded_at(&mut app, ae::Vec2::new(200.0, 600.0));

    // Crouch.
    set_axis_y(&mut app, 1.0);
    app.update();
    assert_eq!(player(&mut app).body_mode, ae::BodyMode::Crouching);

    // Release down — stand-up should be blocked.
    set_axis_y(&mut app, 0.0);
    app.update();
    let p = player(&mut app);
    assert_eq!(p.body_mode, ae::BodyMode::Crouching);
    assert!(p.size.y < p.base_size.y);
}

/// In the air, holding Down does not crouch (crouch is grounded only).
#[test]
fn airborne_down_does_not_crouch() {
    let mut app = body_app(empty_world());
    {
        let mut q = app.world_mut().query_filtered::<
            &mut crate::player::PlayerMovementAuthority,
            With<crate::player::PlayerEntity>,
        >();
        for mut authority in q.iter_mut(app.world_mut()) {
            authority.player.pos = ae::Vec2::new(200.0, 200.0);
            authority.player.on_ground = false;
            authority.player.vel = ae::Vec2::ZERO;
        }
    }
    set_axis_y(&mut app, 1.0);
    app.update();
    assert_eq!(player(&mut app).body_mode, ae::BodyMode::Standing);
}

/// Mid-dash holding Down does not crouch — dash owns the body shape.
#[test]
fn dash_active_blocks_crouch() {
    let mut app = body_app(empty_world());
    set_grounded_at(&mut app, ae::Vec2::new(200.0, 500.0));
    {
        let mut q = app.world_mut().query_filtered::<
            &mut crate::player::PlayerMovementAuthority,
            With<crate::player::PlayerEntity>,
        >();
        for mut authority in q.iter_mut(app.world_mut()) {
            authority.player.dash_timer = 0.05;
        }
    }
    set_axis_y(&mut app, 1.0);
    app.update();
    assert_eq!(player(&mut app).body_mode, ae::BodyMode::Standing);
}

/// Double-tap-down on the ground from Standing curls into MorphBall.
/// The signal is `PlayerInteractionState::double_tap_down_pending` (set
/// by `input_timer_system` because `sandbox_update` consumes a local copy
/// of ControlFrame that doesn't reach later Bevy systems).
#[test]
fn double_tap_down_grounded_enters_morph_ball() {
    let mut app = body_app(empty_world());
    set_grounded_at(&mut app, ae::Vec2::new(200.0, 500.0));
    arm_double_tap_down(&mut app);
    app.update();
    let p = player(&mut app);
    assert_eq!(p.body_mode, ae::BodyMode::MorphBall);
    // MorphBall is smaller than Standing on both axes.
    assert!(p.size.x < p.base_size.x);
    assert!(p.size.y < p.base_size.y);
}

/// Crouching + double-tap-down also curls into MorphBall (reachable
/// from either entry point). Mirrors the input model in the
/// docstring.
#[test]
fn double_tap_down_from_crouch_enters_morph_ball() {
    let mut app = body_app(empty_world());
    set_grounded_at(&mut app, ae::Vec2::new(200.0, 500.0));

    // Crouch first.
    set_axis_y(&mut app, 1.0);
    app.update();
    assert_eq!(player(&mut app).body_mode, ae::BodyMode::Crouching);
    // Then double-tap-down.
    arm_double_tap_down(&mut app);
    app.update();
    assert_eq!(player(&mut app).body_mode, ae::BodyMode::MorphBall);
}

/// Jump-pressed inside MorphBall unmorphs to Standing when there's
/// overhead clearance.
#[test]
fn jump_press_in_morph_ball_unmorphs_to_standing() {
    let mut app = body_app(empty_world());
    set_grounded_at(&mut app, ae::Vec2::new(200.0, 500.0));

    // Drive into MorphBall via the gesture (covers the input path
    // and avoids juggling a second world reference inside a
    // resource borrow).
    arm_double_tap_down(&mut app);
    app.update();
    assert_eq!(player(&mut app).body_mode, ae::BodyMode::MorphBall);

    {
        let mut controls = app.world_mut().resource_mut::<ControlFrame>();
        controls.jump_pressed = true;
    }
    app.update();
    let p = player(&mut app);
    assert_eq!(p.body_mode, ae::BodyMode::Standing);
    assert_eq!(p.size, p.base_size);
}

/// Jump-pressed inside MorphBall under a low ceiling stays curled —
/// the standing AABB doesn't fit.
#[test]
fn jump_press_in_morph_ball_under_low_ceiling_stays_curled() {
    // Ceiling at y in [560, 590]: standing top 577 < 590 → blocks.
    // MorphBall body: base_size 28x46 → MorphBall is (28*0.55,
    // 28*0.55) = (15.4, 15.4). On the floor at pos.y = 600, the
    // morph ball center is at 600 + (46 - 15.4)/2 = 615.3, half
    // 7.7 → top 607.6, bottom 623.0. Crouched would be 597.7→623,
    // so the morph ball clears the ceiling at 590 by an even wider
    // margin. Standing has top 577 → blocked.
    let world = ceiling_world(560.0, 30.0);
    let mut app = body_app(world);
    set_grounded_at(&mut app, ae::Vec2::new(200.0, 600.0));

    // Morph via gesture.
    arm_double_tap_down(&mut app);
    app.update();
    assert_eq!(player(&mut app).body_mode, ae::BodyMode::MorphBall);

    // Try to unmorph.
    {
        let mut controls = app.world_mut().resource_mut::<ControlFrame>();
        controls.jump_pressed = true;
    }
    app.update();
    assert_eq!(player(&mut app).body_mode, ae::BodyMode::MorphBall);
}

/// Airborne double-tap-down does NOT curl (morph is grounded only).
#[test]
fn airborne_double_tap_down_does_not_morph() {
    let mut app = body_app(empty_world());
    {
        let mut q = app.world_mut().query_filtered::<
            &mut crate::player::PlayerMovementAuthority,
            With<crate::player::PlayerEntity>,
        >();
        for mut authority in q.iter_mut(app.world_mut()) {
            authority.player.pos = ae::Vec2::new(200.0, 200.0);
            authority.player.on_ground = false;
        }
    }
    arm_double_tap_down(&mut app);
    app.update();
    assert_eq!(player(&mut app).body_mode, ae::BodyMode::Standing);
}

/// Repro for the ControlFrame-not-flowing-through-sandbox_update
/// bug: setting `controls.fast_fall_pressed = true` directly on
/// the resource (mimicking what `input_timer_system` writes back to
/// the resource) is NOT sufficient to enter MorphBall.
/// The driver reads `PlayerInteractionState::double_tap_down_pending`.
/// This test pins the routing so a future refactor can't accidentally
/// switch the body-mode driver back to reading ControlFrame and
/// silently break the in-game gesture.
#[test]
fn morph_ball_does_not_fire_from_control_frame_alone() {
    let mut app = body_app(empty_world());
    set_grounded_at(&mut app, ae::Vec2::new(200.0, 500.0));
    {
        let mut controls = app.world_mut().resource_mut::<ControlFrame>();
        controls.fast_fall_pressed = true;
    }
    // PlayerInteractionState::double_tap_down_pending is NOT armed.
    app.update();
    assert_eq!(
        player(&mut app).body_mode,
        ae::BodyMode::Standing,
        "the body-mode driver must read PlayerInteractionState::double_tap_down_pending, \
         not controls.fast_fall_pressed (which sandbox_update consumes \
         on a local copy that doesn't reach later systems)"
    );
}

/// Death/respawn reset must restore the player to Standing with the
/// canonical base size, even if they were mid-Crouch or mid-MorphBall
/// when they died. Otherwise a respawn could land in a smaller body
/// and the engine would compute collision against the shrunk AABB
/// until the next crouch input.
#[test]
fn reset_restores_standing_from_crouch() {
    let mut app = body_app(empty_world());
    set_grounded_at(&mut app, ae::Vec2::new(200.0, 500.0));
    set_axis_y(&mut app, 1.0);
    app.update();
    assert_eq!(player(&mut app).body_mode, ae::BodyMode::Crouching);

    let world = empty_world();
    {
        let mut q = app.world_mut().query_filtered::<
            &mut crate::player::PlayerMovementAuthority,
            With<crate::player::PlayerEntity>,
        >();
        for mut authority in q.iter_mut(app.world_mut()) {
            authority.player.reset_to(world.spawn);
            authority.player.refresh_movement_resources(ae::DEFAULT_TUNING);
            authority.player.mana.refill_full();
        }
    }
    let p = player(&mut app);
    assert_eq!(p.body_mode, ae::BodyMode::Standing);
    assert_eq!(p.size, p.base_size);
}

#[test]
fn reset_restores_standing_from_morph_ball() {
    let mut app = body_app(empty_world());
    set_grounded_at(&mut app, ae::Vec2::new(200.0, 500.0));
    arm_double_tap_down(&mut app);
    app.update();
    assert_eq!(player(&mut app).body_mode, ae::BodyMode::MorphBall);

    let world = empty_world();
    {
        let mut q = app.world_mut().query_filtered::<
            &mut crate::player::PlayerMovementAuthority,
            With<crate::player::PlayerEntity>,
        >();
        for mut authority in q.iter_mut(app.world_mut()) {
            authority.player.reset_to(world.spawn);
            authority.player.refresh_movement_resources(ae::DEFAULT_TUNING);
            authority.player.mana.refill_full();
        }
    }
    let p = player(&mut app);
    assert_eq!(p.body_mode, ae::BodyMode::Standing);
    assert_eq!(p.size, p.base_size);
}

/// Wall-cling state owns the player posture; do not crouch from it.
#[test]
fn wall_clinging_blocks_crouch() {
    let mut app = body_app(empty_world());
    set_grounded_at(&mut app, ae::Vec2::new(200.0, 500.0));
    {
        let mut q = app.world_mut().query_filtered::<
            &mut crate::player::PlayerMovementAuthority,
            With<crate::player::PlayerEntity>,
        >();
        for mut authority in q.iter_mut(app.world_mut()) {
            authority.player.wall_clinging = true;
            authority.player.on_wall = true;
            authority.player.on_ground = false;
        }
    }
    set_axis_y(&mut app, 1.0);
    app.update();
    assert_eq!(player(&mut app).body_mode, ae::BodyMode::Standing);
}

/// Build a test world with a single ladder region centered at the
/// player's spawn so `World::climbable_at` returns `Some(...)` from
/// the first tick.
fn ladder_world() -> ae::World {
    ae::World::new(
        "body_mode_ladder",
        ae::Vec2::new(2000.0, 2000.0),
        ae::Vec2::new(200.0, 1000.0),
        Vec::new(),
    )
    .with_climbable_regions(vec![ae::ClimbableRegion::ladder(ae::Aabb::new(
        ae::Vec2::new(200.0, 1000.0),
        ae::Vec2::new(20.0, 200.0),
    ))])
}

/// Set the climbable_contact directly so the body_mode driver
/// sees a populated contact even though `update_body_mode` runs
/// without the engine sweep that normally populates it. In
/// production the engine populates this each tick before the
/// progression chain runs.
fn set_on_ladder(app: &mut App) {
    let world = app.world().resource::<GameWorld>().0.clone();
    let mut q = app.world_mut().query_filtered::<
        &mut crate::player::PlayerMovementAuthority,
        With<crate::player::PlayerEntity>,
    >();
    for mut authority in q.iter_mut(app.world_mut()) {
        let aabb = authority.player.aabb();
        authority.player.climbable_contact = world.climbable_at(aabb);
    }
}

#[test]
fn up_input_inside_ladder_enters_climbing() {
    let mut app = body_app(ladder_world());
    set_grounded_at(&mut app, ae::Vec2::new(200.0, 1000.0));
    set_on_ladder(&mut app);
    // Up press: axis_y < -CROUCH_AXIS_Y_THRESHOLD.
    set_axis_y(&mut app, -1.0);

    app.update();
    assert_eq!(
        player(&mut app).body_mode,
        ae::BodyMode::Climbing,
        "up-input inside a climbable contact should enter Climbing"
    );
}

#[test]
fn down_input_grounded_inside_ladder_stays_crouching_not_climbing() {
    // Important UX: standing on a floor that happens to overlap a
    // ladder bottom and pressing Down should still crouch, not
    // grab the ladder. The driver gates Down→Climbing on
    // `!on_ground` precisely for this.
    let mut app = body_app(ladder_world());
    set_grounded_at(&mut app, ae::Vec2::new(200.0, 1000.0));
    set_on_ladder(&mut app);
    set_axis_y(&mut app, 1.0); // down

    app.update();
    assert_eq!(
        player(&mut app).body_mode,
        ae::BodyMode::Crouching,
        "Down + grounded should crouch, not climb"
    );
}

#[test]
fn jump_press_while_climbing_exits_to_standing() {
    let mut app = body_app(ladder_world());
    set_grounded_at(&mut app, ae::Vec2::new(200.0, 1000.0));
    set_on_ladder(&mut app);
    set_axis_y(&mut app, -1.0);
    app.update();
    assert_eq!(player(&mut app).body_mode, ae::BodyMode::Climbing);

    // Now press jump on the next tick.
    {
        let mut controls = app.world_mut().resource_mut::<ControlFrame>();
        controls.axis_y = 0.0;
        controls.jump_pressed = true;
    }
    // Keep contact populated so the exit happens via the
    // jump-pressed branch, not the lost-contact branch.
    set_on_ladder(&mut app);
    app.update();
    assert_eq!(
        player(&mut app).body_mode,
        ae::BodyMode::Standing,
        "jump press during climb should release back to standing"
    );
}

#[test]
fn losing_climbable_contact_exits_climbing() {
    let mut app = body_app(ladder_world());
    set_grounded_at(&mut app, ae::Vec2::new(200.0, 1000.0));
    set_on_ladder(&mut app);
    set_axis_y(&mut app, -1.0);
    app.update();
    assert_eq!(player(&mut app).body_mode, ae::BodyMode::Climbing);

    // Drop the contact (simulating "moved off the ladder
    // horizontally"). No jump press.
    {
        let mut q = app.world_mut().query_filtered::<
            &mut crate::player::PlayerMovementAuthority,
            With<crate::player::PlayerEntity>,
        >();
        for mut authority in q.iter_mut(app.world_mut()) {
            authority.player.climbable_contact = None;
        }
    }
    {
        let mut controls = app.world_mut().resource_mut::<ControlFrame>();
        controls.axis_y = 0.0;
        controls.jump_pressed = false;
    }
    app.update();
    assert_eq!(
        player(&mut app).body_mode,
        ae::BodyMode::Standing,
        "losing climbable contact should drop the climb mode"
    );
}

#[test]
fn build_morph_ball_image_is_64x64_rgba() {
    let img = build_morph_ball_image();
    assert_eq!(img.texture_descriptor.size.width, 64);
    assert_eq!(img.texture_descriptor.size.height, 64);
    // 64 * 64 * 4 (RGBA) = 16384 bytes.
    assert_eq!(img.data.as_ref().map(|d| d.len()), Some(64 * 64 * 4));
}

#[test]
fn build_morph_ball_image_has_visible_center_and_transparent_corners() {
    let img = build_morph_ball_image();
    let data = img.data.as_ref().expect("image data");
    // Center pixel should be highly opaque (the ball body).
    let cx = 32;
    let cy = 32;
    let center_idx = ((cy * 64 + cx) * 4) as usize;
    let center_alpha = data[center_idx + 3];
    assert!(
        center_alpha >= 200,
        "center alpha should be near opaque, got {center_alpha}"
    );
    // Corner pixel should be fully transparent (outside the circle).
    let corner_idx = (0 * 64 + 0) * 4;
    let corner_alpha = data[corner_idx + 3];
    assert_eq!(corner_alpha, 0);
}
