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

/// ⛔⛔ A FIGHTER, NOT A POSITION. This spawned `BodyKinematics` + `MatchSeat`
/// and nothing else, so **nothing in this file could tell an ally from a corpse**
/// — which is exactly how the dash came to steer at every body in the world.
/// A real match gives every seated fighter `ActorFaction::Player` and its own
/// `MatchTeam` (the smash rules keep global friendly fire OFF and say why:
/// *"teams already decide who may hit whom"*), so a fixture without them was
/// modelling a body that cannot legally fight anybody.
fn body(app: &mut App, seat: usize, at: ae::Vec2) -> Entity {
    app.world_mut()
        .spawn((
            ae::BodyKinematics {
                pos: at,
                facing: 1.0,
                ..Default::default()
            },
            MatchSeat(seat),
            ambition_platformer2d::combat::components::ActorFaction::Player,
            ambition_platformer2d::combat::targeting::MatchTeam::new(format!("seat{seat}")),
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

/// ⛔⛔ A KO'd FIGHTER IS NOT A TARGET, AND THE DASH USED TO STEER AT ONE.
///
/// `assisted_fire_direction` is deliberately GEOMETRIC and assumes its caller
/// supplied foes; this handed it every body in the world filtered only by "not
/// me". ⇒ A fighter who has just lost a stock carries `OutOfPlay` and — because
/// a respawn restores it — FULL HEALTH, so nothing about their numbers says they
/// are gone. The dash bent at a body the player could not even hit.
///
/// ⭐ The fix reuses `body_is_untouchable`, the combat domain's own participation
/// gate, rather than approximating it: its doc is explicit that out-of-play
/// belongs there precisely so TARGET SELECTION sees it, and that folding it into
/// invulnerability would leave "a hunter going on chasing a body it merely could
/// not damage."
#[test]
fn a_ko_d_fighter_does_not_attract_the_dash() {
    let mut app = app();
    let hunter = body(&mut app, 1, ae::Vec2::ZERO);
    let gone = body(&mut app, 0, ae::Vec2::new(160.0, -160.0));
    app.world_mut()
        .entity_mut(gone)
        .insert(ambition_platformer2d::combat::death_rules::OutOfPlay);
    dash(&mut app, hunter);
    let vel = velocity(&app, hunter);
    assert!(
        vel.y > -100.0,
        "the dash bent upward at a fighter who is OUT OF PLAY ({vel:?}) — they \
         are unhittable, so the move spends itself flying at nobody"
    );
}

/// ⛔⛔ AND NEITHER DOES A TEAMMATE.
///
/// The smash rules keep global friendly fire OFF and say why in place: *"teams
/// already decide who may hit whom. Switching global friendly fire on to let two
/// humans trade would make TEAMMATES hittable too."* ⇒ So a dash that steered by
/// "not me" would, in team versus, bend a fighter at the ally standing beside
/// them — a move that actively fights its owner.
///
/// ⭐ THE CONTROL IS THE TEST ABOVE IT, `the_dash_bends_toward_a_foe_inside_the_cone`:
/// same geometry, same cone, one field different (a shared team), opposite
/// outcome. Asserting only that a teammate is ignored would pass for a dash that
/// homes on nothing at all.
#[test]
fn a_teammate_does_not_attract_the_dash() {
    let mut app = app();
    let hunter = body(&mut app, 1, ae::Vec2::ZERO);
    let ally = body(&mut app, 0, ae::Vec2::new(160.0, -160.0));
    let shared = ambition_platformer2d::combat::targeting::MatchTeam::new("blue");
    app.world_mut().entity_mut(hunter).insert(shared.clone());
    app.world_mut().entity_mut(ally).insert(shared);
    dash(&mut app, hunter);
    let vel = velocity(&app, hunter);
    assert!(
        vel.y > -100.0,
        "the dash bent upward at a TEAMMATE ({vel:?}) — friendly fire is off, so \
         it is steering at a body it cannot hit"
    );
}
