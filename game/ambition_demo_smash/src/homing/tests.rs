//! The cone is the move: a dash that finds anybody anywhere is a tracking move
//! nobody has to aim. Every test here pairs a hit with a miss.

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

/// A fighter, not only a position: `ActorFaction::Player` and its own
/// `MatchTeam`, as in a real match (friendly fire is off; teams decide who may
/// hit whom). Without them the fixture cannot tell an ally from a corpse.
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
        .write_message(ActorActionMessage { actor, request, move_instance: None });
    app.update();
}

fn velocity(app: &App, who: Entity) -> ae::Vec2 {
    app.world().get::<ae::BodyKinematics>(who).unwrap().vel
}

/// A foe inside the cone bends the dash toward them.
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

/// A foe outside the cone does not attract it. Without this, a dash that homes
/// on anybody would pass.
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

/// A foe beyond the range does not either: "the way I was pointing" also means
/// how far.
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

/// It ends: a dash that never ran out would carry the fighter off the stage
/// with nothing to punish.
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

/// The committed direction is remembered, not re-read: turning mid-dash must
/// not sweep the cone.
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

/// A KO'd fighter is not a target.
///
/// A fighter who lost a stock carries `OutOfPlay` and, because a respawn
/// restores it, full health. The fix reuses `body_is_untouchable`, the combat
/// domain's participation gate, which covers out-of-play so target selection
/// sees it.
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

/// A teammate is not a target either: in team versus, a "not me" filter would
/// bend a fighter at the ally beside them.
///
/// The control is `the_dash_bends_toward_a_foe_inside_the_cone`: same geometry,
/// one field different (a shared team), opposite outcome.
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

/// A body the dash may hit but must not hunt: different faction, no team, no
/// declared hostility. `CombatRelation` calls this `Neutral`: damageable, but
/// not a target.
///
/// No `MatchTeam` on purpose: team relation outranks faction, so a team would
/// make it a `Foe`.
fn bystander(app: &mut App, at: ae::Vec2) -> Entity {
    app.world_mut()
        .spawn((
            ae::BodyKinematics {
                pos: at,
                facing: 1.0,
                ..Default::default()
            },
            ambition_platformer2d::combat::components::ActorFaction::Npc,
        ))
        .id()
}

/// Arbitration: a nearer bystander does not outbid the foe.
///
/// `damage_lands_between` and `is_target` differ only on `Neutral`. Separate
/// one-candidate tests cannot prove arbitration; only a scene with both can
/// tell "picked the foe" from "picked the only one offered". The bystander is
/// at the same angle and half the distance, so it wins every geometric
/// tie-break if eligibility is wrong.
#[test]
fn a_nearer_bystander_does_not_outbid_the_actual_foe() {
    for foe_first in [true, false] {
        let mut app = app();
        let hunter = body(&mut app, 1, ae::Vec2::ZERO);

        // Construction order is reversed on the second pass, so a filter that
        // keeps the first eligible body it meets fails one of them.
        let (foe, bystander_entity) = if foe_first {
            let foe = body(&mut app, 0, ae::Vec2::new(160.0, -160.0));
            (foe, bystander(&mut app, ae::Vec2::new(80.0, 80.0)))
        } else {
            let near = bystander(&mut app, ae::Vec2::new(80.0, 80.0));
            (body(&mut app, 0, ae::Vec2::new(160.0, -160.0)), near)
        };
        let _ = (foe, bystander_entity);

        dash(&mut app, hunter);
        let vel = velocity(&app, hunter);

        assert!(
            vel.x > 0.0,
            "the dash went backwards (foe_first={foe_first}): {vel:?}"
        );
        assert!(
            vel.y < -100.0,
            "the dash bent DOWNWARD, toward the neutral bystander at half the \
             distance, instead of upward toward the fighter it is actually in a \
             match with (foe_first={foe_first}): {vel:?}. Eligibility is being \
             decided by the DAMAGE relation, which answers `true` for a neutral; \
             the targeting relation answers `false`."
        );
    }
}

/// The scene above means something only if the bystander is damageable. This
/// asks both authorities directly about the same pair and expects opposite
/// answers.
#[test]
fn the_bystander_is_damageable_and_still_not_a_target() {
    use ambition_platformer2d::combat::targeting;
    let mut app = app();
    let hunter = body(&mut app, 1, ae::Vec2::ZERO);
    let other = bystander(&mut app, ae::Vec2::new(80.0, 80.0));

    let hunter_team = app
        .world()
        .get::<targeting::MatchTeam>(hunter)
        .cloned()
        .expect("a seated fighter carries its own team");

    assert!(
        targeting::damage_lands_between(
            ambition_platformer2d::combat::components::ActorFaction::Player,
            ambition_platformer2d::combat::components::ActorFaction::Npc,
            Some(&hunter_team),
            None,
            targeting::FriendlyFire { enabled: false },
            None,
            other,
        ),
        "the bystander is not damageable, so the arbitration scene proves nothing"
    );
    assert!(
        !targeting::combat_relation(
            None,
            ambition_platformer2d::combat::components::ActorFaction::Player,
            None,
            Some(&hunter_team),
            None,
            other,
            ambition_platformer2d::combat::components::ActorFaction::Npc,
            None,
            None,
        )
        .is_target(),
        "the bystander IS a target, so there is no disagreement to guard"
    );
}
