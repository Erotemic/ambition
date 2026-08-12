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
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
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
        brain_profile: Some("combatant".into()),
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

/// **AND HERE IS THE HALF THAT IS NOT DONE.**
///
/// Jon's criterion has a second clause — *"Jump → no jump if its body cannot
/// jump. Smash must not silently give it a generic swipe, a generic humanoid
/// jump, a generic dash"* — and the shipped slug FAILS it. Measured
/// 2026-08-12, through the seam above:
///
/// ```text
///   npc_carl_stargan  jump=true double_jump=true attack=true  (authors nothing)
///   npc_puppy_slug    jump=true double_jump=true attack=true  (authors a body)
/// ```
///
/// Identical, because the slug authors locomotion and contact damage but no
/// `AbilitySet`, and the seat's mask INTERSECTS — with nothing to intersect
/// against, the stage's fighter default wins whole. A slithering wall-crawler
/// double-jumps on the Smash stage today.
///
/// ⛔ **this test does not assert that, and must not.** Pinning the current
/// masks would make a defect the specification. What the slug can DO is a
/// content decision (D96 item 9 / campaign P3.25): its contact damage is not a
/// swipe, and choosing its verb set is authoring, not repair. This function
/// exists so the measurement travels with the test that found it — and so the
/// day somebody authors that mask, the assertion to add is already written down.
///
/// ⚠ deliberately NOT `#[ignore]`d into existence as a red test either: a
/// failing test for a decision nobody has made is noise every run pays for.
#[test]
fn the_ability_mask_is_still_the_stages_and_that_is_a_decision_not_a_bug() {
    let seats = seat_the_cast(vec![cpu("npc_puppy_slug"), cpu("npc_carl_stargan")]);
    let slug = seats.iter().find(|s| s.worn == "npc_puppy_slug").unwrap();
    // The one thing worth asserting here: the slug authors NO mask of its own,
    // which is precisely why the stage's wins. If it ever authors one, this
    // fails and whoever authored it reads the doc above.
    assert!(
        ambition_content::character_catalog::authored_intrinsics(
            "npc_puppy_slug",
            ambition_platformer2d_actor_monolith::character_runtime::CharacterDefinition::new(
                "npc_puppy_slug",
                "Puppy Slug",
                "ambition_content",
            ),
        )
        .abilities
        .is_none(),
        "the slug now authors an ability mask — the measurement in this \
         function's doc is stale, and the seat assertions it describes are the \
         ones to add"
    );
    // Named so the reader knows what was measured rather than trusting a
    // comment: today this is the stage's humanoid default, verbatim.
    assert!(
        slug.abilities.jump,
        "if this ever fails the gap closed and the doc above is the changelog"
    );
}
