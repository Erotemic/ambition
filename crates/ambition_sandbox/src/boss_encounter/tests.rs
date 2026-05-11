use ambition_engine as ae;

use super::*;

#[test]
fn encounter_id_from_name_normalizes_capitalization_and_spaces() {
    assert_eq!(
        encounter_id_from_name("Clockwork Warden"),
        "clockwork_warden"
    );
    assert_eq!(
        encounter_id_from_name("Gradient Sentinel"),
        "gradient_sentinel"
    );
    assert_eq!(
        encounter_id_from_name("BOSS-of-the-Year!"),
        "boss_of_the_year"
    );
    assert_eq!(encounter_id_from_name("   "), "boss");
}

#[test]
fn encounter_id_from_name_handles_empty_input() {
    assert_eq!(encounter_id_from_name(""), "boss");
}

#[test]
fn encounter_id_from_name_collapses_consecutive_separators() {
    // Multiple spaces, multiple punctuation runs collapse to single
    // underscore, matching the per-char sanitizer's invariant.
    assert_eq!(encounter_id_from_name("a   b"), "a_b");
    assert_eq!(encounter_id_from_name("a---b"), "a_b");
    assert_eq!(encounter_id_from_name("a -+= b"), "a_b");
}

#[test]
fn encounter_id_from_name_strips_trailing_underscores() {
    assert_eq!(encounter_id_from_name("Boss!"), "boss");
    assert_eq!(encounter_id_from_name("Boss   "), "boss");
    assert_eq!(encounter_id_from_name("Boss--"), "boss");
    assert_eq!(encounter_id_from_name("Boss_"), "boss");
}

#[test]
fn encounter_id_from_name_preserves_alphanumeric_runs() {
    // Numbers stay as-is; lowercase preserved; mid-word digits OK.
    assert_eq!(encounter_id_from_name("R2D2"), "r2d2");
    assert_eq!(encounter_id_from_name("phase4-monster"), "phase4_monster");
}

/// World with a single solid floor sitting well below the
/// mockingbird's spawn anchor (which the tests place at the
/// world origin). The floor spans the entire world's x-extent so
/// chests dropped at any x land on it — the fast-settle path in
/// `sync_mockingbird_treasure_chest` needs an actual collision
/// surface to terminate against.
fn empty_world() -> ae::World {
    ae::World::new(
        "test_arena",
        ae::Vec2::new(4000.0, 4000.0),
        ae::Vec2::new(20.0, 20.0),
        vec![ae::Block::solid(
            "floor",
            // `Block::solid(name, min, size)` — top-left corner +
            // size, NOT center + half-size. Span covers x in
            // [-2000, 2000] so any test spawn anchor lands on it.
            ae::Vec2::new(-2000.0, 1000.0),
            ae::Vec2::new(4000.0, 40.0),
        )],
    )
}

fn empty_features() -> crate::features::FeatureRuntime {
    crate::features::FeatureRuntime {
        hazards: Vec::new(),
        enemies: Vec::new(),
        bosses: Vec::new(),
        breakables: Vec::new(),
        pickups: Vec::new(),
        chests: Vec::new(),
        npcs: Vec::new(),
        switches: Vec::new(),
        banner: String::new(),
        banner_timer: 0.0,
    }
}

/// Build a minimal `FeatureRuntime` carrying one mockingbird
/// `BossRuntime` at `spawn`. We construct the boss with a struct
/// literal because `BossRuntime::new` is `pub(super)` to the
/// features module — fine here because the type's fields are
/// public, and the sync function only reads `id` and `spawn`.
fn mockingbird_features_with_boss(spawn: ae::Vec2) -> crate::features::FeatureRuntime {
    let mut features = empty_features();
    features.bosses.push(crate::features::BossRuntime {
        id: "MockingbirdSpawn-0".to_string(),
        name: "mockingbird".to_string(),
        pos: spawn,
        spawn,
        size: ae::Vec2::new(48.0, 48.0),
        health: ae::Health::new(30),
        brain: ae::BossBrain::Custom("mockingbird".into()),
        alive: false,
        pattern_timer: 0.0,
        movement_timer: 0.0,
        attack_windup_timer: 0.0,
        attack_timer: 0.0,
        attack_cooldown: 0.0,
        hit_flash: 0.0,
    });
    features
}

#[test]
fn sync_mockingbird_treasure_chest_drops_chest_when_save_marks_cleared() {
    let spawn = ae::Vec2::new(120.0, 80.0);
    let mut features = mockingbird_features_with_boss(spawn);
    let mut registry = BossEncounterRegistry::default();
    registry.ensure(ae::BossEncounterSpec::mockingbird());
    registry.link_runtime(MOCKINGBIRD_ENCOUNTER_ID, "MockingbirdSpawn-0");
    let mut save = ae::SandboxSaveData::default();
    save.set_boss(
        MOCKINGBIRD_ENCOUNTER_ID,
        ae::PersistedEncounterState::Cleared,
    );

    sync_mockingbird_treasure_chest(&mut features, &save, &registry, &empty_world());

    let chest = features
        .chests
        .iter()
        .find(|c| c.id == "encounter_chest_mockingbird")
        .expect("treasure chest must spawn once the bird is dead");
    assert!(
        !chest.opened,
        "fresh-kill chest starts closed (looted flag isn't set yet)",
    );
    assert_eq!(
        chest.pos.x, spawn.x,
        "chest aligns horizontally with the boss anchor",
    );
    assert!(
        chest.pos.y > spawn.y,
        "chest sits below the (airborne) boss spawn anchor",
    );
}

#[test]
fn sync_mockingbird_treasure_chest_is_a_no_op_before_kill() {
    let mut features = mockingbird_features_with_boss(ae::Vec2::new(0.0, 0.0));
    let mut registry = BossEncounterRegistry::default();
    registry.ensure(ae::BossEncounterSpec::mockingbird());
    registry.link_runtime(MOCKINGBIRD_ENCOUNTER_ID, "MockingbirdSpawn-0");
    // No `set_boss` call → save default is `NotStarted`.
    let save = ae::SandboxSaveData::default();

    sync_mockingbird_treasure_chest(&mut features, &save, &registry, &empty_world());

    assert!(
        features.chests.is_empty(),
        "no chest before the boss is cleared",
    );
}

/// On a fresh kill (no looted flag), the chest spawns mid-air
/// near the boss anchor with `falling = true` so the live update
/// loop can play out a visible drop animation. Pinning this
/// behavior protects the "kill the bird → watch the chest fall"
/// UX beat from a future refactor that accidentally fast-settles
/// the first drop.
#[test]
fn sync_mockingbird_treasure_chest_starts_falling_on_first_kill() {
    let mut features = mockingbird_features_with_boss(ae::Vec2::new(0.0, 0.0));
    let mut registry = BossEncounterRegistry::default();
    registry.ensure(ae::BossEncounterSpec::mockingbird());
    registry.link_runtime(MOCKINGBIRD_ENCOUNTER_ID, "MockingbirdSpawn-0");
    let mut save = ae::SandboxSaveData::default();
    save.set_boss(
        MOCKINGBIRD_ENCOUNTER_ID,
        ae::PersistedEncounterState::Cleared,
    );
    // Looted flag is NOT set — this is the fresh-kill path.

    sync_mockingbird_treasure_chest(&mut features, &save, &registry, &empty_world());

    let chest = features
        .chests
        .iter()
        .find(|c| c.id == "encounter_chest_mockingbird")
        .expect("chest must spawn on fresh kill");
    assert!(
        chest.falling,
        "first-kill chest must start falling so the player sees the drop animation"
    );
}

/// On reload, when the chest was looted in a prior session, the
/// player expects the chest to be sitting right where they left
/// it — *not* dropping in again from above. The sync function
/// fast-settles the chest in-line so by the time the renderer
/// reads the position, the chest is already on the floor with
/// `falling = false`.
#[test]
fn sync_mockingbird_treasure_chest_fast_settles_on_reload_when_looted() {
    let mut features = mockingbird_features_with_boss(ae::Vec2::new(0.0, 0.0));
    let mut registry = BossEncounterRegistry::default();
    registry.ensure(ae::BossEncounterSpec::mockingbird());
    registry.link_runtime(MOCKINGBIRD_ENCOUNTER_ID, "MockingbirdSpawn-0");
    let mut save = ae::SandboxSaveData::default();
    save.set_boss(
        MOCKINGBIRD_ENCOUNTER_ID,
        ae::PersistedEncounterState::Cleared,
    );
    save.set_flag(
        crate::encounter::encounter_reward_looted_flag(MOCKINGBIRD_ENCOUNTER_ID),
        true,
    );

    sync_mockingbird_treasure_chest(&mut features, &save, &registry, &empty_world());

    let chest = features
        .chests
        .iter()
        .find(|c| c.id == "encounter_chest_mockingbird")
        .expect("chest must spawn on reload even after looting");
    assert!(
        !chest.falling,
        "looted chest must be settled on reload, not falling again"
    );
    assert!(
        chest.opened,
        "looted flag still mirrors onto the freshly-settled chest"
    );
}

#[test]
fn sync_mockingbird_treasure_chest_mirrors_looted_flag_on_reload() {
    let mut features = mockingbird_features_with_boss(ae::Vec2::new(0.0, 0.0));
    let mut registry = BossEncounterRegistry::default();
    registry.ensure(ae::BossEncounterSpec::mockingbird());
    registry.link_runtime(MOCKINGBIRD_ENCOUNTER_ID, "MockingbirdSpawn-0");
    let mut save = ae::SandboxSaveData::default();
    save.set_boss(
        MOCKINGBIRD_ENCOUNTER_ID,
        ae::PersistedEncounterState::Cleared,
    );
    save.set_flag(
        crate::encounter::encounter_reward_looted_flag(MOCKINGBIRD_ENCOUNTER_ID),
        true,
    );

    sync_mockingbird_treasure_chest(&mut features, &save, &registry, &empty_world());

    let chest = features
        .chests
        .iter()
        .find(|c| c.id == "encounter_chest_mockingbird")
        .expect("re-spawned chest must surface even after looting");
    assert!(
        chest.opened,
        "looted flag must mark the re-spawned chest as already opened",
    );
}

#[test]
fn encounter_id_from_name_drops_non_ascii() {
    // Non-alphanumeric Unicode is treated as a separator (matches
    // the `is_ascii_alphanumeric` predicate). Future i18n work can
    // relax this if needed.
    assert_eq!(encounter_id_from_name("日本語 Boss"), "boss");
    assert_eq!(encounter_id_from_name("Ω-omega"), "omega");
}
