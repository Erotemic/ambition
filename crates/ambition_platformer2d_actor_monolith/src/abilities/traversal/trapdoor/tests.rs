//! ⛔⛔ THE TWO BEATS ARE TESTED SEPARATELY AND AS A PAIR. A submerge that works
//! and a surface that does not is a fighter who never comes back, and it is the
//! failure a test of only the first beat agrees with.

use super::*;

fn solid(name: &str, center: ae::Vec2, half: ae::Vec2) -> ae::Block {
    ae::Block::solid(name, center - half, half * 2.0)
}

/// A platform whose top face is at y = 0, spanning x in [-400, 400].
fn stage() -> ae::World {
    ae::World::new(
        "trapdoor_test",
        ae::Vec2::new(2000.0, 2000.0),
        ae::Vec2::ZERO,
        vec![solid(
            "stage",
            ae::Vec2::new(0.0, 50.0),
            ae::Vec2::new(400.0, 50.0),
        )],
    )
}

const HALF: ae::Vec2 = ae::Vec2::new(16.0, 32.0);

fn app_with_body(pos: ae::Vec2) -> (bevy::prelude::App, bevy::prelude::Entity) {
    let mut app = bevy::prelude::App::new();
    ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
        app.world_mut(),
        ae::RoomGeometry(stage()),
    );
    app.add_message::<ambition_vfx::vfx::VfxMessage>();
    app.add_message::<ambition_sfx::OwnedSfxMessage>();
    app.add_message::<ActorActionMessage>();
    app.add_systems(bevy::prelude::Update, apply_authored_trapdoors);
    app.init_resource::<ambition_platformer2d_shared_tangle::class_b::ClassBRemapLog>();
    let body = app
        .world_mut()
        .spawn((
            ae::BodyAbilities::default(),
            ae::BodyKinematics {
                pos,
                size: HALF * 2.0,
                vel: ae::Vec2::new(120.0, 340.0),
                ..Default::default()
            },
            ae::BodyBaseSize::default(),
            ae::BodyGroundState::default(),
            ae::BodyWallState::default(),
            ae::BodyJumpState::default(),
            ae::BodyDashState::default(),
            ae::BodyFlightState::default(),
            ae::BodyBlinkState::default(),
            ae::BodyLedgeState::default(),
        ))
        .insert((
            ae::BodyDodgeState::default(),
            ae::BodyShieldState::default(),
            ae::BodyModeState::default(),
            ae::BodyEnvironmentContact::default(),
            ae::BodyMana::default(),
            ae::BodyOffense::default(),
            ae::BodyActionBuffer::default(),
            ae::BodyLifetime::default(),
            ae::BodyComboTrace::default(),
        ))
        .insert((
            ae::movement::MotionModel::default(),
            ambition_platformer2d_shared_tangle::frame_env::ResolvedMotionFrame::default(),
        ))
        .id();
    (app, body)
}

fn fire(app: &mut bevy::prelude::App, body: bevy::prelude::Entity, params: TrapdoorParams) {
    app.world_mut().write_message(ActorActionMessage {
        actor: body,
        request: ActionRequest::Special {
            spec: SpecialActionSpec::Special(TRAPDOOR.to_string()),
            params: ambition_entity_catalog::ParamValue::from_typed(&params)
                .expect("trapdoor params serialize"),
        },
    });
    app.update();
}

fn down(surface_reach: f32) -> TrapdoorParams {
    TrapdoorParams {
        leap_speed: 0.0,
        submerge: true,
        surface_reach,
        vfx: "smoke_burst".to_string(),
        sfx: "world.door.heavy_open".to_string(),
    }
}

fn up(surface_reach: f32) -> TrapdoorParams {
    TrapdoorParams {
        leap_speed: 0.0,
        submerge: false,
        ..down(surface_reach)
    }
}

fn mode(app: &bevy::prelude::App, body: bevy::prelude::Entity) -> ae::player_state::BodyMode {
    app.world()
        .entity(body)
        .get::<ae::BodyModeState>()
        .expect("body mode")
        .body_mode
}

fn kin(app: &bevy::prelude::App, body: bevy::prelude::Entity) -> ae::BodyKinematics {
    *app.world()
        .entity(body)
        .get::<ae::BodyKinematics>()
        .expect("kinematics")
}

#[test]
fn the_first_beat_puts_her_under_the_stage() {
    let (mut app, body) = app_with_body(ae::Vec2::new(0.0, -32.0));
    fire(&mut app, body, down(120.0));
    assert_eq!(mode(&app, body), ae::player_state::BodyMode::Submerged);
}

/// ⛔⛔ AND IT ENDS THE FALL SHE ARRIVED WITH. Collision is off the moment the
/// mode is set, so a velocity carried into the tick between this write and the
/// submerged integrator's first step would take her down through the world.
#[test]
fn going_under_ends_the_motion_she_arrived_with() {
    let (mut app, body) = app_with_body(ae::Vec2::new(0.0, -32.0));
    assert_ne!(
        kin(&app, body).vel,
        ae::Vec2::ZERO,
        "premise: she was moving"
    );
    fire(&mut app, body, down(120.0));
    assert_eq!(kin(&app, body).vel, ae::Vec2::ZERO);
}

/// ⛔⛔ THE SECOND BEAT BRINGS HER BACK, and it is the half whose absence is a
/// fighter lost for the rest of the match.
#[test]
fn the_second_beat_stands_her_back_on_the_stage() {
    // Under the boards: the stage's top face is y = 0, she is 60px below it.
    let (mut app, body) = app_with_body(ae::Vec2::new(150.0, 60.0));
    fire(&mut app, body, down(120.0));
    fire(&mut app, body, up(120.0));
    assert_eq!(mode(&app, body), ae::player_state::BodyMode::Standing);
    let after = kin(&app, body);
    assert!(
        (after.pos.y - (0.0 - HALF.y)).abs() < 1e-3,
        "she must surface standing on the floor's top face, and she is at {}",
        after.pos.y
    );
    assert!(
        (after.pos.x - 150.0).abs() < 1e-3,
        "she surfaces where she STEERED to, not where she went under"
    );
}

/// ⭐ THE POINT OF THE MODE. She travels under the stage, so the exit is
/// wherever the player took her — a surfacing that returned her to the entry
/// would make the whole mode decoration.
#[test]
fn she_surfaces_where_she_travelled_to_and_not_where_she_entered() {
    let (mut app, body) = app_with_body(ae::Vec2::new(-200.0, 60.0));
    fire(&mut app, body, down(120.0));
    // The player steered: the submerged integrator would have done this, and
    // moving it directly keeps this arm about SURFACING rather than about
    // integration.
    app.world_mut()
        .entity_mut(body)
        .get_mut::<ae::BodyKinematics>()
        .expect("kinematics")
        .pos = ae::Vec2::new(240.0, 60.0);
    fire(&mut app, body, up(120.0));
    assert!(
        (kin(&app, body).pos.x - 240.0).abs() < 1e-3,
        "she came up at {} rather than where she steered to",
        kin(&app, body).pos.x
    );
}

/// ⛔⛔ THE SURFACING DECLARES ITSELF, and it used to move her in silence.
///
/// It picks a position with `ledge_assisted_arrival` and writes it with
/// `transit_body` — the same shape as blink, dive, mark-recall and the authored
/// teleport, every one of which records a Class-B remap. Without the record an
/// instrument reading this frame sees a body somewhere else with nothing to say
/// why, and same-frame contention between two Class-B writers cannot see this
/// one at all.
///
/// ⛔ THE PAIR IS THE POINT. Going UNDER writes no position — it sets a mode and
/// zeroes velocity — so it must NOT record one. A rule that logged both would
/// make the log a record of the move rather than of the displacement.
#[test]
fn coming_up_declares_the_remap_and_going_under_does_not() {
    use ambition_platformer2d_shared_tangle::class_b::{ClassBRemap, ClassBRemapLog};

    let (mut app, body) = app_with_body(ae::Vec2::new(-200.0, 60.0));
    fire(&mut app, body, down(120.0));
    assert!(
        app.world()
            .resource::<ClassBRemapLog>()
            .entries()
            .is_empty(),
        "going under the stage moved no body, so it must claim no remap"
    );

    fire(&mut app, body, up(120.0));
    let entries: Vec<_> = app
        .world()
        .resource::<ClassBRemapLog>()
        .entries()
        .iter()
        .copied()
        .collect();
    assert_eq!(entries.len(), 1, "surfacing recorded {entries:?}");
    assert_eq!(entries[0].body, body);
    assert_eq!(entries[0].kind, ClassBRemap::ScriptedTeleport);
}

/// ⛔ AND A SURFACE OUT OF REACH LEAVES HER WHERE SHE IS RATHER THAN TELEPORTING
/// HER TO ONE. `ledge_assisted_arrival` refuses past its radius, which for this
/// move means a fighter who wandered past the end of the stage comes up in open
/// air and falls — the honest outcome, and the one the mode's own doc promises.
#[test]
fn surfacing_far_below_any_floor_does_not_snap_her_to_one() {
    let (mut app, body) = app_with_body(ae::Vec2::new(0.0, 800.0));
    fire(&mut app, body, down(120.0));
    fire(&mut app, body, up(120.0));
    assert_eq!(mode(&app, body), ae::player_state::BodyMode::Standing);
    assert!(
        (kin(&app, body).pos.y - 800.0).abs() < 1e-3,
        "an unreachable floor must not pull her up to it"
    );
}

/// ⛔⛔ STAGE FIVE OF FIVE, AND IT SHIPPED DELETED. The Performer's trap authored
/// `LEAP_OUT_SPEED = 430.0` as a `MoveEventKind::Impulse` on the SAME instant as
/// this beat, reasoning that landing them together meant *"the placement and the
/// launch cannot disagree about where she left from."* They never disagreed: an
/// impulse is applied inline in `advance_move_playback` and this beat is a
/// message handled by a LATER system, whose `TransitVelocity::Zero` overwrote it
/// every single time. She surfaced standing exactly where she stopped, for as
/// long as the constant existed, and no reading of either file alone showed it.
///
/// ⇒ the exit velocity belongs to the ONE system that places her. This is the
/// guard that fails if it is ever taken back off.
#[test]
fn surfacing_with_a_leap_speed_launches_her_out_of_the_boards() {
    let (mut app, body) = app_with_body(ae::Vec2::new(300.0, 500.0));
    fire(&mut app, body, down(0.0));
    let mut leap = up(140.0);
    leap.leap_speed = 430.0;
    fire(&mut app, body, leap);
    let vel = kin(&app, body).vel;
    assert!(
        vel.y < -1.0,
        "she surfaced at {vel:?}; a leap is AGAINST gravity and +y is down, so \
         a non-negative y means the placement ate the launch again"
    );
    assert!(
        (vel.y + 430.0).abs() < 1e-3,
        "she left the boards at {vel:?}, wanted the authored 430 exactly"
    );
}

/// ⛔ AND `0.0` STILL SURFACES HER STANDING. The Author's trapdoor authors no
/// leap, so a default that launched everybody would change a move nobody asked
/// to change.
#[test]
fn surfacing_without_a_leap_speed_stands_her_still() {
    let (mut app, body) = app_with_body(ae::Vec2::new(300.0, 500.0));
    fire(&mut app, body, down(0.0));
    fire(&mut app, body, up(140.0));
    assert_eq!(kin(&app, body).vel, ae::Vec2::ZERO);
}
