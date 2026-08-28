//! Verify that the shipped game experiences compose through the public SDK.
//!
//! The test covers experience registration, launcher startup, and gameplay-route
//! reachability through `PlatformerApp`. App-shell facilities such as developer
//! tools, settings, menus, load presentation, and the versus stage are outside
//! this consumer-matrix row.

use ambition_platformer2d::app::prelude::*;

/// Declare one shipped game's manifest, routes, and provider capability.
macro_rules! shipped_game {
    ($name:ident, $id:expr, $route:expr, $plugin:expr) => {
        struct $name;
        impl GameModule for $name {
            fn manifest(&self) -> ModuleManifest {
                ModuleManifest::new($id)
            }
            fn define(&self, module: &mut ModuleDraft) {
                module
                    .experience($id)
                    .launcher_route(ambition_app::app::shell_host::AMBITION_LAUNCHER_ROUTE)
                    .gameplay_route($route)
                    .capability($plugin);
            }
        }
    };
}

shipped_game!(
    SanicGame,
    ambition_demo_sanic::provider::SANIC_EXPERIENCE,
    ambition_demo_sanic::provider::SANIC_GAMEPLAY_ROUTE,
    ambition_demo_sanic::SanicExperiencePlugin
);
shipped_game!(
    MaryOGame,
    ambition_demo_mary_o::provider::MARY_O_EXPERIENCE,
    ambition_demo_mary_o::provider::MARY_O_GAMEPLAY_ROUTE,
    ambition_demo_mary_o::MaryOExperiencePlugin
);

/// The shipped games compose through the SDK and the host boots to its
/// launcher.
#[test]
fn the_shipped_games_compose_through_the_public_api() {
    let mut app = PlatformerApp::headless()
        .start_at_launcher()
        .mount(SanicGame)
        .mount(MaryOGame)
        .try_build()
        .expect("the games this engine ships must compose through the API it publishes");

    let mut status = host_status(&app);
    for _ in 0..600 {
        app.update();
        status = host_status(&app);
        if status.is_running() || status.is_refused() {
            break;
        }
    }
    assert!(
        !status.is_refused(),
        "the shipped composition was refused: {:?}",
        status.refusal()
    );

    // Both games' routes must be REACHABLE, not merely declared — the point of
    // a launcher host is that every game it ships can be entered.
    let catalog = app
        .world()
        .resource::<ambition_platformer2d::game_shell::ShellRouteCatalog>();
    let routes: Vec<String> = catalog.ids().map(str::to_string).collect();
    for expected in [
        ambition_demo_sanic::provider::SANIC_GAMEPLAY_ROUTE,
        ambition_demo_mary_o::provider::MARY_O_GAMEPLAY_ROUTE,
    ] {
        assert!(
            routes.iter().any(|route| route == expected),
            "route {expected:?} is not reachable in the composed host; got {routes:?}"
        );
    }
}

/// A secondary mounted experience can launch with merged cast/routes. Asset
/// catalog policy is still primary-experience scoped: secondary music can fall
/// back to its direct/conventional path, while catalog prefixing and quality
/// variants remain primary-scoped; the SFX bank is likewise attributed to the
/// primary experience. These limits are asserted below rather than interpreted
/// as absence of secondary audio.
#[test]
fn the_second_mounted_experience_launches_and_its_asset_policy_is_the_primarys() {
    use ambition_platformer2d::game_shell::{ShellCommand, ShellRouteId};
    use ambition_platformer2d::view::{ids, Platformer2dAssetCatalog};

    let mut app = PlatformerApp::headless()
        .with_game_assets()
        .start_at_launcher()
        .mount(SanicGame)
        .mount(MaryOGame)
        .try_build()
        .expect("two games must compose");

    for _ in 0..600 {
        app.update();
        if host_status(&app).is_running() || host_status(&app).is_refused() {
            break;
        }
    }
    assert!(
        !host_status(&app).is_refused(),
        "the two-game host was refused: {:?}",
        host_status(&app).refusal()
    );

    // ── Launch the SECOND one ──
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(
            ambition_demo_mary_o::provider::MARY_O_GAMEPLAY_ROUTE,
        )));
    let mut status = host_status(&app);
    for _ in 0..900 {
        app.update();
        status = host_status(&app);
        if matches!(&status, HostStatus::Running { experience, prepared: true, .. }
            if experience == ambition_demo_mary_o::provider::MARY_O_EXPERIENCE)
        {
            break;
        }
        if status.is_refused() {
            break;
        }
    }
    let HostStatus::Running {
        route,
        experience,
        prepared,
    } = &status
    else {
        panic!("the second mounted experience never activated: {status:?}");
    };
    assert_eq!(
        (route.as_str(), experience.as_str(), *prepared),
        (
            ambition_demo_mary_o::provider::MARY_O_GAMEPLAY_ROUTE,
            ambition_demo_mary_o::provider::MARY_O_EXPERIENCE,
            true
        ),
        "the second experience's ENTRY ROOM did not come up under its own \
         route and id"
    );

    // BOTH GAMES' CASTS ARE PUBLISHED.
    //
    // asserted for BOTH games on purpose.
    {
        use ambition_platformer2d::characters::prepared::PreparedCharacterRegistry;
        let registry = app
            .world()
            .get_resource::<PreparedCharacterRegistry>()
            .expect("a composed host publishes a prepared cast");
        assert!(
            registry
                .get(ambition_demo_mary_o::provider::MARY_O_CHARACTER_ID)
                .is_some(),
            "the SECOND mounted game's cast never published: {} character(s) \
             registered",
            registry.len()
        );
        assert!(
            registry.get("sanic").is_some(),
            "the FIRST mounted game's cast was lost when the second published: \
             {} character(s) registered",
            registry.len()
        );
    }

    let catalog = app.world().resource::<Platformer2dAssetCatalog>();

    // ── One character visual, and therefore one logical asset path ──
    let sheet = catalog.path_for(&ids::character_sprite(
        ambition_demo_mary_o::provider::MARY_O_CHARACTER_ID,
    ));
    assert!(
        sheet.is_some(),
        "the secondary experience's own character has no sprite path in the \
         asset catalog the primary's policy built — its cast would draw as the \
         fallback body"
    );

    // ── One audio mapping: the KNOWN limit, pinned ──
    //
    // asserted as a limit rather than left unmeasured. A secondary
    // experience's music is declared (its audio fragment registers a real
    // `MusicRegistry`) and has no path in the asset catalog, because the
    // catalog folds the PRIMARY's registry only.
    let registry = app
        .world()
        .resource::<ambition_platformer2d::audio::catalog::AudioCatalogRegistry>();
    let declared = registry
        .music_for(ambition_demo_mary_o::provider::MARY_O_EXPERIENCE)
        .expect("Mary-O's provider registers a music registry")
        .default_track
        .clone();
    assert!(
        !declared.is_empty(),
        "non-vacuity: the secondary experience declares a default track"
    );
    let catalog = app.world().resource::<Platformer2dAssetCatalog>();
    assert!(
        catalog.path_for(&ids::music_track(&declared)).is_none(),
        "the secondary experience's music track `{declared}` now HAS an asset \
         path, so the per-experience asset policy has been widened — good, but \
         `PlatformerApp`'s stated limit is stale and this test is the record of \
         it"
    );
}

/// WHAT CAST DOES A TWO-DEMO HOST ACTUALLY PUBLISH? — the measurement ledger row named,
/// taken out of the finished world instead of inferred from a silent log.
///
///  so this asks the registry directly.
#[test]
fn a_two_demo_host_publishes_exactly_the_cast_its_demos_register() {
    use ambition_platformer2d::characters::prepared::PreparedCharacterRegistry;

    let mut app = PlatformerApp::headless()
        .with_game_assets()
        .start_at_launcher()
        .mount(SanicGame)
        .mount(MaryOGame)
        .try_build()
        .expect("two games must compose");
    for _ in 0..600 {
        app.update();
        if host_status(&app).is_running() || host_status(&app).is_refused() {
            break;
        }
    }

    let ids: Vec<String> = app
        .world()
        .get_resource::<PreparedCharacterRegistry>()
        .map(|registry| registry.ids().map(str::to_string).collect())
        .unwrap_or_default();
    assert!(
        !ids.is_empty(),
        "this host publishes NO prepared cast at all, so every character-named \
         placement in every room it loads falls back to a generic — which is \
         ledger D75's original finding, live again"
    );
    for expected in ["mary_o", "sanic"] {
        assert!(
            ids.iter().any(|id| id == expected),
            "a host that mounted the {expected} demo did not publish its \
             protagonist: {ids:?}"
        );
    }
    // THE OTHER HALF, and it is the ruling: this host never mounted Ambition,
    // so Ambition's cast is correctly ABSENT. Without this the test would pass on
    // a host that published everything in the workspace, which would make
    // "exactly the cast its demos register" a sentence about nothing.
    assert!(
        !ids.iter().any(|id| id == "player_robot_v3"),
        "a two-demo host published Ambition's protagonist, so registration is \
         not scoped to what a composition MOUNTS: {ids:?}"
    );
}
