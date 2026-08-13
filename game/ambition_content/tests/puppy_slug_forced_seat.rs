//! **FORCE A PUPPY SLUG INTO SMASH AND YOU GET A PUPPY SLUG** — the real
//! creature, not a fixture shaped like the answer.
//!
//! Jon's compositional acceptance test, verbatim: *"Force a Puppy Slug into
//! Smash … movement input → uses Puppy Slug's actual authored locomotion. Jump →
//! no jump if its body cannot jump. Smash must not silently give it a generic
//! swipe, a generic humanoid jump, a generic dash."*
//!
//! ⛔ **the seam version of this exists and is not this test.**
//! `a_crawler_seated_as_a_fighter_keeps_its_own_locomotion` (in the monolith)
//! registers a definition it names "crawler" and authors 36px/s and Slither on.
//! It proves the SEAM carries authored locomotion; it cannot prove that the
//! creature in the shipped game authors any, because the fixture and the
//! assertion were written together.
//!
//! ⚠ campaign P3.27 recorded this as blocked on "puppy_slug's definition
//! authoring its locomotion (it is still an archetype row)". Re-measured
//! 2026-08-12, both halves were stale: `character_archetypes.ron` holds
//! `combatant` and `medium_striker` and nothing else, and
//! `authored/npc_puppy_slug.rs` states run speed, gait, surface cling and
//! contact damage.

use bevy::prelude::*;

use ambition_platformer2d_actor_monolith::character_runtime::{
    activate_the_prepared_match, prepare_the_match, release_the_opening_hold, ControllerBinding,
    MatchParticipant, MatchParticipantRoster, MatchSeat, PreparedCharacterRegistry,
};
use ambition_platformer2d_actor_monolith::features::ActorConfig;

/// The monolith's own seating fixture, with ONE thing changed: the cast comes
/// from `ambition_content`'s shipped registration seam.
fn seating_app_with_the_real_cast() -> App {
    let mut app = App::new();
    app.init_resource::<PreparedCharacterRegistry>();
    // ⛔⛔ **THIS USED TO INSERT `CharacterCatalog::empty()`, AND THAT IS A
    // COMPOSITION PRODUCTION NEVER MAKES.** The fixture registered Ambition's
    // shipped cast — `goblin` among it, which names the shared autonomous
    // profile `medium_striker` — into a world holding no catalog and therefore
    // no `BrainProfileRegistry` to resolve it in. Preparation used to warn and
    // hand the character back with no policy; it is a composition error now
    // (GPT 5.6 review, priority 6), and it caught this fixture rather than any
    // production road. Registering the real catalog is what the shipped app
    // does, and it publishes the policy authority with it.
    ambition_content::character_catalog::register(&mut app);
    app.init_resource::<ambition_sprite_sheet::character::sheets::AuthoredSheets>();
    app.init_resource::<ambition_platformer2d_actor_monolith::features::CharacterRoster>();

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
    ambition_content::player_robot_lineage::register_declared_cast(&mut app);
    app
}

fn cpu(character: &str) -> MatchParticipant {
    MatchParticipant::new(character).driven_by(ControllerBinding::Cpu {
        // ⛔⛔ **THIS SAID `combatant` UNTIL 2026-08-13, AND THAT NAMED AN ENEMY
        // ARCHETYPE ROW.** It resolved only because `seat_brain_profile` had an
        // archetype arm — the second controller-policy authority campaign P2.18
        // deleted — so this fixture was seating through a road production no
        // longer has. `medium_striker` is Ambition's own PUBLISHED policy, from
        // the catalog this fixture registers, which is what a shipped CPU seat
        // names.
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

fn seat_the_cast(participants: Vec<MatchParticipant>) -> Vec<Seat> {
    let mut app = seating_app_with_the_real_cast();
    app.insert_resource(MatchParticipantRoster {
        participants,
        ..Default::default()
    });
    ambition_platformer2d_shared_tangle::app_finalization::finalize(&mut app);
    app.update();

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

/// **THE CREATURE THE GAME SHIPS KEEPS ITS OWN BODY IN A FIGHTER SEAT.**
///
/// ⭐ the control is the point of the pair: `npc_carl_stargan` is registered and
/// authors no body at all, so he receives whatever the stage gives an unmigrated
/// fighter. If the slug and Stargan came out identical, the seat would be
/// ignoring authoring and this test would be measuring the stage's defaults
/// twice.
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

    // ⭐ THE CONTROL. Without this the assertions above pass on a stage that
    // gives every fighter 80.0 and a cling.
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

/// **JUMP → NO JUMP, BECAUSE ITS BODY CANNOT JUMP.**
///
/// The second half of Jon's criterion — *"Jump → no jump if its body cannot
/// jump. Smash must not silently give it a generic swipe, a generic humanoid
/// jump, a generic dash"* — and it FAILED when this file was written. Measured
/// then, through the seam above:
///
/// ```text
///   npc_carl_stargan  jump=true double_jump=true attack=true  (authors nothing)
///   npc_puppy_slug    jump=true double_jump=true attack=true  (authors a body)
/// ```
///
/// Identical, because the slug authored locomotion and contact damage but no
/// `AbilitySet`, and a seat INTERSECTS the stage's fighter mask against the
/// character's — against nothing, the stage's wins whole. A slithering
/// wall-crawler double-jumped on the Smash stage.
///
/// ⭐ Jon, 2026-08-12: *"If the slug does not have a double jump ability it
/// should not be able to double jump. The point of a slug is that it shows that
/// it is spawned happily even though it basically has no moves."* So the slug
/// authors `move_horizontal` and nothing else, and the intersection — which
/// already refused to GRANT a verb a character lacks — now has something to
/// intersect.
///
/// ⚠ **the control is doing double duty.** Stargan still comes out with the
/// stage's humanoid mask, which is correct: he authors no body, and an
/// unmigrated fighter must still be given something or every one of them is a
/// statue. The claim is not "no fighter gets defaults", it is "a character that
/// SAYS what it can do is believed".
/// ⭐ **THE OTHER HALF OF THIS CLAIM IS IN THE ENGINE, and it was missing until
/// 2026-08-12.** A mask saying `jump: false` is only worth asserting if the
/// engine HONOURS it, and the base `jump` flag was the one ability gate nothing
/// pinned — `double_jump`, `double_dash` and `wall_climb` all had tests; the
/// plainest capability in the set did not, and its gate is a single `&&` in
/// `apply_intent`. `movement::tests::ability_gates::jump_ability_controls_the_ground_jump`
/// is that half: press Jump on a grounded body with the flag off and neither the
/// op nor the rise happens, with the same fixture jumping when the flag is on.
/// ⚠ neither test is the claim alone. This one says the shipped slug asks for no
/// jump; that one says asking is what decides.
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

    // ⭐ THE CONTROL, and the reason the four assertions above are about
    // authoring rather than about the stage being empty.
    assert!(
        stargan.abilities.jump && stargan.abilities.double_jump,
        "the character that authors NO mask stopped receiving the stage's — an \
         unmigrated fighter that cannot jump is a statue, and that is a \
         different bug wearing this fix"
    );
}

/// **AND IT IS SPAWNED HAPPILY WITH ALMOST NO MOVES** — Jon's actual point.
///
/// *"The point of a slug is that it shows that it is spawned happily even
/// though it basically has no moves."* The acceptance criterion is not that it
/// fights well; it is that a body carrying ONE verb seats, simulates and
/// survives on a stage built for fighters, rather than crashing, freezing, or
/// being quietly topped up into a humanoid. That is the compositional claim the
/// whole character-template campaign is for, and a creature with one verb is the
/// sharpest instrument for it.
#[test]
fn a_creature_with_one_verb_still_seats_and_simulates() {
    let mut app = seating_app_with_the_real_cast();
    app.insert_resource(MatchParticipantRoster {
        participants: vec![cpu("npc_puppy_slug"), cpu("npc_carl_stargan")],
        ..Default::default()
    });
    ambition_platformer2d_shared_tangle::app_finalization::finalize(&mut app);
    // Many ticks, not one: a body that seats and then divides by zero on its
    // first brain tick would pass a single-update assertion.
    for _ in 0..120 {
        app.update();
    }

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
