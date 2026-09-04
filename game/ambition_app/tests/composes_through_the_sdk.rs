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

/// ⭐⭐ THE FIRST MINIMUM-HOST PROBE, and it answers with evidence a question
/// the composability doctrine could otherwise only argue about: can a consumer
/// OMIT a named optional capability and still have an engine that runs?
///
/// `docs/planning/engine/decomposition.md` names "a platformer without
/// cutscenes" as one of the target compositions. Bevy's `PluginGroup` already
/// supplies the mechanism — `.disable::<P>()` — so the interesting part was
/// never whether a consumer CAN omit a plugin; it is whether the rest of the
/// engine still forms a coherent system when they do.
///
/// ⚠ WHAT THIS DOES AND DOES NOT PROVE. It proves the app builds its schedules
/// and steps eight frames with `CutsceneSchedulePlugin` absent — no panic from a
/// missing resource, no system parameter that fails validation. It does NOT
/// prove that a cutscene-free composition is USEFUL, that content which triggers
/// a cutscene degrades gracefully, or that any other capability can be omitted.
/// Each of those is its own probe.
///
/// ⇒ Kept deliberately small, because the value is the SHAPE: this is what a
/// capability-level minimum-host test looks like, and the doctrine names four
/// more worth writing (a minimal foundation, combat added alone, encounters
/// without boss encounters, simulation without the renderer).
#[test]
fn a_host_that_omits_cutscenes_still_builds_and_steps() {
    use bevy::app::PluginGroup;

    // ⚠ THE CONTROL ARM IS NOT OPTIONAL HERE, and it earned its place twice: the
    // first two runs of this probe failed in the CONTROL, for reasons that had
    // nothing to do with cutscenes. A probe that cannot tell "the capability was
    // needed" from "my host was wrong" measures the second and reports the
    // first.
    let host = |disable_cutscenes: bool| {
        let mut app = bevy::prelude::App::new();
        // ⭐ THE MINIMUM HOST IS ALREADY NAMED, and my first two attempts at this
        // probe reinvented it badly. `add_headless_foundation` is the engine's
        // own declared prerequisite set — MinimalPlugins, asset, image,
        // transform, states, and `init_engine_states` — and hand-rolling a
        // subset of it panicked in `bevy_asset`, then in
        // `finalize_unpresented_room_transition_failure_system` for want of
        // `NextState<GameMode>`. ⇒ The engine's prerequisites are DECLARED; they
        // just are not what `MinimalPlugins` gives you.
        ambition_platformer2d::engine::add_headless_foundation(&mut app);
        let group = ambition_platformer2d::engine::PlatformerEnginePlugins::fixed_tick().build();
        if disable_cutscenes {
            app.add_plugins(
                group.disable::<ambition_platformer2d::actors::cutscene::CutsceneSchedulePlugin>(),
            );
        } else {
            app.add_plugins(group);
        }
        for _ in 0..8 {
            app.update();
        }
    };

    host(false);
    host(true);
}

/// The same probe for two more of the doctrine's named target compositions.
///
/// ⚠ ONE HELPER, because the interesting part is never the plugin — it is that
/// the CONTROL arm and the disabled arm are built the same way. Each call
/// installs the engine group whole, steps it, then installs it minus one plugin
/// and steps that.
fn the_engine_steps_with_and_without<P: bevy::app::Plugin>() {
    use bevy::app::PluginGroup;
    for disable in [false, true] {
        let mut app = bevy::prelude::App::new();
        ambition_platformer2d::engine::add_headless_foundation(&mut app);
        let group = ambition_platformer2d::engine::PlatformerEnginePlugins::fixed_tick().build();
        if disable {
            app.add_plugins(group.disable::<P>());
        } else {
            app.add_plugins(group);
        }
        for _ in 0..8 {
            app.update();
        }
    }
}

// ⚠ "A PLATFORMER WITHOUT DIALOGUE" IS NAMED BY THE DOCTRINE AND IS NOT PROBED
// HERE, deliberately. `DialogSimStatePlugin` is installed by the engine group
// unconditionally, but the facade re-exports `ambition_dialog` only behind
// `#[cfg(feature = "ambition_dialog")]`, which this test target does not enable
// — and `ambition_app` does not depend on that crate directly. So the type
// cannot be NAMED here to disable it.
//
// ⛔ A `#[cfg]`-guarded test would have compiled to nothing and reported
// success, which is the trap this repository already records twice
// (`docs/recipes/checks-that-did-not-run.md`). ⇒ The absence is the note.
// Probing this composition needs the probe to live where the plugin is
// nameable, not a feature flag added to make it nameable here.

/// "A platformer without portals" — likewise.
#[cfg(feature = "portal")]
#[test]
fn a_host_that_omits_portals_still_builds_and_steps() {
    the_engine_steps_with_and_without::<ambition_platformer2d::runtime::PortalSchedulePlugin>();
}

/// ⛔⛔ AND ONE NAMED COMPOSITION IS NOT EXPRESSIBLE AT ALL, which is worth a
/// test's worth of prose even though there is no test.
///
/// "Generic encounters without boss encounters" is one of the doctrine's target
/// compositions. It cannot be written as a `.disable::<P>()` because
/// **`ambition_boss_encounter` contains no `impl Plugin`** — boss encounters
/// ride the actor monolith's schedules rather than installing themselves. So the
/// question "can a consumer omit boss encounters" has no seam to ask it through,
/// which is a stronger statement than a failing probe would have made.
///
/// ⇒ Recorded here rather than in a `#[ignore]`d test, because an ignored test
/// implies a mechanism that is merely switched off.
#[allow(dead_code)]
const BOSS_ENCOUNTERS_HAVE_NO_INSTALLATION_SEAM: () = ();
