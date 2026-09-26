//! Force a Puppy Slug into Smash and you get a Puppy Slug: the real
//! creature, not a fixture shaped like the answer.
//!
//! In Smash, movement uses the Puppy Slug's authored locomotion, and it has no
//! jump if its body cannot jump. Smash must not silently give it a generic
//! swipe, humanoid jump or dash.
//!
//! The seam version is
//! `a_crawler_seated_as_a_fighter_keeps_its_own_locomotion` (in the monolith).
//! It proves the seam carries authored locomotion, but its fixture and
//! assertion were written together, so it cannot prove that the shipped
//! creature authors any.

use bevy::prelude::*;

use ambition_combat::actor_tuning::ActorConfig;
// `PreparedCharacterRegistry` belongs to `ambition_characters::prepared`.
use ambition_characters::prepared::PreparedCharacterRegistry;
use ambition_platformer2d::versus_match::{
    ControllerBinding, MatchParticipant, MatchParticipantRoster, MatchSeat,
};
use ambition_platformer2d_actor_monolith::character_runtime::{
    activate_the_prepared_match, prepare_the_match, release_the_opening_hold,
};

/// The monolith's own seating fixture, with ONE thing changed: the cast comes
/// from `ambition_content`'s shipped registration seam.
fn seating_app_with_the_real_cast() -> App {
    let mut app = App::new();
    app.init_resource::<PreparedCharacterRegistry>();
    // Registering the real cast is what the shipped app does; it also
    // publishes the policy authority.
    ambition_content::character_catalog::register_cast(&mut app);
    app.init_resource::<ambition_sprite_sheet::character::sheets::AuthoredSheets>();
    // A hand-built app installs no plugin, so it must add the ordinal itself.
    // `activate_the_prepared_match` takes `ResMut<SessionMatchOrdinal>`, not
    // `Option`, on purpose: a composition with no ordinal authority would draw
    // every match's items identically. Without it, bevy parameter validation
    // fails with "Resource does not exist" and no system name.
    app.init_resource::<ambition_platformer2d::versus_match::seating::SessionMatchOrdinal>();

    let world = ambition_platformer2d_core::World::new(
        "Arena",
        Vec2::new(960.0, 540.0),
        Vec2::new(480.0, 400.0),
        vec![ambition_platformer2d_core::Block::solid(
            "floor",
            Vec2::new(0.0, 440.0),
            Vec2::new(960.0, 100.0),
        )],
    );
    ambition_platformer2d_shared_tangle::lifecycle::insert_session_world_component(
        app.world_mut(),
        ambition_platformer2d_core::RoomGeometry(world),
    );
    app.add_systems(
        Update,
        (
            prepare_the_match,
            activate_the_prepared_match,
            release_the_opening_hold,
            ambition_platformer2d_actor_monolith::avatar::apply_worn_character_gameplay,
        )
            .chain(),
    );
    app
}

fn cpu(character: &str) -> MatchParticipant {
    MatchParticipant::new(character).driven_by(ControllerBinding::Cpu {
        // `medium_striker` is Ambition's own published policy, from the catalog this
        // fixture registers, which is what a shipped CPU seat names.
        brain_profile: Some("medium_striker".into()),
    })
}

struct Seat {
    worn: String,
    run_speed: f32,
    surface_walker: bool,
    contact_damage: i32,
    abilities: ambition_platformer2d_core::AbilitySet,
}

/// Say why a roster seated nothing, if the composition refused it.
///
/// `prepare_match` refuses a whole roster it cannot resolve: one unresolved
/// participant gives zero seats. It records each reason in
/// `MatchPreparationProblems` and writes nothing to the log.
///
/// A seat-count assertion then reports only `left: 0`, which points at
/// seating. The cause can be content instead: an admission barrier withholds a
/// character, so the same `0` also means "this composition declined this
/// cast". The two causes need different repairs.
///
/// Call this before each seat-count assertion, so a content refusal is not
/// reported as a seat count.
fn say_why_if_the_cast_was_withheld(app: &App) {
    if let Some(problems) = app
        .world()
        .get_resource::<ambition_platformer2d::versus_match::MatchPreparationProblems>()
    {
        panic!(
            "this composition REFUSED the roster, so it built no seat. This is \
             not a seating defect. The refusal says: {problems}"
        );
    }
}

fn seat_the_cast(participants: Vec<MatchParticipant>) -> Vec<Seat> {
    let mut app = seating_app_with_the_real_cast();
    app.insert_resource(MatchParticipantRoster {
        participants,
        ..Default::default()
    });
    // A seating fixture, not an admission one. This app installs no technique
    // handlers, so real admission refuses every character that names a native
    // effect. The raw road is named explicitly so it cannot be reached by
    // accident; see its doc.
    //
    // Without this call the roster seats zero, and the zero comes from the seat
    // road, not from admission. Admission withholds per definition and still
    // publishes `npc_puppy_slug`. But in `ambition_match::prepared` one
    // unresolvable participant (`npc_carl_stargan`) fails the whole preparation,
    // so both seats are lost. A roster is all-or-nothing.
    ambition_characters::prepared::close_preparation_barrier_without_admission(app.world_mut());
    ambition_platformer2d_shared_tangle::app_finalization::finalize(&mut app);
    app.update();

    say_why_if_the_cast_was_withheld(&app);

    let world = app.world_mut();
    let mut q = world.query_filtered::<(
        &ambition_characters::actor::WornCharacter,
        &ActorConfig,
        &ambition_platformer2d_core::BodyAbilities,
    ), With<MatchSeat>>();
    let mut rows: Vec<Seat> = q
        .iter(world)
        .map(|(worn, config, abilities)| Seat {
            worn: worn.id().to_string(),
            run_speed: config.tuning.max_run_speed,
            surface_walker: config.tuning.surface_walker,
            contact_damage: config.tuning.damage_amount,
            abilities: abilities.abilities,
        })
        .collect();
    rows.sort_by(|a, b| a.worn.cmp(&b.worn));
    rows
}

/// The creature the game ships keeps its own body in a fighter seat.
///
/// The control is the point: `npc_carl_stargan` is registered and authors no
/// body, so he gets the stage's defaults for an unmigrated fighter. If the slug
/// and Stargan came out identical, the seat would be ignoring authoring.
#[test]
fn the_shipped_puppy_slug_is_seated_as_itself() {
    let seats = seat_the_cast(vec![cpu("npc_puppy_slug"), cpu("npc_carl_stargan")]);
    assert_eq!(
        seats.len(),
        2,
        "the roster seated the wrong number of bodies"
    );
    let stargan = &seats[0];
    let slug = &seats[1];
    assert_eq!(slug.worn, "npc_puppy_slug");
    assert_eq!(stargan.worn, "npc_carl_stargan");

    assert_eq!(
        slug.run_speed, 80.0,
        "the shipped slug is seated at somebody else's top speed — its own \
         definition authors 80.0"
    );
    assert!(
        slug.surface_walker,
        "the slug lost its surface cling by being seated: a crawler forced into \
         Smash is still a crawler"
    );
    assert_eq!(
        slug.contact_damage, 1,
        "its authored contact damage did not survive the seat"
    );

    // The control. Without it the assertions above pass on a stage that gives
    // every fighter 80.0 and a cling.
    assert!(
        stargan.run_speed != slug.run_speed,
        "the character that authors NO body was seated identically to the one \
         that authors a whole one ({} vs {}) — the seat is not reading authoring",
        stargan.run_speed,
        slug.run_speed
    );
    assert!(
        !stargan.surface_walker,
        "a character that authored no locomotion came out clinging to walls"
    );
    assert_eq!(
        stargan.contact_damage, 0,
        "a fighter that authored no contact damage hurts on touch, which is the \
         engine inventing a capability"
    );
}

/// Jump → no jump, because its body cannot jump. Smash must not give it a
/// generic swipe, humanoid jump or dash.
///
/// ```text
///   npc_carl_stargan  jump=true double_jump=true attack=true  (authors nothing)
///   npc_puppy_slug    jump=true double_jump=true attack=true  (authors a body)
/// ```
///
/// Without authored verbs they are identical: a seat intersects the stage's
/// fighter mask with the character's, and against no `AbilitySet` the stage's
/// mask wins whole. A slithering wall-crawler would double-jump.
///
/// The slug should be spawnable even though it has almost no moves. So it
/// authors `move_horizontal` and nothing else, and the intersection (which
/// never grants a verb the character lacks) has something to intersect.
///
/// The control still gets the stage's humanoid mask, which is correct: he
/// authors no body, and an unmigrated fighter must get something. This test
/// shows that what the character asks for decides.
#[test]
fn a_body_that_cannot_jump_is_not_given_a_jump_by_the_stage() {
    let seats = seat_the_cast(vec![cpu("npc_puppy_slug"), cpu("npc_carl_stargan")]);
    let slug = seats.iter().find(|s| s.worn == "npc_puppy_slug").unwrap();
    let stargan = seats.iter().find(|s| s.worn == "npc_carl_stargan").unwrap();

    assert!(
        !slug.abilities.jump,
        "the slug was given a jump the stage invented for it"
    );
    assert!(
        !slug.abilities.double_jump,
        "the slug was given a DOUBLE jump — Jon's sentence, verbatim: if it does \
         not have the ability it should not be able to"
    );
    assert!(
        !slug.abilities.dash,
        "a generic dash, which is the third thing the acceptance test names"
    );
    assert!(
        !slug.abilities.attack,
        "a generic swipe. Its damage is CONTACT damage — it hurts you by being \
         touched, not by swinging"
    );
    assert!(
        slug.abilities.move_horizontal,
        "and it still CRAWLS: a body stripped to nothing at all would pass every \
         assertion above while being a rock, which is not what was asked for"
    );

    // The control: the assertions above are about authoring, not about an
    // empty stage.
    assert!(
        stargan.abilities.jump && stargan.abilities.double_jump,
        "the character that authors NO mask stopped receiving the stage's — an \
         unmigrated fighter that cannot jump is a statue, and that is a \
         different bug wearing this fix"
    );
}

/// A body with only one authored verb can be seated and simulated on a fighter
/// stage without being padded into a generic humanoid kit.
#[test]
fn a_creature_with_one_verb_still_seats_and_simulates() {
    let mut app = seating_app_with_the_real_cast();
    app.insert_resource(MatchParticipantRoster {
        participants: vec![cpu("npc_puppy_slug"), cpu("npc_carl_stargan")],
        ..Default::default()
    });
    // A seating fixture, not an admission one. This app installs no technique
    // handlers, so real admission refuses every character that names a native
    // effect. The raw road is named explicitly so it cannot be reached by
    // accident; see its doc.
    //
    // Without this call the roster seats zero, and the zero comes from the seat
    // road, not from admission. Admission withholds per definition and still
    // publishes `npc_puppy_slug`. But in `ambition_match::prepared` one
    // unresolvable participant (`npc_carl_stargan`) fails the whole preparation,
    // so both seats are lost. A roster is all-or-nothing.
    ambition_characters::prepared::close_preparation_barrier_without_admission(app.world_mut());
    ambition_platformer2d_shared_tangle::app_finalization::finalize(&mut app);
    // Many ticks, not one: a body that seats and then divides by zero on its
    // first brain tick would pass a single-update assertion.
    for _ in 0..120 {
        app.update();
    }
    say_why_if_the_cast_was_withheld(&app);

    let world = app.world_mut();
    let mut q = world.query_filtered::<(
        &ambition_characters::actor::WornCharacter,
        &ambition_platformer2d_shared_tangle::body::BodyKinematics,
    ), With<MatchSeat>>();
    let rows: Vec<(String, Vec2)> = q
        .iter(world)
        .map(|(worn, kin)| (worn.id().to_string(), kin.pos))
        .collect();

    assert_eq!(
        rows.len(),
        2,
        "a body with one verb did not survive 120 ticks in a fighter seat: {rows:?}"
    );
    for (worn, pos) in &rows {
        assert!(
            pos.x.is_finite() && pos.y.is_finite(),
            "{worn} left the number line: {pos:?}"
        );
    }
}
