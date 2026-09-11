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
                        on_hit: None,
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
    let outcome = reload_move_tables_from(app.world_mut(), &pack, None);
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
        &pack_of(&doc_text(0.2)).expect("compiles"),
        None,
    );
    let first = live_duration(&app);

    let outcome = reload_move_tables_from(
        app.world_mut(),
        &pack_of(&doc_text(0.35)).expect("compiles"),
        None,
    );
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
    let _ = reload_move_tables_from(app.world_mut(), &pack, None);
    let published = app
        .world()
        .resource::<PreparedCharacterRegistry>()
        .generation();

    let outcome = reload_move_tables_from(app.world_mut(), &pack, None);
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
        &pack_of(&doc_text(0.2)).expect("compiles"),
        None,
    );
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
        &pack_of(&doc_text(0.2)).expect("compiles"),
        None,
    );
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
        reload_move_tables_from(app.world_mut(), &pack_of(&foreign).expect("compiles"), None);
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
        &pack_of(&doc_text(0.2)).expect("compiles"),
        None,
    );
    assert_eq!(outcome, MoveReload::NoTechniqueSupport, "got {outcome:?}");
}

/// ⛔⛔ **A RELOAD APPLYING A PACK IT COMPILED AGAINST AN OLDER CAST IS REFUSED,
/// NOT FOLDED** — the I3a staleness rule reaching this road, and the reason
/// `MoveReload::Stale` is a variant rather than a comment.
///
/// ⛔⛤ **AND WRITING THIS ARM IS WHAT FOUND THE RULE COULD NOT FIRE.** With the
/// stamp read from the world inside the staging road, no sequence reached it:
/// activation is the only publisher after the barrier and it drains the staged
/// transaction atomically, so stage-time and fold-time are the same generation
/// by construction. The claim had to come from the CALLER — see
/// `reload_move_tables_from`.
///
/// ⚠ THE SEQUENCE IS THE ONE A RELOAD LOOP ACTUALLY PRODUCES. Compiling a pack
/// is file I/O; a watcher reads the cast when the change arrives and applies
/// when the compile finishes. Anything that publishes in between — a second
/// watcher, an inspector, a scripted edit — moves the cast under it.
#[test]
fn a_pack_compiled_against_an_older_cast_is_refused() {
    let mut app = host_with_a_live_cast();
    let compiled_against = app
        .world()
        .resource::<PreparedCharacterRegistry>()
        .generation();

    // Somebody else publishes while our compile is in flight.
    let theirs = reload_move_tables_from(
        app.world_mut(),
        &pack_of(&doc_text(0.2)).expect("compiles"),
        None,
    );
    assert!(
        matches!(theirs, MoveReload::Activated { .. }),
        "the premise: the cast actually moved under us; got {theirs:?}"
    );
    let active = app
        .world()
        .resource::<PreparedCharacterRegistry>()
        .generation();
    assert_ne!(
        compiled_against, active,
        "the fixture never superseded anything, so the arm below would pass \
         against a generation that never moved"
    );
    let published = live_duration(&app);

    // Our pack, compiled before that, arrives late.
    let ours = reload_move_tables_from(
        app.world_mut(),
        &pack_of(&doc_text(0.35)).expect("compiles"),
        Some(compiled_against),
    );
    assert_eq!(
        ours,
        MoveReload::Stale {
            prepared_against: compiled_against.get(),
            active: active.get(),
        },
        "a reload folded a pack onto a cast it never saw: {ours:?}"
    );
    assert_eq!(
        app.world()
            .resource::<PreparedCharacterRegistry>()
            .generation(),
        active,
        "a stale reload moved the generation"
    );
    assert_eq!(
        live_duration(&app),
        published,
        "the stale pack silently overwrote the publication it never saw"
    );
}

/// ⭐ THE CONTROL. "Refused as stale" is also what a road that refuses every
/// stamped reload reports. A pack compiled against the CURRENT cast still lands.
#[test]
fn a_pack_compiled_against_the_live_cast_still_lands() {
    let mut app = host_with_a_live_cast();
    let live = app
        .world()
        .resource::<PreparedCharacterRegistry>()
        .generation();
    let outcome = reload_move_tables_from(
        app.world_mut(),
        &pack_of(&doc_text(0.2)).expect("compiles"),
        Some(live),
    );
    assert!(
        matches!(outcome, MoveReload::Activated { changed: 1, .. }),
        "an up-to-date reload was refused; got {outcome:?}"
    );
}

/// ⛔⛔ **A REPUBLISHED CAST AND THE APP'S SELECTION MOVE TOGETHER.** Two
/// authorities for "what content is this App running" is the thing App-scoped
/// selection exists to collapse, and a reload is the one operation that can
/// separate them.
#[test]
fn an_activated_reload_becomes_the_apps_selection() {
    let mut app = host_with_a_live_cast();
    let fresh = std::sync::Arc::new(pack_of(&doc_text(0.2)).expect("compiles"));
    let outcome =
        reload_move_tables_selecting(app.world_mut(), std::sync::Arc::clone(&fresh), None);
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

/// ⛔ AND A REFUSED RELOAD LEAVES THE SELECTION WHERE IT WAS — otherwise the
/// cast would be built from one pack while every later read answered from
/// another.
#[test]
fn a_stale_reload_does_not_become_the_apps_selection() {
    let mut app = host_with_a_live_cast();
    let compiled_against = app
        .world()
        .resource::<PreparedCharacterRegistry>()
        .generation();
    let theirs = std::sync::Arc::new(pack_of(&doc_text(0.2)).expect("compiles"));
    assert!(
        matches!(
            reload_move_tables_selecting(app.world_mut(), std::sync::Arc::clone(&theirs), None),
            MoveReload::Activated { .. }
        ),
        "the premise: something published first"
    );

    let ours = std::sync::Arc::new(pack_of(&doc_text(0.35)).expect("compiles"));
    let outcome = reload_move_tables_selecting(
        app.world_mut(),
        std::sync::Arc::clone(&ours),
        Some(compiled_against),
    );
    assert!(
        matches!(outcome, MoveReload::Stale { .. }),
        "the premise: a refused reload; got {outcome:?}"
    );
    let selected = crate::pack::selected(app.world()).expect("a selection");
    assert!(
        std::ptr::eq(selected, std::sync::Arc::as_ref(&theirs)),
        "a refused reload became the App's content anyway"
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
    let mut app = bevy::app::App::new();
    crate::character_catalog::register(&mut app);
    crate::player_robot_lineage::register_declared_cast(&mut app);
    ambition_characters::prepared::close_preparation_barrier_without_admission(app.world_mut());
    let support = support_for_the_live_cast(app.world());
    app.world_mut()
        .insert_resource(ambition_combat::technique::InstalledTechniques(support));
    (app, who)
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

/// ⛔⛔ **A FILE EDITED AFTER THE BINARY WAS BUILT CHANGES WHAT THE CAST PLAYS.**
#[test]
fn a_host_plays_a_move_edited_on_disk_after_it_was_built() {
    let dir = tempfile::tempdir().expect("a temp content root");
    let root = dir.path();
    let written = crate::pack::export_sources_to(root).expect("the sources export");
    assert!(written > 10, "only {written} source(s) exported");

    let (app, who) = host_from_dir(root);
    let mut app = app;
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

    let outcome = reload_move_tables_from_dir(app.world_mut(), root);
    assert!(
        matches!(outcome, MoveReload::Activated { .. }),
        "the edited directory did not reach the cast: {outcome:?}"
    );
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

/// ⭐ THE CONTROL. An UNEDITED export is the shipped pack, so reloading it must
/// report `Unchanged` — without this, "the edit arrived" is satisfied by a road
/// that republishes on every call.
#[test]
fn reloading_an_unedited_export_publishes_nothing() {
    let dir = tempfile::tempdir().expect("a temp content root");
    let root = dir.path();
    crate::pack::export_sources_to(root).expect("exports");
    let (mut app, _) = host_from_dir(root);
    let first = reload_move_tables_from_dir(app.world_mut(), root);
    assert!(
        matches!(
            first,
            MoveReload::Unchanged { .. } | MoveReload::Activated { .. }
        ),
        "the first reload of the shipped bytes reported {first:?}"
    );
    let second = reload_move_tables_from_dir(app.world_mut(), root);
    assert!(
        matches!(second, MoveReload::Unchanged { .. }),
        "re-reading the same unedited directory republished: {second:?}"
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
