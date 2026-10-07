//! Can two `App`s in one process select different packs? Fast-iteration I3
//! step 1's acceptance: *"Two Apps can select different packs without
//! contamination."*
//!
//! `pack::shipped()` is a process-wide `OnceLock`: the first caller compiles,
//! and every later caller in any App or test receives that value. So an App
//! whose runtime content read it could not disagree with another App, a reload
//! had nowhere to put a new pack, and a test could not give a composition its
//! own content.
//!
//! Two kinds of arm:
//!
//! * the move-table arms compose a cast only and read the prepared registry;
//! * the family arms compose the REAL [`crate::AmbitionContentPlugin`] over a
//!   pack that disagrees with the shipped one in every family the plugin
//!   installs, and read what each App installed. Lowering a family from two
//!   explicit packs would prove the lowerer takes an argument; these arms prove
//!   composition honors the App's selection.
//!
//! The music-cue catalog is installed only under the `audio` feature and is not
//! read back here. Its reader takes the pack by argument, so a boot read does
//! not compile, and `music::tests` shows the catalog follows its pack.

use super::*;

/// Every move in every move-table source, half a second longer.
///
/// A typed edit, not a text substitution: a regex bump would change anything
/// else in the file that matched.
///
/// Lengthening is safe: a window must lie inside `[0, duration_s]`, so a
/// longer move cannot invalidate a window that fit. Shortening could, and the
/// refusal would look like a selection failure.
fn half_a_second_longer(path: &str, text: String) -> String {
    if !path.starts_with("data/movesets/") {
        return text;
    }
    let mut doc = ambition_entity_catalog::EntityCatalogDoc::parse(&text)
        .expect("a shipped move table parses");
    for entity in &mut doc.entities {
        if let Some(moveset) = entity.contracts.moveset.as_mut() {
            for spec in &mut moveset.moves {
                spec.duration_s += 0.5;
            }
        }
    }
    doc.to_ron().expect("and serializes back")
}

/// A composition that registered its declared cast against `pack`.
fn app_selecting(pack: std::sync::Arc<PreparedContentPack>) -> bevy::prelude::App {
    let mut app = bevy::prelude::App::new();
    select_pack(&mut app, pack);
    crate::character_catalog::register_cast(&mut app);
    // The raw road, named. This fixture installs no technique handlers, so real
    // admission would withhold every character naming a native effect, which is
    // not what this test asks.
    ambition_characters::prepared::close_preparation_barrier_without_admission(app.world_mut());
    app
}

/// The subject: a character that is in a shipped move table and in the
/// buildable cast, derived, not named.
///
/// Derived, so a roster change does not break a test about pack selection.
fn a_character_in_both(pack: &PreparedContentPack) -> String {
    let table = ambition_characters::moveset_content_schema::lowered_movesets(pack)
        .expect("the shipped pack carries a move section");
    let buildable: std::collections::BTreeSet<&str> =
        crate::character_catalog::shipped_buildable_cast().collect();
    table
        .keys()
        .find(|id| buildable.contains(id.as_str()))
        .expect(
            "no shipped move table names a buildable character, so every arm \
             below would be about a cast that plays nothing",
        )
        .clone()
}

fn duration_of(app: &bevy::prelude::App, id: &str) -> f32 {
    app.world()
        .resource::<ambition_characters::prepared::PreparedCharacterRegistry>()
        .get(id)
        .unwrap_or_else(|| panic!("`{id}` is published"))
        .kit
        .projectable_moveset()
        .unwrap_or_else(|| panic!("`{id}` carries a moveset"))
        .moves[0]
        .duration_s
}

/// The premise first: the two packs must differ, or "no contamination" is
/// satisfied by two Apps reading one pack.
#[test]
fn the_two_packs_really_do_disagree() {
    let shipped = compile_pack().expect("the shipped pack compiles");
    let edited = compile_pack_with(half_a_second_longer).expect("the edited pack compiles");
    let who = a_character_in_both(&shipped);

    let read = |pack: &PreparedContentPack| {
        ambition_characters::moveset_content_schema::lowered_movesets(pack).expect("a section")
            [&who]
            .moves[0]
            .duration_s
    };
    assert!(
        (read(&edited) - read(&shipped) - 0.5).abs() < 1e-6,
        "the edit did not reach the compiled pack: {} vs {}",
        read(&shipped),
        read(&edited)
    );
}

/// Two Apps, two packs, no contamination: I3 step 1's acceptance.
#[test]
fn two_apps_select_different_packs_without_contamination() {
    let shipped = std::sync::Arc::new(compile_pack().expect("compiles"));
    let edited = std::sync::Arc::new(compile_pack_with(half_a_second_longer).expect("compiles"));
    let who = a_character_in_both(&shipped);

    let first = app_selecting(std::sync::Arc::clone(&shipped));
    let second = app_selecting(std::sync::Arc::clone(&edited));

    assert!(
        (duration_of(&second, &who) - duration_of(&first, &who) - 0.5).abs() < 1e-6,
        "`{who}` plays {} in the App that selected the shipped pack and {} in the \
         App that selected the edited one — the two Apps are reading one pack",
        duration_of(&first, &who),
        duration_of(&second, &who)
    );
}

/// Order does not decide it. With a `OnceLock`, whichever App ran first would
/// win for the whole process.
#[test]
fn the_app_that_selects_second_still_gets_its_own_pack() {
    let shipped = std::sync::Arc::new(compile_pack().expect("compiles"));
    let edited = std::sync::Arc::new(compile_pack_with(half_a_second_longer).expect("compiles"));
    let who = a_character_in_both(&shipped);

    let first = app_selecting(std::sync::Arc::clone(&edited));
    let second = app_selecting(std::sync::Arc::clone(&shipped));
    assert!(
        (duration_of(&first, &who) - duration_of(&second, &who) - 0.5).abs() < 1e-6,
        "the App that selected FIRST decided what the second one plays"
    );
}

/// An App that selects nothing still gets the boot pack, which every
/// composition relies on.
#[test]
fn an_app_that_selects_nothing_reads_the_boot_pack() {
    let shipped = compile_pack().expect("compiles");
    let who = a_character_in_both(&shipped);
    let mut app = bevy::prelude::App::new();
    crate::character_catalog::register_cast(&mut app);
    ambition_characters::prepared::close_preparation_barrier_without_admission(app.world_mut());

    let expected = ambition_characters::moveset_content_schema::lowered_movesets(&shipped)
        .expect("a section")[&who]
        .moves[0]
        .duration_s;
    assert!(
        (duration_of(&app, &who) - expected).abs() < 1e-6,
        "an App that selected nothing did not get the shipped pack"
    );
}

/// The App tells the engine which pack. `install_selection` publishes an
/// `ambition_platformer2d_runtime::SelectedContentIdentity` beside the pack,
/// and `prepare_platformer_content` folds it into the prepared content's
/// fingerprint, so sessions prepared under different move tables are
/// different content generations.
///
/// The provider-side test proves the section reaches the fingerprint, using
/// hand-written identity strings, so it cannot see whether this crate puts
/// anything distinguishing into it: dropping `pack.fingerprint` from the
/// format string would pass there. This test catches that.
#[test]
fn two_different_packs_publish_two_different_content_identities() {
    use ambition_platformer2d_runtime::SelectedContentIdentity;

    let shipped = std::sync::Arc::new(compile_pack().expect("compiles"));
    let edited = std::sync::Arc::new(compile_pack_with(half_a_second_longer).expect("compiles"));
    assert_ne!(
        shipped.fingerprint, edited.fingerprint,
        "the premise: the two packs differ"
    );
    assert_eq!(
        (shipped.id.clone(), shipped.version.clone()),
        (edited.id.clone(), edited.version.clone()),
        "the two packs differ in id or version, so this arm would pass without \
         the fingerprint reaching the identity at all"
    );

    let identity_of = |pack: std::sync::Arc<PreparedContentPack>| {
        let mut app = bevy::prelude::App::new();
        select_pack(&mut app, pack);
        app.world().resource::<SelectedContentIdentity>().0.clone()
    };
    assert_ne!(
        identity_of(shipped),
        identity_of(edited),
        "two packs that differ only in their CONTENT publish the same identity, \
         so the engine cannot tell one generation from the other"
    );
}

/// An App that selects nothing publishes nothing. `None` is a real answer
/// ("this composition has no content pack"), and the provider's own test
/// asserts it is a third distinct generation.
#[test]
fn an_app_that_selects_nothing_publishes_no_content_identity() {
    let app = bevy::prelude::App::new();
    assert!(app
        .world()
        .get_resource::<ambition_platformer2d_runtime::SelectedContentIdentity>()
        .is_none());
}

// ---------------------------------------------------------------------------
// Whole-composition arms: one real plugin, several families.
// ---------------------------------------------------------------------------

/// The source whose first `from` becomes `to`, and the path it lives at.
const FAMILY_EDITS: &[(&str, &str, &str)] = &[
    // sfx registry
    (
        "audio/sfx_registry.ron",
        "frequency: 460.0, frequency_end: 720.0,",
        "frequency: 461.0, frequency_end: 720.0,",
    ),
    // music registry
    ("audio/music_registry.ron", "default_track: \"long_lofi_drift\"", "default_track: \"original_lofi_loop\""),
    // quest book
    ("data/quests.ron", "title: \"First Steps\"", "title: \"First Steps, Revised\""),
    // cutscene library
    (
        "data/cutscenes/intro.ron",
        "text: \"Drain Market — STAFF ONLY\"",
        "text: \"Drain Market — STAFF ONLY (revised)\"",
    ),
    // encounter waves
    ("data/encounters/goblin_encounter.ron", "delay: 0.70", "delay: 0.75"),
    // fighter brain ladder
    ("data/fighter_brain_ladder.ron", "reaction_ms: 500.0", "reaction_ms: 499.0"),
    // boss roster
    ("data/boss_profiles.ron", "combat_size: Some((54.0, 56.0)),", "combat_size: Some((54.0, 57.0)),"),
    // character catalog
    ("data/character_catalog.ron", "display_name: \"Companion Dog\"", "display_name: \"Companion Dog, Revised\""),
];

/// A pack that differs from the shipped one in EVERY family the content plugin
/// installs: moves, items, encounters, the fighter ladder, bosses, the
/// character catalog, both audio registries, the quest book and the cutscenes.
///
/// One compile road ([`compile_pack_with`]), so every difference is content.
/// Each edit has a floor: an edit that matched nothing would leave a family
/// agreeing, and the disagreement arm below would then pass for one family less
/// than it claims.
fn pack_that_disagrees_everywhere() -> std::sync::Arc<PreparedContentPack> {
    let mut applied = std::collections::BTreeSet::new();
    let mut items_swapped = false;
    let pack = compile_pack_with(|path, text| {
        let text = half_a_second_longer(path, text);
        if path == "data/items.ron" {
            let mut rows: Vec<ambition_items::ItemMeta> =
                ron::from_str(&text).expect("the shipped item grid parses as its typed document");
            let wired: Vec<usize> = rows
                .iter()
                .enumerate()
                .filter(|(_, row)| row.held_item_id.is_some())
                .map(|(index, _)| index)
                .collect();
            assert!(wired.len() >= 2, "this edit needs two equippable rows to swap");
            let carried = rows[wired[0]].held_item_id.clone();
            rows[wired[0]].held_item_id = rows[wired[1]].held_item_id.clone();
            rows[wired[1]].held_item_id = carried;
            items_swapped = true;
            return ron::ser::to_string_pretty(&rows, ron::ser::PrettyConfig::default())
                .expect("the edited grid serializes");
        }
        for (edit_path, from, to) in FAMILY_EDITS {
            if path == *edit_path && text.contains(from) {
                applied.insert(*edit_path);
                return text.replacen(from, to, 1);
            }
        }
        text
    })
    .expect("the edited pack compiles");
    assert!(items_swapped, "no declared source is `data/items.ron`");
    for (path, from, _) in FAMILY_EDITS {
        assert!(
            applied.contains(path),
            "`{path}` no longer states `{from}`, so that family would AGREE across the two \
             packs and this witness would be about one family fewer than it claims"
        );
    }
    std::sync::Arc::new(pack)
}

/// One App composed with the real content plugin, over `pack` when one is
/// given, over nothing otherwise.
fn composed_with_the_plugin(pack: Option<std::sync::Arc<PreparedContentPack>>) -> bevy::prelude::App {
    use bevy::ecs::system::RunSystemOnce as _;
    use bevy::prelude::*;
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    if let Some(pack) = pack {
        select_pack(&mut app, pack);
    }
    app.add_plugins(crate::AmbitionContentPlugin);
    ambition_characters::prepared::close_preparation_barrier_without_admission(app.world_mut());
    // The quest registry fills at its first sim tick; run that system by hand so
    // the installed quest book can be read without a schedule.
    app.init_resource::<ambition_persistence::save::AmbitionGameSave>();
    app.world_mut()
        .run_system_once(crate::quest::populate_quest_registry)
        .expect("the quest registry populates from the App's pack");
    app
}

/// What a composition holds, family by family. Each field is a different
/// family with a different source file, so a field-wise comparison names the
/// family that was contaminated.
#[derive(Debug, Clone, PartialEq)]
struct Installed {
    move_duration_s: f32,
    items: ambition_items::ItemCatalog,
    first_rung_ms: f32,
    goblin_wave_delay_s: f32,
    warden_combat_size: Option<(f32, f32)>,
    dog_display_name: String,
    default_music_track: String,
    first_sfx_frequency: f32,
    quest_titles: Vec<String>,
    cutscenes: Vec<String>,
}

const DOG: &str = "npc_companion_dog";
const WARDEN: &str = "clockwork_warden";

/// Every family's answer, read from what `app` installed.
fn installed_in(app: &bevy::prelude::App, mover: &str, pack_cutscene_ids: &[String]) -> Installed {
    let world = app.world();
    let catalogs = world.resource::<ambition_platformer2d::audio::catalog::AudioCatalogRegistry>();
    let library = world.resource::<ambition_cutscene::CutsceneLibrary>();
    Installed {
        move_duration_s: duration_of(app, mover),
        items: world.resource::<ambition_items::ItemCatalog>().clone(),
        first_rung_ms: world
            .resource::<ambition_characters::brain::fighter::AuthoredFighterLadder>()
            .0
            .level(1)
            .expect("a level 1")
            .reaction_ms,
        goblin_wave_delay_s: world
            .resource::<ambition_encounter::EncounterWaveBook>()
            .waves("goblin_encounter")
            .expect("the goblin encounter")[1]
            .mobs[1]
            .delay,
        warden_combat_size: world
            .resource::<ambition_boss_encounter::BossCatalog>()
            .behavior(WARDEN)
            .expect("the warden")
            .combat_size
            .map(|size| (size.x, size.y)),
        dog_display_name: world
            .resource::<ambition_characters::actor::character_catalog::CharacterCatalog>()
            .get(DOG)
            .expect("the dog row")
            .display_name
            .clone(),
        default_music_track: catalogs
            .music_for(crate::AMBITION_CONTENT_PROVIDER)
            .expect("music")
            .default_track
            .clone(),
        first_sfx_frequency: catalogs
            .sfx_for(crate::AMBITION_CONTENT_PROVIDER)
            .expect("sfx")
            .sfx[0]
            .frequency,
        quest_titles: world
            .resource::<crate::quest::QuestRegistry>()
            .quests
            .values()
            .map(|quest| quest.spec.title.clone())
            .collect(),
        // Only the scripts the pack authors: other plugins add their own rows to
        // the shared library.
        cutscenes: pack_cutscene_ids
            .iter()
            .map(|id| format!("{:?}", library.scripts.get(id).expect("an authored cutscene")))
            .collect(),
    }
}

/// The same facts, read from the pack alone. The App must end up holding what
/// ITS pack says.
fn expected_from(pack: &PreparedContentPack, mover: &str) -> Installed {
    let library = crate::dialogue::cutscene_defaults::cutscene_library_of(pack);
    Installed {
        move_duration_s: ambition_characters::moveset_content_schema::lowered_movesets(pack)
            .expect("a section")[mover]
            .moves[0]
            .duration_s,
        items: ambition_items::content_schema::lowered_item_catalog(pack)
            .expect("items")
            .clone(),
        first_rung_ms: ambition_combat::brain::fighter::content_schema::lowered_fighter_brain_ladder(pack)
            .expect("a ladder")
            .level(1)
            .expect("a level 1")
            .reaction_ms,
        goblin_wave_delay_s: ambition_encounter::content_schema::lowered_encounter_waves(pack)
            .expect("waves")
            .get("goblin_encounter")
            .expect("the goblin encounter")[1]
            .mobs[1]
            .delay,
        warden_combat_size: crate::bosses::boss_catalog_of(pack)
            .behavior(WARDEN)
            .expect("the warden")
            .combat_size
            .map(|size| (size.x, size.y)),
        dog_display_name: crate::character_catalog::catalog_of(pack)
            .get(DOG)
            .expect("the dog row")
            .display_name
            .clone(),
        default_music_track: crate::audio_registries::music_registry_of(pack).default_track,
        first_sfx_frequency: crate::audio_registries::sfx_registry_of(pack).sfx[0].frequency,
        quest_titles: {
            let mut titles: Vec<(String, String)> = crate::quest::quest_specs_of(pack)
                .into_iter()
                .map(|spec| (spec.id, spec.title))
                .collect();
            titles.sort();
            titles.into_iter().map(|(_, title)| title).collect()
        },
        cutscenes: library
            .scripts
            .values()
            .map(|script| format!("{script:?}"))
            .collect(),
    }
}

fn cutscene_ids(pack: &PreparedContentPack) -> Vec<String> {
    crate::dialogue::cutscene_defaults::cutscene_library_of(pack)
        .scripts
        .into_keys()
        .collect()
}

/// The families on which two compositions disagree, by name.
fn disagreeing_families(a: &Installed, b: &Installed) -> Vec<&'static str> {
    let mut out = Vec::new();
    macro_rules! family {
        ($field:ident, $name:literal) => {
            if a.$field != b.$field {
                out.push($name);
            }
        };
    }
    family!(move_duration_s, "moves");
    family!(items, "items");
    family!(first_rung_ms, "fighter ladder");
    family!(goblin_wave_delay_s, "encounter waves");
    family!(warden_combat_size, "bosses");
    family!(dog_display_name, "character catalog");
    family!(default_music_track, "music registry");
    family!(first_sfx_frequency, "sfx registry");
    family!(quest_titles, "quest book");
    family!(cutscenes, "cutscenes");
    out
}

const EVERY_FAMILY: [&str; 10] = [
    "moves",
    "items",
    "fighter ladder",
    "encounter waves",
    "bosses",
    "character catalog",
    "music registry",
    "sfx registry",
    "quest book",
    "cutscenes",
];

/// ⭐ THE ACCEPTANCE. Two real compositions, two packs that disagree in ten
/// families, and each composition holds ITS pack in every one of them.
///
/// Under the process-global read the edited App would install the boot items,
/// ladder, waves, bosses, catalog, audio and quests, and the per-family
/// comparison against the pack it selected would fail naming each one.
#[test]
fn two_real_compositions_install_their_own_pack_in_every_family() {
    let shipped = std::sync::Arc::new(compile_pack().expect("compiles"));
    let edited = pack_that_disagrees_everywhere();
    let mover = a_character_in_both(&shipped);

    let first = composed_with_the_plugin(Some(std::sync::Arc::clone(&shipped)));
    let second = composed_with_the_plugin(Some(std::sync::Arc::clone(&edited)));

    let ids = cutscene_ids(&shipped);
    let in_first = installed_in(&first, &mover, &ids);
    let in_second = installed_in(&second, &mover, &ids);

    assert_eq!(in_first, expected_from(&shipped, &mover), "the first App does not hold the pack it selected");
    assert_eq!(in_second, expected_from(&edited, &mover), "the second App does not hold the pack it selected");

    // The premise that makes the two lines above mean something.
    assert_eq!(
        disagreeing_families(&in_first, &in_second),
        EVERY_FAMILY,
        "the two packs must disagree in every family, or an App could read the other's \
         pack in the families that agree and still pass"
    );
}

/// Order does not decide it, in either direction, over every family.
#[test]
fn composing_the_edited_app_first_changes_neither_apps_content() {
    let shipped = std::sync::Arc::new(compile_pack().expect("compiles"));
    let edited = pack_that_disagrees_everywhere();
    let mover = a_character_in_both(&shipped);
    let ids = cutscene_ids(&shipped);

    let first = composed_with_the_plugin(Some(std::sync::Arc::clone(&edited)));
    let second = composed_with_the_plugin(Some(std::sync::Arc::clone(&shipped)));

    assert_eq!(installed_in(&first, &mover, &ids), expected_from(&edited, &mover));
    assert_eq!(installed_in(&second, &mover, &ids), expected_from(&shipped, &mover));
}

/// A composition built while another App holds a different pack still gets the
/// shipped content when it chooses nothing.
#[test]
fn an_app_with_no_selection_gets_the_shipped_content_beside_an_edited_one() {
    let shipped = compile_pack().expect("compiles");
    let mover = a_character_in_both(&shipped);
    let ids = cutscene_ids(&shipped);

    let _edited_app = composed_with_the_plugin(Some(pack_that_disagrees_everywhere()));
    let plain = composed_with_the_plugin(None);

    assert_eq!(installed_in(&plain, &mover, &ids), expected_from(&shipped, &mover));
    assert_eq!(
        selected(plain.world()).map(|pack| pack.fingerprint),
        Some(shipped.fingerprint),
        "the App's selection is the shipped pack, by the canonical seam"
    );
}

/// An App that selects a pack AFTER composing does not revise what it
/// installed: selection is not publication. The reload transaction is the only
/// road that revises a published family.
#[test]
fn selecting_after_composition_publishes_nothing() {
    let shipped = compile_pack().expect("compiles");
    let mover = a_character_in_both(&shipped);
    let ids = cutscene_ids(&shipped);

    let mut app = composed_with_the_plugin(None);
    let before = installed_in(&app, &mover, &ids);
    select_pack(&mut app, pack_that_disagrees_everywhere());

    assert_eq!(
        installed_in(&app, &mover, &ids),
        before,
        "choosing a pack republished an installed family without a reload"
    );
}

/// The production source names the boot pack in a short, reasoned list. A
/// runtime reader that reaches for [`shipped`] because it is convenient
/// raises one of these counts and fails here, with the file named.
///
/// Counted per file over non-test code: `#[cfg(test)]` blocks and test files
/// are subjects of their own (they inspect the shipped product, which is what
/// `shipped` is for). The list is the boot-scoped inspection roads:
///
/// * `authored_movesets.rs` — the shipped fighters' tables, for tests and the
///   `moveset_takes` tool;
/// * `*::shipped_*` helpers — one per family, each the boot-scoped sibling of a
///   `*_of(pack)` function that a composition uses;
/// * `content_validation::validate_embedded_content_graph` — offline validation
///   of the shipped world (a composition validates its own pack through
///   `validate_content_graph`);
/// * `moves_are_content.rs` — a test-only module.
///
/// Nothing in `ambition_app` may name it: the host composes through the plugin
/// and the registers, which select.
#[test]
fn production_code_names_the_shipped_pack_only_in_the_boot_scoped_roads() {
    const ALLOWED: &[(&str, usize)] = &[
        ("audio_registries.rs", 1),
        ("authored_movesets.rs", 3),
        ("bosses/mod.rs", 1),
        ("character_catalog.rs", 2),
        ("content_validation.rs", 1),
        ("moves_are_content.rs", 3),
        ("quest.rs", 1),
    ];

    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("a source directory") {
            let path = entry.expect("an entry").path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                out.push(path);
            }
        }
    }

    /// Non-test occurrences of `needle` in `text`: skips comment lines and the
    /// bodies of `#[cfg(test)]` blocks.
    fn production_hits(text: &str, needle: &str) -> usize {
        let mut hits = 0;
        let mut depth: i64 = 0;
        let mut test_block_depth: Option<i64> = None;
        let mut armed = false;
        for line in text.lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") {
                continue;
            }
            if test_block_depth.is_none() && trimmed.starts_with("#[cfg(test)]") {
                armed = true;
            }
            let in_test = test_block_depth.is_some();
            if !in_test && !armed {
                hits += line.matches(needle).count();
            }
            let opens = line.matches('{').count() as i64;
            let closes = line.matches('}').count() as i64;
            if armed && opens > 0 {
                test_block_depth = Some(depth);
                armed = false;
            }
            depth += opens - closes;
            if test_block_depth.is_some_and(|start| depth <= start) {
                test_block_depth = None;
            }
        }
        hits
    }

    let needle = "pack::shipped()";
    let crate_src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    walk(&crate_src, &mut files);
    let mut found: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for path in files {
        let rel = path.strip_prefix(&crate_src).unwrap().to_string_lossy().replace('\\', "/");
        let is_test_file = rel.ends_with("_tests.rs")
            || rel.ends_with("/tests.rs")
            || rel == "tests.rs"
            || rel.contains("/tests/")
            || rel.ends_with("_tests/mod.rs");
        if is_test_file || rel == "pack.rs" {
            continue;
        }
        let hits = production_hits(&std::fs::read_to_string(&path).expect("readable source"), needle);
        if hits > 0 {
            found.insert(rel, hits);
        }
    }
    let allowed: std::collections::BTreeMap<String, usize> = ALLOWED
        .iter()
        .map(|(file, count)| ((*file).to_string(), *count))
        .collect();
    assert_eq!(
        found, allowed,
        "a production file reads the boot pack. If it is a boot-scoped inspection road, \
         add it to ALLOWED with its reason; if it installs or reads App content, take the \
         App's pack (`pack::select` at composition, `Res<SelectedContentPack>` in a system)"
    );

    let app_src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../ambition_app/src");
    let mut app_files = Vec::new();
    walk(&app_src, &mut app_files);
    for path in app_files {
        let text = std::fs::read_to_string(&path).expect("readable source");
        assert_eq!(
            production_hits(&text, "pack::shipped"),
            0,
            "{} reads the boot pack; the host composes through the content registers",
            path.display()
        );
    }
}
