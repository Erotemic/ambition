//! One registration, one prepared authority (§7.6).
//!
//! what is left here drives an `App` through `try_register_character` and
//! `finalize` — it tests COMPOSITION, which is what this crate owns. The tests
//! that ask what a definition PREPARES to followed preparation down to
//! `ambition_characters::prepared`, so preparation's own tests
//! no longer sit one crate above preparation.

use super::*;
use crate::character_runtime::CharacterLoadDemand;
use ambition_characters::binding_namespaces::MoveId;
use ambition_characters::binding_namespaces::PortraitTarget;
use ambition_characters::binding_namespaces::SfxCueId;
use ambition_characters::prepared::CharacterBindings;
use ambition_characters::prepared::CharacterRegistrationError;
use ambition_characters::prepared::PreparedCharacterRegistry;
use ambition_characters::prepared_fixtures::{mary_o, moveset_with, slash};
use ambition_platformer2d_shared_tangle::app_finalization::finalize;
use ambition_platformer2d_shared_tangle::binding::Namespace;
use bevy::prelude::App;

/// Registration DECLARES. It does not load.
///
/// Loading is driven by what a session STAGES (`StagesCharacters` — a room plan,
/// a match roster, a startup spec, a worn identity), never by what exists.
#[test]
fn registration_declares_without_demanding_art() {
    let mut app = App::new();
    app.register_character(mary_o());

    finalize(&mut app);
    let registry = app.world().resource::<PreparedCharacterRegistry>();
    assert_eq!(registry.ids().collect::<Vec<_>>(), vec!["mary_o"]);
    let prepared = registry.get("mary_o").expect("published");
    assert_eq!(prepared.display_name, "Mary-O");
    assert_eq!(prepared.art_load_token(), "mary_o");

    assert!(
        app.world()
            .get_resource::<CharacterLoadDemand>()
            .is_none_or(CharacterLoadDemand::is_empty),
        "registering a character must not demand its art: a registry of what \
         EXISTS is not a list of what a session needs decoded now"
    );
}

/// Preparation provenance survives registration. (A5)
///
/// Those must never read the same; that confusion is the whole reason the binding boundary exists,
/// and a distinction that survives only until the value is stored is not a distinction.
#[test]
fn preparation_provenance_survives_registration() {
    // Registered with NO cue resolver: moves are checked, cues are not.
    let mut app = App::new();
    app.register_character(mary_o());
    finalize(&mut app);
    let unchecked = app
        .world()
        .resource::<PreparedCharacterRegistry>()
        .get("mary_o")
        .expect("published")
        .clone();
    assert!(
        unchecked.was_checked(MoveId::NAME),
        "verb targets are always resolvable from the character's own moves"
    );
    assert!(
        !unchecked.was_checked(SfxCueId::NAME),
        "no cue resolver was supplied, so cues are NOT CHECKED — which must not \
         read as 'checked and fine'"
    );

    // Registered WITH one: now the cue namespace is genuinely verified.
    let mut app = App::new();
    app.try_register_character(
        mary_o(),
        CharacterBindings::default().with_authorized_cues(["swing", "hit_flesh"]),
    )
    .expect("registers");
    finalize(&mut app);
    let checked = app
        .world()
        .resource::<PreparedCharacterRegistry>()
        .get("mary_o")
        .expect("published")
        .clone();
    assert!(
        checked.was_checked(SfxCueId::NAME),
        "a supplied resolver means the cues WERE checked, and the published value \
         must say so"
    );
}

/// The cast's cue inventory is the union over prepared characters — and §4.6 is
/// explicit that this is the cast's CONTRIBUTION, not a session's whole set.
#[test]
fn the_cast_cue_inventory_is_the_union_over_prepared_characters() {
    let mut app = App::new();
    app.register_character(mary_o());
    app.register_character(
        CharacterDefinition::new("sanic", "Sanic", "sanic_demo").with_moveset(moveset_with(
            &[("attack", "roll")],
            vec![slash("roll", "sanic.roll", "sanic.roll.hit")],
        )),
    );

    finalize(&mut app);
    let registry = app.world().resource::<PreparedCharacterRegistry>();
    assert_eq!(
        registry
            .cast_cue_dependencies()
            .into_iter()
            .collect::<Vec<_>>(),
        vec![
            "mary_o.stomp",
            "mary_o.stomp.land",
            "sanic.roll",
            "sanic.roll.hit"
        ]
    );
}

/// A stable id is what saves, replays, and peers key on, so two providers
/// claiming one is a rename, not a merge — and the loser leaves the authority
/// untouched.
#[test]
fn two_providers_cannot_author_the_same_stable_id() {
    let mut app = App::new();
    app.register_character(mary_o());
    let error = app
        .try_register_character(
            CharacterDefinition::new("mary_o", "Impostor", "other_provider"),
            CharacterBindings::default(),
        )
        .err()
        .expect("a duplicate stable id must be refused");
    assert_eq!(
        error,
        CharacterRegistrationError::DuplicateId {
            character_id: "mary_o".to_string(),
            first_provider: "mary_o_demo".to_string(),
            second_provider: "other_provider".to_string(),
        }
    );
    finalize(&mut app);
    assert_eq!(
        app.world()
            .resource::<PreparedCharacterRegistry>()
            .get("mary_o")
            .map(|c| c.display_name.as_str()),
        Some("Mary-O"),
        "the rejected registration must leave the previous authority active"
    );
}

/// The mechanism was there — `with_available_portraits` populates the resolver
/// and `checked()` reports it — and nothing ever called it, so `self.portraits`
/// was `None` everywhere and preparation's honest *"we did not look"* was the
/// permanent answer.
///
/// this asserts the SEAM, not the resolver. It goes through
/// `try_register_character`, which is where `with_engine_vocabularies` is
/// applied — a test that passed the resolver by hand would prove the check works
/// while leaving it disconnected, which is exactly the state this replaces.
#[test]
fn a_portrait_target_nobody_authored_is_named_at_registration() {
    let mut definition = mary_o();
    definition.portrait = Some("no_such_portrait_target".to_string());

    let mut app = App::new();
    app.register_character(definition);
    finalize(&mut app);

    let prepared = app
        .world()
        .resource::<PreparedCharacterRegistry>()
        .get("mary_o")
        .expect("published");
    assert!(
        prepared.was_checked(PortraitTarget::NAME),
        "the registration seam did not supply a portrait vocabulary, so nothing \
         looked — which is the D106 state, not a pass"
    );

    // The vfx twin below has always asserted both halves.
    let unresolved: Vec<&str> = prepared.unresolved_references().collect();
    assert!(
        unresolved
            .iter()
            .any(|line| line.contains("no_such_portrait_target")),
        "a portrait target nobody authored must be REPORTED, naming the bad id \
         so the author can act on it. Got: {unresolved:#?}"
    );
    assert!(
        unresolved.iter().any(|line| line.contains("did you mean")),
        "and the report must carry a suggestion — the vocabulary is a fixed list \
         of a few dozen targets, so a near-miss is the ordinary case and the \
         nearest one is the useful half of the diagnostic. Got: {unresolved:#?}"
    );
}

/// THE POISON for the portrait check: a target that DOES exist resolves
/// clean.
///
/// without this, the test above passes on a vocabulary that rejects
/// everything — including a build where `available_portrait_targets()` came back
/// empty and every shipped character was suddenly "unresolved". An absence
/// assertion needs the presence case beside it or it is measuring the resolver
/// being broken.
#[test]
fn a_portrait_target_the_engine_bakes_resolves_clean() {
    let target = ambition_sprite_sheet::portrait::available_portrait_targets()
        .first()
        .copied()
        .expect(
            "the engine bakes portrait targets at build time; an empty vocabulary \
             is itself the failure this test exists to distinguish from a typo",
        )
        .to_string();

    let mut definition = mary_o();
    definition.portrait = Some(target.clone());

    let mut app = App::new();
    app.register_character(definition);
    finalize(&mut app);

    let prepared = app
        .world()
        .resource::<PreparedCharacterRegistry>()
        .get("mary_o")
        .expect("published");
    assert!(prepared.was_checked(PortraitTarget::NAME));
    let unresolved: Vec<&str> = prepared.unresolved_references().collect();
    assert!(
        !unresolved.iter().any(|line| line.contains(&target)),
        "`{target}` is a target the engine itself bakes, and preparation called \
         it unknown — so the vocabulary the registration seam supplies is not the \
         one the renderer draws from. Got: {unresolved:#?}"
    );
}

/// A display name is an addressing key whether or not it was meant to be one.
///
/// Rooms author `enemy.name`, interactables author `character_id`, rosters author
/// labels — and three authorities resolve those labels independently:
/// `PreparedCharacterRegistry::id_for_display_name` takes the first match in id
/// order, the catalog takes the first match in ITS order, and
/// `CharacterSpriteAssets::declare` inserts into a map so the LAST declaration wins.
/// With two "Hero"s, a demand for "Hero" could stage `alpha`, authorize `alpha`'s
/// provider, and decode `zeta`'s sheet: one character's sounds on another's body.
///
/// So the ambiguity is refused at the seam, rather than each resolver being taught
/// to break the tie the same way — which is the arrangement that produced the split
/// in the first place.
#[test]
fn two_characters_cannot_present_under_the_same_display_name() {
    let mut app = App::new();
    app.register_character(CharacterDefinition::new("alpha", "Hero", "provider_a"));
    let error = app
        .try_register_character(
            CharacterDefinition::new("zeta", "Hero", "provider_b"),
            CharacterBindings::default(),
        )
        .err()
        .expect("an ambiguous display name must be refused");
    assert_eq!(
        error,
        CharacterRegistrationError::AmbiguousDisplayName {
            display_name: "Hero".to_string(),
            first_id: "alpha".to_string(),
            second_id: "zeta".to_string(),
        }
    );

    finalize(&mut app);
    let registry = app.world().resource::<PreparedCharacterRegistry>();
    assert_eq!(
        registry.id_for_display_name("Hero"),
        Some("alpha"),
        "the rejected registration leaves the first character addressable"
    );
    assert!(
        registry.get("zeta").is_none(),
        "and does not publish the second"
    );
}

/// Re-registering the SAME character is a duplicate id, not an ambiguous name.
///
/// Both checks look at the display name, and ordering them wrongly would report
/// `alpha` as ambiguous with itself — a confusing message for the ordinary mistake
/// of registering one character twice.
#[test]
fn re_registering_one_character_still_reports_the_duplicate_id() {
    let mut app = App::new();
    app.register_character(CharacterDefinition::new("alpha", "Hero", "provider_a"));
    let error = app
        .try_register_character(
            CharacterDefinition::new("alpha", "Hero", "provider_b"),
            CharacterBindings::default(),
        )
        .err()
        .expect("a duplicate id must be refused");
    assert!(matches!(
        error,
        CharacterRegistrationError::DuplicateId { .. }
    ));
}
