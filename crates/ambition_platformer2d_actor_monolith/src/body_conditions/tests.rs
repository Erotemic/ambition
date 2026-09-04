use super::*;
use bevy::prelude::*;

/// A body the player is driving, with the given capability set.
fn body_with(app: &mut App, abilities: AbilitySet) {
    app.world_mut().spawn((
        BodyAbilities::new(abilities),
        // ⚠ DRIVING, not `PlayerEntity`. Possession moves `DrivingParticipant`
        // off the home avatar and onto the vessel, so the driven marker is what
        // these conditions ask — see `driven_bodies`.
        ambition_characters::control::DrivingParticipant(
            ambition_characters::control::PlayerSlot::PRIMARY,
        ),
    ));
}

fn ask(world: &World, verb: &str) -> ConditionOutcome {
    can(world, &[AuthoredArg::Name(verb.to_string())])
}

/// THE BODY ANSWERS, AND IT ANSWERS ABOUT THE EFFECTIVE SET.
#[test]
fn a_body_reports_the_verbs_it_actually_has() {
    let mut app = App::new();
    let mut abilities = AbilitySet::default();
    abilities.wall_climb = true;
    abilities.fly = false;
    body_with(&mut app, abilities);
    let world = app.world();

    assert_eq!(ask(world, "wall_climb"), ConditionOutcome::Satisfied);
    assert!(matches!(
        ask(world, "fly"),
        ConditionOutcome::NotSatisfied(_)
    ));
}

/// ⛔ A MISSPELT VERB IS UNANSWERABLE, NOT `false`.
///
/// The difference is the whole reason a route may be gated on this: an author
/// who writes `wallclimb` must get a diagnostic and a wall that stands, not a
/// gate that quietly never opens.
#[test]
fn a_verb_the_ability_set_has_no_field_for_is_unanswerable() {
    let mut app = App::new();
    body_with(&mut app, AbilitySet::default());
    assert!(matches!(
        ask(app.world(), "wallclimb"),
        ConditionOutcome::Unanswerable(_)
    ));
}

/// ⛔ AND IT IS UNANSWERABLE WITH NO BODY IN THE WORLD TOO.
///
/// Resolving the body before the name would report "nothing is driving" for a
/// typo, which sends the author looking in the wrong place.
#[test]
fn a_misspelt_verb_is_a_content_fault_even_with_nobody_to_ask() {
    let app = App::new();
    assert!(matches!(
        ask(app.world(), "wallclimb"),
        ConditionOutcome::Unanswerable(_)
    ));
    // A real verb with no body is an honest `no`, not a fault.
    assert!(matches!(
        ask(app.world(), "wall_climb"),
        ConditionOutcome::NotSatisfied(_)
    ));
}

/// EVERY FIELD OF `AbilitySet` IS ASKABLE BY ITS OWN NAME, AND READS ITS OWN
/// VALUE.
///
/// ⭐ THE FIELD LIST COMES FROM `serde`, NOT FROM THIS TEST. A hand-typed list
/// of 29 names would be a third copy to keep in step, and the one most likely
/// to be forgotten — so the set serializes itself and every key it produces is
/// asked for. Adding a capability therefore extends the test automatically.
///
/// ⛔ TWO MIXED SETS, NOT AN ALL-ON ONE. `ability_named` binds every field and
/// then routes each name to one of them, so an arm reading the WRONG field
/// compiles and is invisible against a uniform set — all-true agrees with any
/// routing. `basic()` and `sane_subset()` differ from each other and from
/// themselves field to field, which is what makes a swap show up.
#[test]
fn every_ability_answers_to_its_own_field_name_and_value() {
    for set in [AbilitySet::basic(), AbilitySet::sane_subset()] {
        let json = serde_json::to_value(set).expect("an AbilitySet serializes");
        let fields = json.as_object().expect("it serializes as a map of fields");
        assert_eq!(
            fields.len(),
            29,
            "the ability vocabulary is 29 fields; if this moved, `ability_named`\
             and this test have both just been told about it"
        );
        for (field, value) in fields {
            assert_eq!(
                ability_named(&set, field),
                value.as_bool(),
                "`body.can {field}` must read the field of that name"
            );
        }
    }
}

/// A body of a given height, driven by the player.
fn body_of_height(app: &mut App, height: f32) {
    let mut kinematics = BodyKinematics::default();
    kinematics.size.y = height;
    app.world_mut().spawn((
        kinematics,
        ambition_characters::control::DrivingParticipant(
            ambition_characters::control::PlayerSlot::PRIMARY,
        ),
    ));
}

fn fits_in(world: &World, opening: f64) -> ConditionOutcome {
    fits(world, &[AuthoredArg::Number(opening)])
}

/// THE BODY IS MEASURED, AND BOTH SIDES OF THE COMPARISON ARE ASSERTED.
///
/// The equal case is the one worth pinning: an opening exactly the body's
/// height is a gap the body passes through, so `fits` is `<=` and a future
/// change to `<` would silently close every route authored at the body's own
/// size — the most likely number for an author to write.
#[test]
fn a_body_fits_an_opening_no_shorter_than_it_is() {
    let mut app = App::new();
    body_of_height(&mut app, 32.0);
    let world = app.world();

    assert_eq!(fits_in(world, 48.0), ConditionOutcome::Satisfied);
    assert_eq!(fits_in(world, 32.0), ConditionOutcome::Satisfied);
    assert!(matches!(
        fits_in(world, 24.0),
        ConditionOutcome::NotSatisfied(_)
    ));
}

/// ⭐ THE CURRENT SIZE, NOT THE STANDING BASELINE — the whole reason this reads
/// `BodyKinematics`.
///
/// A body that crouches into an opening it could not stand up in must open the
/// route, or the wall and the collision doctrine disagree about the same hole.
#[test]
fn a_body_that_crouches_into_the_opening_fits_it() {
    let mut app = App::new();
    body_of_height(&mut app, 64.0);
    assert!(matches!(
        fits_in(app.world(), 32.0),
        ConditionOutcome::NotSatisfied(_)
    ));

    let body = app
        .world_mut()
        .query_filtered::<bevy::prelude::Entity, bevy::prelude::With<BodyKinematics>>()
        .single(app.world())
        .expect("one body");
    app.world_mut()
        .entity_mut(body)
        .get_mut::<BodyKinematics>()
        .expect("it was spawned with one")
        .size
        .y = 30.0;

    assert_eq!(fits_in(app.world(), 32.0), ConditionOutcome::Satisfied);
}

/// A NON-POSITIVE OPENING IS A CONTENT FAULT, not a wall that never opens.
///
/// No body has a height of zero, so `false` would be the right answer for the
/// wrong reason and would hide the authoring mistake behind a route that
/// correctly never opens — which is the failure this whole three-outcome enum
/// exists to prevent.
#[test]
fn an_opening_that_is_not_positive_is_unanswerable() {
    let mut app = App::new();
    body_of_height(&mut app, 32.0);
    let world = app.world();

    assert!(matches!(fits_in(world, 0.0), ConditionOutcome::Unanswerable(_)));
    assert!(matches!(
        fits_in(world, -16.0),
        ConditionOutcome::Unanswerable(_)
    ));
}

/// ⛔ THE ANSWER DOES NOT MOVE WHEN GRAVITY DOES.
///
/// `BodyKinematics::size` is the body's own-frame size, and
/// `aabb_oriented(gravity_dir)` — what the collision doctrine sweeps — swaps
/// width and height under sideways gravity. `body.fits` reads the own-frame
/// height on purpose: a world-space reading would make one authored wall open
/// and close as gravity flipped, so an author could not say what their own
/// crawlspace means without knowing which way gravity pointed when the player
/// arrived.
///
/// This pins the rule by giving the body a footprint whose two axes DISAGREE —
/// 30 tall, 64 wide — so a reading that used the other axis answers differently
/// and is caught, rather than a square body under which both readings agree.
#[test]
fn the_opening_is_measured_against_the_bodys_own_height_not_its_world_footprint() {
    let mut app = App::new();
    let mut kinematics = BodyKinematics::default();
    kinematics.size = bevy::math::Vec2::new(64.0, 30.0);
    app.world_mut().spawn((
        kinematics,
        ambition_characters::control::DrivingParticipant(
            ambition_characters::control::PlayerSlot::PRIMARY,
        ),
    ));

    assert_eq!(
        fits_in(app.world(), 32.0),
        ConditionOutcome::Satisfied,
        "30 tall fits a 32 opening; reading the 64-wide axis would refuse it"
    );
    assert!(
        matches!(fits_in(app.world(), 20.0), ConditionOutcome::NotSatisfied(_)),
        "and it is still a comparison, not a constant yes"
    );
}

// ── POSSESSION: the resting home avatar must not answer for the vessel ───────
//
// ⛔⛔ `control/authority.rs:39` REMOVES `DrivingParticipant` from the home
// avatar and inserts it on the possessed target — and the home avatar keeps
// `PlayerEntity` the whole time. A predicate that accepted either marker
// therefore answered from the body the participant is NOT driving, which is why
// every test below spawns two bodies that DISAGREE. A fixture where both agree
// cannot tell the two populations apart.

/// The world during possession: a home avatar that is `PlayerEntity` and no
/// longer driving, and a vessel that is driving and is not `PlayerEntity`.
fn possession(app: &mut App, home: impl bevy::prelude::Bundle, vessel: impl bevy::prelude::Bundle) {
    app.world_mut()
        .spawn((home, ambition_platformer2d_shared_tangle::markers::PlayerEntity));
    let driven = app
        .world_mut()
        .spawn((
            vessel,
            ambition_characters::control::DrivingParticipant(
                ambition_characters::control::PlayerSlot::PRIMARY,
            ),
        ))
        .id();
    app.insert_resource(
        ambition_platformer2d_shared_tangle::markers::ControlledSubject(Some(driven)),
    );
}

fn abilities_with(wall_climb: bool) -> BodyAbilities {
    let mut set = AbilitySet::default();
    set.wall_climb = wall_climb;
    BodyAbilities::new(set)
}

fn kinematics_of_height(height: f32) -> BodyKinematics {
    let mut kinematics = BodyKinematics::default();
    kinematics.size.y = height;
    kinematics
}

/// ⛔ THE HOME AVATAR'S CLIMBING DOES NOT OPEN A ROUTE FOR A VESSEL THAT CANNOT.
#[test]
fn a_resting_home_avatar_does_not_answer_body_can_for_the_possessed_vessel() {
    let mut app = App::new();
    possession(&mut app, abilities_with(true), abilities_with(false));
    assert!(
        matches!(ask(app.world(), "wall_climb"), ConditionOutcome::NotSatisfied(_)),
        "the participant is driving a vessel that cannot climb; the body it left \
         behind is not the one meeting the route"
    );
}

/// And the other direction, so the guard cannot pass by refusing everything.
#[test]
fn a_possessed_vessel_answers_body_can_even_when_the_home_avatar_cannot() {
    let mut app = App::new();
    possession(&mut app, abilities_with(false), abilities_with(true));
    assert_eq!(ask(app.world(), "wall_climb"), ConditionOutcome::Satisfied);
}

/// ⛔ THE SAME FOR SIZE: a 30-unit home must not open a crawlspace for an
/// 80-unit vessel.
#[test]
fn a_resting_home_avatar_does_not_answer_body_fits_for_the_possessed_vessel() {
    let mut app = App::new();
    possession(&mut app, kinematics_of_height(30.0), kinematics_of_height(80.0));
    assert!(
        matches!(fits_in(app.world(), 40.0), ConditionOutcome::NotSatisfied(_)),
        "the vessel is 80 units tall and the opening is 40"
    );
}

#[test]
fn a_possessed_vessel_answers_body_fits_even_when_the_home_avatar_does_not() {
    let mut app = App::new();
    possession(&mut app, kinematics_of_height(80.0), kinematics_of_height(30.0));
    assert_eq!(fits_in(app.world(), 40.0), ConditionOutcome::Satisfied);
}
