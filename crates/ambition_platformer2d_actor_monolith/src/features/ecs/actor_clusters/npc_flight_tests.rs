//! Does an NPC body fly? The CHARACTER answers, then the catalog.
//!
//! two spawn paths decided aerial-ness and neither asked the character: the
//! peaceful-NPC seed read the catalog's `body_kind: Floating`, the hostile
//! `EnemySpawn` path read `ArchetypeSpec::flies`. The doc on that field names the
//! split, and the Perfect Cellular Automaton is the live disagreement —
//! `Floating` in its catalog row, played grounded by the shipped duel.
//!
//! `CharacterLocomotion:baseline_free_flight` is `Option<bool>` precisely so a character can
//! say NO out loud, which a body kind cannot express — and this file is that ruling reaching
//! the NPC road.

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
    cast_saying_with(flight, 120.0, None)
}

/// The same, with the two facts the seed's tuning now asks the character for.
fn cast_saying_with(
    flight: Option<bool>,
    run_speed: f32,
    max_health: Option<i32>,
) -> crate::character_runtime::PreparedCharacterRegistry {
    let mut registry = crate::character_runtime::PreparedCharacterRegistry::default();
    let mut definition =
        crate::character_runtime::CharacterDefinition::new("npc_test_flyer", "Test Flyer", "test")
            .with_locomotion(CharacterLocomotion {
                run_speed,
                baseline_free_flight: flight,
                ..Default::default()
            });
    definition.vitals.max_health = max_health;
    let finalized = crate::character_runtime::prepare_and_finalize_for_test(
        definition,
        &crate::character_runtime::CharacterBindings::default(),
    );
    registry.insert_prepared(finalized.prepared);
    registry
}

/// The seed for a placement naming `npc_test_flyer`, or naming nothing.
fn seed_for(
    prepared: Option<&crate::character_runtime::PreparedCharacterRegistry>,
    character_id: Option<&str>,
) -> ActorClusterSeed {
    let interactable = npc_at(character_id);
    let aabb = ae::Aabb::new(ae::Vec2::new(100.0, 100.0), ae::Vec2::new(16.0, 24.0));
    let (seed, _render) = ActorClusterSeed::new_peaceful_npc_in(
        &Default::default(),
        &CharacterCatalog::empty(),
        prepared,
        "flyer",
        "Flyer",
        aabb,
        &interactable,
        &[],
    );
    seed
}

// Both halves are unaskable: `ActorClusterSeed` has no `spec` field and no constructor that
// fills one, because there is no `ArchetypeSpec`. Every body is built from a character, which
// is the state this test was watching the migration approach.

/// An NPC that names a migrated character gets ITS vitals and ITS top speed,
/// and the pool matches the maximum.
///
/// and the POOL was a second literal `1`, written independently of the
/// tuning's. The two agreed by coincidence; teaching only the tuning to ask the
/// character would have left a body claiming a maximum of nine and holding one.
#[test]
fn a_named_character_supplies_the_npc_body_it_authored() {
    let seed = seed_for(
        Some(&cast_saying_with(None, 225.0, Some(9))),
        Some("npc_test_flyer"),
    );
    assert_eq!(
        seed.health.max(),
        9,
        "the character's vitals, not the road's 1"
    );
    let tuning = &seed.config.tuning;
    assert_eq!(
        seed.health.health.max, 9,
        "and the POOL is the same number — it was a second literal `1` written \
         independently of the tuning's, and they agreed by coincidence"
    );
    assert_eq!(
        tuning.max_run_speed, 225.0,
        "and its locomotion, not the shared player top speed"
    );

    // AI POLICY IS NOT THE BODY'S TO STATE — and this assertion had to be
    // rewritten when the road changed under it, which is the interesting part.
    //
    //  the invariant, stated so it cannot be satisfied by a coincidence: the
    // amble is the PROFILE's fraction of the body's top speed, and it is strictly
    // slower than the body can move.
    let effort = ambition_combat::actor_tuning::BrainProfile::default().patrol_effort;
    assert_eq!(
        tuning.patrol_speed,
        225.0 * effort,
        "patrol speed is the controller's EFFORT against the body's top speed, \
         not a number either one states alone"
    );
    assert!(
        tuning.patrol_speed < tuning.max_run_speed,
        "and it must still be an amble: a character that authors a fast body \
         does not thereby decide to stroll at a sprint"
    );
    assert_eq!(
        tuning.respawn,
        ambition_entity_catalog::placements::RespawnPolicy::DeadStaysDead,
        "and respawn is PLACEMENT policy — an NPC is a unique named placement \
         (ADR 0022) however its character is authored"
    );
}

/// A character that authors nothing leaves the road's defaults exactly where
/// they were — the poison for the test above.
#[test]
fn an_incomplete_character_uses_peaceful_npc_defaults() {
    // No locomotion at all  not body-complete  the blueprint refuses, which is
    // the state ~150 NPC placements are in.
    let bare = {
        let mut registry = crate::character_runtime::PreparedCharacterRegistry::default();
        let finalized = crate::character_runtime::prepare_and_finalize_for_test(
            crate::character_runtime::CharacterDefinition::new(
                "npc_test_flyer",
                "Test Flyer",
                "test",
            ),
            &crate::character_runtime::CharacterBindings::default(),
        );
        registry.insert_prepared(finalized.prepared);
        registry
    };
    use ambition_characters::actor::DEFAULT_UNAUTHORED_BODY_HEALTH;
    let bare_seed = seed_for(Some(&bare), Some("npc_test_flyer"));
    assert_eq!(bare_seed.health.max(), DEFAULT_UNAUTHORED_BODY_HEALTH);
    let tuning = bare_seed.config.tuning;
    assert_eq!(
        tuning.max_run_speed,
        ambition_platformer2d_core::MAX_RUN_SPEED
    );

    let none_seed = seed_for(None, None);
    assert_eq!(none_seed.health.max(), DEFAULT_UNAUTHORED_BODY_HEALTH);
    let none = none_seed.config.tuning;
    assert_eq!(
        none.max_run_speed,
        ambition_platformer2d_core::MAX_RUN_SPEED
    );
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
        prepared,
        "flyer",
        "Flyer",
        aabb,
        &interactable,
        &[],
    );
    seed.config.tuning.is_aerial
}

/// A character that says it flies, flies — and one that says it does NOT stays
/// on the ground even though nothing else changed.
///
/// the second half is the whole point and it is the half a `body_kind` could
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

/// The catalog rule is a fallback for characters NOBODY REGISTERED, not a
/// second opinion on registered ones.
///
/// this is the poison for the pair above, and writing it corrected the pair's
/// own premise. The ~150 unmigrated NPC placements name characters with no
/// prepared entry AT ALL — that, not "a prepared character that stayed silent",
/// is the state the catalog still answers for. If this lookup had instead
/// answered `false` for every unregistered character, all of them would have been
/// re-decided by a value nobody authored, and against an empty catalog the result
/// would look identical to correct.
#[test]
fn preparation_resolves_silence_and_only_an_unprepared_character_reaches_the_catalog() {
    // So "a silent prepared character" is not a state that exists, and a test named for it
    // would be describing a branch it never took.
    let cast = cast_saying(None);
    assert_eq!(
        cast.get("npc_test_flyer")
            .expect("the fixture registered it")
            .locomotion
            .expect("it authored locomotion")
            .baseline_free_flight,
        Some(false),
        "preparation must settle this, or the catalog rule below is not a \
         fallback for the unmigrated but a second opinion on the migrated"
    );

    assert!(
        !is_aerial(Some(&cast), Some("npc_test_flyer")),
        "and it lands grounded, from its own resolved answer rather than the \
         catalog"
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
