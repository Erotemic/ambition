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
