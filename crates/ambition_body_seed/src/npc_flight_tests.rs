//! Does an NPC body fly? The CHARACTER answers, and only the character.
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
fn cast_saying(flight: Option<bool>) -> ambition_characters::prepared::PreparedCharacterRegistry {
    cast_saying_with(flight, 120.0, None)
}

/// The same, with the two facts the seed's tuning now asks the character for.
fn cast_saying_with(
    flight: Option<bool>,
    run_speed: f32,
    max_health: Option<i32>,
) -> ambition_characters::prepared::PreparedCharacterRegistry {
    let mut registry = ambition_characters::prepared::PreparedCharacterRegistry::default();
    let mut definition = ambition_characters::actor::definition::CharacterDefinition::new(
        "npc_test_flyer",
        "Test Flyer",
        "test",
    )
    .with_locomotion(CharacterLocomotion {
        run_speed,
        baseline_free_flight: flight,
        ..Default::default()
    });
    definition.vitals.max_health = max_health;
    let finalized = ambition_characters::prepared::prepare_and_finalize_for_test(
        definition,
        &ambition_characters::prepared::CharacterBindings::default(),
    );
    registry.insert_prepared(finalized.prepared);
    registry
}

/// The seed for a placement naming `npc_test_flyer`, or naming nothing.
fn seed_for(
    prepared: Option<&ambition_characters::prepared::PreparedCharacterRegistry>,
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
    let patrol_speed = seed.policy.0.patrol_speed(tuning.max_run_speed);
    assert_eq!(
        patrol_speed,
        225.0 * effort,
        "patrol speed is the controller's EFFORT against the body's top speed, \
         not a number either one states alone"
    );
    assert!(
        patrol_speed < tuning.max_run_speed,
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
        let mut registry = ambition_characters::prepared::PreparedCharacterRegistry::default();
        let finalized = ambition_characters::prepared::prepare_and_finalize_for_test(
            ambition_characters::actor::definition::CharacterDefinition::new(
                "npc_test_flyer",
                "Test Flyer",
                "test",
            ),
            &ambition_characters::prepared::CharacterBindings::default(),
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
    prepared: Option<&ambition_characters::prepared::PreparedCharacterRegistry>,
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

/// Preparation settles silence, so a silent character lands grounded from its
/// own resolved answer, and a road with no cast has nothing to ask.
#[test]
fn preparation_resolves_silence_to_grounded() {
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
        "preparation must settle this, or a constructor has to ask again"
    );

    assert!(
        !is_aerial(Some(&cast), Some("npc_test_flyer")),
        "and it lands grounded, from its own resolved answer"
    );
    assert!(
        !is_aerial(None, Some("npc_test_flyer")),
        "and a composition with no cast at all has no flyer"
    );
    assert!(
        !is_aerial(None, None),
        "as is a placement that names no character"
    );
}

/// A character that authors a sprite body but no locomotion takes the peaceful
/// road (it has no blueprint), and is still sized from its sheet: the same
/// `posed_body_geometry(Idle)` the pose pass asks, not the catalog join. The
/// Hall of Characters' `mary_o` is this case; the catalog join built her at
/// 32x48 and the pose pass stood her at 21.3x32 a tick later.
#[test]
fn a_peaceful_npc_that_authors_a_sprite_body_is_built_from_its_sheet() {
    const SHEET: &str = "robot";
    const WORLD_PER_PIXEL: f32 = 0.5;
    let mut definition = ambition_characters::actor::definition::CharacterDefinition::new(
        "npc_test_flyer",
        "Test Flyer",
        "test",
    )
    .with_sheet(SHEET);
    definition.body = Some(ambition_characters::actor::definition::BodySource::SpriteAuthored {
        world_per_pixel: WORLD_PER_PIXEL,
    });
    let finalized = ambition_characters::prepared::prepare_and_finalize_for_test(
        definition,
        &ambition_characters::prepared::CharacterBindings::default(),
    );
    let mut registry = ambition_characters::prepared::PreparedCharacterRegistry::default();
    registry.insert_prepared(finalized.prepared);
    assert!(
        registry.get("npc_test_flyer").unwrap().body_blueprint().is_err(),
        "the premise: no blueprint, so this is the peaceful road's own sizing"
    );
    let from_the_sheet = ambition_sprite_sheet::character::sheets::posed_body_geometry(
        SHEET,
        ambition_sprite_sheet::character::CharacterAnim::Idle,
        WORLD_PER_PIXEL,
    )
    .expect("the baked `robot` sheet resolves a posed body");
    let placement = ae::Vec2::new(32.0, 48.0);
    assert!(
        from_the_sheet.collision.distance(placement) > 1.0,
        "the sheet's box equals the placement's, so this cannot tell them apart"
    );

    let interactable = npc_at(Some("npc_test_flyer"));
    let (seed, render) = ActorClusterSeed::new_peaceful_npc_in(
        &Default::default(),
        &CharacterCatalog::empty(),
        Some(&registry),
        "flyer",
        "Flyer",
        ae::Aabb::new(ae::Vec2::new(100.0, 100.0), placement * 0.5),
        &interactable,
        &[],
    );
    assert_eq!(
        seed.kin.size, from_the_sheet.collision,
        "the peaceful seed was built at a box the pose pass will resize"
    );
    assert_eq!(render, Some(from_the_sheet.render), "the quad is the sheet's too");
    assert_eq!(seed.posed, Some(from_the_sheet));

    // The control: the same placement naming no character keeps its own box.
    assert_eq!(seed_for(None, None).kin.size, ae::Vec2::new(32.0, 48.0));
}

/// A PREPARED CHARACTER WITHOUT A BLUEPRINT KEEPS THE ABILITIES IT AUTHORED.
///
/// "No blueprint" means only that `locomotion` is unauthored; it does not mean
/// nobody described the body. The Hall's `mary_o`/`sanic` take this road and
/// author `RunJump`. Handing them the anonymous NPC's NONE left a possessed
/// Hall body unable to run or jump, and the per-body control gate then
/// faithfully enforced that wrong set.
#[test]
fn a_prepared_npc_without_a_blueprint_keeps_its_authored_abilities() {
    let run_jump = ae::AbilityGrant::RunJump.to_set();
    let definition = ambition_characters::actor::definition::CharacterDefinition::new(
        "npc_test_walker",
        "Test Walker",
        "test",
    )
    .with_abilities(run_jump);
    let finalized = ambition_characters::prepared::prepare_and_finalize_for_test(
        definition,
        &ambition_characters::prepared::CharacterBindings::default(),
    );
    let mut registry = ambition_characters::prepared::PreparedCharacterRegistry::default();
    registry.insert_prepared(finalized.prepared);
    assert!(
        registry.get("npc_test_walker").unwrap().body_blueprint().is_err(),
        "the premise: no locomotion, so no blueprint, so the peaceful road builds it"
    );

    let (seed, _) = ActorClusterSeed::new_peaceful_npc_in(
        &Default::default(),
        &CharacterCatalog::empty(),
        Some(&registry),
        "walker",
        "Walker",
        ae::Aabb::new(ae::Vec2::new(100.0, 100.0), ae::Vec2::new(16.0, 24.0)),
        &npc_at(Some("npc_test_walker")),
        &[],
    );
    assert_eq!(
        seed.body.0.abilities.abilities, run_jump,
        "the no-blueprint road dropped the character's authored abilities"
    );

    // The control: nobody authored the anonymous placement, so it stays NONE.
    assert_eq!(
        seed_for(None, None).body.0.abilities.abilities,
        ae::AbilitySet::NONE
    );
}

/// A catalog row's `body_kind: Floating` does not make an NPC fly.
///
/// The row is prepared like any character (AP30), with no locomotion because
/// nobody authored one. The peaceful road used to fall back to the row's body
/// kind for exactly that case, while the blueprint road read the same silence
/// as grounded: one character, two answers, chosen by which road built it.
#[test]
fn a_floating_catalog_row_is_a_silhouette_not_a_flight_answer() {
    const ROWS: &str = r#"(
        brain_presets: { "idle": StandStill },
        action_set_presets: { "peaceful": (move_style: Walk) },
        characters: {
            "drifter": (
                display_name: "Drifter", spritesheet: "drifter.png",
                manifest: "drifter_spritesheet.ron", tier: MainHall,
                body_kind: Floating, composition: None,
                default_brain: "idle", default_action_set: "peaceful",
                barks: (),
            ),
        },
    )"#;
    let catalog = CharacterCatalog::from_data(
        ambition_characters::actor::character_catalog::parse_catalog(ROWS),
    );
    let cast = ambition_characters::prepared::prepare_cast_for_test(&catalog, []);
    assert!(
        cast.get("drifter").is_some_and(|drifter| drifter.locomotion.is_none()),
        "the fixture must be a prepared row that authored no locomotion, or the \
         assertion below is about a different population"
    );

    let interactable = npc_at(Some("drifter"));
    let aabb = ae::Aabb::new(ae::Vec2::new(100.0, 100.0), ae::Vec2::new(16.0, 24.0));
    let (seed, _render) = ActorClusterSeed::new_peaceful_npc_in(
        &Default::default(),
        &catalog,
        Some(&cast),
        "drifter",
        "Drifter",
        aabb,
        &interactable,
        &[],
    );
    assert!(
        !seed.config.tuning.is_aerial,
        "a Floating row with no flight answer spawned aerial on the peaceful road; \
         the blueprint road grounds the same character"
    );
}
