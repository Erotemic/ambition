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
/// ⭐⭐ MEASURED, AND ITEMS IS AN INSTANCE RATHER THAN THE CASE: **eleven of the
/// twelve domains in `pack.ron` read the process-global `pack::prepared()`** at
/// plugin build or registration. `moveset` is the only one whose reader takes a
/// pack PARAMETER and the only one with a revision road — the same fact twice.
///
/// ⇒ **WHEN ITEMS JOIN THE TRANSACTION, THIS TEST FLIPS BACK**: refusal becomes
/// publication, the assertions below become the ones this file used to carry,
/// and the flip is a far stronger validation of the participant abstraction than
/// either arm alone. Leave the old expectations in this comment for whoever does
/// it.
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
            ),
            None,
        );
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
        ),
        None,
    );

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
        ),
        None,
    );
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
        std::sync::Arc::new(pack_of(&doc_text(0.2)).expect("compiles")),
        None,
    );
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
    let outcome = publish_candidate(app.world_mut(), a_publishable_candidate(), None);
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

    let outcome = publish_candidate(app.world_mut(), a_publishable_candidate(), None);
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

    let outcome = publish_candidate(app.world_mut(), a_publishable_candidate(), None);
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
    let outcome = publish_candidate(app.world_mut(), a_publishable_candidate(), None);
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
        std::sync::Arc::new(pack_of(&doc_text(0.2)).expect("compiles")),
        None,
    );
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
        ),
        None,
    );
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
/// ⛔⛤ **THE FIXTURE HAS TO MINT THE LOAD BECAUSE THE REQUESTER CANNOT.**
/// MEASURED 2026-09-11: `ShellRouter::next_load_transaction` is private, the id
/// is minted in a LATER system, and `ShellCommand::ReplaceWith` carries no slot
/// to put a correlator in — so a reload learns which transaction is its own only
/// by ADOPTING the one the router announces. A fixture that skipped
/// `PreparationRequested` and re-used the already-active experience was testing
/// "any activation publishes", which is exactly the defect.
fn a_preparation_for(app: &mut bevy::app::App, load: &str) -> ActiveShellExperience {
    let barrier = ambition_platformer2d::load::LoadBarrierRef::new(
        ambition_platformer2d::load::LoadId::new(load),
        ambition_platformer2d::load::LoadBarrierId::new("publish"),
    );
    app.world_mut().write_message(
        ambition_platformer2d::game_shell::ShellEvent::PreparationRequested(
            ambition_platformer2d::game_shell::ProviderLoadTransaction {
                route_id: ShellRouteId::new("game"),
                experience_id: ambition_platformer2d::game_shell::ShellExperienceId::new("fixture"),
                barrier: barrier.clone(),
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
        std::sync::Arc::new(pack_of(&doc_text(0.2)).expect("compiles")),
        None,
    );
    shell_active_on(&mut app, true);
    let generation = app
        .world()
        .resource::<PreparedCharacterRegistry>()
        .generation();
    let played = live_duration(&app);

    let outcome = request_reload(app.world_mut(), a_publishable_candidate());
    assert_eq!(
        outcome,
        ReloadRequest::Requested {
            route: "game".to_string()
        },
        "got {outcome:?}"
    );
    assert!(
        matches!(
            issued_commands(&mut app).as_slice(),
            [ShellCommand::ReplaceWith(route)] if route.as_str() == "game"
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
    let _ = reload_move_tables_selecting(app.world_mut(), std::sync::Arc::clone(&pack), None);
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
        std::sync::Arc::new(pack_of(&doc_text(0.2)).expect("compiles")),
        None,
    );
    shell_active_on(&mut app, true);
    app.add_systems(bevy::app::Update, publish_staged_reload_on_activation);
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
        crate::pack::pending(app.world()).is_none(),
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
        std::sync::Arc::new(pack_of(&doc_text(0.2)).expect("compiles")),
        None,
    );
    shell_active_on(&mut app, true);
    app.add_systems(bevy::app::Update, publish_staged_reload_on_activation);
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
/// ⚠ THIS HALF CORRELATES THROUGH THE ROUTER, NOT THE EVENT: measured
/// 2026-09-11, `CommandRejected` carries no barrier at all. What it does have is
/// a router still holding the `pending` route whose barrier failed, and that
/// names the load.
#[test]
fn a_failure_of_another_transaction_cannot_discard_a_pending_reload() {
    let mut app = host_with_a_live_cast();
    let _ = reload_move_tables_selecting(
        app.world_mut(),
        std::sync::Arc::new(pack_of(&doc_text(0.2)).expect("compiles")),
        None,
    );
    shell_active_on(&mut app, true);
    app.add_systems(bevy::app::Update, publish_staged_reload_on_activation);
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
        });
    app.world_mut().write_message(
        ambition_platformer2d::game_shell::ShellEvent::CommandRejected(
            ambition_platformer2d::game_shell::ShellCommandRejection::LoadFailed {
                readiness: ambition_platformer2d::load::BarrierReadiness::Failed,
                failures: Vec::new(),
            },
        ),
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
        std::sync::Arc::new(pack_of(&doc_text(0.2)).expect("compiles")),
        None,
    );
    shell_active_on(&mut app, true);
    app.add_systems(bevy::app::Update, publish_staged_reload_on_activation);
    let before = live_duration(&app);

    assert!(matches!(
        request_reload(app.world_mut(), a_publishable_candidate()),
        ReloadRequest::Requested { .. }
    ));
    // The preparation fails instead of activating.
    app.world_mut().write_message(
        ambition_platformer2d::game_shell::ShellEvent::ExperienceFailed {
            activation_id: ambition_platformer2d::game_shell::ShellActivationId(1),
            message: "the fixture's preparation failed".to_string(),
        },
    );
    app.update();
    assert_eq!(
        live_duration(&app),
        before,
        "a failed preparation published the cast anyway"
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
        std::sync::Arc::new(pack_of(&doc_text(0.2)).expect("compiles")),
        None,
    );
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
    let _ = reload_move_tables_selecting(app.world_mut(), std::sync::Arc::clone(&pack), None);
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
        ),
        None,
    );
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
        ),
        None,
    );
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
    let _ = reload_move_tables_selecting(app.world_mut(), std::sync::Arc::clone(&pack), None);
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
    let _ = reload_move_tables_selecting(app.world_mut(), std::sync::Arc::clone(&live), None);
    shell_active_on(&mut app, true);
    app.add_systems(bevy::app::Update, publish_staged_reload_on_activation);
    let selected = crate::pack::selected(app.world())
        .expect("a selection")
        .fingerprint;

    let candidate = std::sync::Arc::new(pack_of(&doc_text(0.45)).expect("compiles"));
    assert_ne!(candidate.fingerprint, selected, "the premise: it differs");
    assert!(matches!(
        request_reload(
            app.world_mut(),
            ambition_content_pack::CandidateGeneration::prepared_against(
                std::sync::Arc::clone(&candidate),
                None
            ),
        ),
        ReloadRequest::Requested { .. }
    ));
    // ⛔ NOT SELECTED YET, even though preparation must be able to read it.
    assert_eq!(
        crate::pack::selected(app.world())
            .expect("a selection")
            .fingerprint,
        selected,
        "the request installed the candidate as the App's selection"
    );

    // The preparation fails instead of activating.
    app.world_mut().write_message(
        ambition_platformer2d::game_shell::ShellEvent::ExperienceFailed {
            activation_id: ambition_platformer2d::game_shell::ShellActivationId(1),
            message: "the fixture's preparation failed".to_string(),
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
        crate::pack::pending(app.world()).is_none(),
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
        std::sync::Arc::new(pack_of(&doc_text(0.2)).expect("compiles")),
        None,
    );
    shell_active_on(&mut app, true);
    app.add_systems(bevy::app::Update, publish_staged_reload_on_activation);
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
        crate::pack::pending(app.world()).is_none(),
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
        std::sync::Arc::new(pack_of(&doc_text(0.2)).expect("compiles")),
        None,
    );
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
        crate::pack::pending(app.world()).is_none(),
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
    app.add_systems(bevy::app::Update, publish_staged_reload_on_activation);
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
