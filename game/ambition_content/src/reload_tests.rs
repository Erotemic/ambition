//! A pack read as text reaches a live cast, and a pack that does not compile
//! reaches nothing.
//!
//! Most arms use a small synthetic pack, not the shipped one: the subject is
//! the reload, not the roster. `moves_are_content` covers the shipped roster.
//!
//! The document text is serialized from the real types, not written by hand,
//! so a change to `MoveSpec`'s shape does not break parsing here. The edit is
//! still a text change on those bytes, so the tests exercise a changed file.

use super::*;
use bevy::prelude::IntoScheduleConfigs;
use ambition_characters::prepared::{
    close_preparation_barrier, stage_authored_character, CharacterBindings,
    PreparedCharacterRegistry,
};
use ambition_content_pack::{
    compile, AssetsUnchecked, ContentPackDraft, ContentPackManifest, ModuleNamespace, PackId,
    PackVersion, SchemaId, SourceDeclaration,
};
use ambition_entity_catalog::authoring::{strike, Strike};
use ambition_entity_catalog::{
    EntityCatalogDoc, EntityContracts, EntityDef, MovesetContract, ENTITY_CATALOG_SCHEMA_VERSION,
};
use ambition_platformer2d::character::CharacterDefinition;

const RELOAD_ID: &str = "reload_duelist";
const PATH: &str = "moves/duelist.ron";

/// The probe document, as an author would have it on disk.
///
/// `recover_s` is the value the edit changes: it lands in `duration_s`, which
/// is visible in the published moveset and derived from nothing else.
fn doc_text(recover_s: f32) -> String {
    doc_text_bound_to(recover_s, "swat")
}

/// The same document with `attack` bound to `verb_target`, so a test can author
/// a verb pointing at a move that does not exist.
///
/// A parameter, not a text substitution: replacing `"swat"` in the bytes also
/// changed the move's own id, so the document stayed consistent and compiled.
fn doc_text_bound_to(recover_s: f32, verb_target: &str) -> String {
    doc_text_naming(recover_s, verb_target, None)
}

/// The same document whose strike lands `on_hit` technique `technique`.
///
/// A parameter, because a text substitution must match `to_ron`'s exact
/// pretty-print format (`on_hit: None`).
fn doc_text_naming(recover_s: f32, verb_target: &str, technique: Option<&str>) -> String {
    let mut verbs = std::collections::BTreeMap::new();
    verbs.insert("attack".to_string(), verb_target.to_string());
    EntityCatalogDoc {
        schema_version: ENTITY_CATALOG_SCHEMA_VERSION,
        entities: vec![EntityDef {
            id: RELOAD_ID.to_string(),
            contracts: EntityContracts {
                body: None,
                hurtboxes: None,
                presentation: None,
                borrows: None,
                moveset: Some(MovesetContract {
                    verbs,
                    moves: vec![strike(Strike {
                        id: "swat",
                        clip: "slash",
                        startup_s: 0.1,
                        active_s: 0.1,
                        recover_s,
                        offset: (20.0, 0.0),
                        half_extents: (12.0, 10.0),
                        damage: 5,
                        knockback: 40.0,
                        knockback_growth: 0.5,
                        launch_dir: None,
                        on_hit: technique.map(ambition_entity_catalog::EffectRef::new),
                    })],
                }),
            },
        }],
    }
    .to_ron()
    .expect("the probe document serializes")
}

fn total_duration(recover_s: f32) -> f32 {
    0.1 + 0.1 + recover_s
}

fn pack_of(text: &str) -> Result<ambition_content_pack::PreparedContentPack, String> {
    let draft = ContentPackDraft::from_sources(
        ContentPackManifest {
            id: PackId("reload_probe".into()),
            version: PackVersion("1.0.0".into()),
            namespace: ModuleNamespace("probe".into()),
            requires: Vec::new(),
            sources: vec![SourceDeclaration {
                path: PATH.into(),
                schema: SchemaId::new(ambition_characters::moveset_content_schema::MOVESET_SCHEMA),
                version: ambition_characters::moveset_content_schema::MOVESET_VERSION,
            }],
        },
        [(PATH.to_string(), text.to_string())],
    )
    .map_err(|failure| failure.to_string())?;
    compile(&draft, &crate::pack::pack_schemas(), &AssetsUnchecked)
        .map_err(|failure| failure.to_string())
}

/// The same probe pack with no moveset source declared.
///
/// The shipped pack cannot express this transition: the compiler refuses an
/// empty table and a table with its only entity removed. A manifest that stops
/// declaring the family is valid (`game/ambition_demo_smash/assets/pack.ron`
/// has that shape), but `compile_pack_with` edits source text only, not the
/// declaration list.
fn pack_with_no_moveset_section() -> ambition_content_pack::PreparedContentPack {
    let draft = ContentPackDraft::from_sources(
        ContentPackManifest {
            id: PackId("reload_probe".into()),
            version: PackVersion("1.0.0".into()),
            namespace: ModuleNamespace("probe".into()),
            requires: Vec::new(),
            sources: Vec::new(),
        },
        std::iter::empty::<(String, String)>(),
    )
    .expect("a manifest declaring nothing is a legitimate pack");
    compile(&draft, &crate::pack::pack_schemas(), &AssetsUnchecked)
        .expect("a pack that declares no source compiles")
}

/// A world whose cast has been through the real preparation barrier, with one
/// character the probe pack can revise.
fn host_with_a_live_cast() -> bevy::app::App {
    let mut app = bevy::app::App::new();
    stage_authored_character(
        &mut app,
        CharacterDefinition::new(RELOAD_ID, "Reload Duelist", "probe"),
        &CharacterBindings::default(),
    )
    .expect("the probe character stages");
    close_preparation_barrier(app.world_mut());
    app.world_mut()
        .insert_resource(ambition_combat::technique::InstalledTechniques::default());
    app
}

fn live_duration(app: &bevy::app::App) -> f32 {
    app.world()
        .resource::<PreparedCharacterRegistry>()
        .get(RELOAD_ID)
        .expect("the probe character is published")
        .kit
        .projectable_moveset()
        .expect("and carries a moveset")
        .moves[0]
        .duration_s
}

/// Premise: the probe character starts with no moveset.
#[test]
fn the_probe_host_publishes_its_character_with_no_moveset() {
    let app = host_with_a_live_cast();
    let registry = app.world().resource::<PreparedCharacterRegistry>();
    let character = registry.get(RELOAD_ID).expect("the probe is published");
    assert!(
        character.kit.projectable_moveset().is_none(),
        "the fixture already has a moveset, so 'the reload delivered it' would \
         be true before the reload ran"
    );
}

/// A file read after boot changes what the cast plays, with no compile step
/// (fast-iteration I2).
#[test]
fn a_reloaded_pack_republishes_the_cast() {
    let mut app = host_with_a_live_cast();
    let before = app
        .world()
        .resource::<PreparedCharacterRegistry>()
        .generation();

    let pack = pack_of(&doc_text(0.2)).expect("the probe pack compiles");
    let outcome = reload_move_tables_from(app.world_mut(), &pack);
    assert!(
        matches!(outcome, MoveReload::Activated { changed: 1, .. }),
        "the reload did not republish: {outcome:?}"
    );

    assert!(
        app.world()
            .resource::<PreparedCharacterRegistry>()
            .generation()
            .get()
            > before.get(),
        "the cast was republished without moving the generation"
    );
    assert!(
        (live_duration(&app) - total_duration(0.2)).abs() < 1e-6,
        "the published timing is {} and the file says {}",
        live_duration(&app),
        total_duration(0.2)
    );
}

/// The edit is what arrives, not a constant. Without this, a reload that
/// always republished the same table would pass the arm above.
#[test]
fn the_edited_timing_is_the_one_the_cast_ends_up_with() {
    let mut app = host_with_a_live_cast();
    let _ = reload_move_tables_from(
        app.world_mut(),
        &pack_of(&doc_text(0.2)).expect("compiles"));
    let first = live_duration(&app);

    let outcome = reload_move_tables_from(
        app.world_mut(),
        &pack_of(&doc_text(0.35)).expect("compiles"));
    assert!(
        matches!(outcome, MoveReload::Activated { .. }),
        "the second reload did not republish: {outcome:?}"
    );
    assert_ne!(
        live_duration(&app),
        first,
        "the cast kept the first reload's timing"
    );
    assert!(
        (live_duration(&app) - total_duration(0.35)).abs() < 1e-6,
        "the published timing is {} and the edited file says {}",
        live_duration(&app),
        total_duration(0.35)
    );
}

/// Reloading the same file twice does not move the generation. A watcher
/// produces this on every save without an edit. Staleness checks key on the
/// generation, so a no-op publication would invalidate live bodies and cached
/// plans.
#[test]
fn reloading_an_unchanged_pack_publishes_nothing() {
    let mut app = host_with_a_live_cast();
    let pack = pack_of(&doc_text(0.2)).expect("compiles");
    let _ = reload_move_tables_from(app.world_mut(), &pack);
    let published = app
        .world()
        .resource::<PreparedCharacterRegistry>()
        .generation();

    let outcome = reload_move_tables_from(app.world_mut(), &pack);
    assert_eq!(
        outcome,
        MoveReload::Unchanged {
            generation: published.get()
        },
        "a second reload of the same bytes reported {outcome:?}"
    );
}

/// A pack that does not compile never reaches the host, and the refusal names
/// the unresolved target.
///
/// The compiler refuses before any world is involved, so an author learns
/// about an unbound verb from the pack.
#[test]
fn a_refused_pack_never_reaches_the_cast() {
    let mut app = host_with_a_live_cast();
    let _ = reload_move_tables_from(
        app.world_mut(),
        &pack_of(&doc_text(0.2)).expect("compiles"));
    let published = app
        .world()
        .resource::<PreparedCharacterRegistry>()
        .generation();
    let timing = live_duration(&app);

    // A verb bound to a move that does not exist — `CatalogError::UnknownVerbMove`.
    let broken = doc_text_bound_to(0.2, "nothing");
    assert_ne!(
        broken,
        doc_text(0.2),
        "the fixture is not actually different"
    );
    let failure = pack_of(&broken).expect_err("this pack must be refused");
    assert!(
        failure.contains("nothing"),
        "the refusal does not name the unresolved target:\n{failure}"
    );

    let registry = app.world().resource::<PreparedCharacterRegistry>();
    assert_eq!(
        registry.generation(),
        published,
        "a refused pack moved the generation"
    );
    assert_eq!(
        live_duration(&app),
        timing,
        "a refused pack changed what the cast plays"
    );
}

/// A host that has not closed its preparation barrier is told so, and not
/// told its content is wrong.
#[test]
fn a_host_with_no_cast_is_reported_as_such() {
    let mut app = bevy::app::App::new();
    app.world_mut()
        .insert_resource(ambition_combat::technique::InstalledTechniques::default());
    let outcome = reload_move_tables_from(
        app.world_mut(),
        &pack_of(&doc_text(0.2)).expect("compiles"));
    assert_eq!(outcome, MoveReload::NoCast, "got {outcome:?}");
}

/// A section that names a character this build never prepared stages none of
/// it, and the report names the character.
#[test]
fn a_section_for_an_unknown_character_stages_nothing() {
    let mut app = host_with_a_live_cast();
    let published = app
        .world()
        .resource::<PreparedCharacterRegistry>()
        .generation();

    let foreign = doc_text(0.2).replace(RELOAD_ID, "nobody_prepared_this");
    let outcome =
        reload_move_tables_from(app.world_mut(), &pack_of(&foreign).expect("compiles"));
    match &outcome {
        MoveReload::UnknownCharacters(who) => assert!(
            who.iter().any(|w| w.contains("nobody_prepared_this")),
            "the report does not name who: {who:?}"
        ),
        other => panic!("expected an unknown-character report; got {other:?}"),
    }
    assert_eq!(
        app.world()
            .resource::<PreparedCharacterRegistry>()
            .generation(),
        published,
        "a section for an unknown character moved the generation"
    );
}

/// Absent is not empty. A world with no `InstalledTechniques` has not installed
/// combat; `unwrap_or_default()` would report a false roster-wide refusal.
#[test]
fn a_host_that_installed_no_technique_table_is_reported_rather_than_defaulted() {
    let mut app = bevy::app::App::new();
    stage_authored_character(
        &mut app,
        CharacterDefinition::new(RELOAD_ID, "Reload Duelist", "probe"),
        &CharacterBindings::default(),
    )
    .expect("stages");
    close_preparation_barrier(app.world_mut());
    // No `InstalledTechniques` here.
    let outcome = reload_move_tables_from(
        app.world_mut(),
        &pack_of(&doc_text(0.2)).expect("compiles"));
    assert_eq!(outcome, MoveReload::NoTechniqueSupport, "got {outcome:?}");
}

// Cast-generation staleness is covered on the production road by
// `a_candidate_prepared_against_a_pack_that_is_no_longer_selected_is_refused`:
// `admit_candidate` refuses a stale base on the pack fingerprint before
// anything is staged.

/// A republished cast and the App's selection move together. A reload is the
/// one operation that could separate them.
#[test]
fn an_activated_reload_becomes_the_apps_selection() {
    let mut app = host_with_a_live_cast();
    let fresh = std::sync::Arc::new(pack_of(&doc_text(0.2)).expect("compiles"));
    let outcome =
        reload_move_tables_selecting(app.world_mut(), std::sync::Arc::clone(&fresh));
    assert!(
        matches!(outcome, MoveReload::Activated { .. }),
        "the premise: a reload that publishes; got {outcome:?}"
    );
    let selected = crate::pack::selected(app.world()).expect("the App has a selection");
    assert!(
        std::ptr::eq(selected, std::sync::Arc::as_ref(&fresh)),
        "the cast was republished from a pack the App did not adopt"
    );
}

// ---------------------------------------------------------------------------
// I2 acceptance: a prebuilt host plays an edited file.
//
// These arms use a directory that did not exist when this binary was
// compiled. No build step sits between the bytes on disk and the published
// cast. Always use a temp directory, never the repository tree.
// ---------------------------------------------------------------------------

/// One shipped move table's file, and the character it belongs to.
fn a_shipped_table(root: &std::path::Path) -> (std::path::PathBuf, String) {
    let shipped = crate::pack::compile_pack().expect("the shipped pack compiles");
    let table = ambition_characters::moveset_content_schema::lowered_movesets(&shipped)
        .expect("a move section");
    let buildable: std::collections::BTreeSet<&str> =
        crate::character_catalog::buildable_cast().collect();
    // The file whose document names `who`: a file is named for its table, and
    // one table may serve several characters.
    let file_of = |who: &str| {
        std::fs::read_dir(root.join("data/movesets"))
            .ok()?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .find(|path| {
                std::fs::read_to_string(path)
                    .ok()
                    .and_then(|text| ambition_entity_catalog::EntityCatalogDoc::parse(&text).ok())
                    .is_some_and(|doc| doc.entities.iter().any(|e| e.id == who))
            })
    };
    // A character whose file authors its whole table: a borrower's file holds
    // only what it changes, so editing every move in it would not move the
    // move this test reads.
    let authors_its_own = |file: &std::path::Path| {
        std::fs::read_to_string(file)
            .ok()
            .and_then(|text| ambition_entity_catalog::EntityCatalogDoc::parse(&text).ok())
            .is_some_and(|doc| doc.entities.iter().all(|e| e.contracts.borrows.is_none()))
    };
    let who = table
        .keys()
        .filter(|id| buildable.contains(id.as_str()))
        .find(|id| file_of(id).is_some_and(|file| authors_its_own(&file)))
        .expect("some shipped table names a buildable character that authors its own table")
        .clone();
    let file = file_of(&who).expect("the character's table is one of the declared moveset files");
    assert!(
        file.exists(),
        "the export did not write {} — the arm below would edit nothing",
        file.display()
    );
    (file, who)
}

/// A support table declaring every technique the SHIPPED tables reference.
///
/// Derived from the content, so admission always passes here. These arms test
/// the disk road; admission has its own arm
/// (`a_refused_pack_never_reaches_the_cast`). An empty table would refuse 40+
/// shipped effects and hide the subject. The premise asserts the reload was not
/// refused, so a wrong derivation fails loudly.
fn support_for_the_live_cast(
    world: &bevy::ecs::world::World,
) -> ambition_entity_catalog::TechniqueSupport {
    // Read the live cast, not the pack. `overlay_authored_moves` overlays an
    // authored table on the kit-derived moves, and derived moves the table does
    // not name survive (e.g. `pogo_bounce`, which no shipped table mentions). So
    // the pack's keys are not the cast's technique vocabulary.
    let registry = world.resource::<PreparedCharacterRegistry>();
    let mut support = ambition_entity_catalog::TechniqueSupport::default();
    let mut seen = std::collections::BTreeSet::new();
    for spec in registry
        .iter()
        .filter_map(|(_, character)| character.kit.projectable_moveset())
        .flat_map(|contract| contract.moves.iter())
    {
        for (_, reference) in spec.effect_refs() {
            if !seen.insert(reference.key.clone()) {
                continue;
            }
            let _ = support.declare(
                reference.key.clone(),
                ambition_entity_catalog::TechniqueOffer {
                    owner: "reload_fixture",
                    // Accept any params. `TechniqueParams::None` refuses a non-empty map, and
                    // shipped effects carry params.
                    params: ambition_entity_catalog::TechniqueParams::Checked(|_| Ok(())),
                    // `None`: the effect names no other authored definition. The preparation
                    // barrier already checks nested references for this content.
                    references: ambition_entity_catalog::NestedReferences::None,
                    // Both: the shipped tables author effects at volumes and at events.
                    delivery: ambition_entity_catalog::TechniqueDelivery::Either,
                },
            );
        }
    }
    assert!(
        seen.len() > 10,
        "only {} technique key(s) across the live cast — the fixture is not          seeing the content it is meant to support",
        seen.len()
    );
    support
}

fn host_from_dir(root: &std::path::Path) -> (bevy::app::App, String) {
    let (_, who) = a_shipped_table(root);
    (host_with_the_shipped_cast(), who)
}

fn shipped_duration(app: &bevy::app::App, who: &str) -> f32 {
    app.world()
        .resource::<PreparedCharacterRegistry>()
        .get(who)
        .expect("published")
        .kit
        .projectable_moveset()
        .expect("a moveset")
        .moves[0]
        .duration_s
}

/// A file edited after the binary was built changes what the cast plays,
/// through the production road.
///
/// The edit travels: disk → `compile_pack_from` → `request_reload` →
/// `ShellCommand::ReplaceWith` → the router's `PreparationRequested` → the
/// activation. It does not use `reload_move_tables_from_dir`, which is
/// `#[cfg(test)]` and skips the shell transaction, epoch, identity and
/// rollback boundary.
#[test]
fn a_host_plays_a_move_edited_on_disk_after_it_was_built() {
    let dir = tempfile::tempdir().expect("a temp content root");
    let root = dir.path();
    let written = crate::pack::export_sources_to(root).expect("the sources export");
    assert!(written > 10, "only {written} source(s) exported");

    let (app, who) = host_from_dir(root);
    let mut app = app;
    shell_active_on(&mut app, true);
    app.add_systems(
        bevy::app::Update,
        (adopt_preparation_transaction, commit_content_generation)
            .chain(),
    );
    let before = shipped_duration(&app, &who);

    // Edit the file on disk, through the typed document.
    let (file, _) = a_shipped_table(root);
    let text = std::fs::read_to_string(&file).expect("reads back");
    let mut doc = ambition_entity_catalog::EntityCatalogDoc::parse(&text).expect("parses");
    for entity in &mut doc.entities {
        if let Some(moveset) = entity.contracts.moveset.as_mut() {
            for spec in &mut moveset.moves {
                spec.duration_s += 0.5;
            }
        }
    }
    std::fs::write(&file, doc.to_ron().expect("serializes")).expect("writes");

    // Read the cast base before the file I/O: a publication during the read
    // would move the cast under the pack being built.
    let compiled = crate::pack::compile_pack_from(root).expect("the edited root compiles");
    let outcome = request_reload(
        app.world_mut(),
        ambition_content_pack::CandidateGeneration::prepared_against(
            std::sync::Arc::new(compiled),
            None,
        ),
    );
    assert!(
        matches!(outcome, ReloadRequest::Requested { .. }),
        "the edited directory was not even requested: {outcome:?}"
    );
    assert_eq!(
        shipped_duration(&app, &who),
        before,
        "the request published on the spot instead of staging for the activation"
    );

    // The shell announces the transaction and then activates it.
    let mine = a_preparation_for(&mut app, "shell.game.1");
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellEvent::RouteActivated(mine));
    app.update();

    assert!(
        (shipped_duration(&app, &who) - before - 0.5).abs() < 1e-6,
        "`{who}` plays {} and the file on disk says {}",
        shipped_duration(&app, &who),
        before + 0.5
    );
}

/// A root missing a file is refused by name. A per-file fallback to the
/// binary's own text would build a mixed pack.
#[test]
fn a_root_that_does_not_supply_the_whole_pack_is_refused_by_name() {
    let dir = tempfile::tempdir().expect("a temp content root");
    let root = dir.path();
    crate::pack::export_sources_to(root).expect("exports");
    let (file, _) = a_shipped_table(root);
    std::fs::remove_file(&file).expect("removes one source");

    let mut app = bevy::app::App::new();
    let outcome = reload_move_tables_from_dir(app.world_mut(), root);
    match &outcome {
        MoveReload::PackRefused(why) => {
            assert!(
                why.contains("missing"),
                "the refusal does not say what: {why}"
            );
            assert!(
                why.contains(
                    file.file_name()
                        .and_then(|n| n.to_str())
                        .expect("a file name")
                ),
                "the refusal does not name the missing file: {why}"
            );
        }
        other => panic!("expected a refusal naming the missing file; got {other:?}"),
    }
}

/// Control on the same road: an unedited export is the shipped pack, so the
/// request must report `Unchanged` and ask the shell for nothing. Without
/// this, a road that re-prepares on every call would pass "the edit arrived".
/// A watcher fires on every save, so this is the common case.
#[test]
fn requesting_a_reload_of_an_unedited_export_asks_the_shell_for_nothing() {
    let dir = tempfile::tempdir().expect("a temp content root");
    let root = dir.path();
    crate::pack::export_sources_to(root).expect("exports");
    let (mut app, _) = host_from_dir(root);
    shell_active_on(&mut app, true);

    let compiled = crate::pack::compile_pack_from(root).expect("the exported root compiles");
    assert_eq!(
        compiled.fingerprint,
        crate::pack::selected(app.world())
            .expect("a selection")
            .fingerprint,
        "the premise: an unedited export must recompile to the SHIPPED identity, \
         or this control is about a pack that really did change"
    );

    let outcome = request_reload(
        app.world_mut(),
        ambition_content_pack::CandidateGeneration::prepared_against(
            std::sync::Arc::new(compiled),
            None,
        ),
    );
    assert_eq!(
        outcome,
        ReloadRequest::Unchanged,
        "requesting a reload of the shipped bytes reported {outcome:?}"
    );
    assert!(
        issued_commands(&mut app).is_empty(),
        "a complete no-op asked the shell to re-prepare the route"
    );
    assert!(
        crate::reload::pending_pack(app.world()).is_none(),
        "a complete no-op staged a pending generation"
    );
}

/// The published moveset is larger than the authored table, by design.
/// `overlay_authored_moves` overlays an authored table on the kit-derived
/// moves, and a derived move the table does not name survives.
///
/// `authored_intrinsics` calls the pack's table "a replacement, not a merge".
/// That is true of the contract, not of the kit it is folded into. This test
/// pins both facts together.
#[test]
fn the_published_moveset_keeps_the_kit_moves_the_table_does_not_name() {
    let mut app = bevy::app::App::new();
    crate::character_catalog::register_cast(&mut app);
    ambition_characters::prepared::close_preparation_barrier_without_admission(app.world_mut());

    let shipped = crate::pack::compile_pack().expect("compiles");
    let tables = ambition_characters::moveset_content_schema::lowered_movesets(&shipped)
        .expect("a move section");
    let registry = app.world().resource::<PreparedCharacterRegistry>();

    let mut compared = 0;
    let mut carried_extras = 0;
    for (who, authored) in tables {
        let Some(published) = registry
            .get(who)
            .and_then(|character| character.kit.projectable_moveset())
        else {
            continue;
        };
        compared += 1;
        let authored_ids: std::collections::BTreeSet<&str> =
            authored.moves.iter().map(|m| m.id.as_str()).collect();
        let published_ids: std::collections::BTreeSet<&str> =
            published.moves.iter().map(|m| m.id.as_str()).collect();
        assert!(
            authored_ids.is_subset(&published_ids),
            "`{who}` lost authored moves on the way to the cast: {:?}",
            authored_ids.difference(&published_ids).collect::<Vec<_>>()
        );
        if published_ids.len() > authored_ids.len() {
            carried_extras += 1;
        }
    }
    // Floor: an empty comparison makes the subset claim true over nothing.
    assert!(
        compared >= 10,
        "only {compared} shipped table(s) reached a published character"
    );
    assert!(
        carried_extras > 0,
        "no published moveset carries a move its table does not name, so the \
         overlay rule this pins is no longer the rule — read \
         `overlay_authored_moves` before deleting this"
    );
}

// ---------------------------------------------------------------------------
// Cross-domain atomicity.
//
// Move reload must not conclude `Unchanged` from the move material and then
// install a pack whose items changed (`docs/planning/queue.md`). Special-casing
// `Unchanged` does not fix it: `Unchanged` is correct for the move section. The
// fix is a decision on the whole candidate pack.
// ---------------------------------------------------------------------------

/// The declared path of the non-move section this witness re-authors.
const ITEMS_PATH: &str = "data/items.ron";

/// A host whose cast is the shipped roster, because the subject is two
/// compiles of the real pack.
fn host_with_the_shipped_cast() -> bevy::app::App {
    let mut app = bevy::app::App::new();
    crate::character_catalog::register_cast(&mut app);
    ambition_characters::prepared::close_preparation_barrier_without_admission(app.world_mut());
    // An empty technique table would refuse 40+ authored effects. See
    // `support_for_the_live_cast`.
    let support = support_for_the_live_cast(app.world());
    app.world_mut()
        .insert_resource(ambition_combat::technique::InstalledTechniques(support));
    app
}

/// The shipped pack with one entity removed from its moveset table.
///
/// Edited through the typed document. A text substitution on an entity id
/// would also hit verb bindings and move ids with the same prefix.
fn pack_without_entity(victim: &str) -> ambition_content_pack::PreparedContentPack {
    let mut removed = false;
    let pack = crate::pack::compile_pack_with(|_declared, text| {
        let Ok(mut doc) = ambition_entity_catalog::EntityCatalogDoc::parse(&text) else {
            return text;
        };
        let before = doc.entities.len();
        doc.entities.retain(|entity| entity.id != victim);
        if doc.entities.len() == before {
            return text;
        }
        removed = true;
        doc.to_ron().expect("the edited table serializes")
    })
    .expect("the edited pack compiles");
    // Floor: if no table names the victim, the candidate is the shipped pack and
    // the arm tests nothing.
    assert!(
        removed,
        "no declared source names entity `{victim}`, so the candidate is the \
         shipped pack and the witness would pass vacuously"
    );
    pack
}

/// The shipped pack with every authored move half a second longer.
///
/// Names are unchanged: this is the control for the dropped-entity refusal,
/// so it changes the pack without changing which characters are authored.
fn pack_with_every_move_retimed() -> ambition_content_pack::PreparedContentPack {
    let mut retimed = 0usize;
    let pack = crate::pack::compile_pack_with(|_declared, text| {
        let Ok(mut doc) = ambition_entity_catalog::EntityCatalogDoc::parse(&text) else {
            return text;
        };
        let mut touched = false;
        for entity in &mut doc.entities {
            if let Some(moveset) = entity.contracts.moveset.as_mut() {
                for spec in &mut moveset.moves {
                    spec.duration_s += 0.5;
                    retimed += 1;
                    touched = true;
                }
            }
        }
        if !touched {
            return text;
        }
        doc.to_ron().expect("the edited table serializes")
    })
    .expect("the retimed pack compiles");
    assert!(
        retimed > 10,
        "only {retimed} move(s) were retimed, so the control barely differs from \
         the shipped pack"
    );
    pack
}

/// The shipped pack with one item row's mechanical wiring changed, and every
/// other source byte-identical.
///
/// Edited through the typed document. `items.ron` is a positional
/// `Vec<ItemMeta>` and its row count is part of the schema, so adding or
/// removing a line would be refused for an unrelated reason.
///
/// Two existing `held_item_id` values are swapped, so the only difference is
/// which slot grants which held item. That is mechanical content:
/// `Item::from_held_item_id` resolves equipping. The field moves; the rows do
/// not.
fn pack_with_one_item_rewired() -> ambition_content_pack::PreparedContentPack {
    let mut edited = false;
    let pack = crate::pack::compile_pack_with(|declared, text| {
        if declared != ITEMS_PATH {
            return text;
        }
        let mut rows: Vec<ambition_items::ItemMeta> =
            ron::from_str(&text).expect("the shipped item grid parses as its typed document");
        let wired: Vec<usize> = rows
            .iter()
            .enumerate()
            .filter(|(_, row)| row.held_item_id.is_some())
            .map(|(index, _)| index)
            .collect();
        assert!(
            wired.len() >= 2,
            "the shipped grid wires {} equippable row(s); this edit needs two to swap",
            wired.len()
        );
        let (first, second) = (wired[0], wired[1]);
        let carried = rows[first].held_item_id.clone();
        rows[first].held_item_id = rows[second].held_item_id.clone();
        rows[second].held_item_id = carried;
        edited = true;
        ron::ser::to_string_pretty(&rows, ron::ser::PrettyConfig::default())
            .expect("the edited grid serializes")
    })
    .expect("the edited pack compiles");
    // Floor: a wrong declared path would leave the closure unused, and the
    // candidate would be the shipped pack.
    assert!(
        edited,
        "no declared source is spelled `{ITEMS_PATH}`, so the candidate pack is \
         the shipped one and this witness would pass vacuously"
    );
    pack
}

/// Fixture contract for the cross-domain arm: one pack, two contents, and the
/// difference is in no move table.
///
/// Both halves are asserted here, because each failure would make the arm pass
/// for the wrong reason:
///
/// * if the move sections differ, the arm is about two movesets, not
///   atomicity;
/// * if the fingerprints match, the pack's identity cannot see an items-only
///   edit.
///
/// The fingerprint is the `ContentFingerprint` over the canonical bytes, so this
/// also pins that `held_item_id` reaches the canonical form. The `item_catalog`
/// handler prefixes each row with `slot={index}` for this reason.
#[test]
fn the_item_edited_twin_differs_from_the_shipped_pack_in_no_move_table() {
    let shipped = crate::pack::compile_pack().expect("the shipped pack compiles");
    let candidate = pack_with_one_item_rewired();

    let shipped_moves = ambition_characters::moveset_content_schema::lowered_movesets(&shipped)
        .expect("the shipped pack carries a move section");
    let candidate_moves = ambition_characters::moveset_content_schema::lowered_movesets(&candidate)
        .expect("the candidate pack carries a move section");
    assert_eq!(
        shipped_moves, candidate_moves,
        "the item edit moved a move table, so this pair cannot witness a \
         cross-domain difference"
    );
    // Floor: two empty move sections would compare equal.
    assert!(
        shipped_moves.len() >= 10,
        "only {} move table(s) compared; the equality above is close to vacuous",
        shipped_moves.len()
    );

    let shipped_items = ambition_items::content_schema::lowered_item_catalog(&shipped)
        .expect("the shipped pack carries an item catalog");
    let candidate_items = ambition_items::content_schema::lowered_item_catalog(&candidate)
        .expect("the candidate pack carries an item catalog");
    assert_ne!(
        shipped_items, candidate_items,
        "the edit did not reach the lowered item catalog, so no subsystem could \
         observe it"
    );

    assert_ne!(
        shipped.fingerprint, candidate.fingerprint,
        "an items-only edit did not move the pack's ContentFingerprint, so a \
         verdict computed from the pack's complete identity cannot tell these \
         two packs apart"
    );
}

/// The `Arc` this App has actually selected — the pack its cast was built from.
///
/// The host already has a selection at boot: `register_declared_cast` calls
/// `pack::select`, which inserts on fallback. A fresh compile of the shipped
/// pack is a complete no-op against it. So read the baseline; do not install
/// one.
fn live_pack(app: &bevy::app::App) -> std::sync::Arc<ambition_content_pack::PreparedContentPack> {
    std::sync::Arc::clone(&app.world().resource::<crate::pack::SelectedContentPack>().0)
}

fn cast_generation(app: &bevy::app::App) -> u64 {
    app.world()
        .resource::<PreparedCharacterRegistry>()
        .generation()
        .get()
}

/// Composition-level arm: the real pack, cast and selection move together or
/// not at all.
///
/// `candidate_tests.rs` pins the verdict on a synthetic pack. This drives the
/// same decision through `compile_pack_with`, the shipped roster and a live
/// `PreparedCharacterRegistry`.
///
/// An items-only candidate is refused. The live item catalog is installed in
/// `AmbitionContentPlugin::build` from `pack::prepared()` and no reload road
/// replaces it, so publishing would make `PreparedContentIdentity` name N+1
/// while items serve N.
///
/// Items cannot join easily: `pack::prepared()` returns a `&'static` borrow,
/// and the item read side re-exports it (`display_name`, `description`,
/// `dialog_id` return `&'static str`) and adds `ITEM_CATALOG_OVERRIDE`, a
/// second `OnceLock`. The second family is `fighter_brain_ladder` instead; see
/// `the_fighter_ladder_is_the_second_family_the_transaction_carries`. When
/// items join (after `ambition_items` returns owned values), flip this test to
/// expect publication.
///
/// The control runs first. A complete no-op must touch nothing; that is the
/// only result that tells this apart from a `publish_candidate` that always
/// installs. The twin is a fresh compile, not the live `Arc`, so "the selection
/// did not move" is not true by pointer identity. Same fingerprint across two
/// allocations also shows the compile is deterministic, which every
/// `assert_ne!` on fingerprints in this file relies on.
#[test]
fn a_candidate_that_changes_only_items_is_refused_as_an_unsupported_domain() {
    // ── the control: a complete no-op touches nothing ────────────────────────
    {
        let mut app = host_with_the_shipped_cast();
        let live = live_pack(&app);
        let twin =
            std::sync::Arc::new(crate::pack::compile_pack().expect("the shipped pack compiles"));
        assert_eq!(
            twin.fingerprint, live.fingerprint,
            "recompiling the same sources gave a different identity, so every \
             fingerprint comparison in this file is meaningless"
        );
        assert!(
            !std::sync::Arc::ptr_eq(&twin, &live),
            "the twin is the live pack's own allocation, so the pointer assertion \
             below cannot fail and this control has no power"
        );
        let generation_before = cast_generation(&app);

        let outcome = publish_candidate(
            app.world_mut(),
            ambition_content_pack::CandidateGeneration::prepared_against(
                std::sync::Arc::clone(&twin),
                Some(live.fingerprint),
            ));
        assert!(
            matches!(outcome, MoveReload::Unchanged { .. }),
            "a candidate mechanically identical to what is live is the complete \
             no-op; got {outcome:?}"
        );
        assert!(
            std::ptr::eq(
                crate::pack::selected(app.world()).expect("a selection"),
                std::sync::Arc::as_ref(&live)
            ),
            "a complete no-op installed a pack anyway — the selection moved for a \
             candidate that changed nothing"
        );
        assert_eq!(
            cast_generation(&app),
            generation_before,
            "a complete no-op consumed a catalog generation"
        );
    }

    // ── the subject: items changed, moves not ────────────────────────────────
    let mut app = host_with_the_shipped_cast();
    let live = live_pack(&app);
    let candidate = std::sync::Arc::new(pack_with_one_item_rewired());

    assert_eq!(
        ambition_characters::moveset_content_schema::lowered_movesets(&live),
        ambition_characters::moveset_content_schema::lowered_movesets(&candidate),
        "the premise: the candidate must differ in NO move table"
    );
    assert_ne!(
        live.fingerprint, candidate.fingerprint,
        "the premise: the candidate must differ in the pack's COMPLETE identity, \
         or the verdict can never reach `Publish`"
    );
    let generation_before = cast_generation(&app);

    let outcome = publish_candidate(
        app.world_mut(),
        ambition_content_pack::CandidateGeneration::prepared_against(
            std::sync::Arc::clone(&candidate),
            Some(live.fingerprint),
        ));

    match &outcome {
        MoveReload::RefusedUnsupportedChangedDomain(domains) => assert!(
            domains.iter().any(|d| d == "item_catalog"),
            "the refusal does not name the domain that changed: {domains:?}"
        ),
        other => panic!(
            "an items-only candidate must be refused until items participate in \
             the generation transaction; got {other:?}"
        ),
    }
    assert!(
        std::ptr::eq(
            crate::pack::selected(app.world()).expect("a selection"),
            std::sync::Arc::as_ref(&live)
        ),
        "a refused candidate became the App's selection, so the engine's content \
         identity now claims item content is active that is not"
    );
    assert_eq!(
        cast_generation(&app),
        generation_before,
        "a refused candidate moved the cast generation"
    );
}

/// Control for the refusal: a moves-only candidate still publishes. Without
/// it, a road that refuses every change would pass.
#[test]
fn a_candidate_that_changes_only_moves_still_publishes() {
    let mut app = host_with_the_shipped_cast();
    let live = live_pack(&app);
    let candidate = std::sync::Arc::new(
        crate::pack::compile_pack_with(|path, text| {
            if path.starts_with("data/movesets/") {
                let mut doc = ambition_entity_catalog::EntityCatalogDoc::parse(&text)
                    .expect("a shipped move table parses");
                for entity in &mut doc.entities {
                    if let Some(moveset) = entity.contracts.moveset.as_mut() {
                        for spec in &mut moveset.moves {
                            spec.duration_s += 0.5;
                        }
                    }
                }
                doc.to_ron().expect("and serializes")
            } else {
                text
            }
        })
        .expect("the edited pack compiles"),
    );
    assert_ne!(
        live.fingerprint, candidate.fingerprint,
        "the premise: the candidate differs"
    );
    assert_eq!(
        ambition_content_pack::changed_domains(&live, &candidate)
            .iter()
            .map(|s| s.0.as_str())
            .collect::<Vec<_>>(),
        vec!["moveset"],
        "the premise: ONLY the participating domain changed"
    );

    let outcome = publish_candidate(
        app.world_mut(),
        ambition_content_pack::CandidateGeneration::prepared_against(
            std::sync::Arc::clone(&candidate),
            Some(live.fingerprint),
        ));
    assert!(
        matches!(outcome, MoveReload::Activated { .. }),
        "a moves-only candidate was not published; got {outcome:?}"
    );
    assert!(std::ptr::eq(
        crate::pack::selected(app.world()).expect("a selection"),
        std::sync::Arc::as_ref(&candidate)
    ));
}

// ---------------------------------------------------------------------------
// Publication is refused while a rollback timeline is speculating.
//
// `ambition_platformer2d_rollback_ggrs`'s per-frame contract check invalidates
// a live GGRS timeline when the prepared content identity changes under it.
// Refusing is better than publishing and getting a desync.
// ---------------------------------------------------------------------------

use ambition_platformer2d_runtime::rollback::{ActiveRollbackAuthority, RollbackTimelineContract};
use ambition_platformer2d_runtime::SnapshotSchemaFingerprint;
use ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId;

/// An authority that governs this world, with a live timeline.
fn live_authority() -> ActiveRollbackAuthority {
    ActiveRollbackAuthority::installed(
        None,
        Some(SessionScopeId(1)),
        RollbackTimelineContract {
            content: None,
            schema: SnapshotSchemaFingerprint::from_bytes([7u8; 32]),
        },
    )
}

/// A host with a cast, a pack selection, and whatever authority `authority` is.
fn host_with_authority(authority: Option<ActiveRollbackAuthority>) -> (bevy::app::App, f32) {
    let mut app = host_with_a_live_cast();
    let _ = reload_move_tables_selecting(
        app.world_mut(),
        std::sync::Arc::new(pack_of(&doc_text(0.2)).expect("compiles")));
    let published = live_duration(&app);
    if let Some(authority) = authority {
        app.world_mut().insert_resource(authority);
    }
    (app, published)
}

/// A candidate that would publish: a real edit, with no base claim.
fn a_publishable_candidate() -> ambition_content_pack::CandidateGeneration {
    ambition_content_pack::CandidateGeneration::prepared_against(
        std::sync::Arc::new(pack_of(&doc_text(0.45)).expect("compiles")),
        None,
    )
}

/// Control: a candidate that does publish through this road. Without it, a
/// road that refuses everything would pass.
#[test]
fn with_no_rollback_authority_the_same_candidate_publishes() {
    let (mut app, before) = host_with_authority(None);
    let outcome = publish_candidate(app.world_mut(), a_publishable_candidate());
    assert!(
        matches!(outcome, MoveReload::Activated { .. }),
        "the control did not publish: {outcome:?}"
    );
    assert_ne!(live_duration(&app), before, "the control published nothing");
}

/// A live timeline refuses, and nothing moves.
#[test]
fn a_candidate_is_refused_while_a_rollback_timeline_is_live() {
    let (mut app, before) = host_with_authority(Some(live_authority()));
    let selection = crate::pack::selected(app.world())
        .expect("a selection")
        .fingerprint;
    let generation = app
        .world()
        .resource::<PreparedCharacterRegistry>()
        .generation();

    let outcome = publish_candidate(app.world_mut(), a_publishable_candidate());
    assert_eq!(
        outcome,
        MoveReload::RefusedDuringLiveTimeline,
        "got {outcome:?}"
    );
    assert_eq!(
        crate::pack::selected(app.world())
            .expect("a selection")
            .fingerprint,
        selection,
        "a refused publication moved the App's content selection"
    );
    assert_eq!(
        app.world()
            .resource::<PreparedCharacterRegistry>()
            .generation(),
        generation,
        "a refused publication moved the cast generation"
    );
    assert_eq!(
        live_duration(&app),
        before,
        "a refused publication changed what the cast plays"
    );
}

/// Publishing must not heal an unhealthy authority.
/// `RollbackTimelineStatus::carried_from` passes an unhealthy reason to the
/// replacement, and only `acknowledge_and_clear` may clear it.
#[test]
fn publishing_does_not_heal_an_unhealthy_rollback_authority() {
    let mut authority = live_authority();
    authority.invalidate("a deliberate desync, for this test".to_string());
    let (mut app, _) = host_with_authority(Some(authority));
    assert!(
        !app.world()
            .resource::<ActiveRollbackAuthority>()
            .status()
            .is_healthy(),
        "the premise: the authority starts UNHEALTHY"
    );

    let outcome = publish_candidate(app.world_mut(), a_publishable_candidate());
    match &outcome {
        MoveReload::RefusedWhileRollbackUnhealthy(why) => assert!(
            why.contains("deliberate desync"),
            "the refusal dropped the diagnosis: {why}"
        ),
        other => panic!("expected a refusal naming the desync; got {other:?}"),
    }
    let status = app
        .world()
        .resource::<ActiveRollbackAuthority>()
        .status()
        .clone();
    assert!(
        !status.is_healthy(),
        "a local content publication HEALED an unhealthy rollback authority"
    );
    assert_eq!(
        status.invalidation.as_deref(),
        Some("a deliberate desync, for this test"),
        "the diagnosis was replaced rather than preserved"
    );
}

/// A stood-down timeline is not live: nothing is speculating.
#[test]
fn a_stood_down_timeline_does_not_refuse() {
    let mut authority = live_authority();
    authority.stand_down_timeline();
    let (mut app, before) = host_with_authority(Some(authority));
    let outcome = publish_candidate(app.world_mut(), a_publishable_candidate());
    assert!(
        matches!(outcome, MoveReload::Activated { .. }),
        "a stood-down timeline refused a publication: {outcome:?}"
    );
    assert_ne!(live_duration(&app), before);
}

/// A candidate that compiles and then fails admission changes nothing.
///
/// This is a different layer from `a_refused_pack_never_reaches_the_cast`,
/// which refuses at compile. Here the candidate is valid but names a technique
/// this composition did not install. The selection, content identity, cast
/// generation and cast timing must all stay the same.
#[test]
fn a_candidate_refused_at_admission_leaves_every_published_fact_alone() {
    let mut app = host_with_a_live_cast();
    let _ = reload_move_tables_selecting(
        app.world_mut(),
        std::sync::Arc::new(pack_of(&doc_text(0.2)).expect("compiles")));
    let selection = crate::pack::selected(app.world())
        .expect("a selection")
        .fingerprint;
    let identity = app
        .world()
        .resource::<ambition_platformer2d_runtime::SelectedContentIdentity>()
        .clone();
    let generation = app
        .world()
        .resource::<PreparedCharacterRegistry>()
        .generation();
    let timing = live_duration(&app);

    // A move naming a technique nothing installed. It compiles: the content
    // compiler does not know what a host installed.
    let named = doc_text_naming(0.35, "swat", Some("nothing.installed"));
    assert!(
        named.contains("nothing.installed"),
        "the fixture does not name the uninstalled technique, so the arm below \
         would be about an ordinary edit"
    );
    let pack = pack_of(&named).expect("a candidate naming an uninstalled technique still COMPILES");
    let outcome = publish_candidate(
        app.world_mut(),
        ambition_content_pack::CandidateGeneration::prepared_against(
            std::sync::Arc::new(pack),
            None,
        ));
    match &outcome {
        MoveReload::Refused(refusals) => assert!(
            refusals.iter().any(|r| r.contains("nothing.installed")),
            "the refusal does not name the uninstalled technique: {refusals:?}"
        ),
        other => panic!("expected an admission refusal; got {other:?}"),
    }

    assert_eq!(
        crate::pack::selected(app.world())
            .expect("a selection")
            .fingerprint,
        selection,
        "a refused candidate became the App's content selection"
    );
    assert_eq!(
        app.world()
            .resource::<ambition_platformer2d_runtime::SelectedContentIdentity>(),
        &identity,
        "a refused candidate moved the engine's content identity"
    );
    assert_eq!(
        app.world()
            .resource::<PreparedCharacterRegistry>()
            .generation(),
        generation,
        "a refused candidate moved the cast generation"
    );
    assert_eq!(
        live_duration(&app),
        timing,
        "a refused candidate changed what the cast plays"
    );
}

// ---------------------------------------------------------------------------
// The request road: a reload reuses the engine's lifecycle.
//
// These arms test `request_reload`, which issues the shell's
// `PreparationRequested` road. The epoch, fingerprint and publication come
// from `prepare_platformer_content`, and the old generation stays
// authoritative until the new one activates.
// ---------------------------------------------------------------------------

use ambition_platformer2d::game_shell::{
    ActiveShellExperience, ProviderPreparationPlan, ShellCommand, ShellRouteCatalog, ShellRouteId,
    ShellRouteSpec, ShellRouter,
};

fn shell_active_on(app: &mut bevy::app::App, prepares: bool) {
    let mut catalog = ShellRouteCatalog::default();
    let spec = ShellRouteSpec::new("game", "fixture");
    catalog.register(if prepares {
        spec.preparing_with(
            ProviderPreparationPlan::new("Prepare fixture", "ready", "Ready")
                .required("publish", "Publish prepared session"),
        )
    } else {
        spec
    });
    app.world_mut().insert_resource(catalog);
    let mut router = ShellRouter::default();
    router.active = Some(ambition_platformer2d::game_shell::ActiveShellExperience {
        activation_id: ambition_platformer2d::game_shell::ShellActivationId(1),
        route_id: ShellRouteId::new("game"),
        experience_id: ambition_platformer2d::game_shell::ShellExperienceId::new("fixture"),
        parameters: Default::default(),
        load_authorization: None,
        prepared_session: None,
    });
    app.world_mut().insert_resource(router);
    app.add_message::<ShellCommand>();
    app.add_message::<ambition_platformer2d::game_shell::ShellEvent>();
}

/// Play the two shell events one re-preparation produces, and return the
/// activation that transaction authorizes.
///
/// The fixture mints the load because only the router can
/// (`ShellRouter::next_load_transaction` is private), so a reload learns its
/// `LoadId` only from `PreparationRequested`. Reusing the active experience
/// would test "any activation publishes", which is the defect.
///
/// The request id is copied from the command the caller wrote, as the router
/// does. Do not invent one: reading the caller's command tests propagation; a
/// hand-picked constant only tests that fixture and subject agree.
fn a_preparation_for(app: &mut bevy::app::App, load: &str) -> ActiveShellExperience {
    let barrier = ambition_platformer2d::load::LoadBarrierRef::new(
        ambition_platformer2d::load::LoadId::new(load),
        ambition_platformer2d::load::LoadBarrierId::new("publish"),
    );
    // Read, not constructed. `None` means the caller wrote no correlator, and
    // adoption must then refuse; passing it through is intended.
    let requested = issued_commands(app).into_iter().find_map(|command| match command {
        ShellCommand::ReplaceWith { request, .. } => request,
        _ => None,
    });
    a_preparation_carrying(app, barrier, requested)
}

/// [`a_preparation_for`] with the caller's correlator supplied explicitly.
///
/// `issued_commands` sees only the current update's messages. A test that calls
/// `update()` between the request and the announcement reads the id earlier
/// and passes it here. It is still read from the command, not hand-picked.
fn a_preparation_carrying(
    app: &mut bevy::app::App,
    barrier: ambition_platformer2d::load::LoadBarrierRef,
    request: Option<ambition_platformer2d::game_shell::ShellRequestId>,
) -> ActiveShellExperience {
    app.world_mut().write_message(
        ambition_platformer2d::game_shell::ShellEvent::PreparationRequested(
            ambition_platformer2d::game_shell::ProviderLoadTransaction {
                route_id: ShellRouteId::new("game"),
                experience_id: ambition_platformer2d::game_shell::ShellExperienceId::new("fixture"),
                barrier: barrier.clone(),
                request,
            },
        ),
    );
    app.update();
    let mut active = app
        .world()
        .resource::<ShellRouter>()
        .active
        .clone()
        .expect("the fixture is active on a route");
    active.load_authorization = Some(barrier);
    active
}

fn issued_commands(app: &mut bevy::app::App) -> Vec<ShellCommand> {
    let messages = app
        .world()
        .resource::<bevy::ecs::message::Messages<ShellCommand>>();
    messages.iter_current_update_messages().cloned().collect()
}

/// A real edit issues a re-preparation and publishes nothing itself.
#[test]
fn a_changed_candidate_requests_a_re_preparation_of_the_active_route() {
    let mut app = host_with_a_live_cast();
    let _ = reload_move_tables_selecting(
        app.world_mut(),
        std::sync::Arc::new(pack_of(&doc_text(0.2)).expect("compiles")));
    shell_active_on(&mut app, true);
    let generation = app
        .world()
        .resource::<PreparedCharacterRegistry>()
        .generation();
    let played = live_duration(&app);

    let outcome = request_reload(app.world_mut(), a_publishable_candidate());
    assert!(
        matches!(
            &outcome,
            ReloadRequest::Requested { route, request }
                if route == "game" && request.as_str().starts_with("reload.game.")
        ),
        "got {outcome:?}"
    );
    assert!(
        matches!(
            issued_commands(&mut app).as_slice(),
            [ShellCommand::ReplaceWith { route, .. }] if route.as_str() == "game"
        ),
        "the request did not reach the shell as a ReplaceWith on the active route: {:?}",
        issued_commands(&mut app)
    );

    // Nothing is published yet. The new generation appears when the shell
    // activates it; this is the difference from `publish_candidate`.
    assert_eq!(
        app.world()
            .resource::<PreparedCharacterRegistry>()
            .generation(),
        generation,
        "requesting a re-preparation republished the cast on the spot"
    );
    assert_eq!(
        live_duration(&app),
        played,
        "requesting a re-preparation changed what the cast plays"
    );
}

/// A complete no-op requests nothing. A watcher fires on every save, and a
/// re-preparation would spend an epoch, publication and reconstruction.
#[test]
fn an_unchanged_candidate_requests_nothing() {
    let mut app = host_with_a_live_cast();
    let pack = std::sync::Arc::new(pack_of(&doc_text(0.2)).expect("compiles"));
    let _ = reload_move_tables_selecting(app.world_mut(), std::sync::Arc::clone(&pack));
    shell_active_on(&mut app, true);

    let outcome = request_reload(
        app.world_mut(),
        ambition_content_pack::CandidateGeneration::prepared_against(pack, None),
    );
    assert_eq!(outcome, ReloadRequest::Unchanged, "got {outcome:?}");
    assert!(
        issued_commands(&mut app).is_empty(),
        "a complete no-op asked the shell to re-prepare"
    );
}

/// A route with no preparation plan cannot be re-prepared, and the request
/// says so. The retry road (`ambition_load_presentation::shell_adapter`)
/// checks the same thing.
#[test]
fn a_route_with_no_preparation_plan_is_reported_rather_than_requested() {
    let mut app = host_with_a_live_cast();
    shell_active_on(&mut app, false);
    let outcome = request_reload(app.world_mut(), a_publishable_candidate());
    assert_eq!(
        outcome,
        ReloadRequest::RouteHasNoPreparation("game".to_string()),
        "got {outcome:?}"
    );
    assert!(issued_commands(&mut app).is_empty());
}

/// No active route is its own answer: a composition with no shell has nothing
/// to re-prepare, and that is not a content problem.
#[test]
fn a_host_with_no_active_route_is_reported_rather_than_requested() {
    let mut app = host_with_a_live_cast();
    app.add_message::<ShellCommand>();
    let outcome = request_reload(app.world_mut(), a_publishable_candidate());
    assert_eq!(outcome, ReloadRequest::NoActiveRoute, "got {outcome:?}");
}

/// The refusals reach this road too: a live rollback timeline refuses a
/// re-preparation request, as it refuses a direct publication.
#[test]
fn a_live_rollback_timeline_refuses_a_re_preparation_request() {
    let mut app = host_with_a_live_cast();
    shell_active_on(&mut app, true);
    app.world_mut().insert_resource(live_authority());
    let outcome = request_reload(app.world_mut(), a_publishable_candidate());
    assert_eq!(
        outcome,
        ReloadRequest::Refused(MoveReload::RefusedDuringLiveTimeline),
        "got {outcome:?}"
    );
    assert!(
        issued_commands(&mut app).is_empty(),
        "a refused request reached the shell anyway"
    );
}

/// The two halves land at one boundary. The request stages the cast revision
/// and publishes nothing; the shell's `RouteActivated` lets it through. Until
/// then the live cast is the old one.
#[test]
fn the_staged_cast_revision_publishes_when_the_route_activates() {
    let mut app = host_with_a_live_cast();
    let _ = reload_move_tables_selecting(
        app.world_mut(),
        std::sync::Arc::new(pack_of(&doc_text(0.2)).expect("compiles")));
    shell_active_on(&mut app, true);
    app.add_systems(
        bevy::app::Update,
        (adopt_preparation_transaction, commit_content_generation)
            .chain(),
    );
    let before = live_duration(&app);

    assert!(matches!(
        request_reload(app.world_mut(), a_publishable_candidate()),
        ReloadRequest::Requested { .. }
    ));
    assert_eq!(
        live_duration(&app),
        before,
        "the request published the cast on the spot instead of staging it"
    );

    // The shell announces the transaction, then activates it.
    let active = a_preparation_for(&mut app, "shell.game.1");
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellEvent::RouteActivated(active));
    app.update();

    assert_ne!(
        live_duration(&app),
        before,
        "the route activated and the staged cast revision did not publish"
    );
}

/// A candidate that stops naming a character is refused, not merged.
///
/// `stage_move_section` iterates the candidate's keys, and the fold is
/// `active.clone()` plus the staged set. A dropped character would keep its old
/// moveset under the new generation while the pack says it has none.
/// `MovesetRevisionError` cannot express this, since both variants are about
/// names the candidate contains.
#[test]
fn a_candidate_that_stops_naming_a_character_is_refused() {
    let mut app = host_with_the_shipped_cast();
    shell_active_on(&mut app, true);
    let live = live_pack(&app);
    let table = ambition_characters::moveset_content_schema::lowered_movesets(&live)
        .expect("the shipped pack authors movesets");
    // The victim needs a sibling in its own file. Removing a table's only entity
    // is already refused by the compiler. A file with two entities lets one be
    // dropped without deleting a file.
    //
    // The victim is not an archetype that another entity borrows: removing
    // one of those makes the pack fail to compile, which is a different rule.
    let docs: Vec<ambition_entity_catalog::EntityCatalogDoc> = std::fs::read_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/data/movesets"),
    )
    .expect("the shipped move files")
    .filter_map(Result::ok)
    .filter_map(|entry| std::fs::read_to_string(entry.path()).ok())
    .filter_map(|text| ambition_entity_catalog::EntityCatalogDoc::parse(&text).ok())
    .collect();
    let borrowed: std::collections::BTreeSet<&str> = docs
        .iter()
        .flat_map(|doc| &doc.entities)
        .filter_map(|entity| entity.contracts.borrows.as_ref())
        .map(|borrow| borrow.archetype.as_str())
        .collect();
    let victim = docs
        .iter()
        .filter(|doc| doc.entities.len() > 1)
        .flat_map(|doc| doc.entities.iter().map(|entity| entity.id.clone()))
        .filter(|id| !borrowed.contains(id.as_str()))
        .find(|id| table.contains_key(id))
        .expect(
            "no shipped moveset table carries two entities, so a per-entity \
             removal cannot be authored and this arm has no subject",
        );

    // The candidate: the shipped pack with one entity removed from its table.
    let candidate = std::sync::Arc::new(pack_without_entity(&victim));
    let dropped = ambition_characters::moveset_content_schema::lowered_movesets(&candidate)
        .expect("the candidate still authors a moveset section");
    // Premise: exactly this one entity is gone and the rest remain. Losing the
    // whole section is a different case.
    assert!(
        !dropped.contains_key(&victim),
        "the fixture did not actually remove `{victim}`"
    );
    assert_eq!(
        dropped.len() + 1,
        table.len(),
        "the fixture removed more than the one entity it names"
    );

    let outcome = request_reload(
        app.world_mut(),
        ambition_content_pack::CandidateGeneration::prepared_against(
            std::sync::Arc::clone(&candidate),
            Some(live.fingerprint),
        ),
    );
    match &outcome {
        ReloadRequest::Refused(MoveReload::RefusedDroppedMovesetEntities(names)) => assert_eq!(
            names.as_slice(),
            [victim.clone()],
            "the refusal does not name exactly the character that was dropped"
        ),
        other => panic!(
            "a candidate that stops naming `{victim}` must be refused until the \
             moveset participant can represent a removal; got {other:?}"
        ),
    }
    assert!(
        issued_commands(&mut app).is_empty(),
        "a refused request reached the shell anyway"
    );
    assert!(
        crate::reload::pending_pack(app.world()).is_none(),
        "a refused request staged the candidate as pending"
    );
}

/// A candidate that drops the whole family names every character it stops
/// carrying.
///
/// The arm above cannot reach this branch: the transition is in a manifest,
/// and `compile_pack_with` only rewrites source text.
/// `game/ambition_demo_smash/assets/pack.ron` has this shape.
#[test]
fn a_candidate_that_drops_the_moveset_family_names_everyone_it_drops() {
    let base = pack_of(&doc_text(0.2)).expect("the probe pack compiles");
    let named: Vec<String> = ambition_characters::moveset_content_schema::lowered_movesets(&base)
        .expect("the probe pack authors a moveset section")
        .keys()
        .cloned()
        .collect();
    // Premise: the base must author someone, or the expected list is empty.
    assert!(!named.is_empty(), "the probe pack authors nobody");

    let candidate = pack_with_no_moveset_section();
    assert!(
        ambition_characters::moveset_content_schema::lowered_movesets(&candidate).is_none(),
        "the fixture still carries a moveset section, so it is not the \
         removed-family transition"
    );

    assert_eq!(
        ambition_characters::moveset_content_schema::dropped_moveset_entities(&base, &candidate),
        named,
        "a candidate that drops the whole family reported a different set than \
         the characters the base was playing"
    );

    // Mirror: a base that authored none loses nothing, so a first publication of
    // the family is not a removal.
    assert!(
        ambition_characters::moveset_content_schema::dropped_moveset_entities(&candidate, &base).is_empty(),
        "publishing the family for the first time was reported as dropping it"
    );
}

/// Control: a candidate that retimes a move and names every live character
/// is an ordinary reload and must still be requested. Without this, refusing
/// every moveset edit would pass.
#[test]
fn a_candidate_that_renames_nobody_is_still_requested() {
    let mut app = host_with_the_shipped_cast();
    shell_active_on(&mut app, true);
    let live = live_pack(&app);
    let candidate = std::sync::Arc::new(pack_with_every_move_retimed());
    let before =
        ambition_characters::moveset_content_schema::lowered_movesets(&live).expect("a section");
    let after = ambition_characters::moveset_content_schema::lowered_movesets(&candidate)
        .expect("a section");
    // Premise: same names, different content.
    assert_eq!(
        before.keys().collect::<Vec<_>>(),
        after.keys().collect::<Vec<_>>(),
        "the control changed WHICH characters are authored"
    );
    assert_ne!(
        live.fingerprint, candidate.fingerprint,
        "the control did not actually change the pack"
    );

    let outcome = request_reload(
        app.world_mut(),
        ambition_content_pack::CandidateGeneration::prepared_against(
            std::sync::Arc::clone(&candidate),
            Some(live.fingerprint),
        ),
    );
    assert!(
        matches!(outcome, ReloadRequest::Requested { .. }),
        "an ordinary retime was refused: {outcome:?}"
    );
}

/// The production road refuses it too. The direct-road arm alone does not
/// cover the road the game takes.
///
/// When items join the generation transaction, flip this arm: expect
/// `Requested`, and expect a shell command.
#[test]
fn the_request_road_refuses_an_items_only_candidate_too() {
    let mut app = host_with_the_shipped_cast();
    shell_active_on(&mut app, true);
    let live = live_pack(&app);
    let candidate = std::sync::Arc::new(pack_with_one_item_rewired());
    assert_eq!(
        ambition_characters::moveset_content_schema::lowered_movesets(&live),
        ambition_characters::moveset_content_schema::lowered_movesets(&candidate),
        "the premise: the candidate must differ in NO move table"
    );
    assert_ne!(
        live.fingerprint, candidate.fingerprint,
        "the premise: the candidate must differ in the pack's COMPLETE identity"
    );

    let outcome = request_reload(
        app.world_mut(),
        ambition_content_pack::CandidateGeneration::prepared_against(
            std::sync::Arc::clone(&candidate),
            Some(live.fingerprint),
        ),
    );
    match &outcome {
        ReloadRequest::Refused(MoveReload::RefusedUnsupportedChangedDomain(domains)) => assert!(
            domains.iter().any(|d| d == "item_catalog"),
            "the refusal does not name the domain that changed: {domains:?}"
        ),
        other => panic!(
            "an items-only candidate must be refused on the REQUEST road until \
             items participate in the generation transaction; got {other:?}"
        ),
    }
    // And the shell gets no command, so no epoch, publication or reconstruction
    // is spent.
    assert!(
        issued_commands(&mut app).is_empty(),
        "a refused request reached the shell anyway"
    );
    assert!(
        crate::reload::pending_pack(app.world()).is_none(),
        "a refused request staged the candidate as pending"
    );
}

/// Nothing the composition does after the request can refuse the commit.
///
/// The commit carries the value admission computed, so it cannot refuse. The
/// arm changes the world under the pending generation and checks that the
/// generation still lands. Both changes (technique table shrinks, table
/// removed) are covered.
#[test]
fn no_change_to_the_technique_table_after_the_request_can_refuse_the_commit() {
    const KEY: &str = "reload.fixture.technique";
    fn table_with(keys: &[&str]) -> ambition_combat::technique::InstalledTechniques {
        let mut support = ambition_entity_catalog::TechniqueSupport::default();
        for key in keys {
            support
                .declare(
                    (*key).to_string(),
                    ambition_entity_catalog::TechniqueOffer {
                        owner: "reload_fixture",
                        params: ambition_entity_catalog::TechniqueParams::Checked(|_| Ok(())),
                        references: ambition_entity_catalog::NestedReferences::None,
                        delivery: ambition_entity_catalog::TechniqueDelivery::Either,
                    },
                )
                .expect("one declaration per key");
        }
        ambition_combat::technique::InstalledTechniques(support)
    }

    // ── "shrink" removes the key the candidate's strike names; "remove" drops
    //    the whole table ─────────────────────────────────────────────────────
    for sabotage in ["shrink", "remove"] {
        let mut app = host_with_a_live_cast();
        let _ = reload_move_tables_selecting(
            app.world_mut(),
            std::sync::Arc::new(pack_of(&doc_text(0.2)).expect("compiles")));
        shell_active_on(&mut app, true);
        app.add_systems(
        bevy::app::Update,
        (adopt_preparation_transaction, commit_content_generation)
            .chain(),
    );
        app.world_mut().insert_resource(table_with(&[KEY]));

        let candidate = ambition_content_pack::CandidateGeneration::prepared_against(
            std::sync::Arc::new(
                pack_of(&doc_text_naming(0.45, "swat", Some(KEY))).expect("compiles"),
            ),
            None,
        );
        let requested = request_reload(app.world_mut(), candidate);
        assert!(
            matches!(requested, ReloadRequest::Requested { .. }),
            "the premise ({sabotage}): the candidate must be ADMITTED at request \
             time, or this arm never reaches the boundary; got {requested:?}"
        );
        let mine = a_preparation_for(&mut app, "shell.game.1");
        let cast_before = live_duration(&app);
        let selection_before = crate::pack::selected(app.world())
            .expect("a selection")
            .fingerprint;

        // The composition changes under the pending generation.
        if sabotage == "shrink" {
            app.world_mut().insert_resource(table_with(&[]));
        } else {
            app.world_mut()
                .remove_resource::<ambition_combat::technique::InstalledTechniques>();
        }
        app.world_mut()
            .write_message(ambition_platformer2d::game_shell::ShellEvent::RouteActivated(mine));
        app.update();

        assert_ne!(
            live_duration(&app),
            cast_before,
            "the commit refused the cast ({sabotage}) — admission was re-asked at \
             the boundary instead of carried"
        );
        assert_ne!(
            crate::pack::selected(app.world())
                .expect("a selection")
                .fingerprint,
            selection_before,
            "the commit refused the pack ({sabotage})"
        );
        assert!(
            crate::reload::pending_pack(app.world()).is_none(),
            "the commit left the candidate PENDING ({sabotage})"
        );
    }
}

/// A pending generation does not overwrite the App's content identity.
///
/// Preparation fingerprints against an identity. If a reload in flight set
/// `SelectedContentIdentity` to N+1, an unrelated route preparation in that
/// window would take the candidate's stamp. The rollback timeline contract
/// compares that identity.
///
/// Nothing is claimed before the router names the transaction, which is
/// correct.
#[test]
fn a_pending_generation_claims_its_own_transaction_and_not_the_apps_identity() {
    let mut app = host_with_a_live_cast();
    let _ = reload_move_tables_selecting(
        app.world_mut(),
        std::sync::Arc::new(pack_of(&doc_text(0.2)).expect("compiles")));
    shell_active_on(&mut app, true);
    app.add_systems(
        bevy::app::Update,
        (adopt_preparation_transaction, commit_content_generation)
            .chain(),
    );
    let live_identity = app
        .world()
        .resource::<ambition_platformer2d_runtime::SelectedContentIdentity>()
        .0
        .clone();

    assert!(matches!(
        request_reload(app.world_mut(), a_publishable_candidate()),
        ReloadRequest::Requested { .. }
    ));
    assert_eq!(
        app.world()
            .resource::<ambition_platformer2d_runtime::SelectedContentIdentity>()
            .0,
        live_identity,
        "staging a candidate moved the APP's content identity, so every \
         preparation in flight now fingerprints against a pack this App has not \
         selected"
    );
    assert!(
        app.world()
            .get_resource::<ambition_platformer2d_runtime::PendingGenerationInputs>()
            .is_none(),
        "a claim was staked before the router named the transaction, so it names \
         no transaction at all"
    );

    // The router announces the transaction; the claim appears, bound to it.
    let mine = a_preparation_for(&mut app, "shell.game.1");
    let claim = app
        .world()
        .get_resource::<ambition_platformer2d_runtime::PendingGenerationInputs>()
        .cloned()
        .expect("adopting the transaction stakes the claim");
    let candidate_identity = crate::pack::identity_line(
        crate::reload::pending_pack(app.world()).expect("a pending generation"),
    );
    assert_eq!(
        claim.identity, candidate_identity,
        "the claim is not the candidate's identity"
    );
    assert_eq!(
        claim.identity_for("shell.game.1"),
        Some(candidate_identity.as_str()),
        "the owning transaction cannot read its own claim"
    );
    // The main assertion.
    assert_eq!(
        claim.identity_for("shell.menu.4"),
        None,
        "an unrelated transaction reads the candidate's identity, which is the \
         defect this value exists to remove"
    );
    assert_eq!(
        app.world()
            .resource::<ambition_platformer2d_runtime::SelectedContentIdentity>()
            .0,
        live_identity,
        "adopting the transaction moved the APP's identity"
    );

    // The claim does not outlive its generation: a claim naming a spent `LoadId`
    // would fingerprint a retry against a discarded candidate.
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellEvent::RouteActivated(mine));
    app.update();
    assert!(
        app.world()
            .get_resource::<ambition_platformer2d_runtime::PendingGenerationInputs>()
            .is_none(),
        "the claim outlived the generation that staked it"
    );
    assert_ne!(
        app.world()
            .resource::<ambition_platformer2d_runtime::SelectedContentIdentity>()
            .0,
        live_identity,
        "the activation published the pack without moving the App's identity"
    );
}

/// A second request is refused while one is in flight. Otherwise a second
/// request could replace the pending pack before adoption, and the second
/// generation would adopt the first request's `LoadId`. A watcher makes close
/// saves common.
#[test]
fn a_second_request_is_refused_while_a_generation_is_pending() {
    let mut app = host_with_a_live_cast();
    let _ = reload_move_tables_selecting(
        app.world_mut(),
        std::sync::Arc::new(pack_of(&doc_text(0.2)).expect("compiles")));
    shell_active_on(&mut app, true);
    assert!(matches!(
        request_reload(app.world_mut(), a_publishable_candidate()),
        ReloadRequest::Requested { .. }
    ));
    let first = crate::reload::pending_pack(app.world())
        .expect("a pending generation")
        .fingerprint;

    let second = ambition_content_pack::CandidateGeneration::prepared_against(
        std::sync::Arc::new(pack_of(&doc_text(0.66)).expect("compiles")),
        None,
    );
    let outcome = request_reload(app.world_mut(), second);
    assert_eq!(
        outcome,
        ReloadRequest::AlreadyPending {
            route: "game".to_string()
        },
        "got {outcome:?}"
    );
    assert_eq!(
        crate::reload::pending_pack(app.world())
            .expect("a pending generation")
            .fingerprint,
        first,
        "the refused second request replaced the pending generation anyway"
    );
}

/// The request road refuses a composition with no technique table up front.
///
/// Absent is not empty: an absent `InstalledTechniques` means combat is not
/// installed, so nothing can admit the revision. Letting the request through
/// would leave the pending pack staged forever after the engine's half
/// moved.
#[test]
fn a_composition_with_no_technique_table_refuses_the_request() {
    let mut app = host_with_a_live_cast();
    let _ = reload_move_tables_selecting(
        app.world_mut(),
        std::sync::Arc::new(pack_of(&doc_text(0.2)).expect("compiles")));
    shell_active_on(&mut app, true);
    app.world_mut()
        .remove_resource::<ambition_combat::technique::InstalledTechniques>();

    let outcome = request_reload(app.world_mut(), a_publishable_candidate());
    assert_eq!(
        outcome,
        ReloadRequest::Refused(MoveReload::NoTechniqueSupport),
        "got {outcome:?}"
    );
    assert!(
        issued_commands(&mut app).is_empty(),
        "a refused request reached the shell anyway"
    );
    assert!(
        crate::reload::pending_pack(app.world()).is_none(),
        "a refused request staged the candidate as pending"
    );
}

/// An activation this reload did not ask for cannot publish it.
///
/// Both transactions target the same route. A route name cannot separate them
/// (a repeated `ReplaceWith("game")`, a retry, navigating back); only the load
/// id the router minted differs.
#[test]
fn an_activation_of_another_transaction_cannot_publish_a_pending_reload() {
    let mut app = host_with_a_live_cast();
    let _ = reload_move_tables_selecting(
        app.world_mut(),
        std::sync::Arc::new(pack_of(&doc_text(0.2)).expect("compiles")));
    shell_active_on(&mut app, true);
    app.add_systems(
        bevy::app::Update,
        (adopt_preparation_transaction, commit_content_generation)
            .chain(),
    );
    let before = live_duration(&app);

    assert!(matches!(
        request_reload(app.world_mut(), a_publishable_candidate()),
        ReloadRequest::Requested { .. }
    ));
    // The reload adopts the transaction the router announces for it.
    let mine = a_preparation_for(&mut app, "shell.game.7");

    // A different transaction on the same route activates first.
    let mut theirs = mine.clone();
    theirs.load_authorization = Some(ambition_platformer2d::load::LoadBarrierRef::new(
        ambition_platformer2d::load::LoadId::new("shell.game.8"),
        ambition_platformer2d::load::LoadBarrierId::new("publish"),
    ));
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellEvent::RouteActivated(theirs));
    app.update();
    assert_eq!(
        live_duration(&app),
        before,
        "another transaction's activation published a reload it did not own"
    );

    // Control: the reload's own activation still publishes, so the refusal above
    // is correlation, not a reload that never works.
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellEvent::RouteActivated(mine));
    app.update();
    assert_ne!(
        live_duration(&app),
        before,
        "the reload's own activation did not publish it"
    );
}

/// A failure of another transaction cannot discard it either. The failing
/// transaction names itself in `TransactionEnded` by request and by barrier,
/// and neither is ours.
#[test]
fn a_failure_of_another_transaction_cannot_discard_a_pending_reload() {
    let mut app = host_with_a_live_cast();
    let _ = reload_move_tables_selecting(
        app.world_mut(),
        std::sync::Arc::new(pack_of(&doc_text(0.2)).expect("compiles")));
    shell_active_on(&mut app, true);
    app.add_systems(
        bevy::app::Update,
        (adopt_preparation_transaction, commit_content_generation)
            .chain(),
    );
    let before = live_duration(&app);

    assert!(matches!(
        request_reload(app.world_mut(), a_publishable_candidate()),
        ReloadRequest::Requested { .. }
    ));
    let mine = a_preparation_for(&mut app, "shell.game.7");

    // Another transaction is the one the router waits on, and it fails.
    app.world_mut().resource_mut::<ShellRouter>().pending =
        Some(ambition_platformer2d::game_shell::PendingShellRoute {
            reserved_activation: ambition_platformer2d::game_shell::ShellActivationId(1),
            route_id: ShellRouteId::new("game"),
            push_history: false,
            barrier: ambient_barrier("shell.game.8"),
            requires_prepared_session: true,
            terminal_reported: true,
            // A foreign request id, not `None`, so the transaction is clearly someone
            // else's.
            request: Some(
                ambition_platformer2d::game_shell::ShellRequestId::new(
                    "someone.else.1",
                ),
            ),
        });
    app.world_mut().write_message(
        ambition_platformer2d::game_shell::ShellEvent::TransactionEnded {
            route_id: ShellRouteId::new("game"),
            barrier: ambient_barrier("shell.game.8"),
            request: Some(
                ambition_platformer2d::game_shell::ShellRequestId::new(
                    "someone.else.1",
                ),
            ),
            reason: ambition_platformer2d::game_shell::TransactionEnd::Failed,
        },
    );
    app.update();

    // The staged revision survived: its own activation still publishes it.
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellEvent::RouteActivated(mine));
    app.update();
    assert_ne!(
        live_duration(&app),
        before,
        "another transaction's failure discarded a reload it did not own"
    );
}

/// An unrelated rejection that arrives while our load is pending must not
/// discard the reload. Nothing in `CommandRejected` names our transaction.
///
/// The premise is asserted first; if the router is not waiting on our barrier,
/// the arm tests nothing.
#[test]
fn an_unrelated_rejection_while_our_own_load_is_pending_keeps_the_reload() {
    let mut app = host_with_a_live_cast();
    let _ = reload_move_tables_selecting(
        app.world_mut(),
        std::sync::Arc::new(pack_of(&doc_text(0.2)).expect("compiles")));
    shell_active_on(&mut app, true);
    app.add_systems(
        bevy::app::Update,
        (adopt_preparation_transaction, commit_content_generation)
            .chain(),
    );
    let before = live_duration(&app);

    let ReloadRequest::Requested { request, .. } =
        request_reload(app.world_mut(), a_publishable_candidate())
    else {
        panic!("the reload was not requested");
    };
    let mine = a_preparation_carrying(
        &mut app,
        ambient_barrier("shell.game.7"),
        Some(request.clone()),
    );
    app.update();

    // Premise: our own transaction is the one the router waits on.
    app.world_mut().resource_mut::<ShellRouter>().pending =
        Some(ambition_platformer2d::game_shell::PendingShellRoute {
            reserved_activation: ambition_platformer2d::game_shell::ShellActivationId(1),
            route_id: ShellRouteId::new("game"),
            push_history: false,
            barrier: ambient_barrier("shell.game.7"),
            requires_prepared_session: true,
            terminal_reported: false,
            request: Some(request),
        });
    assert!(
        reload_adopted_the_pending_load(&app, "shell.game.7"),
        "the fixture never reached the state it is testing: the reload had not \
         adopted the load the router is waiting on"
    );

    // Something unrelated is rejected.
    app.world_mut().write_message(
        ambition_platformer2d::game_shell::ShellEvent::CommandRejected(
            ambition_platformer2d::game_shell::ShellCommandRejection::HostNotConfigured,
        ),
    );
    app.update();

    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellEvent::RouteActivated(mine));
    app.update();
    assert_ne!(
        live_duration(&app),
        before,
        "an unrelated rejection discarded a reload it says nothing about"
    );
}

/// A superseded reload is told, and the next request is accepted. Without a
/// supersession event, the staged generation would wait forever and
/// [`ReloadRequest::AlreadyPending`] would refuse every later save.
///
/// The second candidate must differ from the selected pack; otherwise
/// `request_reload` returns `Unchanged` before the pending check.
#[test]
fn a_superseded_reload_is_told_and_stops_refusing_later_requests() {
    let mut app = host_with_a_live_cast();
    let _ = reload_move_tables_selecting(
        app.world_mut(),
        std::sync::Arc::new(pack_of(&doc_text(0.2)).expect("compiles")));
    shell_active_on(&mut app, true);
    app.add_systems(
        bevy::app::Update,
        (adopt_preparation_transaction, commit_content_generation)
            .chain(),
    );

    let ReloadRequest::Requested { request, .. } =
        request_reload(app.world_mut(), a_publishable_candidate())
    else {
        panic!("the reload was not requested");
    };
    // Premise: while one is pending, a second is refused.
    assert!(
        matches!(
            request_reload(app.world_mut(), a_publishable_candidate()),
            ReloadRequest::AlreadyPending { .. }
        ),
        "the fixture cannot show a release: nothing was being refused"
    );

    app.world_mut().write_message(
        ambition_platformer2d::game_shell::ShellEvent::TransactionEnded {
            route_id: ShellRouteId::new("game"),
            barrier: ambient_barrier("shell.game.superseding"),
            request: Some(request),
            reason: ambition_platformer2d::game_shell::TransactionEnd::Superseded,
        },
    );
    app.update();

    // The slot is free.
    let again = request_reload(app.world_mut(), a_publishable_candidate());
    assert!(
        matches!(again, ReloadRequest::Requested { .. }),
        "a superseded reload still holds the slot, so no later save can land: \
         {again:?}"
    );
}

/// Has the pending reload adopted the load `load` names?
///
/// Premise check for every correlation arm. Adoption is a system, so it can be
/// missed by one frame or a missing schedule edge.
fn reload_adopted_the_pending_load(app: &bevy::app::App, load: &str) -> bool {
    app.world()
        .get_resource::<PendingGeneration>()
        .and_then(|pending| pending.load_id.clone())
        .is_some_and(|adopted| adopted.as_str() == load)
}

/// The production road's staleness refusal.
///
/// On the request road `admit_candidate` runs first, so a candidate whose base
/// no longer matches the selection is refused before anything is staged. This
/// is the only staleness refusal (see `MoveReload::StaleGeneration`).
///
/// The sequence is what a watcher produces: read the live identity, do file
/// I/O, return late after another publication.
#[test]
fn a_candidate_prepared_against_a_pack_that_is_no_longer_selected_is_refused() {
    let mut app = host_with_a_live_cast();
    let _ = reload_move_tables_selecting(
        app.world_mut(),
        std::sync::Arc::new(pack_of(&doc_text(0.2)).expect("compiles")));
    shell_active_on(&mut app, true);
    app.add_systems(
        bevy::app::Update,
        (adopt_preparation_transaction, commit_content_generation)
            .chain(),
    );
    // What our compile read before it started.
    let compiled_against = crate::pack::selected(app.world())
        .expect("a selection")
        .fingerprint;

    // Someone else publishes while our compile is in flight.
    let theirs = std::sync::Arc::new(pack_of(&doc_text(0.3)).expect("compiles"));
    assert!(
        matches!(
            reload_move_tables_selecting(
                app.world_mut(),
                std::sync::Arc::clone(&theirs)),
            MoveReload::Activated { .. }
        ),
        "the premise: the selection actually moved under us"
    );
    let active = crate::pack::selected(app.world())
        .expect("a selection")
        .fingerprint;
    assert_ne!(
        compiled_against, active,
        "the fixture superseded nothing, so the arm below would refuse a \
         candidate whose base was still current and prove nothing"
    );
    // Captured after their publication, so the final assertion measures only
    // our change.
    let before = live_duration(&app);

    // Our pack, compiled before that, arrives late on the production road.
    let ours = std::sync::Arc::new(pack_of(&doc_text(0.45)).expect("compiles"));
    let outcome = request_reload(
        app.world_mut(),
        ambition_content_pack::CandidateGeneration::prepared_against(
            std::sync::Arc::clone(&ours),
            Some(compiled_against),
        ),
    );
    assert!(
        matches!(
            &outcome,
            ReloadRequest::Refused(MoveReload::StaleGeneration {
                prepared_against,
                active: reported,
            }) if prepared_against == &compiled_against.hex()
                && reported == &active.hex()
        ),
        "a candidate prepared against a pack that is no longer selected was not \
         refused by the request road: {outcome:?}"
    );

    // Nothing moved. A refusal that staged, selected or requested would be half
    // a transaction, and `AlreadyPending` would refuse every later save.
    assert!(
        crate::reload::pending_pack(app.world()).is_none(),
        "a refused candidate was left staged, so no later save can land"
    );
    assert!(
        std::ptr::eq(
            crate::pack::selected(app.world()).expect("a selection"),
            std::sync::Arc::as_ref(&theirs)
        ),
        "a refused candidate became the App's selection anyway"
    );
    assert!(
        issued_commands(&mut app).is_empty(),
        "a refused candidate asked the shell to re-prepare: {:?}",
        issued_commands(&mut app)
    );

    // The same pack, re-based, is accepted: the refusal is about the base, and
    // the documented remedy (re-read and retry) works.
    let again = request_reload(
        app.world_mut(),
        ambition_content_pack::CandidateGeneration::prepared_against(ours, Some(active)),
    );
    assert!(
        matches!(again, ReloadRequest::Requested { .. }),
        "re-basing the same candidate on the live selection was still refused, \
         so the refusal is not about staleness: {again:?}"
    );
    assert_eq!(
        live_duration(&app),
        before,
        "one of these two requests published the cast without an activation"
    );
}

fn ambient_barrier(load: &str) -> ambition_platformer2d::load::LoadBarrierRef {
    ambition_platformer2d::load::LoadBarrierRef::new(
        ambition_platformer2d::load::LoadId::new(load),
        ambition_platformer2d::load::LoadBarrierId::new("publish"),
    )
}

/// A request that never activates discards its staged revision; otherwise the
/// next activation would apply it. The staleness check cannot catch this,
/// because nothing was published and the base is still current.
#[test]
fn a_request_that_fails_discards_its_staged_revision() {
    let mut app = host_with_a_live_cast();
    let _ = reload_move_tables_selecting(
        app.world_mut(),
        std::sync::Arc::new(pack_of(&doc_text(0.2)).expect("compiles")));
    shell_active_on(&mut app, true);
    app.add_systems(
        bevy::app::Update,
        (adopt_preparation_transaction, commit_content_generation)
            .chain(),
    );
    let before = live_duration(&app);

    let ReloadRequest::Requested { request, .. } =
        request_reload(app.world_mut(), a_publishable_candidate())
    else {
        panic!("the reload was not requested");
    };
    // The transaction fails before any preparation exists.
    // `PendingGeneration.load_id` is still `None`, so only the caller's request id
    // can correlate this failure (`reload_owns` cannot).
    app.world_mut().write_message(
        ambition_platformer2d::game_shell::ShellEvent::TransactionEnded {
            route_id: ShellRouteId::new("game"),
            barrier: ambient_barrier("shell.game.never"),
            request: Some(request),
            reason: ambition_platformer2d::game_shell::TransactionEnd::Failed,
        },
    );
    app.update();
    assert_eq!(
        live_duration(&app),
        before,
        "a failed preparation published the cast anyway"
    );
    // Direct check that the generation is gone. The other two assertions stay
    // green even when the terminal handler does nothing, because an unadopted
    // generation publishes nothing anyway. Only this tells "discarded" from
    // "stranded".
    assert!(
        crate::reload::pending_pack(app.world()).is_none(),
        "a failed transaction left its generation pending, so every later save \
         is refused as AlreadyPending"
    );

    // The next activation must not apply it either.
    let active = app
        .world()
        .resource::<ShellRouter>()
        .active
        .clone()
        .expect("still active");
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellEvent::RouteActivated(active));
    app.update();
    assert_eq!(
        live_duration(&app),
        before,
        "a discarded revision was applied by a LATER activation"
    );
}

/// A composition with no game shell must not panic.
///
/// A `MessageReader` for an unregistered message fails parameter validation
/// and panics the schedule. Every other arm here registers `ShellEvent`, so
/// only this arm covers the default-feature case.
///
/// It goes through `reload::register`, not `add_systems`, because the run
/// condition is part of the registration.
#[test]
fn a_composition_with_no_game_shell_does_not_panic() {
    let mut app = bevy::app::App::new();
    crate::reload::register(&mut app);
    assert!(
        app.world()
            .get_resource::<bevy::ecs::message::Messages<
                ambition_platformer2d::game_shell::ShellEvent,
            >>()
            .is_none(),
        "the fixture registered ShellEvent, so it is not the shell-less case"
    );
    app.update();
    app.update();
}

/// Control: with the shell present the same registration does run. Without
/// it, a condition that never passes would satisfy "does not panic".
#[test]
fn the_same_registration_runs_once_the_shell_is_present() {
    let mut app = host_with_a_live_cast();
    let _ = reload_move_tables_selecting(
        app.world_mut(),
        std::sync::Arc::new(pack_of(&doc_text(0.2)).expect("compiles")));
    shell_active_on(&mut app, true);
    crate::reload::register(&mut app);
    let before = live_duration(&app);

    assert!(matches!(
        request_reload(app.world_mut(), a_publishable_candidate()),
        ReloadRequest::Requested { .. }
    ));
    let active = a_preparation_for(&mut app, "shell.game.1");
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellEvent::RouteActivated(active));
    app.update();
    assert_ne!(
        live_duration(&app),
        before,
        "the gated registration never let the system run"
    );
}

/// A complete no-op under a healthy live rollback timeline is `Unchanged`, not
/// a rollback refusal. An identical candidate publishes nothing and cannot
/// invalidate a timeline, so the verdict must be asked before the boundary. A
/// watcher fires on every save, so this is the common case.
#[test]
fn a_complete_no_op_under_a_live_timeline_is_unchanged_not_refused() {
    let mut app = host_with_a_live_cast();
    let pack = std::sync::Arc::new(pack_of(&doc_text(0.2)).expect("compiles"));
    let _ = reload_move_tables_selecting(app.world_mut(), std::sync::Arc::clone(&pack));
    app.world_mut().insert_resource(live_authority());
    let generation = app
        .world()
        .resource::<PreparedCharacterRegistry>()
        .generation();

    let outcome = publish_candidate(
        app.world_mut(),
        ambition_content_pack::CandidateGeneration::prepared_against(
            std::sync::Arc::clone(&pack),
            None,
        ));
    assert!(
        matches!(outcome, MoveReload::Unchanged { .. }),
        "a no-op was refused for a timeline it could not have disturbed: {outcome:?}"
    );
    assert_eq!(
        app.world()
            .resource::<PreparedCharacterRegistry>()
            .generation(),
        generation,
        "the no-op moved the cast generation"
    );

    // Control: a changed candidate under the same timeline is still refused, so
    // this tests the order, not a missing boundary.
    let changed = publish_candidate(
        app.world_mut(),
        ambition_content_pack::CandidateGeneration::prepared_against(
            std::sync::Arc::new(pack_of(&doc_text(0.45)).expect("compiles")),
            None,
        ));
    assert_eq!(
        changed,
        MoveReload::RefusedDuringLiveTimeline,
        "got {changed:?}"
    );
}

/// The same on the request road.
#[test]
fn a_complete_no_op_under_a_live_timeline_requests_nothing_rather_than_refusing() {
    let mut app = host_with_a_live_cast();
    let pack = std::sync::Arc::new(pack_of(&doc_text(0.2)).expect("compiles"));
    let _ = reload_move_tables_selecting(app.world_mut(), std::sync::Arc::clone(&pack));
    shell_active_on(&mut app, true);
    app.world_mut().insert_resource(live_authority());

    let outcome = request_reload(
        app.world_mut(),
        ambition_content_pack::CandidateGeneration::prepared_against(pack, None),
    );
    assert_eq!(outcome, ReloadRequest::Unchanged, "got {outcome:?}");
    assert!(issued_commands(&mut app).is_empty());
}

/// A failed preparation must not leave the candidate selected.
///
/// ```text
/// N is live
/// request N+1        -> SelectedContentPack became N+1
/// preparation fails  -> cast and session stay N, selection stays N+1
/// save N+1 again     -> the verdict compares against the SELECTION, reports
///                       Unchanged, and requests nothing
/// ```
///
/// The candidate stays pending until activation promotes it. A failure
/// discards it and the engine's identity stays on the live pack.
///
/// The fourth assertion is the key one: the first three would also hold if a
/// fix only restored the selection. Re-submitting the same candidate must
/// still be a change.
#[test]
fn a_failed_preparation_does_not_leave_the_candidate_selected_or_silently_unchanged() {
    let mut app = host_with_a_live_cast();
    let live = std::sync::Arc::new(pack_of(&doc_text(0.2)).expect("compiles"));
    let _ = reload_move_tables_selecting(app.world_mut(), std::sync::Arc::clone(&live));
    shell_active_on(&mut app, true);
    app.add_systems(
        bevy::app::Update,
        (adopt_preparation_transaction, commit_content_generation)
            .chain(),
    );
    let selected = crate::pack::selected(app.world())
        .expect("a selection")
        .fingerprint;

    let candidate = std::sync::Arc::new(pack_of(&doc_text(0.45)).expect("compiles"));
    assert_ne!(candidate.fingerprint, selected, "the premise: it differs");
    let ReloadRequest::Requested { request, .. } = request_reload(
        app.world_mut(),
        ambition_content_pack::CandidateGeneration::prepared_against(
            std::sync::Arc::clone(&candidate),
            None,
        ),
    ) else {
        panic!("the reload was not requested");
    };
    // Not selected yet, although preparation must be able to read it.
    assert_eq!(
        crate::pack::selected(app.world())
            .expect("a selection")
            .fingerprint,
        selected,
        "the request installed the candidate as the App's selection"
    );

    // The preparation fails and names this transaction. A bare
    // `ExperienceFailed` carries only an activation id and matches nobody.
    app.world_mut().write_message(
        ambition_platformer2d::game_shell::ShellEvent::TransactionEnded {
            route_id: ShellRouteId::new("game"),
            barrier: ambient_barrier("shell.game.never"),
            request: Some(request),
            reason: ambition_platformer2d::game_shell::TransactionEnd::Failed,
        },
    );
    app.update();
    assert_eq!(
        crate::pack::selected(app.world())
            .expect("a selection")
            .fingerprint,
        selected,
        "a failed preparation left the candidate selected"
    );
    assert!(
        crate::reload::pending_pack(app.world()).is_none(),
        "a failed preparation left the candidate pending"
    );
    assert_eq!(
        app.world()
            .resource::<ambition_platformer2d_runtime::SelectedContentIdentity>()
            .0,
        format!("{} {} {}", live.id, live.version, live.fingerprint),
        "a failed preparation left the ENGINE fingerprinting against a pack this \
         App does not have"
    );

    // Main assertion: the same candidate, submitted again, is still a change.
    let again = request_reload(
        app.world_mut(),
        ambition_content_pack::CandidateGeneration::prepared_against(candidate, None),
    );
    assert!(
        matches!(again, ReloadRequest::Requested { .. }),
        "re-submitting the candidate after a failed preparation reported \
         {again:?} — the developer is told nothing changed while the game is split"
    );
}

/// Control: a successful activation does promote the pending candidate.
/// Without it, a road that never selects would pass "not selected yet".
#[test]
fn a_successful_activation_promotes_the_pending_candidate() {
    let mut app = host_with_a_live_cast();
    let _ = reload_move_tables_selecting(
        app.world_mut(),
        std::sync::Arc::new(pack_of(&doc_text(0.2)).expect("compiles")));
    shell_active_on(&mut app, true);
    app.add_systems(
        bevy::app::Update,
        (adopt_preparation_transaction, commit_content_generation)
            .chain(),
    );
    let before = crate::pack::selected(app.world())
        .expect("a selection")
        .fingerprint;

    assert!(matches!(
        request_reload(app.world_mut(), a_publishable_candidate()),
        ReloadRequest::Requested { .. }
    ));
    let active = a_preparation_for(&mut app, "shell.game.1");
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellEvent::RouteActivated(active));
    app.update();

    assert_ne!(
        crate::pack::selected(app.world())
            .expect("a selection")
            .fingerprint,
        before,
        "the activation did not promote the pending candidate"
    );
    assert!(
        crate::reload::pending_pack(app.world()).is_none(),
        "the candidate is still pending after it was promoted"
    );
}

/// A candidate that would fail admission is refused at request time, so the
/// commit boundary never has to.
///
/// At `RouteActivated` the shell has already committed the new route and
/// session, so a refusal there would leave half a transaction (I3).
///
/// Nothing is left behind: the refusal discards the staged revision and the
/// pending pack.
#[test]
fn a_candidate_that_would_fail_admission_is_refused_before_the_request_is_issued() {
    let mut app = host_with_a_live_cast();
    let _ = reload_move_tables_selecting(
        app.world_mut(),
        std::sync::Arc::new(pack_of(&doc_text(0.2)).expect("compiles")));
    shell_active_on(&mut app, true);
    let selection = crate::pack::selected(app.world())
        .expect("a selection")
        .fingerprint;
    let generation = app
        .world()
        .resource::<PreparedCharacterRegistry>()
        .generation();

    let named = doc_text_naming(0.35, "swat", Some("nothing.installed"));
    assert!(
        named.contains("nothing.installed"),
        "the fixture does not name the uninstalled technique"
    );
    let outcome = request_reload(
        app.world_mut(),
        ambition_content_pack::CandidateGeneration::prepared_against(
            std::sync::Arc::new(pack_of(&named).expect(
                "a candidate naming an \
                 uninstalled technique still COMPILES",
            )),
            None,
        ),
    );
    match &outcome {
        ReloadRequest::Refused(MoveReload::Refused(refusals)) => assert!(
            refusals.iter().any(|r| r.contains("nothing.installed")),
            "the refusal does not name the uninstalled technique: {refusals:?}"
        ),
        other => panic!("expected a request-time admission refusal; got {other:?}"),
    }

    // The request never reached the shell.
    assert!(
        issued_commands(&mut app).is_empty(),
        "a candidate that cannot be admitted asked the shell to re-prepare anyway"
    );
    // Nothing is staged or pending.
    assert!(
        crate::reload::pending_pack(app.world()).is_none(),
        "the refused candidate is still pending"
    );
    assert_eq!(
        crate::pack::selected(app.world())
            .expect("a selection")
            .fingerprint,
        selection,
        "the refused candidate became the selection"
    );
    assert_eq!(
        app.world()
            .resource::<PreparedCharacterRegistry>()
            .generation(),
        generation,
        "the refused candidate moved the cast generation"
    );

    // The next activation publishes nothing, which proves the discard
    // happened.
    app.add_systems(
        bevy::app::Update,
        (adopt_preparation_transaction, commit_content_generation)
            .chain(),
    );
    let active = app
        .world()
        .resource::<ShellRouter>()
        .active
        .clone()
        .expect("active");
    let played = live_duration(&app);
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellEvent::RouteActivated(active));
    app.update();
    assert_eq!(
        live_duration(&app),
        played,
        "an unrelated activation published the candidate that was refused"
    );
}

// ---------------------------------------------------------------------------
// The second mechanical content family.
//
// These arms validate that the transaction absorbs a second family with no
// new authority. The family is `fighter_brain_ladder`, not items: items still
// use a process-global `OnceLock` and return `&'static str`, so they cannot
// participate without a signature change (see
// `a_candidate_that_changes_only_items_is_refused_as_an_unsupported_domain`).
// The ladder is one declared source with one lowering site, published as a
// plain `AuthoredFighterLadder` resource.
// ---------------------------------------------------------------------------

const LADDER_PATH: &str = "data/fighter_brain_ladder.ron";

/// The shipped pack with level one's reaction latency one millisecond faster.
///
/// One field on one rung. `FighterBrainLadder::problems` requires monotone
/// reaction and no instant reaction. 500 → 499 stays above level 2's 450 and
/// above zero, so the candidate is valid and only different.
fn pack_with_a_faster_first_rung() -> ambition_content_pack::PreparedContentPack {
    let mut edited = false;
    let pack = crate::pack::compile_pack_with(|declared, text| {
        if declared != LADDER_PATH {
            return text;
        }
        let out = text.replacen("reaction_ms: 500.0", "reaction_ms: 499.0", 1);
        edited = out != text;
        out
    })
    .expect("the edited pack compiles");
    // Floor on the edit: a renamed source or a retuned level 1 would leave the
    // shipped pack unchanged and the arm testing nothing.
    assert!(
        edited,
        "`{LADDER_PATH}` no longer carries `reaction_ms: 500.0`, so the candidate \
         is the shipped pack and this witness would pass vacuously"
    );
    pack
}

fn live_first_rung(app: &bevy::app::App) -> f32 {
    app.world()
        .resource::<ambition_characters::brain::fighter::AuthoredFighterLadder>()
        .0
        .level(1)
        .expect("the shipped ladder has a level 1")
        .reaction_ms
}

/// The ladder participates, and it lands at the same boundary as the cast.
///
/// The host installs the ladder as `AmbitionContentPlugin::build` does, cloned
/// from the boot pack. Starting with no resource would also pass for a road
/// that only inserts.
#[test]
fn the_fighter_ladder_is_the_second_family_the_transaction_carries() {
    let mut app = host_with_the_shipped_cast();
    app.world_mut()
        .insert_resource(ambition_characters::brain::fighter::AuthoredFighterLadder(
            ambition_combat::brain::fighter::content_schema::lowered_fighter_brain_ladder(
                crate::pack::prepared(),
            )
            .cloned()
            .expect("the shipped pack lowers its ladder"),
        ));
    shell_active_on(&mut app, true);
    app.add_systems(
        bevy::app::Update,
        (adopt_preparation_transaction, commit_content_generation)
            .chain(),
    );

    let live = live_pack(&app);
    let candidate = std::sync::Arc::new(pack_with_a_faster_first_rung());
    assert_eq!(
        ambition_content_pack::changed_domains(&live, &candidate)
            .iter()
            .map(|s| s.0.as_str())
            .collect::<Vec<_>>(),
        vec!["fighter_brain_ladder"],
        "the premise: ONLY the ladder changed, so a refusal or a publication is \
         about this family and nothing else"
    );
    assert_eq!(
        live_first_rung(&app),
        500.0,
        "the premise: the live ladder is the shipped one"
    );

    // A ladder-only edit is requested, not refused as an unsupported domain.
    assert!(
        matches!(
            request_reload(
                app.world_mut(),
                ambition_content_pack::CandidateGeneration::prepared_against(
                    std::sync::Arc::clone(&candidate),
                    Some(live.fingerprint),
                ),
            ),
            ReloadRequest::Requested { .. }
        ),
        "a ladder-only candidate was not accepted as a request"
    );
    assert_eq!(
        live_first_rung(&app),
        500.0,
        "the REQUEST published the ladder on the spot instead of staging it — the \
         old generation must stay authoritative until the new one activates"
    );

    let mine = a_preparation_for(&mut app, "shell.game.1");
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellEvent::RouteActivated(mine));
    app.update();

    assert_eq!(
        live_first_rung(&app),
        499.0,
        "the route activated and the ladder stayed at generation N — the pack \
         promoted while the family it declares did not, which is the \
         half-transaction this road exists to prevent"
    );
    assert!(
        std::ptr::eq(
            crate::pack::selected(app.world()).expect("a selection"),
            std::sync::Arc::as_ref(&candidate)
        ),
        "the candidate did not become the selection"
    );
}

/// A candidate that declares no ladder removes the resource; it does not keep
/// generation N's.
///
/// The moveset family cannot express removal (see
/// `dropped_moveset_entities`), but the ladder can: `profile_for_level` takes
/// an `Option`, and absent means the engine floor.
///
/// Asked of the function directly: the shipped `pack.ron` always declares the
/// ladder and the schema requires nine rungs, so no `compile_pack_with` edit
/// removes it. A synthetic moveset-only pack does.
#[test]
fn a_candidate_that_declares_no_ladder_removes_the_live_one() {
    let mut world = bevy::ecs::world::World::new();
    world.insert_resource(ambition_characters::brain::fighter::AuthoredFighterLadder(
        ambition_combat::brain::fighter::content_schema::lowered_fighter_brain_ladder(
            crate::pack::prepared(),
        )
        .cloned()
        .expect("the shipped pack lowers its ladder"),
    ));
    let ladderless = pack_of(&doc_text(0.2)).expect("the synthetic pack compiles");
    assert!(
        ambition_combat::brain::fighter::content_schema::lowered_fighter_brain_ladder(&ladderless)
            .is_none(),
        "the premise: this candidate declares no ladder"
    );

    crate::reload::publish_participant_families(&mut world, &ladderless);

    assert!(
        world
            .get_resource::<ambition_characters::brain::fighter::AuthoredFighterLadder>()
            .is_none(),
        "the candidate declares no ladder and generation N's rungs are still live, \
         so `profile_for_level` reports authored difficulty the pack no longer \
         carries"
    );
}

const WAVES_PATH: &str = "data/encounters/goblin_encounter.ron";

/// The shipped pack with one mob's spawn delay 50ms later.
///
/// A wave's `delay` is mechanical: it sets when the body appears. A `label`
/// change would alter the lowered artifact but not the fight.
fn pack_with_a_later_second_goblin() -> ambition_content_pack::PreparedContentPack {
    let mut edited = false;
    let pack = crate::pack::compile_pack_with(|declared, text| {
        if declared != WAVES_PATH {
            return text;
        }
        let out = text.replacen("delay: 0.70", "delay: 0.75", 1);
        edited = out != text;
        out
    })
    .expect("the edited pack compiles");
    assert!(
        edited,
        "`{WAVES_PATH}` no longer carries `delay: 0.70`, so the candidate is the \
         shipped pack and this witness would pass vacuously"
    );
    pack
}

fn live_second_goblin_delay(app: &bevy::app::App) -> f32 {
    app.world()
        .resource::<ambition_encounter::EncounterWaveBook>()
        .waves("goblin_encounter")
        .expect("the shipped book authors the goblin encounter")[1]
        .mobs[1]
        .delay
}

/// The third family (`encounter_waves`) also joined with no new authority:
/// one row in `PACK_DERIVED_FAMILIES`, one publisher, the existing boundary.
/// This arm fails if accepting it needed a new resource, system or refusal.
///
/// The host installs the book as `AmbitionContentPlugin::build` does, for the
/// same reason as the ladder arm.
#[test]
fn the_encounter_wave_book_is_the_third_family_the_transaction_carries() {
    let mut app = host_with_the_shipped_cast();
    app.world_mut()
        .insert_resource(ambition_encounter::EncounterWaveBook(
            ambition_encounter::content_schema::lowered_encounter_waves(crate::pack::prepared())
                .cloned()
                .expect("the shipped pack lowers its wave book"),
        ));
    shell_active_on(&mut app, true);
    app.add_systems(
        bevy::app::Update,
        (adopt_preparation_transaction, commit_content_generation)
            .chain(),
    );

    let live = live_pack(&app);
    let candidate = std::sync::Arc::new(pack_with_a_later_second_goblin());
    assert_eq!(
        ambition_content_pack::changed_domains(&live, &candidate)
            .iter()
            .map(|s| s.0.as_str())
            .collect::<Vec<_>>(),
        vec!["encounter_waves"],
        "the premise: ONLY the wave book changed"
    );
    assert_eq!(
        live_second_goblin_delay(&app),
        0.70,
        "the premise: the live book is the shipped one"
    );

    assert!(
        matches!(
            request_reload(
                app.world_mut(),
                ambition_content_pack::CandidateGeneration::prepared_against(
                    std::sync::Arc::clone(&candidate),
                    Some(live.fingerprint),
                ),
            ),
            ReloadRequest::Requested { .. }
        ),
        "a waves-only candidate was not accepted as a request"
    );
    assert_eq!(
        live_second_goblin_delay(&app),
        0.70,
        "the REQUEST published the wave book instead of staging it"
    );

    let mine = a_preparation_for(&mut app, "shell.game.1");
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellEvent::RouteActivated(mine));
    app.update();

    assert_eq!(
        live_second_goblin_delay(&app),
        0.75,
        "the route activated and the wave book stayed at generation N — the pack \
         promoted while the family it declares did not"
    );
}

/// Every row of `PACK_DERIVED_FAMILIES` is reached by one publication.
///
/// The per-family arms above would stay green if a sibling's publisher were
/// dropped. This publishes the shipped pack into a world with neither resource
/// and requires every declared family to arrive. It calls the function
/// directly because the subject is the table, not the boundary.
#[test]
fn publishing_a_generation_installs_every_pack_derived_family() {
    let mut world = bevy::ecs::world::World::new();
    assert!(
        world
            .get_resource::<ambition_characters::brain::fighter::AuthoredFighterLadder>()
            .is_none()
            && world
                .get_resource::<ambition_encounter::EncounterWaveBook>()
                .is_none(),
        "the premise: this world holds no family yet, so an arrival is this \
         function's doing and not a leftover"
    );

    crate::reload::publish_participant_families(&mut world, crate::pack::prepared());

    assert!(
        world
            .get_resource::<ambition_characters::brain::fighter::AuthoredFighterLadder>()
            .is_some(),
        "`fighter_brain_ladder` is declared in PACK_DERIVED_FAMILIES and the \
         shipped pack lowers it, but publishing a generation did not install it"
    );
    assert!(
        world
            .get_resource::<ambition_encounter::EncounterWaveBook>()
            .is_some(),
        "`encounter_waves` is declared in PACK_DERIVED_FAMILIES and the shipped \
         pack lowers it, but publishing a generation did not install it"
    );
}

/// A generation that changes no moveset does not need the combat capability.
///
/// A ladder-only or waves-only edit must not stage move tables or require
/// `InstalledTechniques`. See `moveset_changed`.
///
/// The host here installs no `InstalledTechniques` on purpose
/// (`host_with_the_shipped_cast` adds one).
#[test]
fn a_ladder_only_generation_needs_no_technique_table() {
    let mut app = host_with_the_shipped_cast();
    // Premise: without this removal the request succeeds for the ordinary
    // reason.
    assert!(
        app.world_mut()
            .remove_resource::<ambition_combat::technique::InstalledTechniques>()
            .is_some(),
        "the fixture did not install a technique table, so removing it proves \
         nothing about a generation that does not need one"
    );
    app.world_mut()
        .insert_resource(ambition_characters::brain::fighter::AuthoredFighterLadder(
            ambition_combat::brain::fighter::content_schema::lowered_fighter_brain_ladder(
                crate::pack::prepared(),
            )
            .cloned()
            .expect("the shipped pack lowers its ladder"),
        ));
    shell_active_on(&mut app, true);
    app.add_systems(
        bevy::app::Update,
        (adopt_preparation_transaction, commit_content_generation).chain(),
    );

    let live = live_pack(&app);
    let candidate = std::sync::Arc::new(pack_with_a_faster_first_rung());
    assert_eq!(
        ambition_content_pack::changed_domains(&live, &candidate)
            .iter()
            .map(|s| s.0.as_str())
            .collect::<Vec<_>>(),
        vec!["fighter_brain_ladder"],
        "the premise: this generation changes NO moveset, so nothing on it \
         should consult the technique table"
    );

    let outcome = request_reload(
        app.world_mut(),
        ambition_content_pack::CandidateGeneration::prepared_against(
            std::sync::Arc::clone(&candidate),
            Some(live.fingerprint),
        ),
    );
    assert!(
        matches!(outcome, ReloadRequest::Requested { .. }),
        "a ladder-only generation was refused by the combat capability's \
         absence: {outcome:?}"
    );

    let mine = a_preparation_for(&mut app, "shell.game.1");
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellEvent::RouteActivated(mine));
    app.update();
    assert_eq!(
        live_first_rung(&app),
        499.0,
        "the generation was requested and then published nothing"
    );
}

/// Control: the moveset family still requires it. Without this, a road that
/// stopped asking would pass the arm above.
#[test]
fn a_moveset_generation_still_needs_a_technique_table() {
    let mut app = host_with_the_shipped_cast();
    assert!(
        app.world_mut()
            .remove_resource::<ambition_combat::technique::InstalledTechniques>()
            .is_some(),
        "the premise: the fixture installed one"
    );
    shell_active_on(&mut app, true);
    let live = live_pack(&app);
    let candidate = std::sync::Arc::new(pack_with_every_move_retimed());
    assert!(
        ambition_content_pack::changed_domains(&live, &candidate)
            .iter()
            .any(|s| s.0 == "moveset"),
        "the premise: this generation DOES change the moveset"
    );
    let outcome = request_reload(
        app.world_mut(),
        ambition_content_pack::CandidateGeneration::prepared_against(
            candidate,
            Some(live.fingerprint),
        ),
    );
    assert!(
        matches!(
            outcome,
            ReloadRequest::Refused(MoveReload::NoTechniqueSupport)
        ),
        "a moveset generation was let through without a technique table to \
         admit its authored effects against: {outcome:?}"
    );
}

/// A transaction for the reload's own route that the reload did not issue is
/// not adopted.
///
/// Two `ReplaceWith("game")` in one frame mint `shell.game.N` and
/// `shell.game.N+1`, and `start_route` cancels the first. A route-matching
/// reload could adopt the dead load and wait forever. This arm sends a
/// `PreparationRequested` for the same route with another request id.
#[test]
fn a_same_route_transaction_from_another_caller_is_not_adopted() {
    let mut app = host_with_a_live_cast();
    let _ = reload_move_tables_selecting(
        app.world_mut(),
        std::sync::Arc::new(pack_of(&doc_text(0.2)).expect("compiles")));
    shell_active_on(&mut app, true);
    app.add_systems(
        bevy::app::Update,
        (adopt_preparation_transaction, commit_content_generation).chain(),
    );
    let before = live_duration(&app);
    assert!(matches!(
        request_reload(app.world_mut(), a_publishable_candidate()),
        ReloadRequest::Requested { .. }
    ));

    // Premise: the reload minted a correlator, and this transaction carries a
    // different one.
    let mine = issued_commands(&mut app)
        .into_iter()
        .find_map(|command| match command {
            ShellCommand::ReplaceWith { request, .. } => request,
            _ => None,
        })
        .expect("the reload minted a request id and put it on its command");
    let theirs = ambition_platformer2d::game_shell::ShellRequestId::new("someone.else.1");
    assert_ne!(mine, theirs, "the premise: two different callers");

    let barrier = ambition_platformer2d::load::LoadBarrierRef::new(
        ambition_platformer2d::load::LoadId::new("shell.game.7"),
        ambition_platformer2d::load::LoadBarrierId::new("publish"),
    );
    app.world_mut().write_message(
        ambition_platformer2d::game_shell::ShellEvent::PreparationRequested(
            ambition_platformer2d::game_shell::ProviderLoadTransaction {
                route_id: ShellRouteId::new("game"),
                experience_id: ambition_platformer2d::game_shell::ShellExperienceId::new("other"),
                barrier: barrier.clone(),
                request: Some(theirs),
            },
        ),
    );
    app.update();

    // The stranger's transaction activates. Route-matching adoption would have
    // adopted it above and published here.
    let mut active = app
        .world()
        .resource::<ShellRouter>()
        .active
        .clone()
        .expect("active on a route");
    active.load_authorization = Some(barrier);
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellEvent::RouteActivated(active));
    app.update();
    assert_eq!(
        live_duration(&app),
        before,
        "the reload adopted a transaction for its route that it did not issue, \
         and published its generation on somebody else's activation"
    );

    // The reload still waits for its own transaction, so the arm is about
    // correlation, not a lost generation.
    let mine_now = a_preparation_carrying(
        &mut app,
        ambition_platformer2d::load::LoadBarrierRef::new(
            ambition_platformer2d::load::LoadId::new("shell.game.1"),
            ambition_platformer2d::load::LoadBarrierId::new("publish"),
        ),
        Some(mine),
    );
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellEvent::RouteActivated(mine_now));
    app.update();
    assert_ne!(
        live_duration(&app),
        before,
        "the reload refused a stranger's transaction and then failed to publish \
         on its OWN — so the refusal above discarded the generation instead of \
         declining to adopt"
    );
}

/// A transaction with no correlator is not a wildcard. Every navigation
/// command writes `None`, so matching it would restore route-based
/// inference.
#[test]
fn an_uncorrelated_transaction_is_not_adopted_either() {
    let mut app = host_with_a_live_cast();
    let _ = reload_move_tables_selecting(
        app.world_mut(),
        std::sync::Arc::new(pack_of(&doc_text(0.2)).expect("compiles")));
    shell_active_on(&mut app, true);
    app.add_systems(
        bevy::app::Update,
        (adopt_preparation_transaction, commit_content_generation).chain(),
    );
    let before = live_duration(&app);
    assert!(matches!(
        request_reload(app.world_mut(), a_publishable_candidate()),
        ReloadRequest::Requested { .. }
    ));

    let barrier = ambition_platformer2d::load::LoadBarrierRef::new(
        ambition_platformer2d::load::LoadId::new("shell.game.9"),
        ambition_platformer2d::load::LoadBarrierId::new("publish"),
    );
    app.world_mut().write_message(
        ambition_platformer2d::game_shell::ShellEvent::PreparationRequested(
            ambition_platformer2d::game_shell::ProviderLoadTransaction {
                route_id: ShellRouteId::new("game"),
                experience_id: ambition_platformer2d::game_shell::ShellExperienceId::new("nav"),
                barrier: barrier.clone(),
                request: None,
            },
        ),
    );
    app.update();
    let mut active = app
        .world()
        .resource::<ShellRouter>()
        .active
        .clone()
        .expect("active on a route");
    active.load_authorization = Some(barrier);
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellEvent::RouteActivated(active));
    app.update();
    assert_eq!(
        live_duration(&app),
        before,
        "an uncorrelated navigation transaction was adopted as the reload's own"
    );
}

// ── Publication legality across the transaction interval ─────────────────
//
// `admit_candidate` checks `publication_boundary` once, at request time. The
// generation then waits in `PendingGeneration` until `RouteActivated`, and
// `commit_content_generation` cannot refuse. These arms change the rollback
// authority inside that interval.
//
// They install the authority directly. They do not show that the shipped
// lifecycle produces those transitions in that window (`Q118`).
//
// The fix is not a fallible commit: by then the shell's half is at its
// boundary, so a refusal would activate the route at N+1 with the cast at N.
// The lease breaker cancels early instead.

/// A foreign timeline (`live_authority()` with no `RollbackSessionOwnership`)
/// that goes live mid-flight cancels the pending generation. `admit_candidate`
/// would have refused that world. The sibling
/// `a_healthy_timeline_this_host_maintains_going_live_mid_flight_still_publishes`
/// must not cancel, or hot reload is lost.
#[test]
fn a_foreign_timeline_that_goes_live_mid_flight_cancels_the_pending_generation() {
    let mut app = host_with_a_live_cast();
    let _ = reload_move_tables_selecting(
        app.world_mut(),
        std::sync::Arc::new(pack_of(&doc_text(0.2)).expect("compiles")));
    shell_active_on(&mut app, true);
    app.add_systems(
        bevy::app::Update,
        (
            adopt_preparation_transaction,
            // Same order as `register`: before the commit.
            break_the_publication_lease_when_the_boundary_closes,
            commit_content_generation,
        )
            .chain(),
    );
    let before = live_duration(&app);

    // Premise: it was legal when asked.
    assert!(
        app.world().get_resource::<ActiveRollbackAuthority>().is_none(),
        "the fixture already has an authority, so the request below would be \
         refused and this arm would measure nothing",
    );
    assert!(matches!(
        request_reload(app.world_mut(), a_publishable_candidate()),
        ReloadRequest::Requested { .. }
    ));

    let active = a_preparation_for(&mut app, "shell.game.1");
    // The interval: a speculating timeline appears after the request was
    // accepted and before the activation commits.
    app.world_mut().insert_resource(live_authority());
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellEvent::RouteActivated(active));
    app.update();

    // The breaker cancels: this host may not rebase a foreign timeline.
    assert_eq!(
        live_duration(&app),
        before,
        "a generation published across a healthy timeline that went live \
         mid-flight and that this host may NOT rebase. `admit_candidate` would \
         have refused that same world a moment earlier; publishing here hands \
         new content to a timeline nobody is permitted to stop.",
    );
    assert!(
        !app.world().contains_resource::<PendingGeneration>(),
        "the illegal generation is still pending",
    );
}

/// The other half: a timeline this host maintains goes live mid-flight and the
/// generation still publishes. A reload re-prepares the current route, so the
/// replaced session normally owns a healthy speculating timeline. A breaker
/// that cancelled here would disable hot reload.
#[test]
fn a_healthy_timeline_this_host_maintains_going_live_mid_flight_still_publishes() {
    use ambition_platformer2d::rollback::{
        RollbackSessionOwnership, SyncTestOwner, SyncTestSettings,
    };

    let mut app = host_with_a_live_cast();
    let _ = reload_move_tables_selecting(
        app.world_mut(),
        std::sync::Arc::new(pack_of(&doc_text(0.2)).expect("compiles")),
    );
    shell_active_on(&mut app, true);
    app.add_systems(
        bevy::app::Update,
        (
            adopt_preparation_transaction,
            break_the_publication_lease_when_the_boundary_closes,
            commit_content_generation,
        )
            .chain(),
    );
    let before = live_duration(&app);

    assert!(matches!(
        request_reload(app.world_mut(), a_publishable_candidate()),
        ReloadRequest::Requested { .. }
    ));
    let active = a_preparation_for(&mut app, "shell.game.1");
    app.world_mut().insert_resource(live_authority());
    app.world_mut()
        .insert_resource(RollbackSessionOwnership::LocalSyncTest {
            settings: SyncTestSettings::for_players(1),
            owner: SyncTestOwner::LocalMaintainer,
        });
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellEvent::RouteActivated(active));
    app.update();

    assert_ne!(
        live_duration(&app),
        before,
        "the lease cancelled a reload across a healthy timeline THIS HOST \
         MAINTAINS, which is the one composition that ships hot reload — this is \
         the measured regression the first publication breaker caused"
    );
}

/// Ownership changes under the transaction.
///
/// Admission lets a generation past a healthy timeline only because this host
/// may rebase it. Here the request is admitted against a `LocalMaintainer`
/// session and an `External`/P2P session replaces it before the commit.
/// `rebase_local_timeline_onto_the_new_generation` correctly does nothing to a
/// foreign timeline, so publishing would desync. The lease must cancel.
#[test]
fn losing_the_permission_to_rebase_mid_flight_cancels_the_pending_generation() {
    use ambition_platformer2d::rollback::{
        RollbackSessionOwnership, SyncTestOwner, SyncTestSettings,
    };

    let mut app = host_with_a_live_cast();
    let _ = reload_move_tables_selecting(
        app.world_mut(),
        std::sync::Arc::new(pack_of(&doc_text(0.2)).expect("compiles")),
    );
    shell_active_on(&mut app, true);
    app.add_systems(
        bevy::app::Update,
        (
            adopt_preparation_transaction,
            break_the_publication_lease_when_the_boundary_closes,
            commit_content_generation,
        )
            .chain(),
    );
    let before = live_duration(&app);

    // Premise: the world is admissible at request time (a healthy timeline this
    // host maintains).
    app.world_mut().insert_resource(live_authority());
    app.world_mut()
        .insert_resource(RollbackSessionOwnership::LocalSyncTest {
            settings: SyncTestSettings::for_players(1),
            owner: SyncTestOwner::LocalMaintainer,
        });
    assert!(matches!(
        request_reload(app.world_mut(), a_publishable_candidate()),
        ReloadRequest::Requested { .. }
    ));

    let active = a_preparation_for(&mut app, "shell.game.1");
    // The interval: the permission is withdrawn. Health does not change.
    app.world_mut()
        .insert_resource(RollbackSessionOwnership::External);
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellEvent::RouteActivated(active));
    app.update();

    assert_eq!(
        live_duration(&app),
        before,
        "the generation published after this host LOST permission to rebase the \
         timeline it was admitted against. The timeline is still healthy, so a \
         lease that re-asks only health sees nothing — and the rebase at the \
         commit declines to touch a foreign session, leaving new content on a \
         timeline nobody rebased.",
    );
    assert!(
        !app.world().contains_resource::<PendingGeneration>(),
        "the illegal generation is still pending",
    );
}

/// Control: the breaker on a transaction whose boundary never closed does
/// nothing. The generation publishes, no cancel is written, and the live cast
/// moves. A breaker that refused every reload would pass the mid-flight arms.
#[test]
fn a_lease_that_was_never_broken_publishes_exactly_as_before() {
    let mut app = host_with_a_live_cast();
    let _ = reload_move_tables_selecting(
        app.world_mut(),
        std::sync::Arc::new(pack_of(&doc_text(0.2)).expect("compiles")),
    );
    shell_active_on(&mut app, true);
    app.add_systems(
        bevy::app::Update,
        (
            adopt_preparation_transaction,
            break_the_publication_lease_when_the_boundary_closes,
            commit_content_generation,
        )
            .chain(),
    );
    let before = live_duration(&app);

    assert!(matches!(
        request_reload(app.world_mut(), a_publishable_candidate()),
        ReloadRequest::Requested { .. }
    ));
    let active = a_preparation_for(&mut app, "shell.game.1");
    // No authority is installed; that is the only difference from the arms this
    // controls.
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellEvent::RouteActivated(active));
    app.update();

    assert_ne!(
        live_duration(&app),
        before,
        "the lease cancelled a transaction whose boundary never closed, so the \
         two mid-flight arms beside it prove nothing",
    );
    assert!(
        !app.world()
            .resource::<bevy::ecs::message::Messages<ambition_platformer2d::game_shell::ShellCommand>>()
            .iter_current_update_messages()
            .any(|command| matches!(
                command,
                ambition_platformer2d::game_shell::ShellCommand::CancelPending { .. }
            )),
        "a legal transaction was cancelled",
    );
}

/// An authority that goes unhealthy while a generation is pending cancels the
/// generation. An unhealthy authority is a recorded divergence, and
/// publishing must not hide it
/// (`publishing_does_not_heal_an_unhealthy_rollback_authority`).
#[test]
fn an_authority_that_goes_unhealthy_mid_flight_cancels_the_pending_generation() {
    let mut app = host_with_a_live_cast();
    let _ = reload_move_tables_selecting(
        app.world_mut(),
        std::sync::Arc::new(pack_of(&doc_text(0.2)).expect("compiles")));
    shell_active_on(&mut app, true);
    app.add_systems(
        bevy::app::Update,
        (
            adopt_preparation_transaction,
            break_the_publication_lease_when_the_boundary_closes,
            commit_content_generation,
        )
            .chain(),
    );
    let before = live_duration(&app);

    assert!(
        app.world().get_resource::<ActiveRollbackAuthority>().is_none(),
        "the fixture already has an authority, so the request below would be \
         refused and this arm would measure nothing",
    );
    assert!(matches!(
        request_reload(app.world_mut(), a_publishable_candidate()),
        ReloadRequest::Requested { .. }
    ));

    let active = a_preparation_for(&mut app, "shell.game.1");
    let mut authority = live_authority();
    authority.invalidate("a desync that arrived mid-flight".to_string());
    app.world_mut().insert_resource(authority);
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellEvent::RouteActivated(active));
    app.update();

    // The generation did not publish into the recorded divergence.
    assert_eq!(
        live_duration(&app),
        before,
        "a generation published across an authority that went unhealthy \
         mid-flight, laundering a recorded desync",
    );
    assert!(
        !app.world().contains_resource::<PendingGeneration>(),
        "the illegal generation is still pending",
    );
    assert!(
        !app.world()
            .resource::<ActiveRollbackAuthority>()
            .status()
            .is_healthy(),
        "the publication healed the authority, which is a DIFFERENT and worse \
         defect than the one this arm is about",
    );
}

/// A live timeline this host owns is rebased, not refused (`Q118`).
///
/// A reload re-prepares the current route, so the replaced session normally
/// owns a healthy speculating timeline; refusing would disable hot reload. The
/// LDtk road already stops the local baseline and releases ownership, so
/// `maintain_local_session` starts the next one on the live content.
/// `RollbackSessionOwnership` allows this for local sync-test sessions.
///
/// Control: the same authority with no local ownership is still
/// `RefusedDuringLiveTimeline`.
#[test]
fn a_live_timeline_this_host_maintains_is_rebased_rather_than_refused() {
    use ambition_platformer2d::rollback::{
        RollbackSessionOwnership, SyncTestOwner, SyncTestSettings,
    };

    let (mut app, before) = host_with_authority(Some(live_authority()));
    app.world_mut()
        .insert_resource(RollbackSessionOwnership::LocalSyncTest {
            settings: SyncTestSettings::for_players(1),
            owner: SyncTestOwner::LocalMaintainer,
        });

    let outcome = publish_candidate(app.world_mut(), a_publishable_candidate());
    assert!(
        matches!(outcome, MoveReload::Activated { .. }),
        "a healthy timeline THIS HOST OWNS refused the publication, which deletes \
         hot reload in the one composition that ships it: {outcome:?}"
    );
    assert_ne!(
        live_duration(&app),
        before,
        "the reload was admitted and published nothing"
    );
}

/// Which timelines this host may rebase, for every ownership.
///
/// A locally maintained sync test is this process's to stop. An `External`
/// session belongs to peers and must never be replaced by the local host. A
/// `Caller`-owned one belongs to a match activation or a harness that did not
/// ask for a rebase. No ownership resource (fixture/headless) is also not
/// rebasable.
#[test]
fn only_a_locally_maintained_sync_test_may_be_rebased_for_a_publication() {
    use ambition_platformer2d::rollback::{
        RollbackSessionOwnership, SyncTestOwner, SyncTestSettings,
    };

    let mut app = bevy::app::App::new();
    assert!(
        !crate::reload::rebasable_local_timeline(app.world()),
        "a composition that installs no rollback ownership at all was called \
         rebasable, so the check is reading absence as permission"
    );

    app.world_mut()
        .insert_resource(RollbackSessionOwnership::External);
    assert!(
        !crate::reload::rebasable_local_timeline(app.world()),
        "an EXTERNAL/P2P timeline was called rebasable — peers need a coordinated \
         content barrier and this host would have stopped their session"
    );

    app.world_mut()
        .insert_resource(RollbackSessionOwnership::LocalSyncTest {
            settings: SyncTestSettings::for_players(1),
            owner: SyncTestOwner::Caller,
        });
    assert!(
        !crate::reload::rebasable_local_timeline(app.world()),
        "a CALLER-owned sync test was called rebasable — a match activation or a \
         harness started it and did not ask for a content rebase"
    );

    app.world_mut()
        .insert_resource(RollbackSessionOwnership::LocalSyncTest {
            settings: SyncTestSettings::for_players(1),
            owner: SyncTestOwner::LocalMaintainer,
        });
    assert!(
        crate::reload::rebasable_local_timeline(app.world()),
        "the one timeline this host started and may rebuild was refused, which \
         puts hot reload back where `Q118` found it"
    );
}

/// Each transaction holds its own route under its own id (`Q118`).
///
/// Adoption takes a hold on the route and registers this transaction's hold
/// id with the publication gate. No check releases the hold; the gate answers
/// at activation and the shell consumes the hold there.
///
/// The id must be transaction-specific. `ShellRouteHolds` is keyed
/// `route → set<hold id>`, so with a constant id a late cleanup from
/// transaction A could free successor B's block.
///
/// This arm asserts the id (`content-publication:<request>`) and the release
/// on the terminal path. An arm with two live transactions on one route (A
/// supersedes B) is still owed; see the queue row.
#[test]
fn each_reload_transaction_holds_its_route_under_its_own_id() {
    use ambition_platformer2d::game_shell::{
        ShellActivationGates, ShellRouteHolds, ShellRouteId,
    };

    let mut app = host_with_a_live_cast();
    app.init_resource::<ShellRouteHolds>();
    app.init_resource::<ShellActivationGates>();
    let evaluator = app
        .world_mut()
        .register_system(crate::reload::answer_the_publication_gate);
    app.insert_resource(crate::reload::PublicationGateEvaluator(evaluator));
    app.add_systems(bevy::app::Update, adopt_preparation_transaction);

    // Premise: nothing holds the route before the transaction exists.
    let route = ShellRouteId::new("game");
    assert!(
        !app.world().resource::<ShellRouteHolds>().is_held(&route),
        "the route was already held before any transaction adopted it",
    );

    let _ = reload_move_tables_selecting(
        app.world_mut(),
        std::sync::Arc::new(pack_of(&doc_text(0.2)).expect("compiles")),
    );
    shell_active_on(&mut app, true);
    assert!(matches!(
        request_reload(app.world_mut(), a_publishable_candidate()),
        ReloadRequest::Requested { .. }
    ));
    let first_request = app
        .world()
        .resource::<crate::reload::PendingGeneration>()
        .request_for_tests();
    let _ = a_preparation_for(&mut app, "shell.game.1");
    app.update();

    let held = app.world().resource::<ShellRouteHolds>().held(&route);
    let first_hold = format!("content-publication:{}", first_request.as_str());
    assert!(
        held.iter().any(|hold| hold.as_str() == first_hold),
        "adoption did not hold the route for its own transaction (held: {held:?}), \
         so the shell can activate it with nothing asked about the boundary",
    );
    assert!(
        app.world()
            .resource::<ShellActivationGates>()
            .evaluator(&ambition_platformer2d::game_shell::ShellHoldId::new(
                first_hold.clone()
            ))
            .is_some(),
        "the hold was taken with no evaluator registered for it, so the shell \
         would block the route forever instead of asking",
    );

    // Terminal path: a transaction that ends without activating must release
    // its hold, or every later reload of the route is blocked.
    crate::reload::take_pending_generation_for_tests(app.world_mut());
    assert!(
        !app.world().resource::<ShellRouteHolds>().is_held(&route),
        "a transaction that ended without activating left its hold behind, so \
         every future reload of this route is blocked forever",
    );
}

/// Records the remaining `Q118` interval.
///
/// ```text
/// publication breaker checks the authority
///         ↓
/// the authority changes
///         ↓
/// the shell activates the route
///         ↓
/// the content commit sees RouteActivated and assumes there is nothing to refuse
/// ```
///
/// The commit must stay infallible (a fallible commit would activate the route
/// at N+1 with the cast at N). So the interval is narrowed only by ordering.
///
/// This arm inserts a boundary change after the breaker and before the commit.
/// The generation currently publishes, and the arm asserts that. When one
/// activation authority owns the whole boundary, `Q118` is closed and this
/// assertion must flip.
#[test]
fn a_boundary_that_closes_after_the_breaker_still_publishes() {
    use ambition_platformer2d::rollback::{
        RollbackSessionOwnership, SyncTestOwner, SyncTestSettings,
    };

    let mut app = host_with_a_live_cast();
    let _ = reload_move_tables_selecting(
        app.world_mut(),
        std::sync::Arc::new(pack_of(&doc_text(0.2)).expect("compiles")),
    );
    shell_active_on(&mut app, true);

    /// The test-only mutation, ordered into the interval: after the breaker
    /// checks and before the commit acts on the activation.
    ///
    /// It stays disarmed until the request is adopted. `a_preparation_for` runs
    /// updates, and an armed mutation would close the boundary before the breaker
    /// ran, so the breaker would cancel and the arm would test the fixture.
    #[derive(bevy::prelude::Resource, Default)]
    struct ArmTheInterval(bool);

    fn close_the_boundary_behind_the_breakers_back(world: &mut bevy::ecs::world::World) {
        if !world.resource::<ArmTheInterval>().0 {
            return;
        }
        world.insert_resource(RollbackSessionOwnership::External);
    }

    app.add_systems(
        bevy::app::Update,
        (
            adopt_preparation_transaction,
            break_the_publication_lease_when_the_boundary_closes,
            close_the_boundary_behind_the_breakers_back,
            commit_content_generation,
        )
            .chain(),
    );
    app.init_resource::<ArmTheInterval>();
    let before = live_duration(&app);

    // Premise: the world is admissible at request time (a healthy timeline this
    // host maintains).
    app.world_mut().insert_resource(live_authority());
    app.world_mut()
        .insert_resource(RollbackSessionOwnership::LocalSyncTest {
            settings: SyncTestSettings::for_players(1),
            owner: SyncTestOwner::LocalMaintainer,
        });
    assert!(matches!(
        request_reload(app.world_mut(), a_publishable_candidate()),
        ReloadRequest::Requested { .. }
    ));

    let active = a_preparation_for(&mut app, "shell.game.1");
    // Arm only now, so every earlier frame sees the admissible world.
    app.world_mut().resource_mut::<ArmTheInterval>().0 = true;
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellEvent::RouteActivated(active));
    app.update();

    // Premise of the poison: the boundary really closed.
    assert!(
        !crate::reload::rebasable_local_timeline(app.world()),
        "the test mutation did not close the boundary, so this arm measures a \
         legal publication rather than the interval"
    );

    // Nothing cancelled it: the breaker had already run on a legal world.
    assert!(
        !app.world()
            .resource::<bevy::ecs::message::Messages<ambition_platformer2d::game_shell::ShellCommand>>()
            .iter_current_update_messages()
            .any(|command| matches!(
                command,
                ambition_platformer2d::game_shell::ShellCommand::CancelPending { .. }
            )),
        "the transaction WAS cancelled, so this arm is measuring a breaker that \
         saw the closed boundary rather than the interval after it"
    );
    assert_ne!(
        live_duration(&app),
        before,
        "MEASURED GAP CLOSED? This arm records that a boundary closing AFTER the \
         publication breaker has checked it does NOT stop the generation \
         publishing — the interval `Q118` still owes an owner for. If the \
         generation now stays unpublished, an activation barrier owns the whole \
         boundary: name it, close `Q118`, and make this the opposite assertion.",
    );
}
