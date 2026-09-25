//! A body's physical facts (health, knockback weight, death traits) come from
//! its prepared character on every constructor road, and do not depend on
//! whether the character authored locomotion.
//!
//! The seed used to read the pool through the body blueprint, which exists only
//! for a character with locomotion. A character with vitals and no locomotion
//! (the Hall's `sanic`, one hit point) was built at the unauthored default and
//! handed to the persona derive to correct on a later tick; the construction
//! wear now stamps every prepared character as worn, so nothing corrects it.

use super::*;

fn npc_wearing(character_id: &str) -> ambition_interaction::Interactable {
    let aabb = ae::Aabb::new(ae::Vec2::new(100.0, 100.0), ae::Vec2::new(16.0, 24.0));
    ambition_interaction::Interactable::new(
        "vitals",
        "Talk",
        aabb,
        ambition_interaction::InteractionKind::Npc {
            character_id: Some(character_id.to_string()),
            dialogue_id: None,
            patrol_radius: 0.0,
            patrol_path_id: None,
            brain_override: None,
        },
    )
}

#[test]
fn a_character_without_locomotion_is_built_with_its_authored_vitals() {
    let mut definition = ambition_characters::actor::definition::CharacterDefinition::new(
        "npc_test_fragile",
        "Test Fragile",
        "test",
    )
    .with_death_traits(ambition_characters::actor::CharacterDeathTraits {
        explodes_on_death: true,
        ..Default::default()
    });
    definition.vitals.max_health = Some(1);
    definition.vitals.knockback_weight = Some(1.35);
    let finalized = ambition_characters::prepared::prepare_and_finalize_for_test(
        definition,
        &ambition_characters::prepared::CharacterBindings::default(),
    );
    let mut cast = ambition_characters::prepared::PreparedCharacterRegistry::default();
    cast.insert_prepared(finalized.prepared);
    assert!(
        cast.get("npc_test_fragile")
            .is_some_and(|prepared| prepared.body_blueprint().is_err()),
        "the fixture must be a character with no body blueprint, or the \
         assertions below are about the blueprint road"
    );

    let aabb = ae::Aabb::new(ae::Vec2::new(100.0, 100.0), ae::Vec2::new(16.0, 24.0));
    let (seed, _render) = ActorClusterSeed::new_peaceful_npc_in(
        &Default::default(),
        &CharacterCatalog::empty(),
        Some(&cast),
        "fragile",
        "Fragile",
        aabb,
        &npc_wearing("npc_test_fragile"),
        &[],
    );
    assert_eq!(
        (seed.health.health.current, seed.health.health.max),
        (1, 1),
        "the pool is the character's authored one, not the unauthored default"
    );
    assert_eq!(seed.config.tuning.weight, 1.35, "the authored knockback weight");
    assert_eq!(
        seed.caps,
        ambition_combat::CombatCapabilities::from(&ambition_characters::actor::CharacterDeathTraits {
            explodes_on_death: true,
            ..Default::default()
        }),
        "the authored death traits"
    );
}

/// The blueprint road builds the same physical facts: `new_character_in` is
/// the constructor every character road wears through, and the wear stamps the
/// persona current, so a fact it drops is never supplied. The Hall's exploding
/// and dividing mites author locomotion, took this road, and were built with
/// default capabilities: provoked and killed, neither exploded nor split.
#[test]
fn a_blueprint_body_is_built_with_its_authored_death_traits_and_weight() {
    let traits = ambition_characters::actor::CharacterDeathTraits {
        divides_into: Some("npc_puppy_slug".to_string()),
        ..Default::default()
    };
    let mut blueprint = fixture_body_blueprint("Divider");
    blueprint.death_traits = Some(&traits);
    blueprint.knockback_weight = Some(1.35);
    let aabb = ae::Aabb::new(ae::Vec2::new(100.0, 100.0), ae::Vec2::new(16.0, 24.0));
    let seed = ActorClusterSeed::new_character_in(
        &Default::default(),
        &CharacterCatalog::empty(),
        "divider",
        blueprint,
        aabb,
        ambition_entity_catalog::placements::CharacterBrain::Passive,
        &[],
    );
    assert_eq!(
        seed.caps,
        ambition_combat::CombatCapabilities::from(&traits),
        "the authored death traits"
    );
    assert_eq!(seed.config.tuning.weight, 1.35, "the authored knockback weight");
}
