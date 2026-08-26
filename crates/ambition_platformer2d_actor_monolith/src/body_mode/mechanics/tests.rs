//! Pins for the `update_body_mode` driver: morph-ball entry/exit,
//! ladder grab/exit, the down-jump fall-off + re-grab lock, and
//! flight suppressing ladder auto-climb.

use super::*;
use crate::actor::BodyKinematics;
use crate::actor::{
    BodyBaseSize, BodyEnvironmentContact, BodyGroundState, BodyJumpState, BodyModeState,
};
use crate::actor::{PlayerEntity, PrimaryPlayer};
use crate::body_mode::BodyModeCapabilities;
use ambition_characters::control::SlotInteractionState;
use ambition_characters::actor::control::ActorControlFrame;
use ambition_characters::control::{ActorControl};
use ambition_characters::control::{DrivingParticipant, PlayerSlot};
use ambition_platformer2d_core::world::{ClimbableKind, ClimbableRegion, ClimbableSpec, World};
use ambition_platformer2d_core::Vec2;
use bevy::prelude::{App, Entity, Update};

/// Set the controlled body's `ActorControl` — the body-generic intent the driver
/// consumes (already-resolved locomotion + jump/dash edges), replacing the old
/// per-body raw-input component.
fn set_control(app: &mut App, body: Entity, f: impl FnOnce(&mut ActorControlFrame)) {
    let mut control = app.world_mut().get_mut::<ActorControl>(body).unwrap();
    control.0 = ActorControlFrame::neutral();
    f(&mut control.0);
}

/// Prime the primary controller slot's double-tap-down morph gesture.
fn arm_double_tap_down(app: &mut App) {
    app.world_mut()
        .resource_mut::<SlotInteractionState>()
        .primary_mut()
        .double_tap_down_pending = true;
}

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
        chains: Vec::new(),
        edges: Default::default(),
    }
}

fn build_body_mode_test_app() -> (App, Entity) {
    let mut app = App::new();
    ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
        app.world_mut(),
        ambition_platformer2d_core::RoomGeometry(open_world()),
    );
    app.init_resource::<SlotInteractionState>();
    let world_spawn = ambition_platformer2d_shared_tangle::lifecycle::session_world_component::<
        ambition_platformer2d_core::RoomGeometry,
    >(app.world())
    .expect("session room geometry")
    .0
    .spawn;
    app.add_systems(Update, super::update_body_mode);
    let player = app
        .world_mut()
        .spawn((
            PlayerEntity,
            PrimaryPlayer,
            // Controlled by the primary slot, with the full body-mode kit — the
            // driver keys on `DrivingParticipant` + `BodyModeCapabilities`, not `PlayerEntity`.
            DrivingParticipant(PlayerSlot::PRIMARY),
            ActorControl::default(),
            BodyModeCapabilities::full(),
            BodyKinematics {
                pos: world_spawn,
                size: Vec2::new(30.0, 48.0),
                facing: 1.0,
                ..Default::default()
            },
            crate::features::MotionModel::default(),
            BodyBaseSize {
                base_size: Vec2::new(30.0, 48.0),
            },
            BodyGroundState {
                head_contact: false,
                on_ground: true,
                ..Default::default()
            },
            (
                ambition_platformer2d_core::BodyMotionFacts::default(),
                BodyEnvironmentContact::default(),
                BodyModeState::default(),
                BodyJumpState::default(),
                crate::actor::BodyFlightState::default(),
                ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame::default(),
            ),
        ))
        .id();
    (app, player)
}

/// Spawn a body-mode-capable body at `pos`. `slot = Some` → it carries
/// `DrivingParticipant(slot)` (a driven body); `None` → no seat (a vacated /
/// inert body the driver must skip).
fn spawn_mode_body(app: &mut App, pos: Vec2, slot: Option<PlayerSlot>) -> Entity {
    let mut body = app.world_mut().spawn((
        BodyModeCapabilities::full(),
        ActorControl::default(),
        BodyKinematics {
            pos,
            size: Vec2::new(30.0, 48.0),
            facing: 1.0,
            ..Default::default()
        },
        crate::features::MotionModel::default(),
        BodyBaseSize {
            base_size: Vec2::new(30.0, 48.0),
        },
        BodyGroundState {
            head_contact: false,
            on_ground: true,
            ..Default::default()
        },
        (
            ambition_platformer2d_core::BodyMotionFacts::default(),
            BodyEnvironmentContact::default(),
            BodyModeState::default(),
            BodyJumpState::default(),
            crate::actor::BodyFlightState::default(),
            ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame::default(),
        ),
    ));
    if let Some(slot) = slot {
        body.insert(DrivingParticipant(slot));
    }
    body.id()
}

/// The headline controlled-body guarantee: while a controller drives a non-player
/// ACTOR body, its body-mode input curls THAT body — and the vacated home body,
/// which no longer holds the seat, is untouched.
#[test]
fn controlled_actor_body_mode_input_does_not_affect_home_body() {
    let mut app = App::new();
    ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
        app.world_mut(),
        ambition_platformer2d_core::RoomGeometry(open_world()),
    );
    app.init_resource::<SlotInteractionState>();
    app.add_systems(Update, super::update_body_mode);
    let spawn = ambition_platformer2d_shared_tangle::lifecycle::session_world_component::<
        ambition_platformer2d_core::RoomGeometry,
    >(app.world())
    .expect("session room geometry")
    .0
    .spawn;
    // Vacated home body: has the kit, but NO player brain (someone else is driving).
    let home = spawn_mode_body(&mut app, spawn, None);
    // The possessed actor the primary controller is driving.
    let actor = spawn_mode_body(
        &mut app,
        spawn + Vec2::new(120.0, 0.0),
        Some(PlayerSlot::PRIMARY),
    );
    // Body-mode input (double-tap-down morph) arrives on the primary slot.
    arm_double_tap_down(&mut app);
    app.update();
    assert_eq!(
        app.world().get::<BodyModeState>(actor).unwrap().body_mode,
        ae::BodyMode::MorphBall,
        "the CONTROLLED actor curls into a morph ball",
    );
    assert_eq!(
        app.world().get::<BodyModeState>(home).unwrap().body_mode,
        ae::BodyMode::Standing,
        "the vacated home body must NOT change mode from another body's control",
    );
}

/// Symmetric case: during normal play the home body IS the controlled body (it
/// holds the seat), so its body mode still changes through the new path.
#[test]
fn home_body_mode_still_works_when_home_is_controlled() {
    let (mut app, home) = build_body_mode_test_app();
    // The home body carries PlayerEntity + PrimaryPlayer + DrivingParticipant(PRIMARY).
    arm_double_tap_down(&mut app);
    app.update();
    assert_eq!(
        app.world().get::<BodyModeState>(home).unwrap().body_mode,
        ae::BodyMode::MorphBall,
        "the home body morphs when it is the controlled body",
    );
}

/// Body mode consumes the body's semantic `ActorControl`, which the player
/// brain derives from its slot. No per-body raw-input state participates.
#[test]
fn body_mode_follows_actor_control_without_per_body_input_state() {
    let mut app = App::new();
    ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
        app.world_mut(),
        ambition_platformer2d_core::RoomGeometry(open_world()),
    );
    app.init_resource::<SlotInteractionState>();
    app.add_systems(Update, super::update_body_mode);
    let spawn = ambition_platformer2d_shared_tangle::lifecycle::session_world_component::<
        ambition_platformer2d_core::RoomGeometry,
    >(app.world())
    .expect("session room geometry")
    .0
    .spawn;
    let body = spawn_mode_body(&mut app, spawn, Some(PlayerSlot::PRIMARY));
    set_control(&mut app, body, |c| {
        c.locomotion = ambition_platformer2d_core::LocalAxes::new(0.0, 1.0)
    });
    app.update();
    assert_eq!(
        app.world().get::<BodyModeState>(body).unwrap().body_mode,
        ae::BodyMode::Crouching,
        "body mode must follow the slot-derived ActorControl intent",
    );
}

/// A momentum ride is the same support fact as the AABB path's ground cluster.
/// Without this, holding local Down on a chain or block boundary leaves the body
/// Standing even though the movement kernel is physically attached to a surface.
#[test]
fn momentum_riding_support_allows_the_controlled_body_to_crouch() {
    let (mut app, body) = build_body_mode_test_app();
    app.world_mut()
        .get_mut::<BodyGroundState>(body)
        .unwrap()
        .on_ground = false;
    let mut momentum = crate::features::MomentumMotion::new(Default::default());
    momentum.state = ae::SurfaceMotion::Riding {
        on: ae::SurfaceRef::Block(0),
        s: 10.0,
        v_t: 0.0,
    };
    app.world_mut()
        .entity_mut(body)
        .insert(crate::features::MotionModel::SurfaceMomentum(momentum));
    set_control(&mut app, body, |control| {
        control.locomotion = ambition_platformer2d_core::LocalAxes::new(0.0, 1.0)
    });

    app.update();

    assert_eq!(
        app.world().get::<BodyModeState>(body).unwrap().body_mode,
        ae::BodyMode::Crouching,
        "momentum ride support must drive the same crouch policy as AABB ground support",
    );
}

fn place_player_on_test_ladder(app: &mut App, player: Entity, vel: Option<Vec2>) {
    ambition_platformer2d_shared_tangle::lifecycle::session_world_component_mut::<
        ambition_platformer2d_core::RoomGeometry,
    >(app.world_mut())
    .expect("session room geometry")
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
    let contact = ambition_platformer2d_shared_tangle::lifecycle::session_world_component::<
        ambition_platformer2d_core::RoomGeometry,
    >(app.world())
    .expect("session room geometry")
    .0
    .climbable_at(app.world().get::<BodyKinematics>(player).unwrap().aabb());
    app.world_mut()
        .get_mut::<BodyEnvironmentContact>(player)
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
    // Double-tap-down morph gesture arrives on the controller's slot.
    arm_double_tap_down(&mut app);
    app.update();
    let mode = app.world().get::<BodyModeState>(player).unwrap().body_mode;
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
        let mut body_mode = app.world_mut().get_mut::<BodyModeState>(player).unwrap();
        body_mode.body_mode = ae::BodyMode::MorphBall;
    }
    {
        let mut kin = app.world_mut().get_mut::<BodyKinematics>(player).unwrap();
        kin.size = Vec2::new(14.0, 14.0);
    }
    set_control(&mut app, player, |c| c.jump_pressed = true);
    app.update();
    let mode = app.world().get::<BodyModeState>(player).unwrap().body_mode;
    assert_eq!(
        mode,
        ae::BodyMode::Standing,
        "Jump pressed in MorphBall must transition to Standing when headroom allows",
    );
}

/// Holding local-up (toward the head) also unmorphs, independent of jump. The
/// brain resolves raw device axes into `locomotion` (so gravity/input-mode
/// relativity is the brain's job now); the driver just reads local-up intent.
#[test]
fn local_up_intent_from_morph_ball_transitions_to_standing() {
    let (mut app, player) = build_body_mode_test_app();
    {
        let mut body_mode = app.world_mut().get_mut::<BodyModeState>(player).unwrap();
        body_mode.body_mode = ae::BodyMode::MorphBall;
    }
    {
        let mut kin = app.world_mut().get_mut::<BodyKinematics>(player).unwrap();
        kin.size = Vec2::new(14.0, 14.0);
    }
    // Local-up (toward the head) = negative local-down axis.
    set_control(&mut app, player, |c| {
        c.locomotion = ambition_platformer2d_core::LocalAxes::new(0.0, -1.0)
    });
    app.update();
    let mode = app.world().get::<BodyModeState>(player).unwrap().body_mode;
    assert_eq!(
        mode,
        ae::BodyMode::Standing,
        "local-up intent should transition MorphBall -> Standing",
    );
}

/// Climbing should not trap the player: the burst press is an explicit push
/// off just like jump, so one `update` call should flip the body
/// mode back to Standing and let the press consume cleanly in the
/// player tick.
#[test]
fn dash_press_from_climbing_transitions_to_standing() {
    let (mut app, player) = build_body_mode_test_app();
    place_player_on_test_ladder(&mut app, player, None);
    {
        let mut body_mode = app.world_mut().get_mut::<BodyModeState>(player).unwrap();
        body_mode.body_mode = ae::BodyMode::Climbing;
    }
    set_control(&mut app, player, |c| c.burst_pressed = true);
    app.update();
    let mode = app.world().get::<BodyModeState>(player).unwrap().body_mode;
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
        let mut body_mode = app.world_mut().get_mut::<BodyModeState>(player).unwrap();
        body_mode.body_mode = ae::BodyMode::Climbing;
    }
    set_control(&mut app, player, |c| {
        c.jump_pressed = true;
        c.locomotion = ambition_platformer2d_core::LocalAxes::new(0.0, -1.0); // up
    });
    app.update();
    let mode = app.world().get::<BodyModeState>(player).unwrap().body_mode;
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
        let mut body_mode = app.world_mut().get_mut::<BodyModeState>(player).unwrap();
        body_mode.body_mode = ae::BodyMode::Climbing;
    }
    set_control(&mut app, player, |c| {
        c.jump_pressed = true;
        c.locomotion = ambition_platformer2d_core::LocalAxes::new(0.0, 1.0); // down
    });
    app.update();
    let mode = app.world().get::<BodyModeState>(player).unwrap().body_mode;
    let jump_state = app.world().get::<BodyJumpState>(player).unwrap();
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
        let mut body_mode = app.world_mut().get_mut::<BodyModeState>(player).unwrap();
        body_mode.body_mode = ae::BodyMode::Standing;
    }
    {
        let mut ground = app.world_mut().get_mut::<BodyGroundState>(player).unwrap();
        ground.on_ground = false;
    }
    {
        let mut jump_state = app.world_mut().get_mut::<BodyJumpState>(player).unwrap();
        jump_state.ladder_drop_through_timer = 0.0;
        jump_state.ladder_drop_through_hold_lock = true;
    }
    set_control(&mut app, player, |c| {
        c.locomotion = ambition_platformer2d_core::LocalAxes::new(0.0, 1.0)
    }); // down
    app.update();
    assert_eq!(
        app.world().get::<BodyModeState>(player).unwrap().body_mode,
        ae::BodyMode::Standing,
        "holding down should not re-grab the ladder while the release lock is active",
    );

    set_control(&mut app, player, |c| {
        c.locomotion = ambition_platformer2d_core::LocalAxes::ZERO
    }); // release
    app.update();
    assert!(
        !app.world()
            .get::<BodyJumpState>(player)
            .unwrap()
            .ladder_drop_through_hold_lock,
        "releasing down should clear the ladder drop lock"
    );

    set_control(&mut app, player, |c| {
        c.locomotion = ambition_platformer2d_core::LocalAxes::new(0.0, 1.0)
    }); // down again
    app.update();
    assert_eq!(
        app.world().get::<BodyModeState>(player).unwrap().body_mode,
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
    set_control(&mut app, player, |c| {
        c.locomotion = ambition_platformer2d_core::LocalAxes::new(0.0, -1.0)
    });
    {
        let mut flight = app
            .world_mut()
            .get_mut::<crate::actor::BodyFlightState>(player)
            .unwrap();
        flight.fly_enabled = true;
    }
    app.update();
    assert_eq!(
        app.world().get::<BodyModeState>(player).unwrap().body_mode,
        ae::BodyMode::Standing,
        "flying should suppress ladder auto-climb (Up means fly up, not grab)",
    );

    // Disable flight — the same Up press now engages the ladder.
    {
        let mut flight = app
            .world_mut()
            .get_mut::<crate::actor::BodyFlightState>(player)
            .unwrap();
        flight.fly_enabled = false;
    }
    app.update();
    assert_eq!(
        app.world().get::<BodyModeState>(player).unwrap().body_mode,
        ae::BodyMode::Climbing,
        "with flight off, Up on a ladder climbs",
    );
}
