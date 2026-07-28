//! The engine-owned character load pipeline, tested without any application.
//!
//! Every fixture here builds a bare `App` and adds ENGINE pieces only. That is
//! deliberate and it is the actual regression bar: the defect this module exists
//! to kill was invisible precisely because the only test coverage ran through the
//! one application that happened to install the missing step.

use super::*;

/// Any real baked sheet, to stand in as "a decoded asset".
///
/// Deliberately does not NAME a character: this file is partly about the engine
/// not knowing any character's id, so its fixture should not either. The first
/// baked target is whatever the sheet table happens to hold.
fn any_baked_sheet() -> ambition_sprite_sheet::character::CharacterSpriteAsset {
    use ambition_sprite_sheet::character::sheets::{try_load_spec_for_target, SheetTuning};
    let registry = ambition_sprite_sheet::baked_sheet_registry();
    // Not every baked target is a CHARACTER sheet — effect sheets
    // (`robot_slash`) are in the same table and load no character spec. Take the
    // first that does, sorted so the fixture is stable across runs.
    let mut targets: Vec<&str> = registry.iter().map(|(target, _)| target).collect();
    targets.sort_unstable();
    let spec = targets
        .iter()
        .find_map(|target| try_load_spec_for_target(target, &SheetTuning::default()))
        .expect("at least one baked target loads a character sheet spec");
    ambition_sprite_sheet::character::CharacterSpriteAsset {
        texture: Handle::default(),
        layout: Handle::default(),
        spec,
        pages: Vec::new(),
    }
}

fn declared_sprites(entries: &[(&str, &str)]) -> CharacterSpriteAssets {
    let mut sprites = CharacterSpriteAssets::default();
    for (id, display) in entries {
        sprites.declare(id, display);
    }
    sprites
}

#[test]
fn a_declared_character_is_a_different_answer_from_an_unknown_one() {
    let sprites = declared_sprites(&[("mary_o", "Mary-O")]);

    // Both keys reach the same declaration...
    assert!(matches!(
        sprites.sheet_state("mary_o"),
        CharacterSheetState::Declared {
            character_id: "mary_o"
        }
    ));
    assert!(matches!(
        sprites.sheet_state("Mary-O"),
        CharacterSheetState::Declared {
            character_id: "mary_o"
        }
    ));
    // ...and a typo is UNKNOWN, not "declared but pending". Collapsing these two
    // into `None` is what made a misspelled id and an undecoded sheet look
    // identical for the lifetime of a playtest.
    assert!(sprites.sheet_state("mary_oh").is_unknown());
    assert!(sprites.sheet_state("Mary O").is_unknown());
}

#[test]
fn no_character_id_is_privileged_by_the_sheet_table() {
    // The four ids that used to have typed slots resolve through exactly the same
    // lookup as anyone else's protagonist, and neither is reachable without being
    // declared first. If a `match` on names ever comes back, this fails.
    let sprites = declared_sprites(&[("sanic", "Sanic")]);
    for privileged in ["player", "robot", "goblin", "sandbag"] {
        assert!(
            sprites.sheet_state(privileged).is_unknown(),
            "`{privileged}` must not resolve without a declaration"
        );
    }
    assert!(matches!(
        sprites.sheet_state("sanic"),
        CharacterSheetState::Declared { .. }
    ));
}

#[test]
fn publishing_a_sheet_clears_every_token_that_declared_it() {
    let mut sprites = declared_sprites(&[("mary_o", "Mary-O")]);
    sprites.publish("mary_o", any_baked_sheet());

    // Both the id and the display name now resolve to the decoded sheet, and
    // neither is still declared — a token left in both maps would let a later
    // demand re-decode a sheet that already exists.
    assert!(matches!(
        sprites.sheet_state("mary_o"),
        CharacterSheetState::Ready(_)
    ));
    assert!(matches!(
        sprites.sheet_state("Mary-O"),
        CharacterSheetState::Ready(_)
    ));
    assert!(!sprites.is_declared("mary_o"));
    assert!(sprites.declared_character_ids().is_empty());
}

#[test]
fn demand_is_deduplicated_and_ignores_blank_tokens() {
    let mut demand = CharacterLoadDemand::default();
    demand.request("mary_o");
    demand.request("mary_o");
    demand.request("  ");
    demand.request("");
    demand.request_all(["sanic", "mary_o"]);
    assert_eq!(
        demand.pending().collect::<Vec<_>>(),
        vec!["mary_o", "sanic"]
    );
}

#[test]
fn an_unknown_demand_reaches_a_named_terminal_failure_not_silence() {
    // §4.9: every staged character reaches Ready or a NAMED terminal Failed.
    // "Nothing happened and nobody said anything" is the state this forbids.
    let mut demand = CharacterLoadDemand::default();
    let mut states = CharacterLoadStates::default();
    let mut sprites = declared_sprites(&[("mary_o", "Mary-O")]);
    demand.request("mary_oh");

    // Drive only the unknown-token branch, which needs no asset pipeline at all —
    // that is exactly why an unknown id must be classified BEFORE any decode.
    for token in std::mem::take(&mut demand.pending) {
        if matches!(sprites.sheet_state(&token), CharacterSheetState::Unknown) {
            states.record(
                token.clone(),
                &token,
                CharacterLoadOutcome::Failed(CharacterLoadFailure::UnknownCharacter),
            );
        }
    }
    let _ = &mut sprites;

    assert_eq!(
        states.outcome("mary_oh"),
        Some(CharacterLoadOutcome::Failed(
            CharacterLoadFailure::UnknownCharacter
        ))
    );
    assert_eq!(
        states.failures().collect::<Vec<_>>(),
        vec![("mary_oh", CharacterLoadFailure::UnknownCharacter)]
    );
}

#[test]
fn the_engine_plugin_installs_the_pipeline_without_any_application() {
    let mut app = App::new();
    app.add_plugins(CharacterRuntimePlugin);

    assert!(
        app.world().get_resource::<CharacterLoadDemand>().is_some(),
        "demand must exist so a provider can submit before any room stages"
    );
    assert!(app.world().get_resource::<CharacterLoadStates>().is_some());
    assert!(
        app.world()
            .get_resource::<CharacterMaterializationService>()
            .is_some(),
        "the capability marker is what lets a startup audit name an unusual \
         composition that reached staging with no materializer"
    );
}

#[test]
fn every_worn_body_demands_its_own_art_not_just_the_primary_player() {
    // The regression this closes: the host watched `With<PrimaryPlayer>`, which is
    // correct for exactly one game mode. A versus match is several worn bodies and
    // only player one would ever have had a sheet.
    let mut app = App::new();
    app.add_plugins(CharacterRuntimePlugin);
    app.world_mut()
        .spawn(ambition_characters::actor::WornCharacter("mary_o".into()));
    app.world_mut()
        .spawn(ambition_characters::actor::WornCharacter("sanic".into()));
    app.update();

    // No asset pipeline in this fixture, so the demand is still pending — which is
    // the observable proof that BOTH bodies asked.
    let demand = app.world().resource::<CharacterLoadDemand>();
    assert_eq!(
        demand.pending().collect::<Vec<_>>(),
        vec!["mary_o", "sanic"],
        "every worn body must demand its identity's art"
    );
}

#[test]
fn a_later_identity_swap_demands_the_new_sheet() {
    let mut app = App::new();
    app.add_plugins(CharacterRuntimePlugin);
    let body = app
        .world_mut()
        .spawn(ambition_characters::actor::WornCharacter("mary_o".into()))
        .id();
    app.update();
    app.world_mut().resource_mut::<CharacterLoadDemand>().take();

    // A runtime form change (Mary-O growing into `mary_o_tall`) is a new sheet.
    app.world_mut()
        .entity_mut(body)
        .insert(ambition_characters::actor::WornCharacter(
            "mary_o_tall".into(),
        ));
    app.update();

    assert_eq!(
        app.world()
            .resource::<CharacterLoadDemand>()
            .pending()
            .collect::<Vec<_>>(),
        vec!["mary_o_tall"]
    );
}

// ── §7.2: composition parity as a readiness invariant ──────────────────────────

use super::audit::{
    audit_character_capabilities, character_reveal_ready, report_character_capability_gaps,
    unsettled_staged_characters, CharacterCapabilityGap,
};
use super::staging::{
    DirectStartupSpec, MatchParticipantRoster, RoomStagingPlan, StagesCharacters,
};

/// **The composition-parity test.** Three semantically different entry routes,
/// one materialized result.
///
/// This is deliberately NOT "boot two apps and diff their resources". That test
/// asserts implementation details, goes red on every unrelated resource, and can
/// pass while two apps still stage characters differently. What matters is that
/// the same character reaches the same outcome no matter which door it came in
/// through — and there is no host-application module in this fixture, because the
/// defect being guarded was invisible precisely while the only coverage ran
/// through the one app that happened to work.
#[test]
fn every_entry_route_materializes_the_same_character() {
    let routes: Vec<(&str, Vec<String>)> = vec![
        (
            "direct startup",
            DirectStartupSpec::of(["mary_o"]).character_tokens(),
        ),
        (
            "room staging",
            RoomStagingPlan {
                placement_characters: vec!["mary_o".into()],
                ..Default::default()
            }
            .character_tokens(),
        ),
        (
            "match roster",
            // A mirror match: the same character on two seats must not become two
            // decodes, and must not become a different demand than the other routes.
            MatchParticipantRoster::of(["mary_o", "mary_o"]).character_tokens(),
        ),
    ];

    let mut outcomes = Vec::new();
    for (label, tokens) in routes {
        let mut app = App::new();
        app.add_plugins(CharacterRuntimePlugin);
        {
            let mut demand = app.world_mut().resource_mut::<CharacterLoadDemand>();
            demand.request_all(tokens);
        }
        app.update();
        let demand = app.world().resource::<CharacterLoadDemand>();
        let states = app.world().resource::<CharacterLoadStates>();
        outcomes.push((
            label,
            demand.pending().map(str::to_string).collect::<Vec<_>>(),
            states.outcome("mary_o"),
        ));
    }

    let (first_label, first_demand, first_outcome) = outcomes[0].clone();
    for (label, demand, outcome) in &outcomes[1..] {
        assert_eq!(
            (demand, outcome),
            (&first_demand, &first_outcome),
            "`{label}` staged the same character as `{first_label}` but reached a \
             different result — that difference IS the composition bug this guards"
        );
    }
    assert_eq!(
        first_demand,
        vec!["mary_o".to_string()],
        "every route must project to the one shared demand set, deduplicated"
    );
}

/// The readiness invariant itself: silence is not a terminal state.
#[test]
fn a_staged_character_is_unsettled_until_it_reaches_a_terminal_state() {
    let mut demand = CharacterLoadDemand::default();
    let mut states = CharacterLoadStates::default();
    demand.request("mary_o");

    assert!(
        !character_reveal_ready(&demand, &states),
        "the barrier must NOT open while a staged character has no answer"
    );
    assert_eq!(
        unsettled_staged_characters(&demand, &states),
        vec!["mary_o"]
    );

    // A named FAILURE settles it. Failure is legitimate — an art-free build has
    // no sheets — so the invariant forbids silence, not failure.
    states.record(
        "mary_o".to_string(),
        "mary_o",
        CharacterLoadOutcome::Failed(CharacterLoadFailure::NoSheetResolved),
    );
    assert!(character_reveal_ready(&demand, &states));
    assert!(unsettled_staged_characters(&demand, &states).is_empty());
}

/// The negative test §4.9 asks for: omit the capability, and the audit NAMES it.
#[test]
fn character_materialization_capability_audit_names_the_missing_service() {
    // A hand-assembled composition: someone wired demand without the engine
    // plugin group. Exactly the shape that shipped three times.
    let mut app = App::new();
    app.init_resource::<CharacterLoadDemand>();
    app.world_mut()
        .resource_mut::<CharacterLoadDemand>()
        .request("mary_o");

    let gaps = audit_character_capabilities(app.world());
    assert!(
        gaps.contains(&CharacterCapabilityGap::MaterializationServiceMissing {
            staged: vec!["mary_o".to_string()],
        }),
        "the audit must name the missing materialization service, got {gaps:?}"
    );
    // And it must name the CHARACTER, not just the capability: "something is
    // wrong somewhere" is the report that gets ignored.
    let report = gaps
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        report.contains("mary_o"),
        "report must name the character: {report}"
    );
    assert!(
        report.contains("CharacterMaterialization"),
        "report must name the missing service: {report}"
    );

    // The audit is also honest in the other direction: with the engine plugin
    // installed, the same staging produces NO service gap.
    let mut proper = App::new();
    proper.add_plugins(CharacterRuntimePlugin);
    proper
        .world_mut()
        .resource_mut::<CharacterLoadDemand>()
        .request("mary_o");
    assert!(
        !audit_character_capabilities(proper.world())
            .iter()
            .any(|gap| matches!(
                gap,
                CharacterCapabilityGap::MaterializationServiceMissing { .. }
            )),
        "the engine plugin must satisfy the capability it exists to provide"
    );
}

/// A missing CATALOG is its own named gap, not "no art".
#[test]
fn a_missing_character_catalog_is_named_rather_than_read_as_no_art() {
    let mut app = App::new();
    app.add_plugins(CharacterRuntimePlugin);
    app.world_mut()
        .resource_mut::<CharacterLoadDemand>()
        .request("mary_o");

    let gaps = audit_character_capabilities(app.world());
    assert!(
        gaps.contains(&CharacterCapabilityGap::CharacterCatalogMissing {
            staged: vec!["mary_o".to_string()],
        }),
        "a staged character with no catalog to resolve against must be named, got {gaps:?}"
    );
}

/// Staging nothing is not a broken composition. A main menu stages no cast, and
/// an audit that shouts at it is an audit people learn to ignore.
#[test]
fn an_app_that_stages_no_characters_reports_no_gaps() {
    let mut app = App::new();
    assert!(audit_character_capabilities(app.world()).is_empty());
    app.add_systems(Update, report_character_capability_gaps);
    app.update();
}

/// **A1.** A character declared ONLY through `register_character` is known to the
/// art pipeline.
///
/// This was the sharp end of the review finding. `register_character` accepted a
/// `sheet`, published a `PreparedCharacterDefinition`, and then the materializer
/// consulted `CharacterCatalog` and nothing else — so a character that existed
/// solely on the new seam came back `UnknownCharacter`, which means "no loaded
/// content declares this character; waiting will never fix it". Content had just
/// declared it. The ledger was reporting a typo about a provider's own protagonist.
///
/// Two facts are asserted, and the second is the one that matters: `Declared`
/// versus `Unknown` is the difference between "a decode has not happened yet" and
/// "this id does not exist", and §7.1 separated them precisely because a caller
/// must respond differently.
#[test]
fn a_character_registered_only_through_register_character_gets_art() {
    use crate::character_runtime::definition::CharacterDefinitionAppExt;

    let mut app = App::new();
    app.add_plugins(CharacterRuntimePlugin);
    // A catalog with NO characters in it: the only declaration is the registration.
    app.insert_resource(ambition_characters::actor::character_catalog::CharacterCatalog::empty());
    app.init_resource::<crate::character_sprites::AuthoredSheets>();
    app.insert_resource(ambition_sprite_sheet::game_assets::GameAssets::default());
    app.register_character(
        crate::character_runtime::definition::CharacterDefinition::new(
            "mary_o",
            "Mary-O",
            "mary_o_demo",
        )
        .with_sheet("super_mary_o_spritesheet"),
    );

    app.update();

    let sprites = &app
        .world()
        .resource::<ambition_sprite_sheet::game_assets::GameAssets>()
        .characters;
    assert!(
        !sprites.sheet_state("mary_o").is_unknown(),
        "a registered character must be DECLARED to the sheet table; `Unknown` \
         tells every caller this id does not exist"
    );
    assert!(
        !sprites.sheet_state("Mary-O").is_unknown(),
        "and under its display name, since content names characters both ways"
    );

    // Now stage her, the way a session does, and check the ledger's verdict is not
    // the "no such character" one.
    {
        let mut demand = app.world_mut().resource_mut::<CharacterLoadDemand>();
        demand.request("mary_o");
    }
    app.update();
    let outcome = app
        .world()
        .resource::<CharacterLoadStates>()
        .outcome("mary_o");
    assert!(
        !matches!(
            outcome,
            Some(CharacterLoadOutcome::Failed(
                CharacterLoadFailure::UnknownCharacter
            ))
        ),
        "a registered character must never be reported as unknown (was {outcome:?})"
    );
}

/// **The same frame it was registered in.**
///
/// The test above deliberately spends one whole update letting
/// `declare_registered_characters` run before it demands anything, so it says
/// nothing about the path direct startup and room transitions actually take:
/// they call `materialize_character_demand` SYNCHRONOUSLY, and no schedule edge
/// requires the declaring system to have run first. Losing that race does not
/// delay the character, it reports `UnknownCharacter` — "waiting will never
/// help" — about a character the caller had just handed over.
///
/// Driven against a sheet table nothing has touched, which is exactly the state
/// the synchronous callers can find it in.
#[test]
fn the_decode_path_declares_a_registered_character_itself() {
    let mut app = App::new();
    app.register_character(
        crate::character_runtime::definition::CharacterDefinition::new(
            "mary_o",
            "Mary-O",
            "mary_o_demo",
        )
        .with_sheet("super_mary_o_spritesheet"),
    );
    let registry = app.world().resource::<PreparedCharacterRegistry>().clone();
    let mut sprites = CharacterSpriteAssets::default();
    assert!(
        sprites.sheet_state("mary_o").is_unknown(),
        "the fixture must START in the racy state, or it proves nothing"
    );

    declare_registered_character_into(&mut sprites, &registry, "mary_o", "mary_o");

    assert!(
        !sprites.sheet_state("mary_o").is_unknown(),
        "the decode path must not need another system to have run first"
    );
    assert!(
        !sprites.sheet_state("Mary-O").is_unknown(),
        "and the display-name alias comes with it, since rooms stage by name"
    );

    // A token no provider registered stays unknown. This is the half that must
    // NOT change: `UnknownCharacter` is the right verdict for a typo, and a
    // declaration path that declared everything it was asked about would turn
    // every misspelling into a silent placeholder.
    let mut sprites = CharacterSpriteAssets::default();
    declare_registered_character_into(&mut sprites, &registry, "mary_oh", "mary_oh");
    assert!(sprites.sheet_state("mary_oh").is_unknown());
}
