//! Pins for the `update_body_mode` driver: morph-ball entry/exit,
//! ladder grab/exit, the down-jump fall-off + re-grab lock, and
//! flight suppressing ladder auto-climb.

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

#[test]
fn local_up_press_from_morph_ball_transitions_to_standing_under_sideways_screen_directed() {
    let (mut app, player) = build_body_mode_test_app();
    app.insert_resource(crate::physics::GravityField {
        dir: Vec2::new(1.0, 0.0),
    });
    let mut settings = crate::persistence::settings::UserSettings::default();
    settings.gameplay.input_frame_mode = ae::InputFrameMode::Screen;
    app.insert_resource(settings);
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
        // Gravity points screen-right, so in screen-directed mode local up maps
        // to raw/screen-left. This should unmorph just like raw Up does under
        // normal gravity.
        input.frame = ControlFrame {
            axis_x: -1.0,
            left_pressed: true,
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
        "local-up press should transition MorphBall -> Standing under sideways gravity",
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
