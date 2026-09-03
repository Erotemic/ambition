//! The engine-owned character load pipeline, tested without any application.
//!
//! Every fixture here builds a bare `App` and adds ENGINE pieces only.

use ambition_characters::prepared::PreparedCharacterRegistry;
// Stepping a fixture is `finalize_and_update`, not `update`. Bevy's RUNNERS
// close the plugin-composition barrier; `App::update` does not, and character
// preparation publishes its registry there — so a fixture that only updated
// would register a cast and never publish one. Idempotent, so a helper called
// per step costs a set lookup after the first.
use ambition_platformer2d_shared_tangle::app_finalization::{finalize, finalize_and_update};

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
        requested_tier: ambition_persistence::settings::TextureResolutionScale::Full,
        resolved_tier: ambition_persistence::settings::TextureResolutionScale::Full,
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
fn publishing_a_sheet_reaches_every_token_that_declared_it() {
    let mut sprites = declared_sprites(&[("mary_o", "Mary-O")]);
    sprites.publish("mary_o", any_baked_sheet());

    // Both the id and the display name now resolve to the realization, and
    // neither reads as still-awaiting-a-decode. (The DECLARATION outlives the
    // publish — it is the recipe a quality transition needs to remake the
    // realization — but `Ready` wins the lookup, so a later demand short-circuits
    // instead of re-decoding.)
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
    for token in demand.take() {
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
    // A versus match is several worn bodies and only player one would ever have had a sheet.
    let mut app = App::new();
    app.add_plugins(CharacterRuntimePlugin);
    app.world_mut()
        .spawn(ambition_characters::actor::WornCharacter("mary_o".into()));
    app.world_mut()
        .spawn(ambition_characters::actor::WornCharacter("sanic".into()));
    finalize_and_update(&mut app);

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
    finalize_and_update(&mut app);
    app.world_mut().resource_mut::<CharacterLoadDemand>().take();

    // A runtime form change (Mary-O growing into `mary_o_tall`) is a new sheet.
    app.world_mut()
        .entity_mut(body)
        .insert(ambition_characters::actor::WornCharacter(
            "mary_o_tall".into(),
        ));
    finalize_and_update(&mut app);

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

/// The composition-parity test. Three semantically different entry routes,
/// one materialized result.
///
/// This is deliberately NOT "boot two apps and diff their resources". That test asserts
/// implementation details, goes red on every unrelated resource, and can pass while two apps still
/// stage characters differently.
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
        finalize_and_update(&mut app);
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
    finalize_and_update(&mut app);
}

/// A1. A character declared ONLY through `register_character` is known to the
/// art pipeline.
///
/// `register_character` accepted a `sheet`, published a `PreparedCharacterDefinition`, and then
/// the materializer consulted `CharacterCatalog` and nothing else — so a character that existed
/// solely on the new seam came back `UnknownCharacter`, which means "no loaded content declares
/// this character; waiting will never fix it". Content had just declared it. The ledger was
/// reporting a typo about a provider's own protagonist.
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
    app.init_resource::<ambition_sprite_sheet::character::sheets::AuthoredSheets>();
    app.insert_resource(ambition_sprite_sheet::game_assets::GameAssets::default());
    app.register_character(
        ambition_characters::actor::definition::CharacterDefinition::new(
            "mary_o",
            "Mary-O",
            "mary_o_demo",
        )
        .with_sheet("super_mary_o_spritesheet"),
    );

    finalize_and_update(&mut app);

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
    finalize_and_update(&mut app);
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

/// The same frame it was registered in.
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
        ambition_characters::actor::definition::CharacterDefinition::new(
            "mary_o",
            "Mary-O",
            "mary_o_demo",
        )
        .with_sheet("super_mary_o_spritesheet"),
    );
    finalize(&mut app);
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

// ── Live quality Apply: the residency transition ───────────────────────────────

mod live_quality_apply {
    use super::*;
    use ambition_asset_manager::AssetProfile;
    use ambition_persistence::settings::{
        TextureResolutionScale, UserSettings, VisualQualityProfile,
    };
    use ambition_sprite_sheet::game_assets::{GameAssetConfig, GameAssets};

    /// A composition with a REAL asset pipeline, at `profile`.
    ///
    /// `AssetProfile::AndroidBundle`, on purpose: its load gate *trusts the
    /// packager* and never pre-checks the host filesystem, so the materializer
    /// runs its production path end to end without the fixture depending on
    /// which gitignored PNGs happen to be on this machine. The one thing that
    /// must be present is the BAKED sheet-record table (`build.rs`), and
    /// [`a_character_with_a_scaled_variant`] returns `None` when it is not.
    fn quality_pipeline_app(profile: VisualQualityProfile) -> App {
        quality_pipeline_app_for(profile, crate::character_roster::catalog())
    }

    fn quality_pipeline_app_for(
        profile: VisualQualityProfile,
        characters: ambition_characters::actor::character_catalog::CharacterCatalog,
    ) -> App {
        let mut app = App::new();
        // The IO pool: `asset_server.load` spawns onto it, and without the
        // plugin the materializer panics inside the pool rather than failing.
        app.add_plugins(bevy::app::TaskPoolPlugin::default());
        app.add_plugins(bevy::asset::AssetPlugin::default());
        app.init_asset::<Image>();
        app.init_asset::<bevy::image::TextureAtlasLayout>();
        app.add_plugins(CharacterRuntimePlugin);

        let config = GameAssetConfig {
            asset_profile: AssetProfile::AndroidBundle,
            ..Default::default()
        };
        let catalog = crate::assets::platformer_assets::build_platformer2d_asset_catalog(
            &config,
            &characters,
            &ambition_boss_encounter::BossCatalog::default(),
            &ambition_audio::spec::MusicRegistry {
                default_track: String::new(),
                tracks: Vec::new(),
            },
            &ambition_platformer2d_world::world_manifest::WorldManifest::default(),
        );

        let mut settings = UserSettings::default();
        settings.video.quality.profile = profile;

        let mut assets = GameAssets::default();
        {
            let world = app.world_mut();
            let asset_server = world.resource::<AssetServer>().clone();
            let mut layouts = world.resource_mut::<Assets<bevy::image::TextureAtlasLayout>>();
            assets.characters = crate::character_sprites::load_character_sprites_in(
                &Default::default(),
                &characters,
                &catalog,
                &asset_server,
                &mut layouts,
                None,
            );
        }

        app.insert_resource(characters);
        app.insert_resource(catalog);
        app.insert_resource(config);
        app.insert_resource(settings);
        app.insert_resource(assets);
        app
    }

    /// A catalog id whose sheet has a baked HALF-tier variant, so the two tiers
    /// this transition moves between are both actually reachable.
    fn a_character_with_a_scaled_variant() -> Option<String> {
        use ambition_sprite_sheet::character::sheets::{
            try_load_spec_for_target_scaled, SheetTuning,
        };
        let catalog = crate::character_roster::catalog();
        let mut ids: Vec<&str> = catalog.iter().map(|(cid, _)| cid.as_str()).collect();
        ids.sort_unstable();
        for cid in ids {
            let Some(target) = catalog.get(cid).and_then(|e| e.manifest_target()) else {
                continue;
            };
            if try_load_spec_for_target_scaled(
                target,
                &SheetTuning::default(),
                ambition_sprite_sheet::character::TextureResolutionScale::Half,
            )
            .is_some()
            {
                return Some(cid.to_string());
            }
        }
        None
    }

    /// The asset PATH the resident realization of `token` points at — the one
    /// observable that says which pixels a body is actually drawn from.
    fn resident_image_path(app: &App, token: &str) -> Option<String> {
        let asset = app
            .world()
            .resource::<GameAssets>()
            .characters
            .sheet(token)?;
        app.world()
            .resource::<AssetServer>()
            .get_path(asset.texture.id())
            .map(|path| path.to_string())
    }

    /// A BODY on screen wearing `cid`. Convergence re-demands a retired sheet
    /// only for a character somebody wears or a live actor names — a fixture
    /// that demands a sheet nobody uses is a fixture whose sheet is correctly
    /// left retired — so every "already on screen" arm puts a body there.
    fn wear(app: &mut App, cid: &str) {
        app.world_mut()
            .spawn(ambition_characters::actor::WornCharacter::new(
                cid.to_string(),
            ));
    }

    /// Hit Apply, and let the transition settle.
    fn apply(app: &mut App, profile: VisualQualityProfile) {
        app.world_mut()
            .resource_mut::<UserSettings>()
            .video
            .quality
            .profile = profile;
        // Two steps: the first retires and re-demands, the second is where a
        // transition that needed another frame would show up. A delay is
        // acceptable; not converging is not.
        finalize_and_update(app);
        finalize_and_update(app);
    }

    /// A body already on screen must converge to the applied quality.
    ///
    /// Not "the profile changed" and not "`load_game_assets` ran" — both are
    /// already true while the feature is broken. This asks the only question
    /// that matters: after Apply, do the pixels behind a character that was
    /// ALREADY resident come from the new tier?
    ///
    /// the red, before the fix:
    /// ```text
    /// assertion `left != right` failed: the running game must converge to the
    /// applied quality: `goblin` is still drawn from
    /// `sprites_0_5x/goblin_spritesheet.png`
    ///   left: "sprites_0_5x/goblin_spritesheet.png"
    ///  right: "sprites_0_5x/goblin_spritesheet.png"
    /// ```
    #[test]
    fn apply_converges_an_already_resident_character_to_the_new_tier() {
        let Some(cid) = a_character_with_a_scaled_variant() else {
            panic!("no baked half-tier sheet variant: this fixture cannot prove anything");
        };
        let mut app = quality_pipeline_app(VisualQualityProfile::Medium);
        app.world_mut()
            .resource_mut::<CharacterLoadDemand>()
            .request(cid.clone());
        wear(&mut app, &cid);
        finalize_and_update(&mut app);

        let before = resident_image_path(&app, &cid)
            .unwrap_or_else(|| panic!("`{cid}` must materialize under the Medium profile"));
        assert!(
            before.contains("sprites_0_5x"),
            "the fixture must START on the half tier, or it proves nothing (got `{before}`)"
        );

        // APPLY. Medium (Half) -> High (Full). Nothing else changes: the same
        // body keeps the same identity and the same demand history.
        apply(&mut app, VisualQualityProfile::High);

        let after = resident_image_path(&app, &cid)
            .unwrap_or_else(|| panic!("`{cid}` must still be resident after Apply"));
        assert_ne!(
            before, after,
            "the running game must converge to the applied quality: `{cid}` is still \
             drawn from `{before}`"
        );
        assert!(
            !after.contains("sprites_0_5x"),
            "`{cid}` must be resident at the FULL tier after Apply, got `{after}`"
        );

        // LOGICAL IDENTITY IS UNTOUCHED. The realization moved; the character
        // did not. Both tokens still reach it, and the cast still names the id.
        let display = crate::character_roster::catalog()
            .get(&cid)
            .map(|entry| entry.display_name.clone())
            .expect("the fixture character has a catalog row");
        assert!(
            app.world()
                .resource::<GameAssets>()
                .characters
                .sheet_state(&display)
                .is_ready(),
            "`{display}` (the display-name token) must resolve to the NEW realization too"
        );
        assert!(
            app.world()
                .resource::<CharacterLoadStates>()
                .cast()
                .contains(&cid),
            "a quality change must not disturb the session cast"
        );
    }

    /// And downward, which is the direction memory behaves differently in.
    ///
    /// Full -> Half has to actually release the big pixels, not merely stop
    /// growing: the half realization replaces the full one under every token.
    #[test]
    fn apply_converges_downward_too() {
        let Some(cid) = a_character_with_a_scaled_variant() else {
            panic!("no baked half-tier sheet variant: this fixture cannot prove anything");
        };
        let mut app = quality_pipeline_app(VisualQualityProfile::High);
        app.world_mut()
            .resource_mut::<CharacterLoadDemand>()
            .request(cid.clone());
        wear(&mut app, &cid);
        finalize_and_update(&mut app);
        let before = resident_image_path(&app, &cid).expect("materializes at Full");
        assert!(!before.contains("sprites_0_5x"), "starts full: `{before}`");

        apply(&mut app, VisualQualityProfile::Low);

        let after = resident_image_path(&app, &cid).expect("still resident");
        assert!(
            after.contains("sprites_0_5x"),
            "`{cid}` must fall to the half tier on the way down, got `{after}`"
        );
    }

    /// A DIFFERENT PROFILE IS NOT A DIFFERENT TIER, and this is the guard
    /// that says so.
    ///
    /// `Low` and `Medium` both realize sheets at `Half`. A transition keyed on "the participant
    /// applied something" would retire and re-decode the whole cast to arrive at byte-identical
    /// pixels — a visible hitch, on a setting that changed nothing about sheets.
    #[test]
    fn a_profile_change_that_keeps_the_tier_retires_nothing() {
        let Some(cid) = a_character_with_a_scaled_variant() else {
            panic!("no baked half-tier sheet variant: this fixture cannot prove anything");
        };
        let mut app = quality_pipeline_app(VisualQualityProfile::Low);
        app.world_mut()
            .resource_mut::<CharacterLoadDemand>()
            .request(cid.clone());
        wear(&mut app, &cid);
        finalize_and_update(&mut app);
        let handle = app
            .world()
            .resource::<GameAssets>()
            .characters
            .sheet(&cid)
            .expect("resident")
            .texture
            .clone();

        apply(&mut app, VisualQualityProfile::Medium);

        assert_eq!(
            app.world()
                .resource::<GameAssets>()
                .characters
                .sheet(&cid)
                .expect("still resident")
                .texture,
            handle,
            "Low and Medium realize the same Half pixels; re-decoding for that is \
             a hitch the participant did not ask for"
        );
    }

    /// A retired sheet is re-decoded only for a character somebody still uses.
    ///
    /// Two characters resident at Half; one is worn by a body, the other is
    /// worn by nobody and named by no actor. Apply High: the worn one comes
    /// back at Full, the other stays retired. Before this rule the transition
    /// out of the hall re-decoded the whole gallery cast at Full into a room
    /// that placed five of them (measured 2026-09-02).
    #[test]
    fn apply_re_decodes_only_the_characters_still_in_use() {
        let mut variants = characters_with_scaled_variants(2);
        let (Some(unused), Some(worn_by_a_body)) = (variants.pop(), variants.pop()) else {
            panic!("need two baked half-tier sheet variants");
        };
        let mut app = quality_pipeline_app(VisualQualityProfile::Medium);
        app.world_mut()
            .resource_mut::<CharacterLoadDemand>()
            .request_all([worn_by_a_body.as_str(), unused.as_str()]);
        wear(&mut app, &worn_by_a_body);
        finalize_and_update(&mut app);
        finalize_and_update(&mut app);
        for cid in [&worn_by_a_body, &unused] {
            assert!(
                resident_image_path(&app, cid).is_some_and(|p| p.contains("sprites_0_5x")),
                "premise: `{cid}` is resident at Half before Apply"
            );
        }

        apply(&mut app, VisualQualityProfile::High);
        finalize_and_update(&mut app);

        let worn_after = resident_image_path(&app, &worn_by_a_body)
            .expect("the worn character is re-realized after Apply");
        assert!(
            !worn_after.contains("sprites_0_5x"),
            "the worn character converges to Full: {worn_after}"
        );
        assert!(
            resident_image_path(&app, &unused).is_none(),
            "a character nobody wears or names stays retired after Apply instead of being \
             re-decoded at Full for nobody: {:?}",
            resident_image_path(&app, &unused)
        );
        // Still declared: the next demand realizes it at the new tier.
        assert!(matches!(
            app.world()
                .resource::<GameAssets>()
                .characters
                .sheet_state(&unused),
            ambition_sprite_sheet::character::CharacterSheetState::Declared { .. }
        ));
    }

    /// The invariant: after Apply completes there is exactly ONE active
    /// quality generation across the live residency set — including a
    /// character that was materialized only AFTER the transition.
    ///
    /// A survivor and a newcomer sharing one tier is the whole claim. Two tiers
    /// resident means some body on screen is drawn from pixels nobody asked for.
    #[test]
    fn after_apply_exactly_one_tier_is_resident_survivors_and_newcomers_alike() {
        let mut variants = characters_with_scaled_variants(2);
        let (Some(newcomer), Some(survivor)) = (variants.pop(), variants.pop()) else {
            panic!("need two baked half-tier sheet variants to tell a survivor from a newcomer");
        };

        let mut app = quality_pipeline_app(VisualQualityProfile::Medium);
        app.world_mut()
            .resource_mut::<CharacterLoadDemand>()
            .request(survivor.clone());
        wear(&mut app, &survivor);
        finalize_and_update(&mut app);
        assert_eq!(
            app.world()
                .resource::<GameAssets>()
                .characters
                .resident_tiers(),
            [TextureResolutionScale::Half].into_iter().collect(),
            "the survivor must start at Half or this proves nothing"
        );

        apply(&mut app, VisualQualityProfile::High);

        // A body that arrives AFTER the transition — a room's next enemy, a
        // summon — must land on the same generation the survivor moved to.
        app.world_mut()
            .resource_mut::<CharacterLoadDemand>()
            .request(newcomer.clone());
        wear(&mut app, &newcomer);
        finalize_and_update(&mut app);

        let tiers = app
            .world()
            .resource::<GameAssets>()
            .characters
            .resident_tiers();
        assert_eq!(
            tiers,
            [TextureResolutionScale::Full].into_iter().collect(),
            "one generation for the whole live residency set; got {tiers:?}"
        );
        for cid in [&survivor, &newcomer] {
            assert!(
                app.world()
                    .resource::<GameAssets>()
                    .characters
                    .sheet(cid)
                    .is_some(),
                "`{cid}` must be resident after the transition"
            );
        }
    }

    /// `resident_tiers()` answers PHYSICAL residency, and a fallback is
    /// the only case it exists for.
    ///
    /// Its own docstring promises that more than one tier in the set means
    /// *"some body on screen is being drawn from pixels the user stopped asking
    /// for"*. A `Half` budget that falls back to the authored full-res PNG is
    /// precisely that body — and it was the one case the function could not see,
    /// because the set was built from the tier each realization ANSWERS.
    ///
    /// Two characters, one budget. One has a baked half variant and holds half
    /// pixels; the other's variant lookup cannot resolve and it holds FULL
    /// pixels. the image paths are the second, independent route to the same
    /// fact — the test reads them and asserts the tier set agrees with them,
    /// so it cannot pass by the two answers being wrong together.
    #[test]
    fn resident_tiers_names_the_tier_of_the_pixels_not_the_request() {
        // Any character that is NOT the one whose manifest the fixture breaks.
        let Some(scaled) = characters_with_scaled_variants(3)
            .into_iter()
            .find(|cid| cid != UNBAKED_VARIANT_CID)
        else {
            panic!("need a baked half-tier variant to stand for the honest half of the cast");
        };

        let mut app = quality_pipeline_app_for(
            VisualQualityProfile::Medium,
            roster_with_one_unbaked_variant(UNBAKED_VARIANT_CID),
        );
        for cid in [scaled.as_str(), UNBAKED_VARIANT_CID] {
            app.world_mut()
                .resource_mut::<CharacterLoadDemand>()
                .request(cid);
        }
        // ⚠ TWO characters, so this needs TWO frames: the materializer starts at
        // most `MAX_CHARACTERS_MATERIALIZED_PER_FRAME` per frame, deliberately, so
        // that a fighter's ~470MB of sheets do not all land on one frame. This
        // test is about which TIER the pixels come from, not about pacing, so it
        // steps until the demand is drained.
        finalize_and_update(&mut app);
        for _ in 0..8 {
            if app.world().resource::<CharacterLoadDemand>().is_empty() {
                break;
            }
            finalize_and_update(&mut app);
        }

        // The fixture must actually be mixed, or it proves nothing.
        let scaled_path = resident_image_path(&app, &scaled)
            .unwrap_or_else(|| panic!("`{scaled}` must materialize under Medium"));
        let fallback_path = resident_image_path(&app, UNBAKED_VARIANT_CID)
            .unwrap_or_else(|| panic!("`{UNBAKED_VARIANT_CID}` must materialize under Medium"));
        assert!(
            scaled_path.contains("sprites_0_5x"),
            "the honest half of the cast must hold HALF pixels (got `{scaled_path}`)"
        );
        assert!(
            !fallback_path.contains("sprites_0_5x"),
            "the fallback half of the cast must hold FULL pixels (got `{fallback_path}`)"
        );

        //  two physical tiers ARE resident. The set must say so.
        let tiers = app
            .world()
            .resource::<GameAssets>()
            .characters
            .resident_tiers();
        assert_eq!(
            tiers,
            [TextureResolutionScale::Half, TextureResolutionScale::Full]
                .into_iter()
                .collect(),
            "`{scaled}` is drawn from `{scaled_path}` and `{UNBAKED_VARIANT_CID}` from \
             `{fallback_path}` — two tiers are physically resident and the residency \
             set reported {tiers:?}"
        );
    }

    /// The catalog id the fixture below breaks the variant lookup for. Any id
    /// that resolves a base spec BY ID works; this one is the roster's oldest.
    const UNBAKED_VARIANT_CID: &str = "goblin";

    /// The real roster with ONE character's manifest pointed at a target nobody
    /// baked, so its scaled-variant lookup cannot resolve and the materializer
    /// is forced down its fallback arm — it loads the authored full-res PNG
    /// under a scaled budget. `try_load_spec_for_character_id` still answers for
    /// the id, so the character does materialize.
    ///
    /// constructed rather than found: every character in the shipped roster
    /// currently has every variant baked, so a fixture that went looking for a
    /// gap would pass vacuously on this checkout — and describe a state a fresh
    /// clone (no variants generated at all) is entirely made of.
    fn roster_with_one_unbaked_variant(
        cid: &str,
    ) -> ambition_characters::actor::character_catalog::CharacterCatalog {
        let mut data = crate::character_roster::catalog().data().clone();
        data.characters
            .get_mut(cid)
            .unwrap_or_else(|| panic!("`{cid}` must be in the shipped roster"))
            .manifest = "sprites/a_target_nobody_baked.ron".to_string();
        ambition_characters::actor::character_catalog::CharacterCatalog::from_data(data)
    }

    /// A TIER WITH NO BAKED VARIANT MUST SETTLE, NOT THRASH.
    ///
    /// Not every sheet has every variant generated, so a `Half` budget legitimately loads the
    /// authored full-res PNG for some characters.
    #[test]
    fn a_character_with_no_baked_variant_settles_at_the_requested_tier() {
        // A character whose catalog row names a manifest target nothing baked,
        // so the scaled-variant lookup CANNOT resolve and the materializer is
        // forced down its fallback arm — while `goblin` still resolves a base
        // spec by id, so the character does materialize.
        //
        // constructed rather than found: every character in the shipped roster
        // currently has every variant baked, and a fixture that goes looking for
        // a gap would pass vacuously on this checkout and describe a state a
        // fresh clone (no variants generated at all) is entirely made of.
        let catalog = ambition_characters::actor::character_catalog::CharacterCatalog::from_data(
            ambition_characters::actor::character_catalog::parse_catalog(
                r#"(
                    brain_presets: { "stand_still": StandStill },
                    action_set_presets: {
                        "peaceful": (move_style: Walk, melee: None, ranged: None, special: None),
                    },
                    characters: {
                        "goblin": (
                            display_name: "Fallback Goblin",
                            spritesheet: "sprites/goblin_spritesheet.png",
                            manifest: "sprites/a_target_nobody_baked.ron",
                            tier: MainHall, body_kind: Standard, composition: None,
                            default_brain: "stand_still", default_action_set: "peaceful",
                        ),
                    },
                )"#,
            ),
        );
        let mut app = quality_pipeline_app_for(VisualQualityProfile::Low, catalog);
        app.world_mut()
            .resource_mut::<CharacterLoadDemand>()
            .request("goblin");
        finalize_and_update(&mut app);
        let settled = resident_image_path(&app, "goblin").expect("resident");
        assert!(
            !settled.contains("sprites_0_5x"),
            "the fixture must FALL BACK to the authored PNG, or it proves \
             nothing (got `{settled}`)"
        );

        for _ in 0..3 {
            finalize_and_update(&mut app);
            assert!(
                app.world().resource::<CharacterLoadDemand>().is_empty(),
                "`goblin` is being re-demanded every frame: the transition never \
                 reaches a fixed point"
            );
            assert_eq!(
                resident_image_path(&app, "goblin").as_deref(),
                Some(settled.as_str()),
                "and it is being retired and remade every frame"
            );
        }
    }

    /// Art the engine did not build is not the engine's to delete.
    ///
    /// A host that publishes its own realization
    /// ([`CharacterSpriteAssets::publish_under`]) leaves no declaration behind,
    /// so there is no recipe to remake it. Retiring it on a quality change would
    /// be a one-way deletion — the intro's NPCs would lose their faces the first
    /// time anybody touched the quality slider.
    #[test]
    fn a_host_published_realization_survives_the_transition() {
        let mut app = quality_pipeline_app(VisualQualityProfile::Medium);
        app.world_mut()
            .resource_mut::<GameAssets>()
            .characters
            .publish_under("A Bespoke Extra", any_baked_sheet());
        finalize_and_update(&mut app);

        apply(&mut app, VisualQualityProfile::Potato);

        assert!(
            app.world()
                .resource::<GameAssets>()
                .characters
                .sheet("A Bespoke Extra")
                .is_some(),
            "a realization the engine cannot remake must not be retired"
        );
    }

    /// Up to `n` catalog ids whose sheets have baked half-tier variants.
    fn characters_with_scaled_variants(n: usize) -> Vec<String> {
        use ambition_sprite_sheet::character::sheets::{
            try_load_spec_for_target_scaled, SheetTuning,
        };
        let catalog = crate::character_roster::catalog();
        let mut ids: Vec<&str> = catalog.iter().map(|(cid, _)| cid.as_str()).collect();
        ids.sort_unstable();
        let mut out = Vec::new();
        for cid in ids {
            if out.len() == n {
                break;
            }
            let Some(target) = catalog.get(cid).and_then(|e| e.manifest_target()) else {
                continue;
            };
            if try_load_spec_for_target_scaled(
                target,
                &SheetTuning::default(),
                ambition_sprite_sheet::character::TextureResolutionScale::Half,
            )
            .is_some()
            {
                out.push(cid.to_string());
            }
        }
        out
    }

    /// THE CACHE ALREADY EXISTS, AND THIS IS WHERE THAT IS WRITTEN DOWN.
    ///
    /// The answer is no: `materialize_declared_character_sprite` opens with
    /// `CharacterSheetState:Ready(_) => return`, before any sheet lookup, atlas build or handle
    /// request.
    ///
    /// counted in ATLAS LAYOUTS, not in wall time. A timing assertion here
    /// would be a flaky performance test; a layout that gets built twice is the
    /// actual work, and it is countable. If a future change reintroduces repeat
    /// preparation this grows and says so.
    #[test]
    fn re_demanding_a_resident_character_repeats_no_preparation() {
        let Some(cid) = a_character_with_a_scaled_variant() else {
            panic!("no baked sheet variant: this fixture cannot prove anything");
        };
        let mut app = quality_pipeline_app(VisualQualityProfile::Medium);

        app.world_mut()
            .resource_mut::<CharacterLoadDemand>()
            .request(cid.clone());
        wear(&mut app, &cid);
        finalize_and_update(&mut app);
        let resident = resident_image_path(&app, &cid)
            .unwrap_or_else(|| panic!("`{cid}` must materialize on first demand"));
        let layouts_after_first = app
            .world()
            .resource::<Assets<bevy::image::TextureAtlasLayout>>()
            .len();

        // The same character, demanded again exactly as a second room staging it
        // would — the token, through the ordinary demand seam.
        app.world_mut()
            .resource_mut::<CharacterLoadDemand>()
            .request(cid.clone());
        wear(&mut app, &cid);
        finalize_and_update(&mut app);

        let layouts_after_second = app
            .world()
            .resource::<Assets<bevy::image::TextureAtlasLayout>>()
            .len();
        assert_eq!(
            layouts_after_second, layouts_after_first,
            "re-demanding the resident character `{cid}` built another atlas layout, so \
             every room that stages an already-prepared character pays for it again. The \
             Hall's 18ms manifest frame would then be 18ms on every visit rather than the \
             first"
        );
        assert_eq!(
            resident_image_path(&app, &cid).as_deref(),
            Some(resident.as_str()),
            "and the second demand must not have replaced the resident realization"
        );
    }
}

/// ⭐⭐ A ROSTER MUST RAISE DEMAND BEFORE ANY BODY EXISTS.
///
/// The measured hitch: `demand_actor_character_sheets` keys on
/// `Added<ActorConfig>`, so nothing asked for a fighter's ~7 4096x4096 sheets
/// (~470MB of RGBA) until the body stood on the stage — and the first hardware
/// profile caught +307 megapixels decoding inside a 2.5s window whose worst frame
/// was 516ms.
///
/// ⛔ THE CONTROL IS "NO BODY", AND IT IS THE WHOLE TEST. If a body existed, the
/// spawn-keyed system could satisfy this assertion and the roster path could be
/// dead code. The world here has no entities at all.
#[test]
fn a_roster_demands_its_cast_before_any_body_is_spawned() {
    use super::demand_rostered_character_sheets;
    use super::staging::{MatchParticipant, MatchParticipantRoster};
    use ambition_characters::load_demand::CharacterLoadDemand;

    let mut app = App::new();
    app.init_resource::<CharacterLoadDemand>();
    app.add_systems(bevy::app::Update, demand_rostered_character_sheets);

    let mut roster = MatchParticipantRoster::default();
    roster.participants.push(MatchParticipant::new("noether"));
    roster
        .participants
        .push(MatchParticipant::new("perfect_cellular_automaton"));
    app.insert_resource(roster);
    app.update();

    // ⛔ COUNT NON-RESOURCE ENTITIES, NOT ENTITIES. Bevy 0.19 stores every
    // resource AS A COMPONENT on its own entity, so `entities().len()` here is
    // 16 — the resources this App initialised — and has nothing to do with
    // whether a body was spawned. `IsResource` is the marker Bevy added to
    // separate the two populations, and the control this test rests on is about
    // the second one.
    let spawned = app
        .world_mut()
        .query_filtered::<bevy::prelude::Entity, bevy::prelude::Without<bevy::ecs::resource::IsResource>>()
        .iter(app.world())
        .count();
    assert_eq!(
        spawned, 0,
        "the point of this test is that NOTHING is spawned — if a body exists, \
         the spawn-keyed demand system could be the one satisfying it"
    );
    let demand = app.world().resource::<CharacterLoadDemand>();
    let pending: Vec<&str> = demand.pending().collect();
    assert!(
        pending.contains(&"noether") && pending.contains(&"perfect_cellular_automaton"),
        "a published roster must ask for its whole cast with no body seated, got {pending:?}"
    );
}

/// And the arm that makes the one above mean something: no roster, no demand.
#[test]
fn without_a_roster_nothing_is_demanded() {
    use super::demand_rostered_character_sheets;
    use ambition_characters::load_demand::CharacterLoadDemand;

    let mut app = App::new();
    app.init_resource::<CharacterLoadDemand>();
    app.add_systems(bevy::app::Update, demand_rostered_character_sheets);
    app.update();

    assert_eq!(
        app.world()
            .resource::<CharacterLoadDemand>()
            .pending()
            .count(),
        0,
        "demand appeared from nowhere, so the roster test proves nothing"
    );
}

/// ⭐⭐ A FRAME MAY START ONE CHARACTER, AND THE REST MUST SURVIVE TO THE NEXT.
///
/// A character is ~7 sheets at 4096x4096 (~470MB of RGBA), and draining the whole
/// demand set in one frame is what put `extract_render_asset<GpuImage>` at 454.9ms
/// inside a 516ms frame on hardware. `take_bounded` spreads the STARTS so the
/// finishes land on different frames.
///
/// ⛔ THE SECOND ASSERTION IS THE ONE THAT MATTERS: a limit that DROPPED the
/// remainder would also "fix" the hitch, by never loading the other fighter.
#[test]
fn bounding_the_take_defers_the_rest_instead_of_dropping_it() {
    use ambition_characters::load_demand::CharacterLoadDemand;

    let mut demand = CharacterLoadDemand::default();
    for token in ["author", "noether", "perfect_cellular_automaton"] {
        demand.request(token);
    }

    let first = demand.take_bounded(1);
    assert_eq!(first.len(), 1, "one frame may start exactly one character");
    assert_eq!(
        demand.pending().count(),
        2,
        "the characters not started must remain PENDING — dropping them would \
         hide the hitch by never loading the other fighter"
    );

    let second = demand.take_bounded(1);
    let third = demand.take_bounded(1);
    assert_eq!(
        demand.pending().count(),
        0,
        "everything is eventually taken"
    );

    let mut all: Vec<String> = first.into_iter().chain(second).chain(third).collect();
    all.sort();
    assert_eq!(
        all,
        vec![
            "author".to_string(),
            "noether".to_string(),
            "perfect_cellular_automaton".to_string()
        ],
        "every demanded character is taken exactly once across the frames"
    );
}

/// ⭐ THE RATION IS PIXELS, NOT HEADS. The bound was measured on Full sheets
/// (~470 MB of RGBA per character); at a lower SETTING a sheet is smaller, so
/// a Quarter setting starts sixteen per frame under the SAME byte budget. Full
/// tokens go one per frame, and the first token is always taken so nothing is
/// ever stranded. The tier is the setting's — one for every token; a demand
/// cannot name a lower one (Jon, 2026-09-02).
#[test]
fn the_ration_spends_pixels_so_a_quarter_setting_starts_sixteen_a_frame() {
    use super::{materialization_units, MATERIALIZATION_UNITS_PER_FRAME};
    use ambition_characters::load_demand::CharacterLoadDemand;
    use ambition_persistence::settings::TextureResolutionScale as Tier;
    assert_eq!(
        MATERIALIZATION_UNITS_PER_FRAME, 16,
        "one Full character per frame"
    );

    // 40 tokens at a Quarter setting: 16, 16, 8.
    let mut demand = CharacterLoadDemand::default();
    demand.request_all((0..40).map(|i| format!("pedestal_{i:02}")));
    let frames: Vec<usize> = std::iter::from_fn(|| {
        let taken = demand.take_within_budget(
            MATERIALIZATION_UNITS_PER_FRAME,
            materialization_units(Tier::Quarter),
        );
        (!taken.is_empty()).then_some(taken.len())
    })
    .collect();
    assert_eq!(
        frames,
        vec![16, 16, 8],
        "a Quarter setting fills the ration sixteen at a time"
    );
    assert_eq!(demand.pending().count(), 0);

    // At Full, one per frame.
    let mut demand = CharacterLoadDemand::default();
    demand.request_all(["author", "noether", "turing"]);
    let first = demand.take_within_budget(
        MATERIALIZATION_UNITS_PER_FRAME,
        materialization_units(Tier::Full),
    );
    assert_eq!(first.len(), 1, "a Full character is a whole frame's ration");
    assert_eq!(
        demand.pending().count(),
        2,
        "the rest wait, they are not dropped"
    );

    // Half (4): four per frame, and the first token of a frame is always
    // taken whatever it costs.
    let mut demand = CharacterLoadDemand::default();
    demand.request_all(["a", "b", "c", "d", "e"]);
    let first = demand.take_within_budget(
        MATERIALIZATION_UNITS_PER_FRAME,
        materialization_units(Tier::Half),
    );
    assert_eq!(first.len(), 4);
    let second = demand.take_within_budget(
        MATERIALIZATION_UNITS_PER_FRAME,
        materialization_units(Tier::Half),
    );
    assert_eq!(second, vec!["e".to_string()]);
}

/// A limit of zero, or a set smaller than the limit, takes everything — so the
/// bound can never strand a token.
#[test]
fn an_unbounded_or_undersized_take_drains_completely() {
    use ambition_characters::load_demand::CharacterLoadDemand;

    let mut demand = CharacterLoadDemand::default();
    demand.request("author");
    assert_eq!(demand.take_bounded(4).len(), 1);
    assert_eq!(demand.pending().count(), 0);

    demand.request("noether");
    assert_eq!(
        demand.take_bounded(0).len(),
        1,
        "a zero limit means unbounded"
    );
    assert_eq!(demand.pending().count(), 0);
}

// ── THE LATE-ART INSTRUMENT, BOTH DIRECTIONS ───────────────────────────────

/// A world with one rostered fighter, a match that has already gone live, and
/// whatever load outcome the caller wants that fighter to have.
fn live_match_with_roster_outcome(outcome: Option<super::CharacterLoadOutcome>) -> App {
    let mut app = App::new();
    let mut roster = super::staging::MatchParticipantRoster::default();
    roster
        .participants
        .push(super::staging::MatchParticipant::new("iron_mary"));
    app.insert_resource(roster);

    let mut states = super::CharacterLoadStates::default();
    if let Some(outcome) = outcome {
        states.record("iron_mary".to_string(), "iron_mary", outcome);
    }
    app.insert_resource(states);

    // `activated_on: Some(0)` with a tick of 0 and the default ruleset's zero
    // countdown puts the match past its opening on the first observed frame.
    app.insert_resource(super::seating::ActiveMatch::activated(
        1,
        None,
        None,
        Some(0),
    ));
    app.insert_resource(super::prepared_match::PreparedMatch::for_test_published_by(
        None,
    ));
    app.insert_resource(ambition_time::SimTick::default());
    app.init_resource::<super::audit::LateMatchCriticalArt>();
    app.add_systems(Update, super::audit::report_late_match_critical_art);
    app.update();
    app
}

#[test]
fn a_rostered_fighter_still_resolving_after_the_bell_is_named() {
    let app = live_match_with_roster_outcome(None);
    let late = app.world().resource::<super::audit::LateMatchCriticalArt>();
    assert_eq!(
        late.late_characters().collect::<Vec<_>>(),
        vec!["iron_mary"],
        "a roster character with no terminal outcome once the match is live is \
         decoding on gameplay frames, which is the whole contract this names"
    );
    assert_eq!(late.unready_frames(), 1);
}

#[test]
fn a_rostered_fighter_whose_art_settled_is_not_named() {
    let app = live_match_with_roster_outcome(Some(super::CharacterLoadOutcome::Ready));
    let late = app.world().resource::<super::audit::LateMatchCriticalArt>();
    assert!(late.late_characters().next().is_none());
    assert_eq!(late.unready_frames(), 0);
    // ⚠ THE POPULATION BESIDE THE FINDING. Without this the clean result above
    // is indistinguishable from a system that never ran at all — which is how a
    // zero from an instrument that reports nothing gets believed.
    assert_eq!(
        late.live_frames_observed(),
        1,
        "the instrument must have OBSERVED a live match to report a clean one"
    );
}

#[test]
fn a_fighter_whose_art_failed_is_a_content_defect_not_a_late_load() {
    let app = live_match_with_roster_outcome(Some(super::CharacterLoadOutcome::Failed(
        super::CharacterLoadFailure::NoSheetResolved,
    )));
    let late = app.world().resource::<super::audit::LateMatchCriticalArt>();
    assert!(
        late.late_characters().next().is_none(),
        "a sheet that will never resolve is the art tests' defect. Counting it \
         here would report a missing manifest as a performance violation on \
         every frame of every match"
    );
    assert_eq!(late.live_frames_observed(), 1);
}
