//! ⛔⛔ THESE TESTS ASSERT DAMAGE, NOT A REQUEST, AND THE FIRST DRAFT DID NOT.
//!
//! It asked whether an `EffectRequest::DamageBox` had been written with the
//! fields I had just typed — a question about my own authoring, not about the
//! engine's answer to it. That is the exact shape of assertion that let a thrown
//! bomb, a placed mine and a steered bolt all ship damaging NOBODY, and it would
//! have passed here too: the draft's world-anchored box was either inert
//! (`HitSide::Player` + `World` resolves to no melee path at all) or a hazard
//! that cuts the fighter who threw it (`Environment` consults no
//! self-exclusion). Both spellings were wrong and both would have been green.
//!
//! ⇒ So the fixture runs the REAL resolver, `apply_hitbox_damage`, and asks who
//! actually got hit. `the_cut_lands_on_the_attacker_and_not_on_the_fighter_who_
//! answered` is the whole point of the technique stated as an outcome.

use super::*;
use ambition_platformer2d::combat::events::{HitEvent, HitTarget};
use ambition_platformer2d::combat::hitbox::{LandedBodyHit, ParriedBodyHit};
use ambition_platformer2d::vfx::VfxMessage;

fn app() -> App {
    let mut app = App::new();
    app.add_message::<ActorActionMessage>();
    app.add_message::<HitEvent>();
    app.add_message::<LandedBodyHit>();
    app.add_message::<ParriedBodyHit>();
    app.add_message::<VfxMessage>();
    app.add_systems(
        Update,
        (
            cut_where_a_riposte_answers,
            ambition_platformer2d::combat::hitbox::apply_hitbox_damage,
        )
            .chain(),
    );
    app
}

/// A complete enough combat body to be hit, seated on its own team.
///
/// ⭐ THE TEAM IS NOT DECORATION. Both fighters are `ActorFaction::Player`, and
/// `damage_lands_between` with friendly fire off would refuse a same-side hit —
/// so a fixture without teams would prove the cut lands on nobody and call it a
/// pass. A real match gives every seat its own team (`prepared::team_for`:
/// "each seat gets its own team, producing free-for-all relationships").
fn fighter(app: &mut App, seat: usize, at: ae::Vec2, facing: f32) -> Entity {
    app.world_mut()
        .spawn((
            ae::BodyKinematics {
                pos: at,
                facing,
                ..Default::default()
            },
            ae::CenteredAabb::new(at, ae::Vec2::new(14.0, 20.0)),
            ambition_platformer2d::world::ResolvedMotionFrame::default(),
            ambition_platformer2d::combat::components::ActorFaction::Player,
            ambition_platformer2d::combat::targeting::MatchTeam::new(format!("seat{seat}")),
            ae::BodyOffense::default(),
            ae::BodyMotionFacts::default(),
            ae::BodyShieldState::default(),
            ambition_platformer2d::characters::actor::BodyCombat::default(),
        ))
        .id()
}

fn params() -> RiposteStrikeParams {
    RiposteStrikeParams {
        damage: 11,
        // A FEEL MULTIPLIER. See the params' own doc: this is not a speed.
        knockback: 1.35,
        reach: 46.0,
        half_extents: (30.0, 16.0),
        lifetime_s: 0.08,
    }
}

fn answer(app: &mut App, actor: Entity, params: &RiposteStrikeParams) {
    let request = ActionRequest::Special {
        spec: SpecialActionSpec::Special(RIPOSTE_STRIKE.to_string()),
        params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(params)
            .expect("riposte params serialize"),
    };
    app.world_mut()
        .write_message(ActorActionMessage { actor, request });
    // Twice: the first tick spawns the cut through `Commands`, the second
    // resolves it. Nothing here depends on which sync point Bevy chooses.
    app.update();
    app.update();
}

/// Every body hit the resolver produced, drained ONCE.
///
/// ⛔ ONCE IS THE POINT. The first version of this helper took a body and
/// drained the queue per call, so the second question in a test — "and did it
/// miss the fighter who threw it?" — was asked of an EMPTY queue and answered
/// yes no matter what. Three tests failed against working code because of it,
/// and the failure looked exactly like "the cut damages nobody".
fn body_hits(app: &mut App) -> Vec<(Entity, i32)> {
    app.world_mut()
        .resource_mut::<bevy::ecs::message::Messages<HitEvent>>()
        .drain()
        .filter_map(|event| match event.target {
            HitTarget::Body(body) => Some((body, event.damage)),
            _ => None,
        })
        .collect()
}

fn damage_to(hits: &[(Entity, i32)], body: Entity) -> Vec<i32> {
    hits.iter()
        .filter(|(who, _)| *who == body)
        .map(|(_, damage)| *damage)
        .collect()
}

/// ⭐⭐ THE TECHNIQUE'S WHOLE CLAIM, AS AN OUTCOME: the fighter who swung takes
/// the cut, and the fighter who answered does not.
#[test]
fn the_cut_lands_on_the_attacker_and_not_on_the_fighter_who_answered() {
    let mut app = app();
    let defender = fighter(&mut app, 0, ae::Vec2::new(100.0, 50.0), 1.0);
    // Standing 46px away, which is where the authored reach puts the cut.
    let attacker = fighter(&mut app, 1, ae::Vec2::new(146.0, 50.0), -1.0);
    answer(&mut app, defender, &params());

    let hits = body_hits(&mut app);
    let on_defender = damage_to(&hits, defender);
    let on_attacker = damage_to(&hits, attacker);
    assert_eq!(
        on_attacker,
        vec![11],
        "the answering cut did not land on the fighter who swung",
    );
    assert!(
        on_defender.is_empty(),
        "the counter cut the fighter who threw it for {on_defender:?} — which is \
         what a world-anchored hazard would have done",
    );
}

/// ⛔⛔ A CUT WIDE ENOUGH TO COVER ITS OWN THROWER STILL SPARES HIM, AND THIS
/// TEST EXISTS BECAUSE THE ONE ABOVE COULD NOT PROVE IT.
///
/// Poisoning `HitSide::Player` to `Environment` — the hazard spelling, which by
/// design consults no self-exclusion — left every test green. The reason was
/// geometry, not correctness: at 46px of reach and 30px of half-width the cut
/// spans x 116..176 and the defender's body ends at 114, so "he was not hit"
/// held for a reason that had nothing to do with the side. ⇒ A wide cut puts
/// him inside it, and then only the owner rule can keep him out.
#[test]
fn a_cut_wide_enough_to_cover_its_thrower_still_spares_him() {
    let mut app = app();
    let defender = fighter(&mut app, 0, ae::Vec2::new(100.0, 50.0), 1.0);
    let attacker = fighter(&mut app, 1, ae::Vec2::new(146.0, 50.0), -1.0);
    answer(
        &mut app,
        defender,
        &RiposteStrikeParams {
            // Spans x 86..206 around a cut centred at 146: the thrower's own
            // body (86..114) is inside it.
            half_extents: (60.0, 16.0),
            ..params()
        },
    );
    let hits = body_hits(&mut app);
    assert_eq!(
        damage_to(&hits, attacker),
        vec![11],
        "the wide cut did not reach the fighter who swung",
    );
    assert!(
        damage_to(&hits, defender).is_empty(),
        "the cut damaged the fighter who threw it — which is what the hazard \
         side does, by design, and why this technique is not one",
    );
}

/// ⛔ THE MIRROR, because `facing` is a signed float and the sign IS the
/// implementation: a cut that always lands to the right is a move that only
/// works facing one way.
#[test]
fn facing_left_cuts_left() {
    let mut app = app();
    let defender = fighter(&mut app, 0, ae::Vec2::new(100.0, 50.0), -1.0);
    let behind = fighter(&mut app, 1, ae::Vec2::new(146.0, 50.0), -1.0);
    let ahead = fighter(&mut app, 2, ae::Vec2::new(54.0, 50.0), 1.0);
    answer(&mut app, defender, &params());
    let hits = body_hits(&mut app);
    assert!(
        damage_to(&hits, behind).is_empty(),
        "a left-facing riposte cut somebody standing to its right",
    );
    assert_eq!(damage_to(&hits, ahead), vec![11]);
}

/// ⛔ THE CUT TRACKS ITS OWNER. `FollowOwner` offsets are body-local, so a
/// fighter who is moved after answering carries the cut with them; a
/// world-anchored one would hang in the air where they parried.
#[test]
fn the_cut_follows_the_fighter_rather_than_the_spot_they_parried_on() {
    let mut app = app();
    let defender = fighter(&mut app, 0, ae::Vec2::new(100.0, 50.0), 1.0);
    let attacker = fighter(&mut app, 1, ae::Vec2::new(246.0, 50.0), -1.0);
    answer(&mut app, defender, &params());
    assert!(
        damage_to(&body_hits(&mut app), attacker).is_empty(),
        "the cut reached 100px further than its authored reach",
    );
    // Move the answering fighter next to them; the live cut comes along.
    //
    // ⛔ BOTH, AND `CenteredAabb` IS THE ONE THAT MATTERS. `apply_hitbox_damage`
    // resolves a `FollowOwner` anchor against the owner's published
    // `CenteredAabb` and falls back to `BodyKinematics` only for bare test
    // bodies. Moving the kinematics alone left the cut where it started and the
    // test failed against correct code — the engine reads the position a body
    // PUBLISHES, not the one it integrates.
    {
        let world = app.world_mut();
        world
            .get_mut::<ae::BodyKinematics>(defender)
            .unwrap()
            .pos = ae::Vec2::new(200.0, 50.0);
        *world.get_mut::<ae::CenteredAabb>(defender).unwrap() =
            ae::CenteredAabb::new(ae::Vec2::new(200.0, 50.0), ae::Vec2::new(14.0, 20.0));
    }
    app.update();
    assert_eq!(
        damage_to(&body_hits(&mut app), attacker),
        vec![11],
        "the cut stayed where the parry happened instead of following its owner",
    );
}

/// ⛔ THE UNITS ERROR, REFUSED RATHER THAN SPAWNED. `knockback` is a feel
/// multiplier; 104.0 is a launch SPEED copied off a `Strike`, the mistake three
/// shipped moves made. Nothing is spawned, so nobody is hit.
#[test]
fn a_cut_authored_in_the_wrong_units_is_refused() {
    let mut app = app();
    let defender = fighter(&mut app, 0, ae::Vec2::new(100.0, 50.0), 1.0);
    let attacker = fighter(&mut app, 1, ae::Vec2::new(146.0, 50.0), -1.0);
    answer(
        &mut app,
        defender,
        &RiposteStrikeParams {
            knockback: 104.0,
            ..params()
        },
    );
    assert!(
        damage_to(&body_hits(&mut app), attacker).is_empty(),
        "a cut with a launch speed in the feel-multiplier field was spawned",
    );
}

/// ⛔ AND A MOVE THAT IS NOT THIS ONE IS LEFT ALONE — the arm that keeps every
/// special in the game from ending in a sword cut.
#[test]
fn another_technique_does_not_cut() {
    let mut app = app();
    let defender = fighter(&mut app, 0, ae::Vec2::new(100.0, 50.0), 1.0);
    let attacker = fighter(&mut app, 1, ae::Vec2::new(146.0, 50.0), -1.0);
    let request = ActionRequest::Special {
        spec: SpecialActionSpec::Special("smash.teleport".to_string()),
        params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(&params())
            .expect("params serialize"),
    };
    app.world_mut().write_message(ActorActionMessage {
        actor: defender,
        request,
    });
    app.update();
    app.update();
    assert!(
        damage_to(&body_hits(&mut app), attacker).is_empty(),
        "a teleport cut somebody",
    );
}

