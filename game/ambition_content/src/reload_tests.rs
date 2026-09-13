//! ⭐ THE CLAIM: a pack read as text reaches a LIVE cast, and a pack that does
//! not compile reaches nothing.
//!
//! ⛔ A SMALL SYNTHETIC PACK, not the shipped one. The shipped roster's ids are
//! the nineteen this provider registers, so a witness built on it would need the
//! whole cast staged before it could say anything — and the subject here is the
//! RELOAD, not the roster. The section's fidelity over the real nineteen is
//! `moves_are_content`'s job.
//!
//! ⛔⛔ **AND THE DOCUMENT TEXT IS SERIALIZED FROM THE REAL TYPES, NOT HAND
//! WRITTEN.** A hand-written RON fixture encodes my guess at `MoveSpec`'s shape;
//! when the shape changes the fixture stops PARSING and the test fails for a
//! reason that has nothing to do with reloading. The EDIT is still a text
//! substitution on those bytes, so what this exercises is a changed FILE.

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
/// ⚠ `recover_s` IS WHAT THE EDIT MOVES, because it lands in `duration_s` — one
/// number, visible in the published moveset, that no other part of the fixture
/// derives.
fn doc_text(recover_s: f32) -> String {
    doc_text_bound_to(recover_s, "swat")
}

/// The same document with `attack` bound to `verb_target`, so a test can author
/// a verb pointing at a move that does not exist.
///
/// ⛔⛤ **A PARAMETER RATHER THAN A TEXT SUBSTITUTION, AND THE SUBSTITUTION IS
/// WHY.** Replacing `"swat"` in the serialized bytes hit the verb binding AND
/// the move's own id, producing a document that was still internally consistent
/// — so the arm asserting "this pack must be refused" failed against a pack that
/// compiled perfectly. A fixture edit that lands in two places is the same
/// family as a poison that does not apply.
fn doc_text_bound_to(recover_s: f32, verb_target: &str) -> String {
    doc_text_naming(recover_s, verb_target, None)
}

/// The same document whose strike lands `on_hit` technique `technique`.
///
/// ⛔⛤ **A PARAMETER, BECAUSE THE TEXT SUBSTITUTION I TRIED FIRST DID NOT
/// MATCH.** `to_ron` pretty-prints `on_hit: None` with a space and I wrote
/// `on_hit:None`, so the edit applied to nothing. The premise assert caught it —
/// which is the whole reason a fixture edit gets one.
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

/// The same probe pack with NO moveset source declared at all.
///
/// ⛔⛤ **THE REMOVED-SECTION TRANSITION CANNOT BE AUTHORED INTO THE SHIPPED
/// PACK, WHICH IS WHY THIS EXISTS.** MEASURED 2026-09-12: emptying a table is
/// refused (`data/movesets/author.ron` declares the `moveset` schema and carries
/// no move contract for any of its 0 entities), and so is removing a table's only
/// entity — the compiler closes both. What it does NOT close is a manifest that
/// stops declaring the family, and `game/ambition_demo_smash/assets/pack.ron`
/// ships exactly that shape. So the transition is real, reachable by editing a
/// manifest, and unreachable through `compile_pack_with` — which edits source
/// TEXT and never the declaration list.
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

/// ⭐ THE PREMISE. Without it every arm below is a claim about a host that never
/// published the character they are all about.
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

/// ⭐⭐ **A FILE THE HOST READ AFTER IT BOOTED CHANGES WHAT THE CAST PLAYS.**
/// Nothing between the text and the published registry is a compile step — this
/// is the whole of fast-iteration I2's promise, witnessed end to end.
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

/// ⛔ AND THE EDIT IS WHAT ARRIVED, not a constant. Without this, a reload that
/// republished the same table every time would pass the arm above.
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

/// ⛔⛔ **RELOADING THE SAME FILE TWICE DOES NOT MOVE THE GENERATION.** This is
/// the case a watcher produces constantly — a save with no edit — and the one
/// `RevisionOutcome::Unchanged` exists for. A generation is what every staleness
/// check in the session keys on, so publishing a no-op would invalidate live
/// bodies and cached plans for nothing.
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

/// ⛔ A PACK THAT DOES NOT COMPILE NEVER REACHES THE HOST, and the refusal names
/// the unresolved target rather than saying "invalid".
///
/// ⚠ THE REFUSAL IS THE COMPILER'S, raised before a world is involved at all —
/// which is the layering this road is for: an author learns a verb is unbound
/// from the pack, not from a fighter standing still in a match.
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

/// ⛔⛔ **A HOST THAT HAS NOT CLOSED ITS PREPARATION BARRIER IS TOLD SO**, and is
/// not told its content is wrong. A reload road that reported this as an unknown
/// character would send an author to edit files over a lifecycle fact about the
/// caller.
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

/// ⛔ A SECTION NAMING A CHARACTER THIS BUILD NEVER PREPARED STAGES NONE OF IT,
/// and the report names who.
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

/// ⛔⛤ **ABSENT IS NOT EMPTY.** A world with no `InstalledTechniques` has not
/// installed the combat capability at all; `unwrap_or_default()` there would
/// report a roster-wide technique refusal for a composition that was never asked
/// the question.
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
    // ⛔ AND NO `InstalledTechniques`, which is the whole point.
    let outcome = reload_move_tables_from(
        app.world_mut(),
        &pack_of(&doc_text(0.2)).expect("compiles"));
    assert_eq!(outcome, MoveReload::NoTechniqueSupport, "got {outcome:?}");
}

// ⛔⛤ **THREE ARMS THAT WITNESSED THE CAST STALENESS CLOCK WERE DELETED ON
// 2026-09-12** — `a_pack_compiled_against_an_older_cast_is_refused`, its control
// `a_pack_compiled_against_the_live_cast_still_lands`, and
// `a_stale_reload_does_not_become_the_apps_selection`. All three entered through
// `reload_move_tables_from` / `reload_move_tables_selecting`, which are
// `#[cfg(test)]`, and all three supplied a cast base by hand. MEASURED: no
// production caller supplies one, and on the request road `admit_candidate`
// refuses a stale base on the PACK fingerprint before anything is staged.
//
// ⇒ All three guarantees are carried by
// `a_candidate_prepared_against_a_pack_that_is_no_longer_selected_is_refused`,
// on the production road: it asserts the refusal names both fingerprints,
// nothing was staged, THE SELECTION IS STILL THE OTHER PUBLICATION'S PACK, no
// shell command was issued, and the same candidate re-based on the live
// selection is accepted. It landed FIRST (`cbc92fa38`), before this deletion.

/// ⛔⛔ **A REPUBLISHED CAST AND THE APP'S SELECTION MOVE TOGETHER.** Two
/// authorities for "what content is this App running" is the thing App-scoped
/// selection exists to collapse, and a reload is the one operation that can
/// separate them.
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
// ⭐⭐ **I2's ACCEPTANCE: A PREBUILT HOST PLAYS AN EDITED FILE.**
//
// The three arms below use a directory that did not exist when this binary was
// compiled. Nothing between the bytes on disk and the published cast is a build
// step — that is the whole claim, and it was a plan until the content root
// stopped being `env!("CARGO_MANIFEST_DIR")`.
//
// ⛔ A TEMP DIRECTORY, NEVER THE REPOSITORY. A guard whose subject MUTATES THE
// TREE has already cost this repository a day.
// ---------------------------------------------------------------------------

/// One shipped move table's file, and the character it belongs to.
fn a_shipped_table(root: &std::path::Path) -> (std::path::PathBuf, String) {
    let shipped = crate::pack::compile_pack().expect("the shipped pack compiles");
    let table = ambition_characters::moveset_content_schema::lowered_movesets(&shipped)
        .expect("a move section");
    let buildable: std::collections::BTreeSet<&str> =
        crate::character_catalog::buildable_cast().collect();
    let who = table
        .keys()
        .find(|id| buildable.contains(id.as_str()))
        .expect("some shipped table names a buildable character")
        .clone();
    let file = crate::authored_movesets::TABLE_CHARACTERS
        .iter()
        .find(|(_, characters)| characters.contains(&who.as_str()))
        .map(|(name, _)| root.join("data/movesets").join(format!("{name}.ron")))
        .expect("the character's table is one of the declared moveset files");
    assert!(
        file.exists(),
        "the export did not write {} — the arm below would edit nothing",
        file.display()
    );
    (file, who)
}

/// A support table declaring every technique the SHIPPED tables reference.
///
/// ⛔⛤ **AND IT IS DERIVED FROM THE CONTENT ON PURPOSE, WHICH MAKES ADMISSION
/// VACUOUS HERE — DELIBERATELY, AND ONLY HERE.** These arms are about the DISK
/// ROAD: does a file edited after the binary was built reach the cast. Admission
/// is a different question with its own arm
/// (`a_refused_pack_never_reaches_the_cast`), and an empty table would make every
/// one of these fail with a roster of technique refusals that say nothing about
/// what is being tested. MEASURED: an empty table refused 40+ effects across the
/// shipped roster, which is the CORRECT answer to a question this fixture is not
/// asking.
///
/// ⚠ The premise asserts the reload was NOT refused, so a wrong derivation
/// surfaces as a failure rather than as a silently skipped edit.
fn support_for_the_live_cast(
    world: &bevy::ecs::world::World,
) -> ambition_entity_catalog::TechniqueSupport {
    // ⛔⛤ **THE LIVE CAST, NOT THE PACK — AND ASSUMING THEY WERE THE SAME COST ME
    // A FAILING FIXTURE THAT LOOKED LIKE A RELOAD BUG.** `overlay_authored_moves`
    // is this repository's ONE stated rule for the two move sources: an authored
    // table OVERLAYS the kit-derived one, and a derived move whose id the table
    // does not name SURVIVES. MEASURED: `author.ron` carries 26 moves and the
    // published `author` plays 33 — the seven extras are derived kit moves, and
    // one of them authors `pogo_bounce`, a key no shipped move table mentions.
    // ⇒ The pack's keys are not the cast's technique vocabulary.
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
                    // ⚠ ACCEPT WHATEVER THE CONTENT AUTHORS. `TechniqueParams::None`
                    // REFUSES a non-empty map, and shipped effects carry params —
                    // a fixture declaring `None` would report a params error where
                    // the real composition reports nothing.
                    params: ambition_entity_catalog::TechniqueParams::Checked(|_| Ok(())),
                    // ⚠ `None` IS A CLAIM that the effect names no other authored
                    // definition. It is the right one for a fixture that installs
                    // no cast beyond the shipped one: a nested-reference walker
                    // here would re-ask a question the preparation barrier already
                    // answers for the same content.
                    references: ambition_entity_catalog::NestedReferences::None,
                    // ⚠ BOTH ROADS, because the shipped tables author effects at
                    // volumes AND at events; declaring one would refuse the other
                    // for a reason that has nothing to do with the disk road.
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

/// ⛔⛔ **A FILE EDITED AFTER THE BINARY WAS BUILT CHANGES WHAT THE CAST PLAYS —
/// THROUGH THE ROAD PRODUCTION TAKES.**
///
/// ⛤ **IT USED TO GO THROUGH `reload_move_tables_from_dir`, AND THAT MADE THE
/// ACCEPTANCE WITNESS TESTIFY ABOUT A ROAD THE GAME CANNOT TAKE.** That helper
/// publishes the cast and installs the selection on the spot, with no shell
/// transaction, no content epoch, no prepared-content identity and no rollback
/// boundary — it is `#[cfg(test)]` for exactly that reason. The sentence this
/// arm exists to prove is *"a prebuilt host plays the edited artifact"*, and a
/// host proves it by doing what a host does: request a re-preparation and let
/// the activation land it.
///
/// ⇒ So the edit now travels: disk → `compile_pack_from` → `request_reload` →
/// `ShellCommand::ReplaceWith` → the router's `PreparationRequested` → the
/// activation. Every refusal and every correlation on that road has to let it
/// through for this assert to pass.
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

    // The edit, made on DISK, through the typed document.
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

    // ⚠ THE CAST BASE IS READ BEFORE THE FILE I/O, not after — reading a
    // directory takes time, and anything that publishes while we read it moves
    // the cast under the pack we are building.
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

/// ⛔ A ROOT MISSING A FILE IS REFUSED BY NAME, not compiled out of whatever
/// happened to be there. A per-file fallback to the binary's own text would
/// build a MIXED pack and nobody could say which half they played.
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

/// ⭐ THE CONTROL, ON THE SAME ROAD AS THE SUBJECT. An UNEDITED export is the
/// shipped pack, so requesting a reload of it must report `Unchanged` and ask
/// the shell for NOTHING — without this, "the edit arrived" is satisfied by a
/// road that re-prepares on every call, which would spend an epoch, a
/// publication and a world reconstruction on every save.
///
/// ⛤ A WATCHER FIRES ON A SAVE, NOT ON A CHANGE, so this is the common case in
/// the loop the whole road exists for.
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

/// ⛔⛤ **THE PUBLISHED MOVESET IS LARGER THAN THE AUTHORED TABLE, AND THAT IS
/// THE STATED RULE RATHER THAN A LEAK.** `overlay_authored_moves` is this
/// repository's ONE statement of how the two move sources combine: an authored
/// table OVERLAYS the kit-derived one, and a derived move whose id the table
/// does not name SURVIVES — *"a character that authors none keeps whatever the
/// kit folded."*
///
/// ⚠ IT IS PINNED HERE BECAUSE IT SURPRISED A READER WHO HAD EVERY REASON TO
/// EXPECT OTHERWISE. `authored_intrinsics`' own comment says the pack's table is
/// "A REPLACEMENT, NOT A MERGE", which is true of the CONTRACT it hands over and
/// not of the kit that contract is folded into. A fixture built on the pack's
/// keys alone reported a roster of technique refusals that looked like a reload
/// defect; the extras are where `pogo_bounce` comes from, and no shipped move
/// table mentions it.
///
/// ⇒ Two sentences that are both true and read as contradictory is exactly what
/// a test is for.
#[test]
fn the_published_moveset_keeps_the_kit_moves_the_table_does_not_name() {
    let mut app = bevy::app::App::new();
    crate::character_catalog::register(&mut app);
    crate::player_robot_lineage::register_declared_cast(&mut app);
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
    // ⛔ THE FLOOR. An empty comparison would make the subset claim above
    // trivially true over nothing.
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
// ⛔⛔ **CROSS-DOMAIN ATOMICITY — THE KNOWN HOLE, FILED AGAINST THIS PROTOTYPE.**
//
// `docs/planning/queue.md`, under *"⛔⛔ NEXT ARCHITECTURE ACTION"*: *"move
// reload can conclude `Unchanged` for the move material and still install the
// whole newly-loaded pack as `SelectedContentPack`. Moves identical + items
// changed = one subsystem believing nothing changed while another observes new
// mechanical content."* This is that sentence as an executable arm — the
// queue's P0 *"Poison tests for … cross-domain atomicity"* row.
//
// ⛔ IT IS NOT CLOSED BY SPECIAL-CASING `Unchanged`, and the queue says so in
// its own words: that hides the missing abstraction. `Unchanged` is a TRUE and
// correct answer about the material `reload_move_tables_from` examined. The
// defect is that a claim scoped to the MOVE section is spent as a claim about
// the PACK, and only the complete candidate-bundle transaction can say the
// larger thing.
// ---------------------------------------------------------------------------

/// The declared path of the non-move section this witness re-authors.
const ITEMS_PATH: &str = "data/items.ron";

/// A host whose cast is the SHIPPED roster.
///
/// ⚠ The synthetic `host_with_a_live_cast` cannot serve here: the subject is
/// two compiles of the REAL pack, so the cast has to be the one that pack
/// publishes.
fn host_with_the_shipped_cast() -> bevy::app::App {
    let mut app = bevy::app::App::new();
    crate::character_catalog::register(&mut app);
    crate::player_robot_lineage::register_declared_cast(&mut app);
    ambition_characters::prepared::close_preparation_barrier_without_admission(app.world_mut());
    // ⚠ WITHOUT THIS, ADMISSION REFUSES THE SHIPPED ROSTER FOR REASONS THAT ARE
    // NOT THE SUBJECT: an empty technique table refuses 40+ authored effects.
    // See `support_for_the_live_cast`'s own note.
    let support = support_for_the_live_cast(app.world());
    app.world_mut()
        .insert_resource(ambition_combat::technique::InstalledTechniques(support));
    app
}

/// The shipped pack with ONE item row's mechanical wiring re-authored, and
/// every other declared source byte-identical.
///
/// ⛔⛤ **EDITED THROUGH THE TYPED DOCUMENT, NOT BY TEXT SUBSTITUTION.**
/// `items.ron` is a POSITIONAL `Vec<ItemMeta>` whose ROW COUNT is part of its
/// schema — `content_schema.rs` refuses a short file because *"deleting one row
/// does not remove one item, it renames twenty-three"*. A regex edit that
/// dropped or added a line would be refused for a reason that has nothing to do
/// with this subject.
///
/// ⚠ TWO IDS THE GRID ALREADY CARRIES, SWAPPED — not one invented. An unknown
/// `held_item_id` would make this an is-the-content-valid question; swapping two
/// real ones leaves exactly one difference, *which slot grants which held item*,
/// and that is MECHANICAL content: `Item::from_held_item_id` is what equipping
/// resolves through.
///
/// ⛔ THE FIELD MOVES, THE ROW DOES NOT. Swapping whole rows would change the
/// positional binding, which is a different (and already-guarded) defect.
/// The shipped pack with ONE entity removed from its moveset table.
///
/// ⛔⛤ **EDITED THROUGH THE TYPED DOCUMENT, and the reason is the same one the
/// items fixture gives.** A text substitution on an entity id would hit the
/// table's verb bindings and every move id that carries the same prefix,
/// producing a document that is either still internally consistent or refused
/// for a reason that has nothing to do with this subject.
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
    // ⛔ THE FLOOR ON THE EDIT. A victim no table names would leave the closure
    // returning every source untouched, and the "candidate" would be the shipped
    // pack — the arm would pass while testing nothing.
    assert!(
        removed,
        "no declared source names entity `{victim}`, so the candidate is the \
         shipped pack and the witness would pass vacuously"
    );
    pack
}

/// The shipped pack with every authored move half a second longer.
///
/// ⚠ THE NAMES ARE UNTOUCHED ON PURPOSE: this is the control for the dropped-
/// entity refusal, so it has to change the pack WITHOUT changing which
/// characters are authored.
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
    // ⛔ THE FLOOR ON THE EDIT ITSELF. A mistyped declared path would leave the
    // closure never firing, and the "candidate" would be the shipped pack —
    // every assertion below would then pass while testing nothing. This
    // repository has been bitten four separate ways by a poison that silently
    // did not apply.
    assert!(
        edited,
        "no declared source is spelled `{ITEMS_PATH}`, so the candidate pack is \
         the shipped one and this witness would pass vacuously"
    );
    pack
}

/// ⛔⛔ **THE FIXTURE CONTRACT FOR THE CROSS-DOMAIN ARM: ONE PACK, TWO
/// CONTENTS, AND THE DIFFERENCE IS IN NO MOVE TABLE.**
///
/// The composition-level atomicity arm rests entirely on this pair existing,
/// and on the pack's own identity NOTICING a difference the move section cannot
/// see. Both halves are asserted here rather than assumed inside that arm,
/// because they fail in opposite directions and each failure would make the arm
/// green for the wrong reason:
///
/// * if the move sections DIFFER, the arm is about two different movesets and
///   says nothing about atomicity;
/// * if the FINGERPRINTS MATCH, an items-only edit is invisible to the pack's
///   complete identity — every verdict computed from that identity would answer
///   "no change" for a pack that plainly changed, and the arm would pass while
///   the thing it exists to detect went unnoticed.
///
/// ⚠ THE FINGERPRINT IS THE `ContentFingerprint` OVER THE PACK'S CANONICAL
/// BYTES, so this also pins that an item row's `held_item_id` reaches that
/// canonical form. `item_catalog`'s handler puts `slot={index}` in front of each
/// row's canonical text precisely so a positional file cannot move content
/// without moving the fingerprint.
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
    // ⛔ THE FLOOR. Two EMPTY move sections would compare equal and satisfy the
    // line above while comparing nothing.
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
/// ⛔⛤ **THE HOST BOOTS WITH A SELECTION ALREADY, AND ASSUMING OTHERWISE COST
/// ME THE FIRST VERSION OF THE ARM BELOW.** `register_declared_cast` calls
/// `pack::select`, whose fallback is an INSERT and not a read-through (its own
/// doc says so). So a test that "establishes a baseline" by publishing a freshly
/// compiled shipped pack is publishing a candidate that is mechanically
/// identical to what is live — and gets the complete no-op, correctly, leaving
/// the App holding the boot pack and the test comparing pointers to two
/// different allocations of the same content.
///
/// ⇒ The baseline is not something a fixture installs. It is something the host
/// already has, and the test must read it.
fn live_pack(app: &bevy::app::App) -> std::sync::Arc<ambition_content_pack::PreparedContentPack> {
    std::sync::Arc::clone(&app.world().resource::<crate::pack::SelectedContentPack>().0)
}

fn cast_generation(app: &bevy::app::App) -> u64 {
    app.world()
        .resource::<PreparedCharacterRegistry>()
        .generation()
        .get()
}

/// ⛔⛔ **THE COMPOSITION-LEVEL ARM: THE REAL PACK, THE REAL CAST AND THE REAL
/// SELECTION MOVE TOGETHER OR NOT AT ALL.**
///
/// `candidate_tests.rs` pins the verdict itself on a synthetic two-section pack.
/// What a synthetic fixture cannot answer is whether the SHIPPED schemas behave
/// the way that arithmetic assumes — so this drives the same decision through
/// `compile_pack_with`, the shipped roster and a live `PreparedCharacterRegistry`.
///
/// ⛔⛤ **THIS ARM ASSERTED THE WRONG ANSWER AND A REVIEW CAUGHT IT.** It used to
/// require an items-only candidate to PUBLISH — moves unchanged, pack adopted —
/// on the reasoning that the pack really did change and refusing it would be
/// special-casing. The reasoning was right about the pack and wrong about the
/// world:
///
/// * the live item catalog is installed in `AmbitionContentPlugin::build` from
///   `pack::prepared()`, and no reload road replaces it;
/// * so publishing left `PreparedContentIdentity` and the selected pack naming
///   generation N+1 while the live items served N.
///
/// That is WORSE than not supporting item reload: the canonical engine identity
/// would claim mechanical content is active when it is not.
///
/// ⭐⭐ MEASURED 2026-09-12, AND ITEMS IS THE WORST INSTANCE RATHER THAN A
/// TYPICAL ONE. The axis is not "does the reader call `pack::prepared()`" —
/// almost everything does — it is what the reader DOES with the borrow.
/// `pack::prepared()` is `-> &'static PreparedContentPack`, so a domain that
/// re-exports that borrow is making a structural claim that there is exactly one
/// generation forever; a domain that `.clone()`s into an App-owned value is not.
/// Items re-export it (`display_name`/`description`/`dialog_id` return
/// `&'static str` across ~80 external uses) AND add a second process-global on
/// top (`ITEM_CATALOG_OVERRIDE`, a `OnceLock` whose own comment reports that a
/// different second catalog "was IGNORED").
///
/// ⇒ **SO THE SECOND FAMILY IS `fighter_brain_ladder`, NOT ITEMS** — see
/// `the_fighter_ladder_is_the_second_family_the_transaction_carries` at the
/// bottom of this file. It clones into a plain resource, so it needed no new
/// authority, no new ordering edge and no signature change.
///
/// ⇒ **WHEN ITEMS JOIN THE TRANSACTION, THIS TEST FLIPS BACK**: refusal becomes
/// publication, the assertions below become the ones this file used to carry.
/// Leave the old expectations in this comment for whoever does it — and note
/// that the work it waits on is making `ambition_items`' read side return an
/// owned value, not wiring a road.
///
/// ⛔⛤ **THE CONTROL RUNS FIRST, AND IT IS NOT DECORATION.** "The selection
/// became the candidate" is also what a `publish_candidate` that installed
/// unconditionally would print. Only the complete no-op — a candidate
/// mechanically identical to what is live, which must touch NOTHING —
/// distinguishes the two, and a control read after the subject is one you
/// consult only after you have already believed the result.
///
/// ⚠ **THE TWIN IS A FRESH COMPILE, NOT THE LIVE `Arc`.** Reusing the live
/// allocation would make "the selection did not move" true by pointer identity
/// whatever the function did. A DIFFERENT allocation carrying the SAME
/// fingerprint is the only version of this control with power — and asserting
/// those two facts about it is simultaneously the recompile-stability control
/// for every `assert_ne!` on fingerprints in this file: without it, a
/// nondeterministic compile would make them all pass for no reason.
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

/// ⭐ THE CONTROL FOR THE REFUSAL: a MOVES-only candidate — the one participating
/// domain — still publishes. Without it, "unsupported domains are refused" is
/// satisfied by a road that refuses every change.
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
// ⛔⛔ **PUBLICATION IS REFUSED WHILE A ROLLBACK TIMELINE IS SPECULATING.**
//
// ⭐⭐ THE CONTRACT IS DERIVED, NOT INVENTED. MEASURED 2026-09-11:
// `ambition_platformer2d_rollback_ggrs`'s per-frame contract check already
// INVALIDATES a live GGRS timeline when the prepared content identity changes
// under it — a timeline promised the identity it rewinds. So publishing anyway
// earns a desync diagnosis, and refusing is strictly better than
// publish-and-be-invalidated. It is also the explicit contract a REMOTE session
// needs, instead of behaviour that "mostly works locally".
// ---------------------------------------------------------------------------

use ambition_platformer2d_runtime::rollback::{ActiveRollbackAuthority, RollbackTimelineContract};
use ambition_platformer2d_runtime::SnapshotSchemaFingerprint;
use ambition_platformer2d_shared_tangle::lifecycle::SessionScopeId;

/// An authority that GOVERNS this world, with a live timeline.
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

/// A candidate that WOULD publish — a real edit, against no base claim.
fn a_publishable_candidate() -> ambition_content_pack::CandidateGeneration {
    ambition_content_pack::CandidateGeneration::prepared_against(
        std::sync::Arc::new(pack_of(&doc_text(0.45)).expect("compiles")),
        None,
    )
}

/// ⭐ THE CONTROL, FIRST. Without a candidate that DOES publish through this
/// exact road, "refused while a timeline is live" is satisfied by a road that
/// refuses everything — and a control read after the subject is one you consult
/// only once you have already believed the result.
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

/// ⛔ A LIVE TIMELINE REFUSES, AND NOTHING MOVES.
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

/// ⛔⛔ **AND PUBLISHING MUST NOT HEAL AN UNHEALTHY AUTHORITY** — the poison the
/// architecture review asks for by name. `RollbackTimelineStatus::carried_from`
/// hands an unhealthy timeline's reason to its replacement, and
/// `acknowledge_and_clear` is the ONLY sanctioned way to clear one: *"a tool that
/// has shown the divergence to a human and been told to carry on"*. A content
/// publication that established a fresh timeline would launder a desync into
/// health by a side door.
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

/// ⚠ A STOOD-DOWN TIMELINE IS NOT A LIVE ONE. The gameplay session outlived its
/// timeline; nothing is speculating, so there is nothing to invalidate.
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

/// ⛔⛔ **A CANDIDATE THAT COMPILES AND THEN FAILS ADMISSION CHANGES NOTHING** —
/// the architecture review's "Admission refusal" acceptance, and the half that a
/// compile-time refusal cannot stand in for.
///
/// ⚠ THE TWO REFUSALS ARE DIFFERENT LAYERS AND ONLY ONE OF THEM IS THIS ONE.
/// `a_refused_pack_never_reaches_the_cast` refuses at COMPILE: the document is
/// structurally wrong and no world is involved. This one is a mechanically VALID
/// candidate whose authored effect names a technique this composition did not
/// install — the question only a host can answer — and it must leave the
/// selection, the engine's content identity, the cast generation and what the
/// cast plays all exactly as they were.
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

    // A move naming a technique this fixture installed nothing for. It compiles:
    // the content compiler does not know what a host installed, and deliberately
    // does not try to.
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
// ⭐⭐ **THE REQUEST ROAD: A RELOAD REUSES THE ENGINE'S OWN LIFECYCLE.**
//
// `publish_candidate` publishes the CAST directly and is the prototype. These
// arms are about `request_reload`, which issues the shell's existing
// `PreparationRequested` road instead — so the epoch, the content fingerprint
// and the publication all come from `prepare_platformer_content`, and the old
// generation stays authoritative until the new one activates.
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

/// Play the two shell events ONE re-preparation produces, and hand back the
/// activation that transaction authorizes.
///
/// ⛔⛤ **THE FIXTURE MINTS THE LOAD BECAUSE ONLY THE ROUTER CAN.**
/// MEASURED 2026-09-11: `ShellRouter::next_load_transaction` is private and the
/// id is minted in a LATER system than the request, so a reload learns its
/// `LoadId` only from the announcement. A fixture that skipped
/// `PreparationRequested` and re-used the already-active experience was testing
/// "any activation publishes", which is exactly the defect.
///
/// ⭐⭐ **AND IT ECHOES THE REQUEST ID OFF THE COMMAND THE CALLER WROTE, WHICH IS
/// THE ROUTER'S ACTUAL JOB.** `ShellCommand::ReplaceWith` now carries a
/// caller-minted `ShellRequestId` and `start_route` copies it onto the
/// transaction. ⛔ THE FIXTURE MUST NOT INVENT ONE: a hand-picked id tests
/// whether the fixture and the subject agree about a constant, where reading the
/// caller's own command tests PROPAGATION. This repository has the lesson
/// already — a fixture that chose 7 and 3 certified nothing, because the runtime
/// gives 0 and 0.
fn a_preparation_for(app: &mut bevy::app::App, load: &str) -> ActiveShellExperience {
    let barrier = ambition_platformer2d::load::LoadBarrierRef::new(
        ambition_platformer2d::load::LoadId::new(load),
        ambition_platformer2d::load::LoadBarrierId::new("publish"),
    );
    // ⛔ READ, NOT CONSTRUCTED. `None` here means the caller wrote no correlator,
    // and the adoption must then refuse — so passing `None` through is part of
    // what this fixture is for rather than a gap in it.
    let requested = issued_commands(app).into_iter().find_map(|command| match command {
        ShellCommand::ReplaceWith { request, .. } => request,
        _ => None,
    });
    a_preparation_carrying(app, barrier, requested)
}

/// [`a_preparation_for`] with the caller's correlator supplied explicitly.
///
/// ⛔⛤ **`issued_commands` ONLY SEES THE CURRENT UPDATE'S MESSAGES**, so a test
/// that calls `update()` between the request and the announcement cannot read the
/// id off the command any more — and a fixture that silently passed `None` there
/// would look like a correlation defect in the subject. Found exactly that way:
/// the arm asserting a reload still publishes on its OWN transaction failed
/// because the FIXTURE had lost the id, not because the reload had.
///
/// ⚠ THE CALLER STILL READS IT FROM THE COMMAND — it just reads it earlier. This
/// is not a hand-picked constant.
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

/// ⛔⛔ **A REAL EDIT ISSUES A RE-PREPARATION AND PUBLISHES NOTHING ITSELF.**
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

    // ⛔ AND NOTHING IS PUBLISHED YET. That is the entire difference between this
    // road and `publish_candidate`: the new generation appears when the shell
    // activates it, and the current one is authoritative until then.
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

/// ⭐ A COMPLETE NO-OP REQUESTS NOTHING. A file watcher fires on a SAVE, not on a
/// change; re-preparing for identical content would consume an epoch, a
/// publication and a reconstruction cycle for no reason.
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

/// ⛔ A ROUTE WITH NO PREPARATION PLAN CANNOT BE RE-PREPARED, and the request says
/// so rather than issuing a command that would reach nothing. The retry road
/// (`ambition_load_presentation::shell_adapter`) already checks exactly this.
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

/// ⛔ AND NO ACTIVE ROUTE IS ITS OWN ANSWER — a headless composition with no
/// shell has nothing to re-prepare, and that is not a content problem.
#[test]
fn a_host_with_no_active_route_is_reported_rather_than_requested() {
    let mut app = host_with_a_live_cast();
    app.add_message::<ShellCommand>();
    let outcome = request_reload(app.world_mut(), a_publishable_candidate());
    assert_eq!(outcome, ReloadRequest::NoActiveRoute, "got {outcome:?}");
}

/// ⛔⛔ AND THE REFUSALS REACH THIS ROAD TOO. A live rollback timeline refuses a
/// re-preparation request for the same reason it refuses a direct publication:
/// the runtime already answers a mid-session content change by invalidating the
/// timeline.
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

/// ⛔⛔ **THE TWO HALVES LAND AT ONE BOUNDARY.** The request stages the cast
/// revision and publishes nothing; the shell's `RouteActivated` is what lets it
/// through. Until then the live cast is the old one, which is what "the old
/// generation stays authoritative until the candidate passes" means in practice.
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

/// ⛔⛔ **A CANDIDATE THAT STOPS NAMING A CHARACTER IS REFUSED, NOT MERGED.**
///
/// ⛤ **THE SILENT MERGE THIS CLOSES IS ONE LEVEL BELOW THE ONE THE STALENESS
/// RULE CLOSES, AND IT IS REACHABLE BY DELETING ONE ENTITY.** `stage_move_section`
/// iterates the CANDIDATE's keys, so a character the candidate stops naming is
/// never visited; the fold is `active.clone()` plus the staged set, so its OLD
/// moveset survives untouched and is RE-PUBLISHED under the new generation. The
/// pack then says the family has no such entity while the cast plays its moves.
/// Nothing at any layer could represent that: `MovesetRevisionError` has two
/// variants and both are about the candidate NAMING something.
///
/// ⚠ `cellular_automaton.ron` CARRIES TWO ENTITIES, so this does not even need a
/// deleted file — which is why the fixture drops ONE entity rather than a whole
/// table.
#[test]
fn a_candidate_that_stops_naming_a_character_is_refused() {
    let mut app = host_with_the_shipped_cast();
    shell_active_on(&mut app, true);
    let live = live_pack(&app);
    let table = ambition_characters::moveset_content_schema::lowered_movesets(&live)
        .expect("the shipped pack authors movesets");
    // ⛔⛤ **THE VICTIM MUST HAVE A SIBLING IN ITS OWN FILE, AND MY FIRST FIXTURE
    // DID NOT KNOW THAT.** Removing a table's ONLY entity leaves a source that
    // declares the `moveset` schema and carries no move contract, and the
    // compiler refuses it by name — so that transition is already closed and
    // cannot reach this rule. `cellular_automaton.ron` carries TWO entities,
    // which is what makes the hole reachable without deleting a file.
    let victim = crate::authored_movesets::TABLE_CHARACTERS
        .iter()
        .find(|(_, characters)| characters.len() > 1)
        .and_then(|(_, characters)| {
            characters
                .iter()
                .find(|id| table.contains_key(**id))
                .map(|id| (*id).to_string())
        })
        .expect(
            "no shipped moveset table carries two entities, so a per-entity \
             removal cannot be authored and this arm has no subject",
        );

    // The candidate: the shipped pack with ONE entity removed from its table.
    let candidate = std::sync::Arc::new(pack_without_entity(&victim));
    let dropped = ambition_characters::moveset_content_schema::lowered_movesets(&candidate)
        .expect("the candidate still authors a moveset section");
    // ⛔ THE PREMISE, BOTH HALVES: exactly this one entity is gone, and the rest
    // are still there — a candidate that lost the whole section would exercise a
    // different row of the table.
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

/// ⛔⛔ **AND A CANDIDATE THAT DROPS THE WHOLE FAMILY NAMES EVERY CHARACTER IT
/// STOPS CARRYING.**
///
/// ⛤ THE SIBLING ABOVE CANNOT WITNESS THIS BRANCH, AND A POISON PROVED IT:
/// making the no-section case return an empty list left every arm green. The
/// transition is authored in a MANIFEST rather than in a source, so
/// `compile_pack_with` — which rewrites source text — can never produce it, and
/// `game/ambition_demo_smash/assets/pack.ron` ships a pack of exactly this shape.
#[test]
fn a_candidate_that_drops_the_moveset_family_names_everyone_it_drops() {
    let base = pack_of(&doc_text(0.2)).expect("the probe pack compiles");
    let named: Vec<String> = ambition_characters::moveset_content_schema::lowered_movesets(&base)
        .expect("the probe pack authors a moveset section")
        .keys()
        .cloned()
        .collect();
    // ⛔ THE PREMISE: the base must author somebody, or "every character it drops"
    // is the empty list and the arm is satisfied by a predicate that never fires.
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

    // ⭐ AND THE MIRROR: a base that authored NONE loses nothing, so a first
    // publication of the family is not a removal.
    assert!(
        ambition_characters::moveset_content_schema::dropped_moveset_entities(&candidate, &base).is_empty(),
        "publishing the family for the first time was reported as dropping it"
    );
}

/// ⭐ THE CONTROL, AND WITHOUT IT THE RULE ABOVE IS SATISFIED BY REFUSING EVERY
/// MOVESET EDIT. A candidate that RETIMES a move while naming every character
/// the live cast plays is the ordinary reload, and it must still be requested.
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
    // ⛔ THE PREMISE: same names, different content. A control that changed the
    // names would pass the containment test for the wrong reason.
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

/// ⛔⛔ **AND THE PRODUCTION ROAD REFUSES IT TOO, WHICH THE SIBLING ABOVE
/// CANNOT WITNESS.**
///
/// ⛤ MEASURED, NOT ASSUMED: poisoning the shared preflight's domain check
/// failed exactly ONE arm — the direct road's. The rule that keeps a
/// non-participating family from riding a move reload into the game was
/// certified only on a road the game does not take.
///
/// ⚠ WHEN ITEMS JOIN THE GENERATION TRANSACTION THIS ARM FLIPS rather than being
/// deleted: the expected answer becomes `Requested`, and the assertion that no
/// shell command was issued becomes the assertion that one was.
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
    // ⛔ AND IT COSTS THE SHELL NOTHING. A refusal that still asked for a
    // re-preparation would spend an epoch, a publication and a world
    // reconstruction on content the transaction cannot carry.
    assert!(
        issued_commands(&mut app).is_empty(),
        "a refused request reached the shell anyway"
    );
    assert!(
        crate::reload::pending_pack(app.world()).is_none(),
        "a refused request staged the candidate as pending"
    );
}

/// ⛔⛔ **NOTHING THE COMPOSITION DOES AFTER THE REQUEST CAN REFUSE THE COMMIT.**
///
/// ⭐⭐ THIS ARM REPLACED A WEAKER ONE OF MINE, AND THE REPLACEMENT IS THE POINT.
/// It used to assert that a refusal at the activation boundary left both halves
/// on the previous generation — a correct property of a commit path that can
/// still say no. A commit path that carries the value admission already computed
/// cannot say no at all, so the honest assertion is that the world CHANGES
/// UNDERNEATH IT AND THE GENERATION LANDS ANYWAY.
///
/// ⚠ BOTH TRANSITIONS, because they used to reach different branches: removing
/// the technique table returned before admission, shrinking it reached the
/// refusal. Neither is a branch any more.
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

    // ── the table SHRINKS: this used to reach `activate_staged_revision`'s
    //    refusal, because the candidate's strike names a key that stops being
    //    offered ──────────────────────────────────────────────────────────────
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

        // ⛔ THE COMPOSITION CHANGES OUT FROM UNDER THE PENDING GENERATION.
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

/// ⛔⛔ **A PENDING GENERATION NO LONGER OVERWRITES THE APP'S CONTENT IDENTITY.**
///
/// ⛤ IT DID, AND THAT HANDED THE CANDIDATE'S STAMP TO STRANGERS. Preparation
/// fingerprints against an identity, and the only road to it was the App-wide
/// `SelectedContentIdentity` — so a reload in flight reported
/// `SelectedContentPack = N` alongside `SelectedContentIdentity = N+1`, and an
/// UNRELATED route preparation in that window inherited a generation stamp for
/// content it never prepared. That identity is exactly what the rollback
/// timeline contract compares.
///
/// ⚠ AND NOTHING IS CLAIMED BEFORE THE ROUTER NAMES THE TRANSACTION, which is
/// correct rather than a gap: a preparation nobody has correlated to this
/// generation must not use it.
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
    // ⛔ THE ASSERTION THE WHOLE ARM IS FOR.
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

    // ⭐ AND THE CLAIM DOES NOT OUTLIVE ITS GENERATION: a claim naming a spent
    // `LoadId` would fingerprint a retry against a candidate this App discarded.
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

/// ⛔⛔ **AND A SECOND REQUEST IS REFUSED WHILE ONE IS IN FLIGHT.**
///
/// ⛤ THE THREE SINGLETONS THIS REPLACED COORDINATED BY OVERWRITING EACH OTHER.
/// A second `request_reload` before the router announced the first's transaction
/// replaced the pending pack and the (still unadopted) correlation, so the
/// FIRST request's `LoadId` was then adopted by the SECOND generation. A file
/// watcher makes closely spaced saves entirely ordinary.
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

/// ⛔⛔ **AND THE REQUEST ROAD REFUSES THAT COMPOSITION UP FRONT.**
///
/// ⛤ ABSENT IS NOT EMPTY. An empty `InstalledTechniques` is a legitimate value
/// meaning "this host installs nothing", and admitting against it correctly
/// refuses every authored effect. An ABSENT resource means the composition never
/// installed the combat capability, so nothing can admit the revision at all —
/// and letting the request through meant the activation reached a branch with no
/// answer, returned, and left the pending pack staged forever while the engine's
/// half had already moved.
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

/// ⛔⛔ **AN ACTIVATION THIS RELOAD DID NOT ASK FOR CANNOT PUBLISH IT.**
///
/// ⛤ **AND BOTH TRANSACTIONS TARGET THE SAME ROUTE, WHICH IS THE WHOLE POINT.**
/// A route name cannot separate them — `ReplaceWith("game")` issued twice, a
/// retry after a failure, a navigation back to a route a reload is waiting on:
/// every one of those produces a second `game` activation, and under a
/// name comparison the FIRST of them publishes content that was staged for the
/// second. The load id the router minted is the only thing that differs.
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

    // ⛔ A DIFFERENT TRANSACTION, SAME ROUTE, ACTIVATES FIRST.
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

    // ⭐ THE CONTROL: the reload's OWN activation still publishes, so the
    // refusal above is a correlation and not a reload that simply never works.
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellEvent::RouteActivated(mine));
    app.update();
    assert_ne!(
        live_duration(&app),
        before,
        "the reload's own activation did not publish it"
    );
}

/// ⛔⛔ **AND A FAILURE OF ANOTHER TRANSACTION CANNOT DISCARD IT EITHER.**
///
/// ⭐⭐ THIS ARM CORRELATES THROUGH THE EVENT, WHICH IS THE WHOLE POINT OF
/// `TransactionEnded`. Its earlier form wrote a bare
/// `CommandRejected(LoadFailed { .. })` and relied on the router still holding a
/// foreign `pending` — a correlation through state the router had already
/// overwritten in the case that mattered. The failing transaction now names
/// itself, by request AND by barrier, and neither is ours.
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

    // ⛔ SOMEONE ELSE'S TRANSACTION IS THE ONE THE ROUTER IS WAITING ON, AND IT
    // FAILS.
    app.world_mut().resource_mut::<ShellRouter>().pending =
        Some(ambition_platformer2d::game_shell::PendingShellRoute {
            route_id: ShellRouteId::new("game"),
            push_history: false,
            barrier: ambient_barrier("shell.game.8"),
            requires_prepared_session: true,
            terminal_reported: true,
            // ⛔ A FOREIGN REQUEST ID, NOT `None`. `None` would leave the
            // fixture agnostic about correlation; a name that is not ours makes
            // the transaction demonstrably somebody else's.
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

    // ⭐ THE STAGED REVISION SURVIVED, and the proof is that its own activation
    // still publishes it.
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellEvent::RouteActivated(mine));
    app.update();
    assert_ne!(
        live_duration(&app),
        before,
        "another transaction's failure discarded a reload it did not own"
    );
}

/// ⛔⛔ **AN UNRELATED REJECTION ARRIVING WHILE OUR OWN LOAD IS THE PENDING ONE
/// MUST NOT DISCARD THE RELOAD.** This is the review's finding 3 stated as an
/// arm: the old handler read `ShellRouter.pending` to decide whose failure it
/// was, so ANY `CommandRejected` — a host that is not configured, a route nobody
/// knows, a stale activation — threw away a staged edit for the sole reason that
/// the reload's load happened to be the one in flight. Nothing about the event
/// said so.
///
/// ⚠ THE PREMISE IS ASSERTED FIRST. A fixture where the router is NOT waiting on
/// our barrier would pass this under the old code too, and would be testing
/// nothing.
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

    // ⛔ THE PREMISE: OUR OWN transaction is the one the router is waiting on.
    app.world_mut().resource_mut::<ShellRouter>().pending =
        Some(ambition_platformer2d::game_shell::PendingShellRoute {
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

    // ⛤ SOMETHING ENTIRELY ELSE IS REJECTED.
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

/// ⛔⛔ **A SUPERSEDED RELOAD IS TOLD, AND THE PROOF IS THAT THE NEXT REQUEST IS
/// ACCEPTED.** Before `TransactionEnded`, supersession emitted no event at all:
/// `start_route` took `self.pending`, cancelled its prepared record and returned
/// events about the NEW route only. The staged generation waited on a load that
/// would never activate, and because [`ReloadRequest::AlreadyPending`] refuses
/// while one is in flight, every later save was refused FOREVER — a file watcher
/// makes that the ordinary case, not the exotic one.
///
/// ⚠ `Unchanged` WOULD MASK THIS. The second request must carry a candidate that
/// differs from the SELECTED pack, or `request_reload` short-circuits before it
/// ever reaches the pending check and the arm passes for the wrong reason.
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
    // ⛔ THE PREMISE, AND IT IS THE OLD BUG: while one is pending, a second is
    // refused.
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

    // ⭐ THE SLOT IS FREE.
    let again = request_reload(app.world_mut(), a_publishable_candidate());
    assert!(
        matches!(again, ReloadRequest::Requested { .. }),
        "a superseded reload still holds the slot, so no later save can land: \
         {again:?}"
    );
}

/// Has the pending reload ADOPTED the load `load` names?
///
/// ⭐ THE PREMISE CHECK FOR EVERY CORRELATION ARM. An arm that means to say "the
/// router is waiting on OUR load" is testing nothing unless the reload actually
/// took that id — and adoption is a system, not an assignment, so it can be
/// missed by one frame or one missing schedule edge.
fn reload_adopted_the_pending_load(app: &bevy::app::App, load: &str) -> bool {
    app.world()
        .get_resource::<PendingGeneration>()
        .and_then(|pending| pending.load_id.clone())
        .is_some_and(|adopted| adopted.as_str() == load)
}

/// ⛔⛔ **THE PRODUCTION ROAD'S STALENESS REFUSAL HAD NO WITNESS AT ALL.**
///
/// MEASURED 2026-09-12 at `647971bf1`: `git grep StaleGeneration` returned
/// exactly two hits, both in `reload.rs` — the variant's declaration and the one
/// place `admit_candidate` returns it. **Zero tests.** Meanwhile the CAST
/// clock's refusal (`MoveReload::Stale`, since DELETED) had two witnesses here
/// and two more in `ambition_characters::prepared_tests`, and every one of them
/// entered through `reload_move_tables_from` /
/// `reload_move_tables_selecting` / `activate_staged_revision` — all
/// `#[cfg(test)]`.
///
/// ⇒ **THE TESTED REFUSAL WAS THE ONE THAT COULD NOT FIRE IN PRODUCTION AND THE
/// REACHABLE ONE WAS UNTESTED.** On the request road `admit_candidate` runs
/// FIRST, so a candidate whose base no longer matches the selection is refused
/// before anything is staged; the cast stamp that would have said the same thing
/// was read from the live world one line before it was compared, and could
/// therefore never disagree. **This arm is what the cast clock's deletion rests
/// on, and it landed first for that reason.**
///
/// ⚠ THE SEQUENCE IS THE ONE A WATCHER ACTUALLY PRODUCES: read the live
/// identity, do file I/O, come back late. Compiling a pack is not instant and a
/// second save is ordinary.
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

    // ⛤ SOMEBODY ELSE PUBLISHES WHILE OUR COMPILE IS IN FLIGHT.
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
    // ⚠ CAPTURED *AFTER* THEIR PUBLICATION. Taking this before it would make
    // the final assertion fail on THEIR change and name mine.
    let before = live_duration(&app);

    // ⭐ OUR PACK, COMPILED BEFORE THAT, ARRIVES LATE — ON THE PRODUCTION ROAD.
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

    // ⛔ AND NOTHING MOVED. A refusal that staged, selected or requested is a
    // half-transaction, and `AlreadyPending` would then refuse every later save.
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

    // ⭐ AND THE SAME PACK, RE-BASED, IS ACCEPTED — so the refusal is about the
    // BASE and not about the pack, and the caller's documented remedy (re-read
    // and try again) actually works.
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

/// ⛔⛔ **A REQUEST THAT NEVER ACTIVATES DISCARDS ITS STAGED REVISION.** Left
/// staged, it would be applied by whatever activation came next — content nobody
/// asked for, arriving at a boundary nobody connected it to. The staleness stamp
/// cannot save it: nothing published, so its base is still current.
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
    // ⭐⭐ THE TRANSACTION FAILS BEFORE ANY PREPARATION EXISTS, which is the
    // window `reload_owns` cannot see into: `PendingGeneration.load_id` is still
    // `None`, so the ONLY thing that can correlate this failure to this reload is
    // the request id the caller minted.
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
    // ⛔⛔ **THE DIRECT ASSERTION, AND A POISON IS WHY IT IS HERE.** With the
    // terminal handler made inert, the two assertions around this one both
    // STAYED GREEN: the pending generation had adopted no load, so the later
    // activation carried no authorization matching it and published nothing for
    // reasons that have nothing to do with discarding. Only asking whether the
    // generation is GONE distinguishes "discarded" from "stranded".
    assert!(
        crate::reload::pending_pack(app.world()).is_none(),
        "a failed transaction left its generation pending, so every later save \
         is refused as AlreadyPending"
    );

    // ⛔ AND THE NEXT ACTIVATION MUST NOT APPLY IT EITHER — that is the whole
    // point of discarding rather than leaving it staged.
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

/// ⛔⛔ **A COMPOSITION WITH NO GAME SHELL MUST NOT PANIC, AND IT DID.**
///
/// A `MessageReader` for an unregistered message does not read nothing — it
/// FAILS PARAMETER VALIDATION and panics the schedule. Every other arm in this
/// file registers `ShellEvent` itself, so the entire crate was green while the
/// shipped default-feature workspace run panicked in
/// `publish_staged_reload_on_activation`. The `--rust` lane found it; no test
/// here could have.
///
/// ⚠ IT GOES THROUGH `reload::register`, NOT `add_systems`, deliberately: the run
/// condition is HALF of how this system is installed, and a test that spelled it
/// again would stay green if the real registration dropped it.
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

/// ⭐ THE CONTROL. With the shell present the same registration DOES run — or
/// "does not panic" would be satisfied by a condition that never lets it through.
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

/// ⛔⛤ **A COMPLETE NO-OP UNDER A HEALTHY LIVE ROLLBACK TIMELINE IS `Unchanged`,
/// NOT A ROLLBACK REFUSAL — AND IT WAS THE REFUSAL.**
///
/// I asked the publication boundary before asking whether there was anything to
/// publish. A mechanically identical candidate publishes nothing, allocates
/// nothing, reconstructs nothing and CANNOT invalidate a timeline; reporting it
/// as `RefusedDuringLiveTimeline` says the reload failed when in truth there was
/// nothing to do. A watcher fires on every SAVE, so that was the common case.
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

    // ⭐ THE CONTROL: a CHANGED candidate under the same timeline is still
    // refused, so this is about the verdict's ORDER and not about the boundary
    // being gone.
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

/// ⛔ AND THE SAME ON THE REQUEST ROAD.
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

/// ⛔⛔ **A FAILED PREPARATION MUST NOT LEAVE THE CANDIDATE SELECTED — AND IT DID,
/// WITH A SILENT PERMANENT SPLIT AS THE CONSEQUENCE.**
///
/// ```text
/// N is live
/// request N+1        -> SelectedContentPack became N+1
/// preparation fails  -> cast and session stay N, selection stays N+1
/// save N+1 again     -> the verdict compares against the SELECTION, reports
///                       Unchanged, and requests nothing
/// ```
///
/// The game stayed split for the rest of the session while the reload machinery
/// told the developer nothing had changed. ⇒ The candidate is PENDING until the
/// activation promotes it; a failure discards it and puts the engine's identity
/// back to the pack that is actually live.
///
/// ⚠ THE FOURTH ASSERTION IS THE ONE THAT MATTERS. The first three would all hold
/// under a fix that merely restored the selection; only re-submitting the SAME
/// candidate proves the machinery has not been taught to lie about it.
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
    // ⛔ NOT SELECTED YET, even though preparation must be able to read it.
    assert_eq!(
        crate::pack::selected(app.world())
            .expect("a selection")
            .fingerprint,
        selected,
        "the request installed the candidate as the App's selection"
    );

    // The preparation fails instead of activating, NAMING THIS transaction —
    // a bare `ExperienceFailed` carries only an activation id and is nobody's.
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

    // ⛔⛔ THE ASSERTION THE WHOLE TEST IS FOR: the same candidate, submitted
    // again, must still be a change.
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

/// ⭐ THE CONTROL: a SUCCESSFUL activation does promote the pending candidate, or
/// "not selected yet" is satisfied by a road that never selects anything.
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

/// ⛔⛔ **A CANDIDATE THAT WOULD FAIL ADMISSION IS REFUSED AT REQUEST TIME, SO THE
/// COMMIT BOUNDARY NEVER HAS TO.**
///
/// Without this, an authored effect naming a technique this composition never
/// installed is discovered by `RouteActivated` — at which point the shell has
/// already committed the new route and prepared session, the engine's half of
/// the generation is N+1, and the cast's half refuses. That half-transaction is
/// what I3 exists to prevent: a commit path is not where a candidate may learn
/// it is invalid.
///
/// ⚠ AND NOTHING IS LEFT BEHIND. The refusal discards the staged revision and
/// the pending pack, or the next activation would publish content this request
/// was told it could not have.
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

    // ⛔ THE REQUEST NEVER REACHED THE SHELL.
    assert!(
        issued_commands(&mut app).is_empty(),
        "a candidate that cannot be admitted asked the shell to re-prepare anyway"
    );
    // ⛔ AND NOTHING IS STAGED OR PENDING for a later activation to find.
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

    // ⛔⛔ AND THE NEXT ACTIVATION PUBLISHES NOTHING — the assertion that proves
    // the discard happened rather than the refusal merely being reported.
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
// ⭐⭐ **THE SECOND MECHANICAL CONTENT FAMILY.**
//
// The architecture review's gate was *"the SECOND mechanical content family —
// only after I3 closes, and as validation that the transaction absorbs it with
// no new authority."* These arms are that validation.
//
// ⛔⛤ **AND THE FAMILY IS NOT THE ONE THE REST OF THIS FILE ASSUMES.**
// `a_candidate_that_changes_only_items_is_refused_as_an_unsupported_domain`
// says of itself that it "flips back" when items join. It does not flip yet, and
// the census that picked the second family says why: MEASURED 2026-09-12,
// `install_item_catalog` writes a SECOND process-global `OnceLock` — its own
// comment reports that a different second catalog "was IGNORED" — and the read
// side returns `&'static str` across ~80 external uses. A borrow whose lifetime
// IS the `OnceLock` is a structural claim that there is exactly one generation
// forever; no ordering makes that family participate, only a signature change.
//
// ⇒ `fighter_brain_ladder` is the family that was already ready: one declared
// source, one lowering call site, and a publication that is
// `app.insert_resource(AuthoredFighterLadder(..))` — a plain newtype resource
// rather than a provider-keyed fragment registry.
// ---------------------------------------------------------------------------

const LADDER_PATH: &str = "data/fighter_brain_ladder.ron";

/// The shipped pack with LEVEL ONE's reaction latency one millisecond faster.
///
/// ⛔ ONE FIELD ON ONE RUNG, AND THE VALUE IS CHOSEN BY THE SCHEMA'S OWN RULES.
/// `FighterBrainLadder::problems` refuses a ladder that is not monotone in
/// reaction and refuses a rung that reacts instantly. 500 → 499 stays above
/// level 2's 450 and above zero, so the candidate is refused for nothing except
/// being different — which is the only property this witness needs.
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
    // ⛔ THE FLOOR ON THE EDIT. A renamed source or a retuned level 1 would leave
    // the closure a no-op, and every assertion below would pass on the SHIPPED
    // pack while testing nothing. This file already carries three of these for
    // exactly that reason.
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

/// ⛔⛔ **THE LADDER IS NOT AN UNSUPPORTED DOMAIN ANY MORE, AND IT LANDS AT THE
/// SAME BOUNDARY AS THE CAST.**
///
/// ⚠ **THE HOST INSTALLS THE LADDER THE WAY `AmbitionContentPlugin::build`
/// DOES**, because that is the state a running game is in: the resource was
/// cloned out of the boot pack once and nothing has replaced it since. Starting
/// from an absent resource would make "the reload installed it" true of a road
/// that only ever inserts, which is the weaker claim.
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

    // ⛔ BEFORE THIS COMMIT THIS WAS `Refused(RefusedUnsupportedDomains)`: the
    // ladder was not a participant, so a ladder-only edit could not even be
    // requested.
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

/// ⛔⛔ **A CANDIDATE THAT DECLARES NO LADDER REMOVES THE RESOURCE RATHER THAN
/// LEAVING GENERATION N's.**
///
/// ⭐ THIS IS THE ARM THE MOVESET FAMILY CANNOT HAVE, and it is why the ladder
/// was the clean second family to take. `dropped_moveset_entities`'s own
/// transition table records that a candidate which stops naming a character
/// re-publishes that character's OLD moveset under the new generation, because
/// nothing at any layer can represent "this entity's authored moveset is gone".
/// The ladder's absence IS representable: `profile_for_level` takes an `Option`
/// and states that absent means the engine floor.
///
/// ⚠ ASKED OF THE FUNCTION DIRECTLY, because the SHIPPED corpus cannot reach it:
/// `pack.ron` always declares the ladder source and the schema refuses a file
/// with anything other than nine rungs, so no `compile_pack_with` edit produces
/// a ladder-less pack. A synthetic moveset-only pack does.
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

/// The shipped pack with ONE mob's spawn delay 50ms later.
///
/// ⛔ ONE FIELD ON ONE MOB, AND IT IS THE MECHANICAL KIND. A wave's `delay` is
/// when the body appears, which is what the encounter director acts on — not a
/// label, not a comment. Changing a `label` would leave the schema's lowered
/// artifact different and the FIGHT identical, which is a weaker subject.
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

/// ⛔⛔ **THE THIRD FAMILY, AND IT COST NO NEW AUTHORITY EITHER — WHICH IS THE
/// CLAIM THE REVIEW'S GATE ACTUALLY ASKED TO VALIDATE.**
///
/// The ladder proved a second family could join. One family is a special case
/// and two is a pattern only if the SECOND one needed nothing the first one
/// invented — so this arm exists to fail if `PACK_DERIVED_FAMILIES` had to grow
/// a resource, a system or a refusal to accept `encounter_waves`. It did not:
/// one row in the table, one publisher, and the boundary this system already
/// had.
///
/// ⚠ **THE HOST INSTALLS THE BOOK THE WAY `AmbitionContentPlugin::build` DOES**,
/// for the same reason the ladder arm does: a running game's resource was cloned
/// out of the boot pack once and nothing has replaced it. Starting from an
/// absent resource would make "the reload installed it" true of a road that only
/// ever inserts.
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

/// ⛔⛔ **EVERY ROW OF `PACK_DERIVED_FAMILIES` IS REACHED BY ONE PUBLICATION.**
///
/// ⛔⛤ THE DEFECT THIS REFUSES IS A TABLE THAT NAMES A FAMILY IT DOES NOT
/// PUBLISH. The two arms above each drive ONE family end to end, and both would
/// stay green if the OTHER family's publisher were dropped — a per-family
/// witness cannot see a per-family omission in a sibling. This one asks the
/// question about the table: publish into a world holding NEITHER resource from
/// the shipped pack, and require that every declared family arrived.
///
/// ⚠ ASKED OF THE FUNCTION, NOT THROUGH THE SHELL, because the subject is the
/// TABLE rather than the boundary — and the boundary already has two witnesses.
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

/// ⛔⛔ **A GENERATION THAT CHANGES NO MOVESET DOES NOT ASK THE COMBAT
/// CAPABILITY FOR PERMISSION.**
///
/// ⛔⛤ **THE TRANSACTION USED TO BE A MOVE RELOAD WITH OTHER FAMILIES PUBLISHED
/// BESIDE IT**, and this arm is the difference. `request_reload` staged the move
/// section, demanded `InstalledTechniques` and admitted a revision even when
/// `changed_domains` was exactly `{"fighter_brain_ladder"}` — so a composition
/// with legitimate ladder or wave content and no combat capability was refused
/// `NoTechniqueSupport` for the absence of something its family never consults,
/// and an unrelated edit re-staged every unchanged move table to publish
/// somebody else's content.
///
/// ⚠ **THE HOST HERE DELIBERATELY INSTALLS NO `InstalledTechniques`**, which is
/// the whole subject: `host_with_the_shipped_cast` inserts one because admission
/// refuses 40+ authored effects without it. Removing it is what makes this arm
/// about the coupling rather than about the ladder.
#[test]
fn a_ladder_only_generation_needs_no_technique_table() {
    let mut app = host_with_the_shipped_cast();
    // ⛔ THE PREMISE. Without this removal the request would succeed for the
    // ordinary reason and the arm would certify nothing.
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

/// ⛔ AND THE MOVESET FAMILY STILL DOES REQUIRE IT — the control, without which
/// the arm above is satisfied by a road that simply stopped asking.
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

/// ⛔⛔ **A TRANSACTION FOR THE RELOAD'S OWN ROUTE THAT THE RELOAD DID NOT ISSUE
/// IS NOT ADOPTED.**
///
/// ⛔⛤ **THIS WAS REACHABLE AND THE CODE'S OWN COMMENT SAID SO WHILE DOING IT.**
/// Adoption matched `pending.route == transaction.route_id` beside a paragraph
/// reading *"a route name is not that identity: two generations can target one
/// route, which is exactly what a reload does."* Two `ReplaceWith("game")`
/// queued in one frame mint `shell.game.N` and `shell.game.N+1`, and
/// `start_route` CANCELS the first when the second begins — so a route-matching
/// reload could adopt a load already dead, then wait forever for an activation
/// that cannot come, leaving `AlreadyPending` on every later save.
///
/// ⇒ `ShellCommand::ReplaceWith` carries a caller-minted `ShellRequestId` now,
/// and this arm plays the hostile case directly: a `PreparationRequested` for
/// the SAME route, carrying somebody ELSE's request id.
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

    // ⛔ THE PREMISE: the reload really did mint a correlator, and this
    // transaction carries a DIFFERENT one. Without both halves the arm could
    // pass because nothing was pending or because nothing correlates at all.
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

    // Now that stranger's transaction activates. Under the old route-matching
    // adoption the reload would have adopted it above and published here.
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

    // ⭐ AND IT IS STILL WAITING FOR ITS OWN, which is the half that says the arm
    // above is about correlation rather than about the generation being gone.
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

/// ⛔ AND A TRANSACTION CARRYING NO CORRELATOR IS NOT A WILDCARD.
///
/// `None` means "nobody is correlating", which is the right answer for ordinary
/// navigation — and treating it as a match would restore the inference the
/// request id exists to remove, since every navigation command in the workspace
/// writes `None`.
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

// ── Publication legality across the transaction INTERVAL ─────────────────────
//
// ⛔⛤ **THE LEGALITY CHECK COVERS THE INSTANT SOMEBODY ASKED, NOT THE INTERVAL
// THE TRANSACTION LIVES IN — MEASURED 2026-09-12, AND THESE TWO ARMS ARE THE
// MEASUREMENT.**
//
// `admit_candidate` asks `publication_boundary` and refuses a live timeline or
// an unhealthy authority. That refusal is real and
// `a_live_rollback_timeline_refuses_a_reload_request` proves it. But the
// generation then spends time in `PendingGeneration` — through shell
// preparation, to `RouteActivated` — and `commit_content_generation` asks
// NOTHING: *"there is nothing in it that can say no"*, which is deliberate and
// correct as far as it goes.
//
// ⇒ So the implementation carries an unstated assumption: **that nothing can
// establish or invalidate a rollback authority between the request and the
// activation.** These two arms show what happens when it does, and the answer is
// that the generation publishes into a world the same check would have refused
// a moment earlier.
//
// ⚠ **WHAT THESE DO NOT SHOW, STATED SO THE NEXT READER DOES NOT OVERCLAIM
// THEM:** that the shipped lifecycle naturally produces those transitions in
// that window. They INSTALL the authority directly. The structural gap is
// measured; its natural reachability is not, and that is the open question —
// see `Q118` in the decision ledger.
//
// ⛔ AND THE FIX IS NOT A SECOND `publication_boundary` CALL IN
// `commit_content_generation`. By then the shell's engine/session half of the
// generation transition is already at its commit boundary, so a fallible content
// half there would recreate exactly the split I3 exists to prevent — a route
// activated at N+1 with a cast still at N. The direction is an authorization
// that COVERS the interval and is broken early, or a lifecycle that stops and
// rebases rollback as part of the same transaction.

/// A rollback timeline that goes live while a generation is pending does not
/// stop that generation publishing.
#[test]
fn a_generation_publishes_across_a_timeline_that_went_live_mid_flight() {
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

    // ⛔ THE PREMISE: it was LEGAL when asked. Without this the arm is about a
    // refused request rather than about the interval.
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
    // The interval: a healthy, speculating timeline appears AFTER the request was
    // accepted and BEFORE the activation commits it. Asked at this instant,
    // `publication_boundary` answers `LiveTimeline` and the same candidate is
    // refused — see `a_live_rollback_timeline_refuses_a_reload_request`.
    app.world_mut().insert_resource(live_authority());
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellEvent::RouteActivated(active));
    app.update();

    assert_ne!(
        live_duration(&app),
        before,
        "MEASURED GAP CLOSED? This arm records that the generation publishes \
         across a timeline that went live mid-flight. If it now refuses, the \
         interval is sealed and this arm should become the opposite assertion \
         naming whatever seals it.",
    );
}

/// An authority that goes UNHEALTHY while a generation is pending does not stop
/// that generation publishing either — and this is the worse of the two.
///
/// ⚠ WORSE because an unhealthy authority is a RECORDED DIVERGENCE.
/// `publishing_does_not_heal_an_unhealthy_rollback_authority` exists because
/// content publication must not launder a desync into health by a side door; a
/// generation that crosses the interval publishes into exactly that world
/// without ever asking.
#[test]
fn a_generation_publishes_across_an_authority_that_went_unhealthy_mid_flight() {
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

    assert_ne!(
        live_duration(&app),
        before,
        "MEASURED GAP CLOSED? This arm records that the generation publishes \
         across an authority that went unhealthy mid-flight.",
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
