//! Prepared casts for body integration tests.

use ambition_characters::actor::definition::CharacterDefinition;
use ambition_characters::prepared::PreparedCharacterRegistry;

/// Body-complete fixture cast for tests that need registered characters but do
/// not care which creatures they are.
#[cfg(test)]
pub(crate) fn fixture_cast(ids: &[&str]) -> PreparedCharacterRegistry {
    let mut registry = PreparedCharacterRegistry::default();
    for id in ids {
        let mut definition = CharacterDefinition::new(*id, *id, "test")
            .with_locomotion(ambition_characters::actor::CharacterLocomotion {
                run_speed: 155.0,
                move_style: ambition_characters::brain::MoveStyleSpec::Walk,
                ..Default::default()
            })
            .with_contact_damage(ambition_characters::actor::ContactDamage {
                strength: 0.70,
                amount: 1,
            })
            .with_autonomous_profile(ambition_characters::brain::BrainProfile {
                patrol_effort: 0.6774,
                chase_effort: 1.0,
                aggro_radius: 460.0,
                attack_range: 150.0,
                ..Default::default()
            });
        definition.vitals.max_health = Some(4);
        let finalized = ambition_characters::prepared::prepare_and_finalize_for_test(
            definition,
            &ambition_characters::prepared::CharacterBindings::default(),
        );
        registry.insert_prepared(finalized.prepared);
    }
    registry
}

