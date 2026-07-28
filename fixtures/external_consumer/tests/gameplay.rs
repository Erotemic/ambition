//! The external consumer's own acceptance gate (Phase-6 / GPT 5.6 review:
//! "the fixture should contain integration tests rather than relying only on
//! binaries that print success"). Run from the engine repo with
//! `cargo test --manifest-path fixtures/external_consumer/Cargo.toml` — the
//! independent workspace resolves its own dependency graph, so this is
//! exactly the build a third-party consumer gets.

/// Boot → activate → verify population → charge the beacon → walk the ridge
/// gate. One test, the whole authored surface: the room (construction), the
/// character (catalog), the sentry (roster + stager, lowered as a construction
/// plan row), the consumer's own authoritative component (§authority), and the
/// transition (`transit_body`) — all through the public `ambition` umbrella with
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

/// **Task 1's exit criterion, answered from outside the engine.**
///
/// *"A feature-owned authoritative component and system are mechanically
/// accounted, run under the simulation gate, and survive real
/// rewind/resimulation without edits to a giant runtime list."*
///
/// Every word of that is checked here, and it has to be here: the engine's own
/// registrations are crate-private conveniences away from being unusable by
/// anyone else, and a test living inside the workspace cannot tell the
/// difference. `BeaconCharge` is declared in this crate, encoded by this crate,
/// registered by this crate through `ambition::runtime::rollback`, and named in
/// no engine file.
///
/// The rewind is REAL, not simulated: a GGRS sync-test session resimulates every
/// frame from a restored snapshot and compares checksums, so a component that
/// failed to round-trip — or an encoder that dropped `ticks` while keeping
/// `seconds` — panics inside the engine before this test's own assertions run.
/// What the assertions add is the part a checksum cannot see: that the state was
/// non-trivial, and that it landed on the same value the fixed-tick host reached.
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

/// **A third party gets REAL ART, not coloured rectangles.** (Phase 6, the
/// visible-shell half)
///
/// The visible binary's own comment used to record the gap: the in-repo demo
/// shells each hand-rolled a ~90-line asset-resource install that no umbrella
/// helper offered, so a consumer following the demos doctrine drew the world as
/// primitives. A stranger cloning this engine and running their game saw
/// untextured boxes — the most visible way "an engine another game can be built
/// on" can fail, and it failed for want of a function.
///
/// `ambition::game_assets::PlatformerAssetsPlugin` is that function, and this is
/// the test that it works from OUTSIDE the workspace: the fixture resolves its
/// own dependency graph, so this is the build a third party gets.
///
/// Asserts the two resources the generic presentation actually reads. A
/// compile-only proof would say nothing — the failure being closed here is
/// precisely "it composes and draws nothing".
#[test]
fn the_umbrella_asset_install_gives_an_external_consumer_real_sprites() {
    use bevy::prelude::*;

    let mut app = App::new();
    // The visible binary's composition minus the window: the asset plugin
    // pointed at the ENGINE's tree (recorded leak #3 — a consumer that forgets
    // this line gets bare boxes), states, then the engine group.
    app.add_plugins(bevy::MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.add_plugins(bevy::asset::AssetPlugin {
        file_path: ambition::asset_manager::actors_desktop_asset_root(),
        ..Default::default()
    });
    app.init_asset::<Image>();
    app.init_asset::<TextureAtlasLayout>();
    ambition::engine::init_engine_states(&mut app);
    app.add_plugins(ambition::engine::PlatformerEnginePlugins::fixed_tick());
    outlander::compose_outlander_shell(&mut app);
    app.add_plugins(
        ambition::game_assets::PlatformerAssetsPlugin::for_experience(
            outlander::OUTLANDER_EXPERIENCE,
        )
        .with_room(outlander::outlander_room().metadata),
    );
    app.update();

    let catalog = app
        .world()
        .get_resource::<ambition::asset_manager::sandbox_assets::SandboxAssetCatalog>()
        .expect(
            "the plugin did not install a SandboxAssetCatalog, so every asset path \
             policy the presentation reads is missing",
        );
    assert!(
        catalog
            .path_for(&ambition::asset_manager::sandbox_assets::ids::sfx_bank())
            .is_some(),
        "the installed catalog resolves no paths at all, so it was built from \
         nothing — the composition-order failure this plugin is supposed to make \
         loud"
    );

    let assets = app
        .world()
        .get_resource::<ambition::sprite_sheet::game_assets::GameAssets>()
        .expect("the plugin did not install GameAssets");
    assert!(
        !assets.entities.is_empty(),
        "GameAssets carries no entity sprites, so the world still draws as \
         coloured primitives — which is the exact state this plugin was written \
         to end, and a green compile would not have noticed"
    );
}

/// **A consumer's OWN art has a home.** (Phase 6, recorded SDK leak #3)
///
/// The visible binary's second recorded finding read: "the AssetServer file root
/// must be pointed at the ENGINE's asset tree … consumer-owned art still has no
/// home, and a consumer that forgets this line gets bare boxes." So a third
/// party could load the engine's sprites or nothing; there was nowhere for its
/// own to live, because the two-root reader that lets Ambition's content crate
/// own a world tree lived inside `ambition_app`'s CLI module.
///
/// It is `ambition_asset_manager::consumer_source` now. This asserts BOTH
/// directions, because either alone is a different bug: a consumer file must win
/// over the engine tree, and a file the consumer never authored must still
/// resolve out of the engine's.
#[test]
fn a_consumer_owns_its_own_asset_tree_and_still_sees_the_engines() {
    use bevy::prelude::*;

    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    outlander::register_outlander_asset_source(&mut app);
    app.add_plugins(bevy::asset::AssetPlugin {
        file_path: ambition::asset_manager::actors_desktop_asset_root(),
        ..Default::default()
    });

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

/// **A third party who authors bad content is TOLD WHAT IS WRONG.** (Phase 6,
/// Milestone E's remaining clause)
///
/// The campaign doc lists this one as open: "deliberate authoring failures
/// produce actionable diagnostics (the `from_ron` seams reject malformed
/// content; a systematic error-quality pass remains open)". Rejecting is the
/// easy half. The half that decides whether somebody can build a game on this
/// engine is whether the rejection says which FILE, which ID, and which FIELD —
/// from outside the workspace, where the reader cannot go and read the parser.
///
/// Every case below asserts on the message a consumer would actually see. The
/// requirement is deliberately concrete: the message must name the thing the
/// author has to go and change. "invalid catalog" is a rejection; "fragment
/// 'outlander' names missing default character 'typo_id'" is a diagnostic.
#[test]
fn authoring_mistakes_name_the_thing_the_author_must_fix() {
    use ambition::characters::actor::character_catalog::CharacterCatalogFragment;

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
    let missing_default = CharacterCatalogFragment::from_ron(
        "outlander",
        Some("wandrer_typo"),
        good_ron,
    )
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

    // Syntactically broken RON. The author needs to know it was THEIR fragment
    // and that the failure was a parse, not a validation rule they can argue
    // with.
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

    // The ROSTER seam, the fixture's other public authoring surface. A roster is
    // a second file naming ids the catalog owns, which is precisely where a
    // rename goes wrong.
    {
        use ambition::actors::features::CharacterRosterFragment;
        let broken = CharacterRosterFragment::from_ron(
            "outlander",
            None::<String>,
            "( roster: { \"missing\": ",
        )
        .expect_err("truncated roster RON must be refused");
        let message = broken.to_string();
        assert!(
            message.contains("outlander"),
            "the roster diagnostic does not name the provider: {message}"
        );
    }

    // **WHICH FILE.** The clause this test is named after asks for file, id and
    // field. The id and the field were always there (the validator says
    // `character 'x' has empty spritesheet path`); the FILE could not be, because
    // both seams took an anonymous `&str` and there was nothing in the API to
    // report (GPT 5.6, 2026-07-28). `from_ron_at` is where an author says where
    // its text came from, and the diagnostic repeats it back.
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

        use ambition::actors::features::CharacterRosterFragment as Roster;
        let broken = Roster::from_ron_at(source, "outlander", None::<String>, "( roster: {")
            .expect_err("truncated roster RON must be refused");
        assert!(
            broken.to_string().contains(source),
            "the roster diagnostic does not name the file either: {broken}"
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

/// **A third party can say what its own character LOOKS LIKE.** (queue U1)
///
/// Owning the art was two gaps, not one. The first was addressing — a catalog
/// path was reduced to a basename and rebuilt under the engine's sprite folder,
/// so `game://sprites/outlander.png` became `sprites/game://sprites/outlander.png`.
/// The second is this one, and it was the larger: sheet METADATA — frame size,
/// rows, where the body sits — was read only from a table baked at build time
/// from `crates/ambition_actors/assets/sprites`. `manifest_target()` does not
/// return a path; it strips `_spritesheet.ron` and returns a NAME to look up in
/// that table. So a consumer could ship any art it liked and its character still
/// resolved no spec and drew the placeholder rectangle.
///
/// This asserts the seam from where it matters: an App composed exactly as
/// `compose_outlander_shell` composes it, asking the ENGINE's own resolution
/// function for a character the ENGINE has never heard of, and getting back the
/// frame size this crate authored.
#[test]
fn a_consumer_authors_the_sheet_its_own_character_renders_from() {
    use ambition::sprite_sheet::character::sheets::AuthoredSheets;
    use ambition::sprite_sheet::AuthoredSheetAppExt;
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
    let catalog = ambition::characters::actor::character_catalog::CharacterCatalog::from_data(
        ambition::characters::actor::character_catalog::parse_catalog(
            outlander::outlander_catalog_ron(),
        ),
    );
    let spec = ambition::actors::character_sprites::sheet_for_declared_character(
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

/// **The art this crate owns is a real image, and the engine reaches it.**
/// (queue T2, the half U1 left open)
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
    let bytes = std::fs::read(&png).unwrap_or_else(|error| {
        panic!("build.rs did not generate {}: {error}", png.display())
    });

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

/// **And the engine's own asset source reads it**, through the consumer's
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

    let mut app = App::new();
    app.add_plugins(bevy::MinimalPlugins);
    outlander::register_outlander_asset_source(&mut app);
    app.add_plugins(bevy::asset::AssetPlugin {
        file_path: ambition::asset_manager::actors_desktop_asset_root(),
        ..Default::default()
    });

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

/// **A third party's character can reach for NOTHING, and the engine honours it.**
///
/// The character-definition seam — what a character can DO, as opposed to the
/// catalog row saying it exists — had every one of its callers inside the engine
/// workspace until this fixture registered one. That makes it a claim about the
/// repo rather than about an engine, which is the keystone rule this whole
/// fixture exists to enforce.
///
/// Outlander authors an EMPTY `ActionSet`, which is the harder half of the claim.
/// `Some(empty)` means "this character reaches for nothing" and must outrank the
/// catalog exactly as a filled set would. Its own row declares
/// `playable_kit: HostCode`, which rebuilds the HOST PROTAGONIST'S kit from the
/// body's abilities — so a resolver that collapsed "authored as empty" into
/// "authored nothing" would fall through to that row and hand a third party's
/// wanderer Ambition's sword and bolt.
///
/// That is the same distinction Sanic needs in-workspace (his kit is the
/// momentum ride and the ball dash, and giving him a punch would be authoring
/// against the design), proved here by somebody outside it.
#[test]
fn a_consumers_character_that_authors_no_kit_is_not_handed_the_hosts() {
    use ambition::characters::brain::ActionSet;

    let mut app = outlander::build_outlander_app();
    outlander::run_outlander_walkthrough(&mut app)
        .unwrap_or_else(|error| panic!("the Outlander walkthrough failed: {error}"));

    let world = app.world_mut();
    let mut bodies = world.query::<(&ambition::characters::actor::WornCharacter, &ActionSet)>();
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
             catalog row's `playable_kit: HostCode` rebuilds the host \
             protagonist's own melee and bolt onto somebody else's character."
        );
    }
}
