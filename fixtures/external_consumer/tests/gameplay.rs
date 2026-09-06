//! The external consumer's own acceptance gate (Phase-6 /:
//! "the fixture should contain integration tests rather than relying only on
//! binaries that print success"). Run from the engine repo with
//! `cargo test --manifest-path fixtures/external_consumer/Cargo.toml` — the
//! independent workspace resolves its own dependency graph, so this is
//! exactly the build a third-party consumer gets.

/// Boot → activate → verify population → charge the beacon → walk the ridge
/// gate. One test, the whole authored surface: the room (construction), the
/// character (catalog), the sentry (character definition + stager, lowered as a
/// construction plan row), the consumer's own authoritative component (§authority), and the
/// transition (`transit_body`) — all through the public `ambition_platformer2d` umbrella with
/// zero engine edits.
#[test]
fn outlander_boots_activates_and_walks_the_ridge_gate() {
    let mut app = outlander::build_outlander_app();
    let report = outlander::run_outlander_walkthrough(&mut app)
        .unwrap_or_else(|error| panic!("the Outlander walkthrough failed: {error}"));
    assert!(
        report.player_pos.y < 300.0,
        "the gate must deliver the player to the upper ledge, got {:?}",
        report.player_pos
    );
    assert!(
        report.beacon.is_full(),
        "the gate is supposed to be GATED on the consumer's own authoritative \
         state; a gate that fired on an uncharged beacon is testing nothing: {:?}",
        report.beacon
    );
}

/// Task 1's exit criterion, answered from outside the engine.
///
/// *"A feature-owned authoritative component and system are mechanically
/// accounted, run under the simulation gate, and survive real
/// rewind/resimulation without edits to a giant runtime list."*
///
/// Every word of that is checked here, and it has to be here: the engine's own
/// registrations are crate-private conveniences away from being unusable by
/// anyone else, and a test living inside the workspace cannot tell the
/// difference. `BeaconCharge` is declared in this crate, encoded by this crate,
/// registered by this crate through `ambition_platformer2d::runtime::rollback`, and named in
/// no engine file.
///
/// The rewind is REAL, not simulated: a GGRS sync-test session resimulates every frame from a
/// restored snapshot and compares checksums, so a component that failed to round-trip — or an
/// encoder that dropped `ticks` while keeping `seconds` — panics inside the engine before this
/// test's own assertions run.
#[test]
fn consumer_owned_authoritative_state_survives_real_resimulation() {
    let mut app = outlander::build_outlander_rollback_app()
        .unwrap_or_else(|error| panic!("the Outlander rollback host failed to start: {error}"));
    let rollback = outlander::run_outlander_walkthrough(&mut app).unwrap_or_else(|error| {
        panic!("the Outlander walkthrough failed under the rollback host: {error}")
    });

    assert!(
        rollback.beacon.ticks > 0 && rollback.beacon.is_full(),
        "the beacon never charged under the rollback host, so the resimulation \
         compared a component that was `default()` on every frame and the \
         checksum agreement is vacuous: {:?}",
        rollback.beacon
    );
    assert!(
        rollback.player_pos.y < 300.0,
        "the gate never fired under the rollback host, so nothing downstream of \
         the rewound state was exercised, got {:?}",
        rollback.player_pos
    );

    // Same content, same input, two hosts. The fixed-tick run is the reference
    // timeline; a rollback host that resimulates correctly has to reproduce it
    // exactly, and `ticks` is an integer so "exactly" is literal.
    let mut fixed = outlander::build_outlander_app();
    let reference = outlander::run_outlander_walkthrough(&mut fixed)
        .unwrap_or_else(|error| panic!("the fixed-tick reference walk failed: {error}"));
    assert_eq!(
        rollback.beacon, reference.beacon,
        "the rollback host reached a different authoritative state than the \
         fixed-tick host from the same content and the same inputs"
    );
    assert_eq!(
        rollback.ticks_to_gate, reference.ticks_to_gate,
        "the two hosts opened the gate on different ticks, so one of them is not \
         running the timeline the other is"
    );
}

/// A third party gets REAL ART, not coloured rectangles. (Phase 6, the
/// visible-shell half)
///
/// A stranger cloning this engine and running their game saw untextured boxes — the most visible
/// way "an engine another game can be built on" can fail, and it failed for want of a function.
///
/// `ambition_platformer2d::game_assets::PlatformerAssetsPlugin` is that function, and this is
/// the test that it works from OUTSIDE the workspace: the fixture resolves its
/// own dependency graph, so this is the build a third party gets.
///
/// Asserts the two resources the generic presentation actually reads.
#[test]
fn the_umbrella_asset_install_gives_an_external_consumer_real_sprites() {
    // The REAL composition, not a hand-rolled subset of it.
    //
    // `with_game_assets` because that is exactly the subject: a display-less
    // host that still prepares art. It is policy rather than a face, and the
    // default is off — preparing art is not free.
    let mut app = ambition_platformer2d::app::PlatformerApp::headless()
        .with_game_assets()
        .mount(outlander::OutlanderModule)
        .build();
    app.update();

    let catalog = app
        .world()
        .get_resource::<ambition_platformer2d::view::Platformer2dAssetCatalog>()
        .expect(
            "the plugin did not install a Platformer2dAssetCatalog, so every asset path \
             policy the presentation reads is missing",
        );
    assert!(
        catalog
            .path_for(&ambition_platformer2d::view::ids::sfx_bank())
            .is_some(),
        "the installed catalog resolves no paths at all, so it was built from \
         nothing — the composition-order failure this plugin is supposed to make \
         loud"
    );

    let assets = app
        .world()
        .get_resource::<ambition_platformer2d::view::GameAssets>()
        .expect("the plugin did not install GameAssets");
    assert!(
        !assets.entities.is_empty(),
        "GameAssets carries no entity sprites, so the world still draws as \
         coloured primitives — which is the exact state this plugin was written \
         to end, and a green compile would not have noticed"
    );
}

/// A consumer's OWN art has a home. (Phase 6, recorded SDK leak #3)
///
/// The visible binary's second recorded finding read: "the AssetServer file root
/// must be pointed at the ENGINE's asset tree … consumer-owned art still has no
/// home, and a consumer that forgets this line gets bare boxes." So a third
/// party could load the engine's sprites or nothing; there was nowhere for its
/// own to live, because the two-root reader that lets Ambition's content crate
/// own a world tree lived inside `ambition_app`'s CLI module.
///
/// It is `ambition_asset_manager::consumer_source` now.
#[test]
fn a_consumer_owns_its_own_asset_tree_and_still_sees_the_engines() {
    use bevy::prelude::*;

    // Through the game's real composition: the `game://` source is DECLARED on
    // the module and installed by the engine before `AssetPlugin` seals its
    // sources. A test that registered it by hand would be asserting that the
    // test can call `register_asset_source`, not that a consumer's declaration
    // reaches the AssetServer.
    let app = outlander::build_outlander_app();

    // Through the real `AssetServer`, which is what every load path uses — not
    // a hand-built reader that would only prove this test can construct one.
    let server = app.world().resource::<AssetServer>();
    let source = server
        .get_source(bevy::asset::io::AssetSourceId::from("game"))
        .expect("the consumer registered a `game://` source");
    let reader = source.reader();

    let read = |path: &str| -> bool {
        bevy::tasks::block_on(async { reader.read(std::path::Path::new(path)).await.is_ok() })
    };

    assert!(
        read("sprites/outlander_marker.txt"),
        "a file that exists ONLY in this consumer's own assets dir did not \
         resolve through its `game://` source, so consumer-owned art still has \
         nowhere to live"
    );
    assert!(
        read("sprites/robot_spritesheet.ron"),
        "a file that exists only in the ENGINE's tree did not resolve, so the \
         consumer source shadowed the engine instead of layering over it — the \
         opposite failure, and the one that makes every shared sprite vanish"
    );
    assert!(
        !read("sprites/definitely_not_authored_anywhere.txt"),
        "a path in NEITHER tree resolved, so this test cannot tell a hit from a \
         reader that says yes to everything"
    );
}

/// Authoring failures exposed through the public API must name the file, ID, or
/// field the external consumer needs to change.
///
/// Every case below asserts on the message a consumer would actually see. The
/// requirement is deliberately concrete: the message must name the thing the
/// author has to go and change. "invalid catalog" is a rejection; "fragment
/// 'outlander' names missing default character 'typo_id'" is a diagnostic.
#[test]
fn authoring_mistakes_name_the_thing_the_author_must_fix() {
    use ambition_platformer2d::character::CharacterCatalogFragment;

    // A catalog that parses and names a default character it does not contain —
    // the single most common authoring slip, a typo in one id.
    let good_ron = r#"(
        brain_presets: { "still": StandStill },
        action_set_presets: {
            "none": (move_style: Walk, melee: None, ranged: None, special: None),
        },
        characters: {
            "outlander_wanderer": (
                display_name: "Wanderer",
                spritesheet: "sprites/robot_spritesheet.png",
                manifest: "sprites/robot_spritesheet.ron",
                tier: Basement,
                body_kind: Standard,
                composition: None,
                default_brain: "still",
                default_action_set: "none",
                tags: [],
            ),
        },
    )"#;
    let missing_default =
        CharacterCatalogFragment::from_ron("outlander", Some("wandrer_typo"), good_ron)
            .expect_err("a default character that is not in the fragment must be refused");
    let message = missing_default.to_string();
    for needle in ["outlander", "wandrer_typo"] {
        assert!(
            message.contains(needle),
            "the diagnostic for a mistyped default character does not name \
             `{needle}`, so an author outside this workspace cannot tell which \
             fragment or which id to fix: {message}"
        );
    }

    let malformed = CharacterCatalogFragment::from_ron(
        "outlander",
        None::<String>,
        "( characters: { \"x\": ( display_name: ",
    )
    .expect_err("truncated RON must be refused");
    let message = malformed.to_string();
    assert!(
        message.contains("outlander"),
        "a malformed fragment's diagnostic does not name the provider, so a host \
         composing several cannot tell whose content is broken: {message}"
    );
    assert!(
        message.to_lowercase().contains("malformed") || message.to_lowercase().contains("ron"),
        "the diagnostic does not say the content failed to PARSE, which is the \
         difference between a typo and a rule the author has to look up: {message}"
    );

    // WHICH FILE. The clause this test is named after asks for file, id and field. The id
    // and the field were always there (the validator says `character 'x' has empty spritesheet
    // path`); the FILE could not be, because both seams took an anonymous `&str` and there was
    // nothing in the API to report.
    {
        let source = "assets/data/outlander_catalog.ron";
        let mistyped = CharacterCatalogFragment::from_ron_at(
            source,
            "outlander",
            Some("wandrer_typo"),
            good_ron,
        )
        .expect_err("a default character that is not in the fragment must be refused");
        assert!(
            mistyped.to_string().contains(source),
            "the diagnostic does not name the FILE the author has to open, which \
             is the clause this test exists for: {mistyped}"
        );
    }

    // An empty provider id — the mistake a host makes rather than an author.
    let anonymous = CharacterCatalogFragment::from_ron("  ", None::<String>, good_ron)
        .expect_err("an anonymous fragment must be refused");
    assert!(
        anonymous.to_string().to_lowercase().contains("provider"),
        "the diagnostic does not mention the provider id at all: {anonymous}"
    );
}

/// A third party can author an autonomous creature without the deleted roster.
#[test]
fn a_consumer_authors_an_enemy_through_the_character_definition_seam() {
    use ambition_platformer2d::character::{
        CharacterBrainTemplate, ContactDamage, MoveStyleSpec, PreparedCharacterRegistry,
    };

    let mut app = outlander::build_outlander_app();
    // `build()` does not publish the registry, and this test asserted that it
    // did. `stage_authored_character` only STAGES; the cast is folded and
    // inserted by `CharacterPreparationPlugin`, whose triggers are `App::finish`
    // and a `PreStartup` backstop — both of which a `PlatformerApp` reaches on
    // its first update, not at build time. Reading the resource off the freshly
    // built app panicked with "resource does not exist" for every consumer who
    // tried it, which is exactly the shape a third party hits first.
    app.update();
    let prepared = app.world().resource::<PreparedCharacterRegistry>();
    let sentry = prepared
        .get(outlander::OUTLANDER_SENTRY_CHARACTER_ID)
        .expect("the external provider registered its sentry character");
    let body = sentry
        .body_blueprint()
        .expect("the external sentry is a complete character-first body");

    assert_eq!(body.max_health, 2);
    assert_eq!(body.locomotion.run_speed, 38.0);
    assert_eq!(body.locomotion.move_style, MoveStyleSpec::Walk);
    assert_eq!(
        body.contact_damage,
        Some(ContactDamage {
            strength: 0.5,
            amount: 1,
        })
    );
    let profile = body
        .autonomous_profile
        .expect("the sentry authors its controller policy");
    assert_eq!(profile.template, CharacterBrainTemplate::Wanderer);
    assert_eq!(profile.patrol_effort, 1.0);
    assert_eq!(profile.chase_effort, 1.0);
    assert_eq!(profile.aggro_radius, 0.0);
    assert_eq!(profile.attack_range, 0.0);
}

/// External character art resolves through consumer-authored sheet metadata.
/// The test composes Outlander through the public API and asks the engine's
/// resolver for frame dimensions authored only by the external consumer.
#[test]
fn a_consumer_authors_the_sheet_its_own_character_renders_from() {
    use ambition_platformer2d::character::AuthoredSheetAppExt;
    use ambition_platformer2d::character::AuthoredSheets;
    use bevy::prelude::App;

    let mut app = App::new();
    app.register_character_sheet_ron("outlander", outlander::OUTLANDER_SHEET_RON);

    let authored = app.world().resource::<AuthoredSheets>();
    let record = authored
        .get("outlander")
        .expect("the sheet this crate authored is registered under the target its catalog names");
    assert_eq!(
        (record.frame_width, record.frame_height),
        (32, 48),
        "the registry returned somebody else's sheet"
    );
    assert_eq!(
        record.image, "game://sprites/outlander.png",
        "the record's image path lost its source, which is the OTHER half of \
         owning your art and would put the engine's tree back in charge"
    );

    // And the engine's own resolution path finds it — the assertion that
    // distinguishes "a registry accepted my RON" from "my character resolves".
    let catalog = ambition_platformer2d::character::CharacterCatalog::from_data(
        ambition_platformer2d::character::parse_catalog(outlander::outlander_catalog_ron()),
    );
    let spec = ambition_platformer2d::character::sheet_for_declared_character(
        authored,
        &catalog,
        None,
        outlander::OUTLANDER_CHARACTER_ID,
    )
    .expect(
        "the engine resolved no sheet for a character whose provider authored one — \
         a consumer can address its art and still cannot describe it",
    );
    assert_eq!(
        (spec.frame_width, spec.frame_height),
        (32, 48),
        "the spec came from somewhere other than the authored sheet"
    );
}

/// The art this crate owns is a real image, and the engine reaches it.
///
/// U1 made a consumer able to ADDRESS its art (`game://sprites/outlander.png`
/// survives catalog assembly) and DESCRIBE it (`register_character_sheet_ron`).
/// Neither of those decodes a byte, so "a consumer's character renders from
/// consumer-owned art" stayed a statement about plumbing — and this repo does
/// not commit binary art, so the obvious proof was unavailable.
///
/// `build.rs` generates it instead: eighty lines of `std` writing a genuine PNG
/// into this crate's own asset tree. That closes the loop without committing a
/// byte, and it exercises the same path a third party's real art takes.
///
/// Asserted here: the file exists, it is a PNG whose IHDR says 32×48, and those
/// are the SAME dimensions the sheet RON declares — because two numbers written
/// in two places is how art and metadata drift apart in the first place.
#[test]
fn the_consumers_own_art_is_a_real_png_matching_the_sheet_it_authored() {
    let png = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/sprites/outlander.png");
    let bytes = std::fs::read(&png)
        .unwrap_or_else(|error| panic!("build.rs did not generate {}: {error}", png.display()));

    assert_eq!(
        &bytes[..8],
        &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A],
        "the generated file is not a PNG — a file named .png is not art"
    );
    // IHDR width/height live at bytes 16..24, immediately after the signature,
    // the length field and the chunk type.
    let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
    let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());

    // The sheet this crate authored, parsed rather than repeated: if the RON
    // says 32×48 and the image is 64×64, the character draws a crop of itself
    // and nothing in the engine can tell.
    let sheet = outlander::OUTLANDER_SHEET_RON;
    let declared_w: u32 = sheet
        .split("frame_width:")
        .nth(1)
        .and_then(|rest| rest.split(',').next())
        .and_then(|value| value.trim().parse().ok())
        .expect("the authored sheet declares a frame width");
    let declared_h: u32 = sheet
        .split("frame_height:")
        .nth(1)
        .and_then(|rest| rest.split(',').next())
        .and_then(|value| value.trim().parse().ok())
        .expect("the authored sheet declares a frame height");

    assert_eq!(
        (width, height),
        (declared_w, declared_h),
        "the generated image and the authored sheet disagree about the frame — \
         the character would draw a crop of itself, and no engine check could \
         see it"
    );
}

/// And the engine's own asset source reads it, through the consumer's
/// `game://` scheme rather than a filesystem path this test invented.
///
/// The sibling reader test proves a marker TEXT file resolves. This proves the
/// thing a character actually loads — the image its catalog row names — comes
/// out of the consumer's tree. Together with
/// `a_consumer_authors_the_sheet_its_own_character_renders_from`, the chain is
/// complete: named, described, and present.
#[test]
fn the_engine_reads_the_consumers_generated_art_through_its_own_source() {
    use bevy::prelude::*;

    // Through the game's real composition: the `game://` source is DECLARED on
    // the module and installed by the engine before `AssetPlugin` seals its
    // sources. A test that registered it by hand would be asserting that the
    // test can call `register_asset_source`, not that a consumer's declaration
    // reaches the AssetServer.
    let app = outlander::build_outlander_app();

    // Through the real `AssetServer`, like its sibling test: a hand-built
    // reader would only prove this test can construct one.
    let server = app.world().resource::<AssetServer>();
    let source = server
        .get_source(bevy::asset::io::AssetSourceId::from("game"))
        .expect("the consumer registered a `game://` source");
    let reader = source.reader();
    let read = |path: &str| -> bool {
        bevy::tasks::block_on(async { reader.read(std::path::Path::new(path)).await.is_ok() })
    };

    assert!(
        read("sprites/outlander.png"),
        "the consumer's own generated sprite did not resolve through its own \
         asset source — the catalog row names `game://sprites/outlander.png`, so \
         this is the exact path a character load takes"
    );
}

/// A third party's character can reach for NOTHING, and the engine honours it.
///
/// The character-definition seam — what a character can DO, as opposed to the
/// catalog row saying it exists — had every one of its callers inside the engine
/// workspace until this fixture registered one. That makes it a claim about the
/// repo rather than about an engine, which is the keystone rule this whole
/// fixture exists to enforce.
///
/// Outlander authors an EMPTY `ActionSet`, which is the harder half of the claim.
/// `Some(empty)` means "this character reaches for nothing" and must outrank the
/// catalog exactly as a filled set would — so a resolver that collapsed
/// "authored as empty" into "authored nothing" would fall through to its own row
/// and hand a third party's wanderer the `drifter` kit it declined.
///
/// The fall-through is smaller now and the claim is the same one.
///
/// That is the same distinction Sanic needs in-workspace (his kit is the
/// momentum ride and the ball dash, and giving him a punch would be authoring
/// against the design), proved here by somebody outside it.
#[test]
fn a_consumers_character_that_authors_no_kit_is_not_handed_the_hosts() {
    use ambition_platformer2d::character::ActionSet;

    let mut app = outlander::build_outlander_app();
    outlander::run_outlander_walkthrough(&mut app)
        .unwrap_or_else(|error| panic!("the Outlander walkthrough failed: {error}"));

    let world = app.world_mut();
    let mut bodies =
        world.query::<(&ambition_platformer2d::character::WornCharacter, &ActionSet)>();
    let outlanders: Vec<&ActionSet> = bodies
        .iter(world)
        .filter(|(worn, _)| worn.id() == outlander::OUTLANDER_CHARACTER_ID)
        .map(|(_, set)| set)
        .collect();

    assert!(
        !outlanders.is_empty(),
        "no body is wearing the consumer's character, so this proves nothing \
         about what such a body reaches for"
    );
    for set in outlanders {
        assert!(
            set.melee.is_none() && set.ranged.is_none() && set.special.is_none(),
            "the consumer's wanderer was handed a kit it never authored: {set:?}. \
             Its definition authors an EMPTY action set; falling through to the \
             catalog row hands it the `drifter` set it deliberately declined."
        );
    }
}
