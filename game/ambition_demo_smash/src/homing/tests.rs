//! ⛔⛔ THE CONE IS THE MOVE. "It steers toward a foe" would pass against a dash
//! that finds anybody anywhere — which is a tracking move nobody has to aim, and
//! the opposite of a read. Every test here pairs a hit with a miss.

use super::*;
use ambition_platformer2d::actor::MatchSeat;

fn app() -> App {
    let mut app = App::new();
    app.init_resource::<ambition_platformer2d::time::WorldTime>();
    app.add_message::<ActorActionMessage>();
    let mut time = app
        .world_mut()
        .resource_mut::<ambition_platformer2d::time::WorldTime>();
    time.scaled_dt = 1.0 / 60.0;
    time.raw_dt = 1.0 / 60.0;
    app.add_systems(
        Update,
        (begin_authored_homing_dashes, carry_homing_dashes).chain(),
    );
    app
}

fn body(app: &mut App, seat: usize, at: ae::Vec2) -> Entity {
    app.world_mut()
        .spawn((
            ae::BodyKinematics {
                pos: at,
                facing: 1.0,
                ..Default::default()
            },
            MatchSeat(seat),
        ))
        .id()
}

fn params() -> HomingDashParams {
    HomingDashParams {
        speed: 900.0,
        duration_s: 0.28,
        cone_degrees: 60.0,
        max_range: 320.0,
    }
}

fn dash(app: &mut App, actor: Entity) {
    let request = ActionRequest::Special {
        spec: SpecialActionSpec::Special(HOMING_DASH.to_string()),
        params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(&params())
            .expect("homing params serialize"),
    };
    app.world_mut()
        .write_message(ActorActionMessage { actor, request });
    app.update();
}

fn velocity(app: &App, who: Entity) -> ae::Vec2 {
    app.world().get::<ae::BodyKinematics>(who).unwrap().vel
}

/// ⭐ A foe inside the cone bends the dash toward them.
#[test]
fn the_dash_bends_toward_a_foe_inside_the_cone() {
    let mut app = app();
    let hunter = body(&mut app, 1, ae::Vec2::ZERO);
    // Ahead and well above: only a bent heading reaches him.
    let _prey = body(&mut app, 0, ae::Vec2::new(160.0, -160.0));
    dash(&mut app, hunter);
    let vel = velocity(&app, hunter);
    assert!(vel.x > 0.0, "it went backwards: {vel:?}");
    assert!(
        vel.y < -100.0,
        "the dash did not bend upward toward him: {vel:?}"
    );
}

/// ⛔ AND A FOE OUTSIDE THE CONE DOES NOT. Without this the guard passes against
/// a dash that homes on anybody, which is a different move.
#[test]
fn a_foe_behind_the_fighter_does_not_attract_the_dash() {
    let mut app = app();
    let hunter = body(&mut app, 1, ae::Vec2::ZERO);
    let _behind = body(&mut app, 0, ae::Vec2::new(-160.0, -160.0));
    dash(&mut app, hunter);
    let vel = velocity(&app, hunter);
    assert!(
        vel.x > 0.0,
        "a foe BEHIND him turned the dash around, so the cone is not enforced: {vel:?}"
    );
    assert!(
        vel.y.abs() < 1.0,
        "he bent toward somebody outside the cone: {vel:?}"
    );
}

/// ⛔ AND ONE BEYOND THE RANGE DOES NOT EITHER — the other half of "the way I was
/// pointing" is HOW FAR.
#[test]
fn a_foe_beyond_the_range_does_not_attract_the_dash() {
    let mut app = app();
    let hunter = body(&mut app, 1, ae::Vec2::ZERO);
    let _far = body(&mut app, 0, ae::Vec2::new(900.0, -900.0));
    dash(&mut app, hunter);
    let vel = velocity(&app, hunter);
    assert!(
        vel.y.abs() < 1.0,
        "a foe past `max_range` bent the dash: {vel:?}"
    );
}

/// ⛔⛔ IT ENDS. A dash whose clock never ran out would carry the fighter through
/// his own recovery and off the stage, and there would be nothing to punish.
#[test]
fn the_dash_stops_when_its_clock_runs_out() {
    let mut app = app();
    let hunter = body(&mut app, 1, ae::Vec2::ZERO);
    let _prey = body(&mut app, 0, ae::Vec2::new(160.0, -160.0));
    dash(&mut app, hunter);
    for _ in 0..(0.28 * 60.0) as usize + 4 {
        app.update();
    }
    assert!(
        app.world().get::<HomingDash>(hunter).is_none(),
        "the dash outlived its authored duration"
    );
}

/// ⭐ THE COMMITTED DIRECTION IS REMEMBERED, NOT RE-READ. Turning the fighter
/// mid-dash must not sweep the cone across the stage — that would turn a read
/// into a search.
#[test]
fn turning_mid_dash_does_not_sweep_the_cone() {
    let mut app = app();
    let hunter = body(&mut app, 1, ae::Vec2::ZERO);
    let _behind = body(&mut app, 0, ae::Vec2::new(-160.0, -160.0));
    dash(&mut app, hunter);
    // He turns to face the foe behind him. The cone must not follow.
    let mut kin = app.world_mut().get_mut::<ae::BodyKinematics>(hunter).unwrap();
    kin.facing = -1.0;
    app.update();
    let vel = velocity(&app, hunter);
    assert!(
        vel.x > 0.0,
        "turning mid-dash re-aimed it, so the commanded direction is being \
         re-read rather than remembered: {vel:?}"
    );
}
