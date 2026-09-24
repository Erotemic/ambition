//! These tests assert damage, not only the request.
//!
//! Asserting an `EffectRequest::DamageBox` with the typed fields checks only the
//! authoring. A box can be inert (`HitSide::Player` + `World` has no melee path)
//! or hit its own thrower (`Environment` has no self-exclusion). So the
//! fixture runs the real resolver, `apply_hitbox_damage`, and asks who got
//! hit. `the_cut_lands_on_the_attacker_and_not_on_the_fighter_who_answered` is
//! the technique stated as an outcome.

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
/// The team matters: both fighters are `ActorFaction::Player`, and
/// `damage_lands_between` with friendly fire off refuses a same-side hit. A
/// real match gives every seat its own team (`prepared::team_for`).
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
            ae::BodyMotionFacts::default(),
            ae::BodyShieldState::default(),
            ambition_platformer2d::characters::actor::BodyCombat::default(),
        ))
        .id()
}

fn params() -> RiposteStrikeParams {
    RiposteStrikeParams {
        damage: 11,
        // A feel multiplier (see the params' doc), not a speed.
        knockback: 1.35,
        reach: 46.0,
        half_extents: (30.0, 16.0),
        lifetime_s: 0.08,
        // Unvoiced by default; the voice has its own test below.
        hit_sfx: None,
    }
}

fn answer(app: &mut App, actor: Entity, params: &RiposteStrikeParams) {
    let request = ActionRequest::Special {
        spec: SpecialActionSpec::Special(RIPOSTE_STRIKE.to_string()),
        params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(params)
            .expect("riposte params serialize"),
    };
    app.world_mut()
        .write_message(ActorActionMessage { actor, request, move_instance: None });
    // Twice: the first tick spawns the cut through `Commands`, the second
    // resolves it.
    app.update();
    app.update();
}

/// Every body hit the resolver produced, drained once. Draining per query
/// would make a second question read an empty queue and always pass.
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

/// Two techniques with different voices are heard apart.
///
/// Without an authored strike sound, a technique strike falls back to the
/// victim's material sound (`resolve_strike_sfx`). This asserts the two ids
/// differ, not only that a field is set, so a constant stamped on every strike
/// fails.
#[test]
fn two_voices_spawn_two_different_strike_sounds() {
    let voice = |name: &str| {
        let mut app = app();
        let defender = fighter(&mut app, 0, ae::Vec2::new(100.0, 50.0), 1.0);
        let _attacker = fighter(&mut app, 1, ae::Vec2::new(146.0, 50.0), -1.0);
        answer(
            &mut app,
            defender,
            &RiposteStrikeParams {
                hit_sfx: Some(name.to_string()),
                ..params()
            },
        );
        app.world_mut()
            .query::<&ambition_platformer2d::combat::hitbox::Hitbox>()
            .iter(app.world())
            .find_map(|hitbox| hitbox.strike_sfx)
            .expect("the cut carries the voice it was authored with")
    };
    let blade = voice("player.slash");
    let blunt = voice("world.rock.hit");
    assert_ne!(
        blade, blunt,
        "a blade and a rock resolved to the same sound, so the authored name is \
         not reaching the hitbox",
    );
}

/// The cut is body-local: both its reach and its shape rotate with the
/// fighter's frame.
///
/// Other fixtures use `ResolvedMotionFrame::default()`, where body and world
/// axes agree. The authored path (`place_body_local_volume`) rotates offset and
/// extents through `AccelerationFrame::to_world` / `to_world_half`; so must
/// this one. Three victims, each refuting a different wrong answer:
///
/// | victim | reached when |
/// |---|---|
/// | `in_front` | the offset is rotated |
/// | `beside` | it is not (the unrotated bug) |
/// | `past_the_tip` | the extents are rotated too, so the blade is long |
///
/// The frame is 90°, so `to_world_half` is exact (30×16 becomes 16×30).
#[test]
fn the_cut_lands_body_local_when_the_frame_is_rotated() {
    let mut app = app();
    let defender = fighter(&mut app, 0, ae::Vec2::new(100.0, 100.0), 1.0);
    // Normal gravity is down=(0,1); y grows toward the feet. Turning down to
    // +x turns her side axis (`to_world`'s x) to -y.
    {
        let rotated = ae::MotionFrame::from_direction(ae::Vec2::new(1.0, 0.0), 0.0);
        let mut defender_mut = app.world_mut().entity_mut(defender);
        let mut frame = defender_mut
            .get_mut::<ambition_platformer2d::world::ResolvedMotionFrame>()
            .expect("the fixture gives every fighter a frame");
        frame.publish_resolved_frame(rotated);
    }
    // Body-forward of her: reach 46 along world -y. Cut spans y 24..84, x 84..116.
    let in_front = fighter(&mut app, 1, ae::Vec2::new(100.0, 54.0), -1.0);
    // World +x, where an unrotated offset would put the cut.
    let beside = fighter(&mut app, 2, ae::Vec2::new(146.0, 100.0), -1.0);
    // Body y -9..31: inside the rotated blade's 30-long half, outside an
    // unrotated 16. Only rotated extents reach him.
    let past_the_tip = fighter(&mut app, 3, ae::Vec2::new(100.0, 11.0), -1.0);
    answer(&mut app, defender, &params());

    let hits = body_hits(&mut app);
    assert!(
        !damage_to(&hits, in_front).is_empty(),
        "the cut missed the fighter standing in front of her OWN frame: {hits:?}",
    );
    assert!(
        damage_to(&hits, beside).is_empty(),
        "the cut reached along WORLD x instead of her body forward: {hits:?}",
    );
    assert!(
        !damage_to(&hits, past_the_tip).is_empty(),
        "the blade kept its un-rotated 16px reach across her forward axis, so it \
         stopped short of a fighter the 30px blade covers: {hits:?}",
    );
}

/// The technique's claim as an outcome: the fighter who swung takes the cut,
/// and the fighter who answered does not.
#[test]
fn the_cut_lands_on_the_attacker_and_not_on_the_fighter_who_answered() {
    let mut app = app();
    let defender = fighter(&mut app, 0, ae::Vec2::new(100.0, 50.0), 1.0);
    // 46px away, where the authored reach puts the cut.
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

/// A cut wide enough to cover its own thrower still spares him.
///
/// In the test above, geometry alone keeps the defender out of the cut. Here
/// he is inside it, so only the owner rule can spare him.
#[test]
fn a_cut_wide_enough_to_cover_its_thrower_still_spares_him() {
    let mut app = app();
    let defender = fighter(&mut app, 0, ae::Vec2::new(100.0, 50.0), 1.0);
    let attacker = fighter(&mut app, 1, ae::Vec2::new(146.0, 50.0), -1.0);
    answer(
        &mut app,
        defender,
        &RiposteStrikeParams {
            // Spans x 86..206 around a cut centred at 146, so the thrower's
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

/// The mirror: `facing` is a signed float, and a cut that always lands right
/// works in only one direction.
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

/// The cut tracks its owner: `FollowOwner` offsets are body-local, so a moved
/// fighter carries the cut; a world-anchored one would stay where they parried.
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
    // Move the answering fighter; the live cut comes along.
    //
    // Move both: `apply_hitbox_damage` resolves `FollowOwner` against the
    // owner's published `CenteredAabb`, and uses `BodyKinematics` only for
    // bare test bodies.
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

/// Wrong units are refused: `knockback` is a feel multiplier, and 104.0 is a
/// launch speed. Nothing is spawned, so nobody is hit.
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

/// A move that is not this one is left alone, so not every special ends in a
/// sword cut.
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
        move_instance: None,
    });
    app.update();
    app.update();
    assert!(
        damage_to(&body_hits(&mut app), attacker).is_empty(),
        "a teleport cut somebody",
    );
}

/// The seam: a parry reaches the blade.
///
/// The counter's tests prove a parry emits the named technique; the tests
/// above prove the technique damages the right fighter. This checks the key
/// one side writes is the key the other reads.
///
/// The stance is authored here. The shipped move (`polygon_riposte`) lives in
/// `ambition_content`, which this crate does not depend on; its numbers are
/// pinned by `his_down_b_is_a_counter_that_answers_with_the_blade`. This test
/// owns only the wiring.
#[test]
fn a_parry_answered_with_the_blade_cuts_the_attacker() {
    use ambition_platformer2d::entity_catalog::smash_counter::{counter_move, CounterParams};
    use ambition_platformer2d::combat::hitbox::{LandedBodyHit, ParriedBodyHit};
    use ambition_platformer2d::combat::moveset::MovePlayback;

    let stance_spec = counter_move(
        "a_stance_that_answers_with_the_blade",
        "attack_down",
        0.07,
        0.15,
        0.42,
        CounterParams {
            window_s: 0.05,
            answers_the_attacker: false,
            response: RIPOSTE_STRIKE.to_string(),
            response_params: ambition_platformer2d::entity_catalog::ParamValue::from_typed(
                &params(),
            )
            .expect("riposte params serialize"),
            absorbs_projectiles: false,
        },
    );
    // Mid-stance: the Active window is the second of `counter_move`'s three,
    // and a parry counts only inside it.
    let stance = stance_spec.windows[1].clone();
    let mut playback = MovePlayback::new(stance_spec, 1.0);
    playback.t = (stance.start_s + stance.end_s) * 0.5;

    let mut app = app();
    app.add_message::<ParriedBodyHit>();
    app.add_message::<LandedBodyHit>();
    app.add_systems(
        Update,
        (
            crate::counter::hold_counter_parry_windows,
            crate::counter::answer_a_parry_with_the_authored_counter,
        )
            .before(cut_where_a_riposte_answers),
    );

    let defender = fighter(&mut app, 0, ae::Vec2::new(100.0, 50.0), 1.0);
    app.world_mut().entity_mut(defender).insert(playback);
    let attacker = fighter(&mut app, 1, ae::Vec2::new(146.0, 50.0), -1.0);

    app.world_mut().write_message(ParriedBodyHit {
        defender,
        attacker,
        hitbox: attacker,
        contact: ae::Vec2::new(120.0, 50.0),
    });
    // The response is written on the first tick; its cut resolves on the
    // second.
    app.update();
    app.update();

    let hits = body_hits(&mut app);
    assert_eq!(
        damage_to(&hits, attacker),
        vec![11],
        "the parry did not reach the blade — the counter and the cut agree on a \
         key or they do not, and nothing between them said so",
    );
    assert!(
        damage_to(&hits, defender).is_empty(),
        "his own counter cut him",
    );
}
