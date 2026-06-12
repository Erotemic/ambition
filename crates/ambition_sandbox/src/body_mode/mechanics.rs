//! Sandbox-side body-mode driver (crouch + morph ball + collision-safe
//! stand-up).
//!
//! Listens to the deadzoned `axis_y` from `ControlFrame` and the
//! double-tap-down gesture (`fast_fall_pressed`) and asks the engine
//! to flip the player's body mode between `Standing`, `Crouching`, and
//! `MorphBall`. `try_change_body_mode_clusters` does the per-frame
//! collision-safe resize: if a low ceiling would clip the larger body
//! the helper rejects the transition and the player stays in the
//! smaller stance. Auto-detected `PlayerModeChanged` trace events
//! fire from the trace recorder diffing `body_mode` between snapshots,
//! so this driver does not push events itself.
//!
//! Body-mode mutations happen directly on `BodyKinematics` +
//! `PlayerBodyModeState` cluster components via
//! `try_change_body_mode_clusters` — no `ae::Player` aggregate, no
//! `engine_player_bridge` round-trip (both deleted 2026-05-28).
//!
//! Input model:
//! - Standing + Down held + grounded → Crouching.
//! - Standing/Crouching + double-tap Down + grounded → MorphBall.
//! - MorphBall + Jump pressed → try Standing (gated). If a low
//!   ceiling blocks the standing body, the morph ball stays curled.
//! - Crouching + Down released → Standing (gated).
//! - Standing/Crouching + Up/Down inside `climbable_contact` → Climbing.
//! - Climbing + Up + Jump → ladder jump boost, stay Climbing.
//! - Climbing + Jump without Up or Dash → push off, exit to Standing.
//!   Climbing + losing contact → exit to Standing automatically.
//! - Mid-action mechanics (dash, blink-aim, wall-cling/climb, ledge grab,
//!   swim) own the player shape; the driver no-ops while any of them are
//!   active.

use crate::engine_core as ae;
use bevy::prelude::*;

/// Threshold on `axis_y` for treating Down as "held" for crouch.
/// Mirrors the threshold used by ledge-grab drop and the engine's
/// drop-through gesture so the player feel stays consistent.
const CROUCH_AXIS_Y_THRESHOLD: f32 = 0.4;

pub fn update_body_mode(
    world: Res<crate::GameWorld>,
    mut player_q: Query<
        (
            &mut crate::player::BodyKinematics,
            &crate::player::PlayerBaseSize,
            &mut crate::player::PlayerBodyModeState,
            &mut crate::player::PlayerJumpState,
            &crate::player::PlayerGroundState,
            &crate::player::PlayerWallState,
            &crate::player::PlayerDashState,
            &crate::player::PlayerBlinkState,
            &crate::player::PlayerLedgeState,
            &crate::player::PlayerEnvironmentContact,
            &mut crate::player::PlayerInteractionState,
            &crate::player::PlayerInputFrame,
            &crate::player::PlayerFlightState,
        ),
        With<crate::player::PlayerEntity>,
    >,
) {
    let Ok((
        mut kinematics,
        base_size,
        mut body_mode_state,
        mut jump_state,
        ground,
        wall,
        dash,
        blink,
        ledge,
        env_contact,
        mut interaction,
        input,
        flight,
    )) = player_q.single_mut()
    else {
        return;
    };
    let controls = &input.frame;

    // Mid-action mechanics own the body shape — don't fight them.
    if dash.timer > 0.0 || blink.aiming {
        return;
    }
    // Wall / ledge state owns its own posture; reverting it via crouch
    // would break the ledge-grab anchor invariant.
    if wall.wall_clinging || wall.wall_climbing || ledge.grab.is_some() {
        return;
    }
    // In-water posture: leave water swim mechanics alone.
    if env_contact.water.is_some() {
        return;
    }

    let down_held = controls.axis_y > CROUCH_AXIS_Y_THRESHOLD;
    let up_held = controls.axis_y < -CROUCH_AXIS_Y_THRESHOLD;
    let on_ground = ground.on_ground;
    let mode = body_mode_state.body_mode;
    let solid = |b: &ae::Block| matches!(b.kind, ae::BlockKind::Solid);
    let climbable_contact_present = env_contact.climbable.is_some();

    // Consume the double-tap-down edge regardless of branch so we
    // don't latch a stale signal across frames or gameplay states.
    let double_tap_down = std::mem::take(&mut interaction.double_tap_down_pending);

    if !down_held {
        jump_state.ladder_drop_through_hold_lock = false;
    }

    // Climbing exits: plain jump / dash pushes off, losing contact
    // drops the mode. Jump+Up is handled by movement as a climb-speed
    // boost while keeping the ladder state.
    // Engine's `integrate_climb` defensive-zeros velocity if contact
    // is None mid-climb, so the visible result of a contact loss is a
    // one-frame velocity stall before this driver flips back to
    // Standing — acceptable for the first slice.
    if mode == ae::BodyMode::Climbing {
        if controls.jump_pressed && down_held {
            jump_state.ladder_drop_through_timer = ae::movement::ONE_WAY_DROP_THROUGH_GRACE;
            let _ = ae::try_change_body_mode_clusters(
                &mut kinematics,
                base_size,
                &mut body_mode_state,
                ae::BodyMode::Standing,
                &world.0,
                solid,
            );
            return;
        }
        let exit_via_jump = controls.jump_pressed && !up_held;
        let exit_via_dash = controls.dash_pressed;
        let exit_via_lost_contact = !climbable_contact_present;
        if exit_via_jump || exit_via_dash || exit_via_lost_contact {
            let _ = ae::try_change_body_mode_clusters(
                &mut kinematics,
                base_size,
                &mut body_mode_state,
                ae::BodyMode::Standing,
                &world.0,
                solid,
            );
            return;
        }
        // Otherwise stay Climbing — engine drives motion through
        // integrate_climb. No body-mode change this frame.
        return;
    }

    // Climbing entry: holding Up or Down inside a climbable contact
    // engages the ladder. Down is gated to NOT trigger climbing while
    // grounded (so a Down-press on a floor stays a crouch). Up, by
    // contrast, can engage from grounded as a "step onto the ladder
    // from below" gesture.
    // While flying, holding Up is "fly up", not "grab the ladder" — flight
    // suppresses ladder auto-climb so you can fly past / over a ladder without
    // snapping onto it. (Land or disable flight to climb.)
    let climb_initiator = up_held || (down_held && !on_ground && !controls.jump_pressed);
    if climbable_contact_present
        && climb_initiator
        && !flight.fly_enabled
        && jump_state.ladder_drop_through_timer <= 0.0
        && !jump_state.ladder_drop_through_hold_lock
        && mode != ae::BodyMode::MorphBall
    {
        let _ = ae::try_change_body_mode_clusters(
            &mut kinematics,
            base_size,
            &mut body_mode_state,
            ae::BodyMode::Climbing,
            &world.0,
            solid,
        );
        return;
    }

    // MorphBall has the smallest AABB. Exiting it means re-checking
    // overhead clearance; sourcing the exit input from `jump_pressed`
    // mirrors how a player would naturally try to "stand up" out of
    // the ball. Up-pressed (a tap, not held) is also accepted as the
    // unmorph gesture so keyboards that bind Up to a different
    // physical key can still escape the ball without committing to a
    // jump arc.
    if mode == ae::BodyMode::MorphBall {
        if controls.jump_pressed || controls.up_pressed {
            let _ = ae::try_change_body_mode_clusters(
                &mut kinematics,
                base_size,
                &mut body_mode_state,
                ae::BodyMode::Standing,
                &world.0,
                solid,
            );
        }
        return;
    }

    // Double-tap-down on the ground from Standing or Crouching curls
    // into MorphBall.
    if on_ground && double_tap_down {
        let _ = ae::try_change_body_mode_clusters(
            &mut kinematics,
            base_size,
            &mut body_mode_state,
            ae::BodyMode::MorphBall,
            &world.0,
            solid,
        );
        return;
    }

    let target = if down_held && on_ground {
        ae::BodyMode::Crouching
    } else {
        ae::BodyMode::Standing
    };

    if mode == target {
        return;
    }

    let _ = ae::try_change_body_mode_clusters(
        &mut kinematics,
        base_size,
        &mut body_mode_state,
        target,
        &world.0,
        solid,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine_core::world::{ClimbableKind, ClimbableRegion, ClimbableSpec, World};
    use crate::engine_core::Vec2;
    use crate::input::ControlFrame;
    use crate::player::{
        BodyKinematics, PlayerBaseSize, PlayerBlinkState, PlayerBodyModeState, PlayerDashState,
        PlayerEntity, PlayerEnvironmentContact, PlayerGroundState, PlayerInputFrame,
        PlayerInteractionState, PlayerJumpState, PlayerLedgeState, PlayerWallState, PrimaryPlayer,
    };
    use bevy::prelude::{App, Entity, Update};

    /// Minimal world with enough headroom that both Standing (~48 px
    /// tall) and MorphBall (14 px) shapes fit at the spawn position. No
    /// ceiling-clearance gotchas — the driver's `fits_at` predicate
    /// should pass both directions.
    fn open_world() -> ae::World {
        let w = 1600.0;
        let h = 900.0;
        World {
            name: "morph_ball test world".to_string(),
            size: Vec2::new(w, h),
            spawn: Vec2::new(210.0, h - 80.0),
            blocks: vec![ae::Block::solid(
                "floor",
                Vec2::new(0.0, h - 16.0),
                Vec2::new(w, 16.0),
            )],
            water_regions: Vec::new(),
            climbable_regions: Vec::new(),
        }
    }

    fn build_body_mode_test_app() -> (App, Entity) {
        let mut app = App::new();
        app.insert_resource(crate::GameWorld(open_world()));
        let world_spawn = app.world().resource::<crate::GameWorld>().0.spawn;
        app.add_systems(Update, super::update_body_mode);
        let player = app
            .world_mut()
            .spawn((
                PlayerEntity,
                PrimaryPlayer,
                BodyKinematics {
                    pos: world_spawn,
                    size: Vec2::new(30.0, 48.0),
                    facing: 1.0,
                    ..Default::default()
                },
                PlayerBaseSize {
                    base_size: Vec2::new(30.0, 48.0),
                },
                PlayerGroundState {
                    on_ground: true,
                    ..Default::default()
                },
                PlayerWallState::default(),
                PlayerDashState::default(),
                PlayerBlinkState::default(),
                PlayerLedgeState::default(),
                PlayerEnvironmentContact::default(),
                PlayerInteractionState::default(),
                PlayerInputFrame::default(),
                PlayerBodyModeState::default(),
                PlayerJumpState::default(),
                crate::player::PlayerFlightState::default(),
            ))
            .id();
        (app, player)
    }

    fn place_player_on_test_ladder(app: &mut App, player: Entity, vel: Option<Vec2>) {
        app.world_mut()
            .resource_mut::<crate::GameWorld>()
            .0
            .climbable_regions
            .push(ClimbableRegion::new(
                ae::Aabb::new(Vec2::new(210.0, 820.0), Vec2::new(20.0, 200.0)),
                ClimbableKind::Ladder,
                ClimbableSpec::default(),
            ));
        {
            let mut kin = app.world_mut().get_mut::<BodyKinematics>(player).unwrap();
            kin.pos = Vec2::new(210.0, 820.0);
            if let Some(vel) = vel {
                kin.vel = vel;
            }
        }
        let contact = app
            .world()
            .resource::<crate::GameWorld>()
            .0
            .climbable_at(app.world().get::<BodyKinematics>(player).unwrap().aabb());
        app.world_mut()
            .get_mut::<PlayerEnvironmentContact>(player)
            .unwrap()
            .climbable = contact;
    }

    /// The headline morph-ball entry path. With the player standing on
    /// the ground and `double_tap_down_pending = true`, one `update`
    /// call should flip `body_mode` to `MorphBall`. Pins the gesture
    /// → body-mode transition that the rest of the morph-ball visual
    /// chain depends on.
    #[test]
    fn double_tap_down_on_ground_transitions_to_morph_ball() {
        let (mut app, player) = build_body_mode_test_app();
        // Pre-poison so a missing transition trips loudly.
        let mut interaction = app
            .world_mut()
            .get_mut::<PlayerInteractionState>(player)
            .unwrap();
        interaction.double_tap_down_pending = true;
        app.update();
        let mode = app
            .world()
            .get::<PlayerBodyModeState>(player)
            .unwrap()
            .body_mode;
        assert_eq!(
            mode,
            ae::BodyMode::MorphBall,
            "double-tap-down on the ground must transition Standing → MorphBall",
        );
    }

    /// The exit path: from MorphBall, pressing Jump (or Up) flips back
    /// to Standing when there's headroom. Pins the
    /// `controls.jump_pressed || controls.up_pressed` exit branch.
    #[test]
    fn jump_press_from_morph_ball_transitions_to_standing() {
        let (mut app, player) = build_body_mode_test_app();
        {
            let mut body_mode = app
                .world_mut()
                .get_mut::<PlayerBodyModeState>(player)
                .unwrap();
            body_mode.body_mode = ae::BodyMode::MorphBall;
        }
        {
            let mut kin = app.world_mut().get_mut::<BodyKinematics>(player).unwrap();
            kin.size = Vec2::new(14.0, 14.0);
        }
        {
            let mut input = app.world_mut().get_mut::<PlayerInputFrame>(player).unwrap();
            input.frame = ControlFrame {
                jump_pressed: true,
                ..Default::default()
            };
        }
        app.update();
        let mode = app
            .world()
            .get::<PlayerBodyModeState>(player)
            .unwrap()
            .body_mode;
        assert_eq!(
            mode,
            ae::BodyMode::Standing,
            "Jump pressed in MorphBall must transition to Standing when headroom allows",
        );
    }

    /// Climbing should not trap the player: dash is an explicit push
    /// off just like jump, so one `update` call should flip the body
    /// mode back to Standing and let the dash consume cleanly in the
    /// player tick.
    #[test]
    fn dash_press_from_climbing_transitions_to_standing() {
        let (mut app, player) = build_body_mode_test_app();
        place_player_on_test_ladder(&mut app, player, None);
        {
            let mut body_mode = app
                .world_mut()
                .get_mut::<PlayerBodyModeState>(player)
                .unwrap();
            body_mode.body_mode = ae::BodyMode::Climbing;
        }
        {
            let mut input = app.world_mut().get_mut::<PlayerInputFrame>(player).unwrap();
            input.frame = ControlFrame {
                dash_pressed: true,
                ..Default::default()
            };
        }
        app.update();
        let mode = app
            .world()
            .get::<PlayerBodyModeState>(player)
            .unwrap()
            .body_mode;
        assert_eq!(
            mode,
            ae::BodyMode::Standing,
            "Dash pressed while climbing must transition to Standing on the same frame",
        );
    }

    /// Jump should not eject the player from climbing any more; the
    /// movement tick turns it into a ladder-speed boost instead.
    #[test]
    fn jump_press_from_climbing_keeps_climbing_mode() {
        let (mut app, player) = build_body_mode_test_app();
        place_player_on_test_ladder(&mut app, player, None);
        {
            let mut body_mode = app
                .world_mut()
                .get_mut::<PlayerBodyModeState>(player)
                .unwrap();
            body_mode.body_mode = ae::BodyMode::Climbing;
        }
        {
            let mut input = app.world_mut().get_mut::<PlayerInputFrame>(player).unwrap();
            input.frame = ControlFrame {
                jump_pressed: true,
                axis_y: -1.0,
                ..Default::default()
            };
        }
        app.update();
        let mode = app
            .world()
            .get::<PlayerBodyModeState>(player)
            .unwrap()
            .body_mode;
        assert_eq!(
            mode,
            ae::BodyMode::Climbing,
            "Jump pressed while climbing should keep the player in Climbing so movement can boost the ladder climb",
        );
    }

    /// Down + Jump on a ladder should make the player fall off, with
    /// a short grace window that prevents instantly re-grabbing.
    #[test]
    fn down_jump_from_climbing_falls_off_ladder() {
        let (mut app, player) = build_body_mode_test_app();
        place_player_on_test_ladder(&mut app, player, Some(Vec2::new(0.0, -100.0)));
        {
            let mut body_mode = app
                .world_mut()
                .get_mut::<PlayerBodyModeState>(player)
                .unwrap();
            body_mode.body_mode = ae::BodyMode::Climbing;
        }
        {
            let mut input = app.world_mut().get_mut::<PlayerInputFrame>(player).unwrap();
            input.frame = ControlFrame {
                jump_pressed: true,
                axis_y: 1.0,
                ..Default::default()
            };
        }
        app.update();
        let mode = app
            .world()
            .get::<PlayerBodyModeState>(player)
            .unwrap()
            .body_mode;
        let jump_state = app.world().get::<PlayerJumpState>(player).unwrap();
        assert_eq!(
            mode,
            ae::BodyMode::Standing,
            "Down + Jump on a ladder should fall off instead of staying in Climbing",
        );
        assert!(
            jump_state.ladder_drop_through_timer > 0.0,
            "ladder fall-off should create a grace window that blocks immediate re-grab"
        );
    }

    /// The ladder drop lock should only clear on a down release,
    /// and only then should down be able to re-grab the ladder.
    #[test]
    fn down_release_rearms_ladder_regrab() {
        let (mut app, player) = build_body_mode_test_app();
        place_player_on_test_ladder(&mut app, player, None);
        {
            let mut body_mode = app
                .world_mut()
                .get_mut::<PlayerBodyModeState>(player)
                .unwrap();
            body_mode.body_mode = ae::BodyMode::Standing;
        }
        {
            let mut ground = app
                .world_mut()
                .get_mut::<PlayerGroundState>(player)
                .unwrap();
            ground.on_ground = false;
        }
        {
            let mut jump_state = app.world_mut().get_mut::<PlayerJumpState>(player).unwrap();
            jump_state.ladder_drop_through_timer = 0.0;
            jump_state.ladder_drop_through_hold_lock = true;
        }
        {
            let mut input = app.world_mut().get_mut::<PlayerInputFrame>(player).unwrap();
            input.frame = ControlFrame {
                axis_y: 1.0,
                ..Default::default()
            };
        }
        app.update();
        assert_eq!(
            app.world()
                .get::<PlayerBodyModeState>(player)
                .unwrap()
                .body_mode,
            ae::BodyMode::Standing,
            "holding down should not re-grab the ladder while the release lock is active",
        );

        {
            let mut input = app.world_mut().get_mut::<PlayerInputFrame>(player).unwrap();
            input.frame = ControlFrame {
                axis_y: 0.0,
                ..Default::default()
            };
        }
        app.update();
        assert!(
            !app.world()
                .get::<PlayerJumpState>(player)
                .unwrap()
                .ladder_drop_through_hold_lock,
            "releasing down should clear the ladder drop lock"
        );

        {
            let mut input = app.world_mut().get_mut::<PlayerInputFrame>(player).unwrap();
            input.frame = ControlFrame {
                axis_y: 1.0,
                ..Default::default()
            };
        }
        app.update();
        assert_eq!(
            app.world()
                .get::<PlayerBodyModeState>(player)
                .unwrap()
                .body_mode,
            ae::BodyMode::Climbing,
            "once the player has released down, pressing it again should re-grab the ladder",
        );
    }

    #[test]
    fn flying_suppresses_ladder_auto_climb() {
        let (mut app, player) = build_body_mode_test_app();
        // A ladder column the player is standing in.
        place_player_on_test_ladder(&mut app, player, None);
        // Hold Up + enable flight.
        {
            let mut input = app.world_mut().get_mut::<PlayerInputFrame>(player).unwrap();
            input.frame = ControlFrame {
                axis_y: -1.0,
                ..Default::default()
            };
        }
        {
            let mut flight = app
                .world_mut()
                .get_mut::<crate::player::PlayerFlightState>(player)
                .unwrap();
            flight.fly_enabled = true;
        }
        app.update();
        assert_eq!(
            app.world()
                .get::<PlayerBodyModeState>(player)
                .unwrap()
                .body_mode,
            ae::BodyMode::Standing,
            "flying should suppress ladder auto-climb (Up means fly up, not grab)",
        );

        // Disable flight — the same Up press now engages the ladder.
        {
            let mut flight = app
                .world_mut()
                .get_mut::<crate::player::PlayerFlightState>(player)
                .unwrap();
            flight.fly_enabled = false;
        }
        app.update();
        assert_eq!(
            app.world()
                .get::<PlayerBodyModeState>(player)
                .unwrap()
                .body_mode,
            ae::BodyMode::Climbing,
            "with flight off, Up on a ladder climbs",
        );
    }
}
