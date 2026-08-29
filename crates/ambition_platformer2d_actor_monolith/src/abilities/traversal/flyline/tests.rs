//! ⛔⛔ THIS FILE PROVES THE EXECUTOR TIES THE KNOT — IT DOES NOT PROVE THE MOVE.
//! The lift, the swing and the release are the kernel's, and their guard is in
//! `movement::tests::wire`. What is checked here is the half a moveset test
//! cannot see: that the beat reaches a body at all, that it puts a REAL wire on
//! it rather than a defaulted one, and that nothing on this road asks for the
//! teleport's cue.

use super::*;

const HALF: ae::Vec2 = ae::Vec2::new(16.0, 32.0);

fn app_with_body(pos: ae::Vec2) -> (bevy::prelude::App, bevy::prelude::Entity) {
    let mut app = bevy::prelude::App::new();
    app.add_message::<ambition_vfx::vfx::VfxMessage>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<ActorActionMessage>();
    app.add_systems(bevy::prelude::Update, apply_authored_flylines);
    let body = app
        .world_mut()
        .spawn((
            ae::BodyKinematics {
                pos,
                size: HALF * 2.0,
                vel: ae::Vec2::new(120.0, 340.0),
                ..Default::default()
            },
            ae::movement::MotionModel::default(),
            ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame::default(),
        ))
        .id();
    (app, body)
}

fn params() -> FlylineParams {
    FlylineParams {
        rope_length: 720.0,
        rise: 420.0,
        lift_s: 0.55,
        max_swing_deg: 18.0,
        swing_accel: 3.4,
        release_rise: 90.0,
        vfx: "four_point_glint".to_string(),
        sfx: "world.door.heavy_open".to_string(),
    }
}

fn fire(app: &mut bevy::prelude::App, body: bevy::prelude::Entity, params: FlylineParams) {
    app.world_mut().write_message(ActorActionMessage {
        actor: body,
        request: ActionRequest::Special {
            spec: SpecialActionSpec::Special(FLYLINE.to_string()),
            params: ambition_entity_catalog::ParamValue::from_typed(&params)
                .expect("flyline params serialize"),
        },
    });
    app.update();
}

fn wire(app: &bevy::prelude::App, body: bevy::prelude::Entity) -> Option<ae::WireState> {
    match app
        .world()
        .entity(body)
        .get::<ae::movement::MotionModel>()
        .expect("motion model")
    {
        ae::movement::MotionModel::AxisSwept(axis) => axis.state.wire,
        other => panic!("body is not axis-swept: {other:?}"),
    }
}

/// The knot is tied, and it is tied ABOVE her.
///
/// ⛔ THE ANCHOR IS THE ONE FACT THIS SYSTEM DERIVES rather than copies, so it
/// is the one worth asserting: a wire that reaches down from the sky hangs from
/// a point `rope_length` along the body's own UP, and she starts at rest
/// directly beneath it. An anchor placed at her own position — the shape a
/// careless `pos + rope_length` would produce under `+y`-is-down — would give a
/// zero-length pendulum whose every direction is the same point.
#[test]
fn the_wire_comes_down_from_the_sky_and_she_starts_beneath_it() {
    let at = ae::Vec2::new(40.0, 200.0);
    let (mut app, body) = app_with_body(at);
    fire(&mut app, body, params());
    let wire = wire(&app, body).expect("she is on the wire");
    assert_eq!(
        wire.anchor,
        ae::Vec2::new(40.0, 200.0 - 720.0),
        "the anchor hangs 720px along the frame's UP from her, not below her"
    );
    assert_eq!(wire.angle, 0.0, "she starts at rest under the anchor");
    assert_eq!(wire.ang_vel, 0.0);
    assert_eq!(wire.length, 720.0);
}

/// ⛔⛔ THE WINCH SPEED IS DERIVED, AND A DEFAULTED ONE IS THE FAILURE THIS
/// CATCHES. `rise / lift_s` is the only rate that makes both authored numbers
/// true; a zero here is a wire that hangs her in the air for the whole beat and
/// puts her down where she started, which reads in play as "the up-B does
/// nothing" and reads in a spec test as a perfectly well-formed move.
#[test]
fn the_winch_reels_at_the_rate_the_authored_rise_and_time_imply() {
    let (mut app, body) = app_with_body(ae::Vec2::new(0.0, 0.0));
    fire(&mut app, body, params());
    let wire = wire(&app, body).expect("she is on the wire");
    assert!(
        (wire.winch_speed - 420.0 / 0.55).abs() < 0.01,
        "winch reels at rise/lift_s, got {}",
        wire.winch_speed
    );
    assert!((wire.lift_remaining_s - 0.55).abs() < 1e-6);
    assert!(
        (wire.max_angle - 18.0_f32.to_radians()).abs() < 1e-6,
        "degrees in the authoring, radians in the kernel"
    );
    assert!((wire.release_rise - 90.0).abs() < 1e-6);
}

/// ⛔⛔ NOT ONE `player.blink` ON THIS ROAD, AND THAT IS THE WHOLE ASK.
///
/// Jon, 2026-08-29: *"It is not a teleport and should not get the teleport
/// sound."* The cue this move used to make came from `apply_authored_teleports`,
/// not from any timeline — so the assertion that means anything is on the
/// EMITTED cue out of the executor that actually runs, which is this one.
#[test]
fn the_flyline_never_asks_for_the_teleport_cue() {
    let (mut app, body) = app_with_body(ae::Vec2::new(0.0, 0.0));
    fire(&mut app, body, params());
    let cues: Vec<ambition_sfx::SfxId> = app
        .world()
        .resource::<bevy::ecs::message::Messages<ambition_sfx::OwnedSfxMessage>>()
        .iter_current_update_messages()
        .filter_map(|owned| match &owned.request {
            ambition_sfx::SfxMessage::Play { id, .. } => Some(*id),
            _ => None,
        })
        .collect();
    assert!(
        !cues.is_empty(),
        "the catch is audible — a silent wire is its own bug"
    );
    assert!(
        !cues.contains(&ambition_sfx::ids::PLAYER_BLINK),
        "the flyline asked for the teleport cue: {cues:?}"
    );
}

/// ⚠ A BEAT AIMED AT A BODY THAT CANNOT TAKE A WIRE MAKES NO SHOW OF IT.
/// Advertising a lift that is not going to happen is worse than silence: the
/// opponent reads a recovery and the fighter falls.
#[test]
fn a_body_on_another_policy_is_refused_quietly() {
    let (mut app, body) = app_with_body(ae::Vec2::new(0.0, 0.0));
    app.world_mut()
        .entity_mut(body)
        .insert(ae::movement::MotionModel::adhesive_crawler(
            ae::CrawlerParams::default(),
        ));
    fire(&mut app, body, params());
    let effects = app
        .world()
        .resource::<bevy::ecs::message::Messages<ambition_vfx::vfx::VfxMessage>>()
        .iter_current_update_messages()
        .count();
    assert_eq!(effects, 0, "a refused wire draws nothing");
}
