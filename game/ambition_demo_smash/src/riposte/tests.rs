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
        // The default fixture stays unvoiced, so every existing assertion in this
        // file keeps testing what it tested. The voice has its own test below.
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

/// ⭐⭐ TWO TECHNIQUES WITH DIFFERENT VOICES ARE HEARD APART.
///
/// `spawn_body_strike` hard-coded `strike_sfx: None` until 2026-09-06, so every
/// technique-spawned strike in the game fell back to the VICTIM's material sound.
/// That is not silence — `resolve_strike_sfx` has that fallback deliberately —
/// but it meant a swordfighter's counter and a brawler's ground shock, which are
/// the same mechanic here, were the same EVENT to anybody not watching the
/// animation.
///
/// ⛔⛔ THIS ASSERTS THE TWO IDS DIFFER, NOT THAT A FIELD IS SET. A test that only
/// found `Some(_)` passes against a seam that ignores the authored string and
/// stamps one constant on everything, which is exactly the bug in a different
/// costume. ⇒ Name the edit that would make this false: hard-coding any single
/// id, or dropping the `.map()` at the call site.
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

/// ⭐⭐ THE CUT IS BODY-LOCAL, WHICH MEANS BOTH ITS REACH AND ITS SHAPE ROTATE
/// WITH THE FIGHTER'S FRAME.
///
/// ⛔⛔ NEITHER DID, AND NO EXISTING TEST IN THIS FILE COULD SEE IT: every other
/// fixture leaves `ResolvedMotionFrame::default()` installed, where the body
/// frame and the world axes agree and an un-rotated offset is indistinguishable
/// from a body-local one. `riposte.rs` built `Vec2::new(facing * reach, 0.0)`
/// and handed it, with raw `half_extents`, to `spawn_body_strike`, which stores
/// them as a `FollowOwner` offset — owner position PLUS that vector, never
/// rotated. The authored path (`place_body_local_volume`) rotates BOTH through
/// `AccelerationFrame::to_world` / `to_world_half`.
///
/// ⚠ THREE VICTIMS, EACH REFUTING A DIFFERENT WRONG ANSWER, because a fix that
/// rotates the centre and forgets the extents is the likely partial:
///
/// | victim | reached when |
/// |---|---|
/// | `in_front` | the OFFSET is rotated (any fix at all) |
/// | `beside` | it is NOT — this is where the shipped bug cut |
/// | `past_the_tip` | the EXTENTS are rotated too, so the blade is long |
///
/// The frame is a 90° one, so `to_world_half` is exact (it swaps 30×16 for
/// 16×30) rather than the bounding over-approximation an off-axis frame gets.
#[test]
fn the_cut_lands_body_local_when_the_frame_is_rotated() {
    let mut app = app();
    let defender = fighter(&mut app, 0, ae::Vec2::new(100.0, 100.0), 1.0);
    // Normal gravity here is down=(0,1) — this world's y grows toward the feet.
    // Turning down to +x turns her side axis, which is `to_world`'s x, to -y.
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
    // World +x, where the un-rotated offset used to put the cut.
    let beside = fighter(&mut app, 2, ae::Vec2::new(146.0, 100.0), -1.0);
    // Body y -9..31: inside the rotated blade's 30-long half, outside the 16
    // an un-rotated one would have. Only a fix that turns the extents reaches him.
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


/// ⭐⭐ THE SEAM: A PARRY REACHES THE BLADE.
///
/// The counter's tests prove a parry in a live stance emits the technique its
/// author named. The tests above prove that technique, once requested, damages
/// the fighter in front and not the one who threw it. Between them is the
/// question a player asks — does a counter actually CUT? — and each half can be
/// green while the key one WRITES and the key the other READS have drifted.
///
/// ⚠ THE STANCE IS AUTHORED HERE AND THE SHIPPED ONE IS PINNED ELSEWHERE, which
/// is a deliberate split rather than a shortcut. `counter.rs`'s own fixture
/// argues for testing the move a player presses, and the move that presses this
/// key is the Pointed Polygon's `polygon_riposte` — in `ambition_content`, which
/// this crate does not depend on and should not grow a dependency on to reach
/// one moveset. ⇒ The Swordie's own numbers are pinned where they live
/// (`his_down_b_is_a_counter_that_answers_with_the_blade` checks his response
/// key and that his cut is usable); what THIS test owns is the wiring between
/// the two systems, and for that any stance naming the key will do.
#[test]
fn a_parry_answered_with_the_blade_cuts_the_attacker() {
    use ambition_platformer2d::characters::smash_counter::{counter_move, CounterParams};
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
    // Mid-stance: the Active window is the second of the three `counter_move`
    // builds, and a parry only counts while it is under the clock.
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
    // The response is written on the first tick and the cut it spawns resolves
    // on the second.
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
