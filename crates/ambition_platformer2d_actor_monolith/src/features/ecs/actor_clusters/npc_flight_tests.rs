//! **Does an NPC body fly? The CHARACTER answers, then the catalog.**
//!
//! ⛔ two spawn paths decided aerial-ness and neither asked the character: the
//! peaceful-NPC seed read the catalog's `body_kind: Floating`, the hostile
//! `EnemySpawn` path read `ArchetypeSpec::flies`. The doc on that field names the
//! split, and the Perfect Cellular Automaton is the live disagreement —
//! `Floating` in its catalog row, played grounded by the shipped duel.
//!
//! D89's ruling is that `body_kind` describes a SHAPE and stopped deciding
//! whether a body flies. `CharacterLocomotion::baseline_free_flight` is
//! `Option<bool>` precisely so a character can say NO out loud, which a body kind
//! cannot express — and this file is that ruling reaching the NPC road.

use super::*;
use ambition_characters::actor::CharacterLocomotion;

fn npc_at(character_id: Option<&str>) -> ambition_interaction::Interactable {
    let aabb = ae::Aabb::new(ae::Vec2::new(100.0, 100.0), ae::Vec2::new(16.0, 24.0));
    ambition_interaction::Interactable::new(
        "flyer",
        "Talk",
        aabb,
        ambition_interaction::InteractionKind::Npc {
            character_id: character_id.map(str::to_string),
            dialogue_id: None,
            patrol_radius: 0.0,
            patrol_path_id: None,
            brain_override: None,
        },
    )
}

/// A registry holding one character whose locomotion says what it is given.
fn cast_saying(flight: Option<bool>) -> crate::character_runtime::PreparedCharacterRegistry {
    let mut registry = crate::character_runtime::PreparedCharacterRegistry::default();
    let definition =
        crate::character_runtime::CharacterDefinition::new("npc_test_flyer", "Test Flyer", "test")
            .with_locomotion(CharacterLocomotion {
                run_speed: 120.0,
                baseline_free_flight: flight,
                ..Default::default()
            });
    let finalized = crate::character_runtime::prepare_and_finalize_for_test(
        definition,
        &crate::character_runtime::CharacterBindings::default(),
    );
    registry.insert_prepared(finalized.prepared);
    registry
}

fn is_aerial(
    prepared: Option<&crate::character_runtime::PreparedCharacterRegistry>,
    character_id: Option<&str>,
) -> bool {
    let interactable = npc_at(character_id);
    let aabb = ae::Aabb::new(ae::Vec2::new(100.0, 100.0), ae::Vec2::new(16.0, 24.0));
    let (seed, _render) = ActorClusterSeed::new_peaceful_npc_in(
        &Default::default(),
        &CharacterCatalog::empty(),
        &crate::features::enemies::test_roster(),
        prepared,
        "flyer",
        "Flyer",
        aabb,
        &interactable,
        &[],
    );
    seed.config.tuning.is_aerial
}

/// **A character that says it flies, flies — and one that says it does NOT stays
/// on the ground even though nothing else changed.**
///
/// ⭐ the second half is the whole point and it is the half a `body_kind` could
/// never state. An empty catalog means the old rule answers "not floating" for
/// both, so the only thing separating these two runs is what the character said.
#[test]
fn the_character_decides_whether_an_npc_body_flies() {
    assert!(
        is_aerial(Some(&cast_saying(Some(true))), Some("npc_test_flyer")),
        "a character whose locomotion authors free flight must spawn aerial"
    );
    assert!(
        !is_aerial(Some(&cast_saying(Some(false))), Some("npc_test_flyer")),
        "and one that authors NO must not — `Option<bool>` exists so a character \
         can refuse flight out loud, which is what `body_kind` cannot say"
    );
}

/// **Silence is not a refusal: an unmigrated character leaves the catalog rule
/// exactly where it was.**
///
/// ⛔ this is the poison for the pair above. If the new lookup answered `false`
/// for a character that said nothing, every one of the ~150 unmigrated NPC
/// placements would have been re-decided by a value nobody authored — and with an
/// empty catalog the result would look identical to correct.
#[test]
fn a_silent_character_does_not_decide_anything() {
    assert!(
        !is_aerial(Some(&cast_saying(None)), Some("npc_test_flyer")),
        "a silent character falls through to the catalog, which here says nothing"
    );
    assert!(
        !is_aerial(None, Some("npc_test_flyer")),
        "and a composition with no cast at all is unchanged by this rule"
    );
    assert!(
        !is_aerial(None, None),
        "as is a placement that names no character"
    );
}
