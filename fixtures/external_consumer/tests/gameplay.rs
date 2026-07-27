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
