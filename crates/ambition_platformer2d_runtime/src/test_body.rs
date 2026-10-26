//! Test bodies built through prepared character admission and the public body builder.

pub(crate) fn home_body(
    scratch: ambition_platformer2d_core::BodyClusterScratch,
    health: ambition_characters::actor::Health,
) -> impl bevy::prelude::Bundle {
    use ambition_characters::brain::RangedExecution;
    use ambition_characters::prepared::{
        prepare_and_finalize_for_test, CharacterBindings, PreparedCharacterRegistry,
    };
    let definition = ambition_characters::actor::definition::CharacterDefinition::new(
        "fixture_home",
        "Fixture home",
        "test",
    )
    .with_action_set(ambition_combat::worn_kit::default_player_action_set(
        scratch.abilities.abilities,
    ))
    .with_ranged_execution(RangedExecution::ChargedProjectile);
    let finalized = prepare_and_finalize_for_test(definition, &CharacterBindings::default());
    let mut cast = PreparedCharacterRegistry::default();
    cast.insert_prepared(finalized.prepared);
    let body = ambition_platformer2d_actor_monolith::avatar::PlayerSimulationBundle::from_scratch_as_character(
        ambition_characters::control::PlayerSlot::PRIMARY,
        scratch, health, "fixture_home", Some(&cast), &mut RangedExecution::MovesetVerb,
    );
    (
        body,
        ambition_platformer2d_shared_tangle::markers::PrimaryPlayer,
        ambition_platformer2d_shared_tangle::body::PrimaryBody,
    )
}
