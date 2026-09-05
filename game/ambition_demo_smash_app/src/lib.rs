//! This crate exists for one reason that the content crate cannot supply: until
//! something runs the stage, every claim about the stocks loop is a unit test.
//! The loop is spend → respawn → eliminate → end, and each step is covered
//! individually; what is NOT covered anywhere is that a fighter knocked off THIS
//! platform, with THIS blast margin, reaches the world's edge at all.
//!
//! That is the gap a demo app closes and a fixture cannot: the numbers stop
//! being arguments and start being a fight.

use bevy::prelude::*;

/// Assemble the demo: foundation + engine group + host group + the smash
/// experience under a standalone shell host. Zero engine edits, zero
/// `ambition_app`.
pub mod stage_diagram;

/// The `smash_tool` subcommands. One module each; see `tools/mod.rs` for why
/// they are modules of the library rather than nine separate binaries.
pub mod tools;

pub fn build_demo_app() -> App {
    let mut app = App::new();
    ambition_platformer2d::engine::add_headless_foundation(&mut app);
    app.add_plugins(ambition_platformer2d::engine::PlatformerEnginePlugins::fixed_tick());
    app.add_plugins(ambition_platformer2d::windowed_host::PlatformerHostPlugins);
    #[cfg(feature = "visible")]
    app.add_plugins(ambition_platformer2d::render::rendering::debug_viz::DebugVizPlugin::default());
    compose_smash_shell(&mut app);
    // This shell was the third and never joined.
    //
    // AFTER `compose_smash_shell`, because the plugin READS the catalogs it
    // registers — the plugin's own doc says so, and it panics naming the
    // composition-order mistake rather than booting art-less.
    //
    // `visible` only, deliberately. `build_demo_app` is also the harness
    // for this crate's regression tests, and they assert on a stepping
    // simulation rather than on pixels; sanic draws the same line by keeping its
    // asset install in `build_windowed_demo_app`.
    #[cfg(feature = "visible")]
    {
        // No world manifest: the stage is authored in Rust, so this demo ships
        // no `.ldtk` and a world-less catalog contributes no world rows while
        // every other entry still lands — the same shape as sanic.
        app.add_plugins(
            ambition_platformer2d::game_assets::PlatformerAssetsPlugin::for_experience(
                ambition_demo_smash::SMASH_EXPERIENCE,
            )
            // Startup binding precedes activation, so the theme comes from the
            // authored stage rather than a session root that does not exist yet.
            .with_room(ambition_demo_smash::smash_stage().metadata.clone()),
        );
        app.add_plugins(ambition_platformer2d::presentation::PlatformerPresentationPlugin);
        select_smash_portal_presentation(&mut app);
    }
    // Pin the frame dt to the tick dt so one `update()` is exactly one sim tick.
    let timestep = app.world().resource::<Time<Fixed>>().timestep();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(timestep));
    app
}

/// The same demo, DRAWN.
///
/// [`build_demo_app`] starts from `add_headless_foundation` — `MinimalPlugins`
/// plus assets, images, transforms and states — so it has no renderer, no
/// window and no `winit`, whatever features are on. Under `visible` it gains
/// presentation SYSTEMS and still has nothing to present to, and `main`'s
/// `app.run()` then spins the schedules against no display. **This demo has
/// therefore never been seen through its own shell**; Smash on a screen has
/// been the main app's versus route, which is a different composition.
///
/// This is that missing half, and it goes through the ENGINE's windowed
/// foundation rather than a fourth hand-rolled `DefaultPlugins`.
/// [`Display::Offscreen`] is what a capture runs on: a real backend with no
/// window, which also means no app runner, so an offscreen caller steps the app
/// itself.
///
/// ✅ mary-o, sanic and twintrack were migrated to this same foundation on
/// 2026-08-29, so there is no longer a hand-rolled `DefaultPlugins` among the
/// demos. Each keeps its own `RenderMode` enum as its public vocabulary and maps
/// it to `Display` in one `From` impl.
///
/// ⛔ THE ONE THING THAT DID NOT TRANSFER, and it is worth knowing before adding
/// a fourth demo: those builders also installed `ScheduleRunnerPlugin` on their
/// offscreen arm. This foundation deliberately does NOT — `Display::Offscreen`
/// is CALLER-STEPPED, which is what a capture wants — so each demo's `capture_*`
/// binary now asks for the runner itself, because it is the only consumer that
/// calls `run()`.
#[cfg(feature = "visible")]
pub fn build_windowed_demo_app(display: ambition_platformer2d::app::Display) -> App {
    let mut app = App::new();
    ambition_platformer2d::app::install_windowed_foundation(
        &mut app,
        "Super Smash Siblings",
        display,
    );
    app.add_plugins(ambition_platformer2d::engine::PlatformerEnginePlugins::fixed_tick());
    app.add_plugins(ambition_platformer2d::windowed_host::PlatformerHostPlugins);
    compose_smash_shell(&mut app);
    // AFTER the shell, because the plugin READS the catalogs it registers —
    // the same order `build_demo_app` states and for the same reason.
    app.add_plugins(
        ambition_platformer2d::game_assets::PlatformerAssetsPlugin::for_experience(
            ambition_demo_smash::SMASH_EXPERIENCE,
        )
        .with_room(ambition_demo_smash::smash_stage().metadata.clone()),
    );
    app.add_plugins(ambition_platformer2d::presentation::PlatformerPresentationPlugin);
    select_smash_portal_presentation(&mut app);
    // One `update()` is exactly one sim tick, as in `build_demo_app`. For a
    // capture this is the difference between "about a second later" and "sixty
    // ticks later".
    let timestep = app
        .world()
        .resource::<bevy::time::Time<bevy::time::Fixed>>()
        .timestep();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(timestep));
    app
}

/// ⭐⭐ A VERSUS MATCH CANNOT USE THE SEAMLESS PORTAL PRESENTATION, AND SAYING SO
/// HERE IS THE POINT — Jon, 2026-09-05: *"because it is not a 1 player game, we
/// can't use the seamless portal presentation. This will be a good check to make
/// sure that it can be disabled on a case by case basis. I think we can use the
/// static small cone presentation though."*
///
/// ⛔ THE REASON IS THE CAMERA, NOT TASTE. `PortalCameraTransitMode::Continuous`
/// maps the viewpoint through an aperture while a CONTROLLED BODY straddles it —
/// a single-subject effect by construction. A smash camera frames the whole
/// cast, so there is no one body whose transit the view should follow, and a
/// continuity roll taken on one fighter's behalf moves the frame the OTHER
/// player is reading. ⇒ `Pop`: the camera behaves exactly as it always does, and
/// a portal becomes something that happens inside the shot rather than to it.
///
/// ⭐ AND `Static` RATHER THAN `Off`, which is the more interesting half. The
/// view window still draws — Alice's up-B opens two apertures and a player has
/// to see where the far one leads — it simply stops being viewer-dependent.
/// `Dynamic` gates the cone on one viewer's line of sight, which asks the same
/// unanswerable question the camera does: WHOSE. The authored cone is identical
/// for both players, and small because a fighter-sized aperture is what she
/// opens.
///
/// ⇒ **This is the case-by-case check Jon asked for, and it is one resource
/// each.** Both are plain overwrites on the host, which `camera_continuity`'s
/// own module doc names as the sanctioned road: *"The resource in this module is
/// the single source of truth for the live mode. Hosts may surface it in a debug
/// menu, but should not mirror the default into a second `DeveloperTools` or
/// settings field."* ⛔ So the smash demo gains no portal-presentation setting of
/// its own; it states its answer to the one that already exists.
///
/// ⚠ CALLED AFTER `PlatformerPresentationPlugin`, deliberately. The plugin
/// installs the engine defaults (`Continuous`, `Dynamic`), and an override
/// placed before it would be silently replaced or not depending on whether the
/// plugin used `init_resource` or `insert_resource` — which is not a detail a
/// host should have to know to be correct.
fn select_smash_portal_presentation(app: &mut App) {
    use ambition_platformer2d::portal_presentation as portal_view;

    app.insert_resource(portal_view::PortalCameraContinuitySelection {
        mode: portal_view::PortalCameraTransitMode::Pop,
    });
    app.insert_resource(portal_view::PortalViewConeConfig {
        mode: portal_view::PortalViewConeMode::Static,
        ..Default::default()
    });
}

fn compose_smash_shell(app: &mut App) {
    // HOME is the select screen, not the stage. A platform fighter that opens on
    // the stage has already decided who you are, and leaving a match should
    // return to the screen that chose it rather than to a launcher listing one
    // experience.
    ambition_platformer2d::provider::ShellComposition::new(
        ambition_demo_smash::SMASH_EXPERIENCE,
        ambition_demo_smash::SMASH_SELECT_ROUTE,
        ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
    )
    // Boot onto SELECT. Neither `PrimaryGameplay` nor `Launcher` says this, which
    // is why `starting_at` exists.
    .starting_at(ambition_demo_smash::SMASH_SELECT_ROUTE)
    // by keying frontend audio on the route instead of the process.
    //
    // What stays here is the DEFAULT for this app's other frontend routes —
    // loading, and anything a future screen adds — which is a claim about this
    // composition and belongs to the composition.
    .with_frontend_audio(
        ambition_platformer2d::audio::selection::FrontendAudioProfile::new(
            ambition_demo_smash::SMASH_EXPERIENCE,
        )
        .with_title_track(ambition_demo_smash::SMASH_SELECT_TRACK)
        .with_sfx([
            ambition_platformer2d::sfx::ids::UI_MENU_MOVE,
            ambition_platformer2d::sfx::ids::UI_MENU_ACCEPT,
            ambition_platformer2d::sfx::ids::UI_MENU_BACK,
        ]),
    )
    .install(app, ambition_demo_smash::SmashExperiencePlugin);
}

#[cfg(test)]
mod portal_presentation_tests {
    use super::*;

    /// ⭐⭐ THE SMASH HOST TURNS THE SEAMLESS PORTAL PRESENTATION OFF, AND THE
    /// ENGINE DEFAULT PROVES IT IS AN OVERRIDE — Jon, 2026-09-05: *"a good check
    /// to make sure that it can be disabled on a case by case basis."*
    ///
    /// ⛔ THE CONTROL IS THE WHOLE TEST. Asserting only that the host holds `Pop`
    /// and `Static` would pass just as well if those were the ENGINE defaults and
    /// this function did nothing — the case-by-case claim would be untested and
    /// the day the default flips, silently false. So both halves are asserted:
    /// the engine ships `Continuous`/`Dynamic`, and the host ships neither.
    ///
    /// ⚠ It also fails LOUDLY if somebody promotes `Pop` to the engine default,
    /// which is correct: at that moment this override stops being a per-case
    /// decision and becomes a restatement, and somebody should have to look.
    #[test]
    fn the_smash_host_overrides_both_portal_presentation_defaults() {
        use ambition_platformer2d::portal_presentation as portal_view;

        let engine_camera = portal_view::PortalCameraContinuitySelection::default().mode;
        let engine_cones = portal_view::PortalViewConeConfig::default().mode;
        assert_eq!(
            engine_camera,
            portal_view::PortalCameraTransitMode::Continuous,
            "the engine no longer defaults to the seamless camera, so this host's \
             `Pop` is a restatement rather than an override and the case-by-case \
             claim needs re-checking"
        );
        assert_eq!(
            engine_cones,
            portal_view::PortalViewConeMode::Dynamic,
            "the engine no longer defaults to viewer-dependent cones, so this \
             host's `Static` proves nothing about disabling it per case"
        );

        let mut app = App::new();
        select_smash_portal_presentation(&mut app);

        assert_eq!(
            app.world()
                .resource::<portal_view::PortalCameraContinuitySelection>()
                .mode,
            portal_view::PortalCameraTransitMode::Pop,
            "the smash host kept the seamless camera — a continuity roll taken on \
             one fighter's behalf moves the frame the other player is reading"
        );
        assert_eq!(
            app.world().resource::<portal_view::PortalViewConeConfig>().mode,
            portal_view::PortalViewConeMode::Static,
            "the smash host kept viewer-dependent cones, which asks the same \
             unanswerable question the camera does: WHOSE line of sight"
        );
    }
}
