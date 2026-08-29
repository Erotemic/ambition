//! See `main.rs` for the doctrine this file encodes.

use bevy::prelude::*;

/// Assemble the demo: foundation + the engine group + the host group + the Sanic
/// experience under a standalone shell host. Zero engine edits, zero
/// `ambition_app`.
///
/// The shell owns entry: `initial_route = sanic_gameplay` (direct standalone
/// entry) and `home_route = sanic_launcher`, so a `QuitToHome` returns to a
/// Sanic-only launcher and a relaunch rebuilds a fresh, scope-clean session. The
/// SAME [`SanicExperiencePlugin`] powers direct entry and launcher relaunch.
///
/// Headless-foundation here; a windowed shell swaps that one call for
/// `DefaultPlugins` + `ambition_platformer2d::engine::init_engine_states`.
pub fn build_demo_app() -> App {
    build_demo_app_with_home(ambition_demo_sanic::SANIC_LAUNCHER_ROUTE)
}

/// The same standalone Sanic host, but with an explicitly named home route.
///
/// Exposed so a lifecycle test can build a SECOND host from the identical
/// provider and prove that `QuitToHome` resolves relative to whichever home this
/// host declared — the provider never names either launcher.
pub fn build_demo_app_with_home(home_route: &str) -> App {
    let mut app = App::new();
    ambition_platformer2d::engine::add_headless_foundation(&mut app);
    app.add_plugins(ambition_platformer2d::engine::PlatformerEnginePlugins::fixed_tick());
    app.add_plugins(ambition_platformer2d::windowed_host::PlatformerHostPlugins);
    // TODO(sanic-demo-trail-toggle): `PlatformerHostPlugins` currently carries
    // the sandbox's B-key trail debug affordance. Move that behind an explicit
    // host/dev capability later; it is inherited here, not a Sanic ability.
    compose_sanic_shell(&mut app, home_route);
    // Pin the frame dt to the tick dt so one `update()` is exactly one sim tick.
    let timestep = app.world().resource::<Time<Fixed>>().timestep();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(timestep));
    app
}

/// Compose the Sanic experience under a thin standalone host: the session-scope
/// mechanism, the minimal shell, the reusable Sanic provider, and a launcher
/// home. The provider is host-independent — only these host lines (the two
/// routes, and the host spec) are host-specific.
fn compose_sanic_shell(app: &mut App, home_route: &str) {
    use ambition_demo_sanic::{SanicExperiencePlugin, SANIC_GAMEPLAY_ROUTE};

    // The seven standard host steps, plus the one thing this demo actually
    // decides: its launcher speaks, so the frontend context carries the three
    // menu cues instead of the bare default.
    ambition_platformer2d::provider::ShellComposition::new(
        ambition_demo_sanic::SANIC_EXPERIENCE,
        home_route,
        SANIC_GAMEPLAY_ROUTE,
    )
    .with_frontend_audio(
        ambition_platformer2d::audio::selection::FrontendAudioProfile::new(
            ambition_demo_sanic::SANIC_EXPERIENCE,
        )
        .with_sfx([
            ambition_platformer2d::sfx::ids::UI_MENU_MOVE,
            ambition_platformer2d::sfx::ids::UI_MENU_ACCEPT,
            ambition_platformer2d::sfx::ids::UI_MENU_BACK,
        ]),
    )
    .install(app, SanicExperiencePlugin);

    // The shell-gated simulation stays dormant until the provider publishes
    // its exact SessionRoot during activation. No process-resident bootstrap
    // world is installed here; loading and launcher frames have zero world
    // authority by construction.
}

/// The same demo, DRAWN — foundation swapped for `DefaultPlugins`, plus the
/// engine's generic presentation face (oracle-violation OV1).
///
/// The simulation/content composition is identical to [`build_demo_app`]. The
/// visible shell swaps the foundation, adds the generic presentation face, and
/// starts this demo's authored soundtrack when it owns a real window.
///
/// `render` decides whether a rasterizer is created. `RenderMode::Headless` builds
/// the full render graph against no wgpu backend and opens no window — the
/// standard Bevy recipe for exercising presentation in CI. The entities, the
/// camera, and the schedule are identical either way, which is what makes
/// `tests/ov1_draws_the_world.rs` meaningful without a GPU.
#[cfg(feature = "visible")]
pub fn build_windowed_demo_app(render: RenderMode) -> App {
    build_windowed_demo_app_with_home(render, ambition_demo_sanic::SANIC_LAUNCHER_ROUTE)
}

/// The windowed host with an explicitly named home route.
///
/// a capture needs the GAMEPLAY route, not the launcher — booting the
/// default home and counting frames photographs a menu. Mary-O's binary learned
/// this by writing a blank file first.
#[cfg(all(feature = "visible", not(target_arch = "wasm32")))]
pub fn build_windowed_demo_app_with_home(render: RenderMode, home_route: &str) -> App {
    let mut app = App::new();
    // Sanic-specific and it stays: a headless shell RECORDS audio rather than
    // opening an output device. Inserted before the foundation for the same
    // reason it was inserted before `DefaultPlugins` — the audio plugin reads it
    // as it builds.
    if matches!(render, RenderMode::Headless) {
        app.insert_resource(ambition_platformer2d::audio::AudioOutputMode::Recording);
    }
    // ⭐ THE ENGINE'S FACE, NOT A THIRD COPY OF IT — see D183. The asset root,
    // the window/exit/close matrix and the per-mode plugin disables were a
    // duplicate of `install_windowed_foundation`, and so was this crate's own
    // `desktop_asset_root()`: both resolve to
    // `crates/ambition_platformer2d_actor_monolith/assets` by canonicalizing the
    // same directory from a different manifest, with the same `BEVY_ASSET_ROOT`
    // early-out and the same `"assets"` fallback. `init_engine_states` is called
    // by the foundation and is no longer called again below.
    //
    // ⛔ The offscreen arm also installed `ScheduleRunnerPlugin`, because
    // disabling `winit` removes the app RUNNER and `run()` would otherwise do ONE
    // update and return — a capture exiting 0 having drawn nothing. The engine's
    // `Display::Offscreen` is deliberately CALLER-STEPPED, so that runner moved
    // to `bin/capture_sanic.rs`, the consumer that calls `run()`.
    ambition_platformer2d::app::install_windowed_foundation(&mut app, "Sanic", render.into());
    ambition_platformer2d::engine::init_engine_states(&mut app);
    app.add_plugins(ambition_platformer2d::engine::PlatformerEnginePlugins::fixed_tick());
    app.add_plugins(ambition_platformer2d::windowed_host::PlatformerHostPlugins);
    // TODO(sanic-demo-trail-toggle): `PlatformerHostPlugins` currently carries
    // the sandbox's B-key trail debug affordance. Move that behind an explicit
    // host/dev capability later; it is inherited here, not a Sanic ability.

    // Visible and headless hosts share one provider/shell/session lifecycle.
    // The provider installs Sanic's content definitions before the shared asset
    // catalog is assembled below.
    compose_sanic_shell(&mut app, home_route);
    // Sanic owns procedural/self-contained rooms and ships no `.ldtk` file, so
    // no world manifest: a world-less catalog contributes no world rows and
    // every other entry still lands.
    app.add_plugins(
        ambition_platformer2d::game_assets::PlatformerAssetsPlugin::for_experience(
            ambition_demo_sanic::SANIC_EXPERIENCE,
        )
        // Startup binding precedes activation, so the theme (and the skybridge
        // parallax stack) comes from the authored world rather than a session
        // root that does not exist yet.
        .with_room(ambition_demo_sanic::sanic_session_world().metadata.0),
    );

    // OV1, closed: a camera, the room's static visuals, and the sprite/animation
    // chain. No HUD, no menus, no dev stack — those are the GAME's.
    app.insert_resource(ClearColor(Color::srgb(0.025, 0.045, 0.09)));
    app.add_plugins(ambition_platformer2d::presentation::PlatformerPresentationPlugin);
    // The engine's opt-in F1 debug visualizations (collision blocks, surface
    // chains + normals, rebound vectors, read-model body/feature boxes).
    // Shapes only — no dev HUD. Starts OFF; press F1 in-game.
    app.add_plugins(ambition_platformer2d::render::rendering::debug_viz::DebugVizPlugin::default());

    // Both paths therefore exercise the same provider resolver, ownership, bank, and
    // playback-evidence systems.
    install_sanic_audio(&mut app);
    app
}

/// Install the engine-owned audio runtime plus Sanic's resident catalog cache.
/// The same selection, intent, director, SFX, and frontend-reset path is used by
/// the multi-game host; only provider-authored resources differ here.
#[cfg(feature = "visible")]
fn install_sanic_audio(app: &mut App) {
    use bevy::prelude::IntoScheduleConfigs as _;

    // `SfxBankAssetPath` is published by `PlatformerAssetsPlugin` from the same
    // catalog it builds — one resolution, not an app-local repeat that could
    // name a different path.

    // Use the same engine-owned audio runtime as the multi-game host. The
    // standalone app contributes only its provider catalogs and resident asset
    // library; selection, intent priority, playback state, channels, and the
    // director are installed once by `Platformer2dAudioPlugin`.
    app.add_plugins(ambition_platformer2d::actors::audio::Platformer2dAudioPlugin)
        .add_systems(
            Startup,
            setup_sanic_audio_library
                .in_set(ambition_platformer2d::platformer::schedule::PresentationSetupSet),
        );
}

#[cfg(feature = "visible")]
fn setup_sanic_audio_library(
    mut commands: Commands,
    catalogs: Res<ambition_platformer2d::audio::catalog::AudioCatalogRegistry>,
    mut audio_sources: ResMut<Assets<bevy_kira_audio::prelude::AudioSource>>,
) {
    let music = catalogs
        .music_for(ambition_demo_sanic::SANIC_EXPERIENCE)
        .expect("Sanic provider registered its App-local music catalog");
    let sfx = catalogs
        .sfx_for(ambition_demo_sanic::SANIC_EXPERIENCE)
        .expect("Sanic provider registered its App-local SFX catalog");
    let (library, music_state) =
        ambition_platformer2d::audio::library::AudioLibrary::new_with_playback_state(
            &mut audio_sources,
            sfx,
            music,
            None,
            None,
            None,
        );
    commands.insert_resource(library);
    commands.insert_resource(music_state);
}

/// Whether [`build_windowed_demo_app`] opens a window and creates a GPU device.
#[cfg(feature = "visible")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderMode {
    /// A real window and a real backend. What `cargo run --features visible` wants.
    Windowed,
    /// The render graph, no backend, no window. What CI wants.
    Headless,
    /// No window and a REAL backend — the mode that produces pixels without
    /// a display. `Headless` cannot: it sets `backends: None`, so there is no
    /// RenderApp and nothing to read a texture out of. See
    /// `ambition_render::capture` and Mary-O's identical variant.
    OffscreenGpu,
}
// Gated exactly as `RenderMode` is.
#[cfg(feature = "visible")]
impl From<RenderMode> for ambition_platformer2d::app::Display {
    fn from(mode: RenderMode) -> Self {
        match mode {
            RenderMode::Windowed => Self::Window,
            RenderMode::Headless => Self::NoGpu,
            RenderMode::OffscreenGpu => Self::Offscreen,
        }
    }
}

#[cfg(all(test, feature = "visible", not(target_arch = "wasm32")))]
mod tests {
    #[test]
    /// ⭐ ASSERTS THE ROOT THE SHELL ACTUALLY USES. This used to call a
    /// crate-local `desktop_asset_root()` that duplicated the engine's — same
    /// `BEVY_ASSET_ROOT` early-out, same canonicalized target, same fallback.
    /// Once the shell moved to `install_windowed_foundation` that copy was dead,
    /// and a test asserting a function nothing calls proves nothing.
    fn development_asset_root_contains_the_shared_shader_tree() {
        if std::env::var_os("BEVY_ASSET_ROOT").is_some() {
            return;
        }
        let root = std::path::PathBuf::from(
            ambition_platformer2d::asset_manager::actors_desktop_asset_root(),
        );
        assert!(
            root.join("shaders/hit_flash.wgsl").is_file(),
            "Sanic's visible shell must resolve the shared Ambition asset tree; got {}",
            root.display()
        );
    }

    #[test]
    fn headless_demo_uses_the_device_free_recording_audio_backend() {
        let app = super::build_windowed_demo_app(super::RenderMode::Headless);
        let backend = app
            .world()
            .resource::<ambition_platformer2d::audio::AudioBackendState>();
        assert_eq!(
            backend.mode,
            ambition_platformer2d::audio::AudioOutputMode::Recording
        );
        assert!(!backend.device_backend_installed);
    }

    #[test]
    fn published_local_sanic_forms_bind_through_game_assets() {
        let root = std::path::PathBuf::from(
            ambition_platformer2d::asset_manager::actors_desktop_asset_root(),
        );
        let forms = [
            (ambition_demo_sanic::SANIC_CHARACTER_ID, "sanic_spritesheet"),
            (
                ambition_demo_sanic::SUPER_SANIC_CHARACTER_ID,
                "super_sanic_spritesheet",
            ),
        ];
        if !forms.iter().all(|(_, stem)| {
            root.join(format!("sprites/{stem}.png")).is_file()
                && root.join(format!("sprites/{stem}.ron")).is_file()
        }) {
            return;
        }

        let mut app = super::build_windowed_demo_app(super::RenderMode::Headless);
        // This is an asset-publication test, not a shell/load-lifecycle test.
        // Run the real Startup schedule without also advancing Update into
        // provider preparation/activation. Full standalone-host lifecycle
        // coverage lives in the integration tests, while this test stays focused
        // on PNG+RON -> GameAssets binding.
        //
        // The binding this test asserts comes from the PROVIDER's `register_character` (each form
        // publishes its definition and demands its art in one call), which is why the test passes
        // without it.
        app.finish();
        app.cleanup();
        app.world_mut().run_schedule(bevy::app::Startup);
        // Both forms must be DECLARED, then materialize through the ENGINE.
        let declared: Vec<String> = {
            let assets = app
                .world()
                .resource::<ambition_platformer2d::sprite_sheet::game_assets::GameAssets>();
            assets
                .characters
                .declared_character_ids()
                .into_iter()
                .map(str::to_string)
                .collect()
        };
        for (character_id, _) in forms {
            assert!(
                declared.iter().any(|id| id == character_id),
                "published {character_id} must be declared for materialization \
                 (declared: {declared:?})"
            );
        }

        // Demand them, run one Update, and the engine's materializer decodes
        // them. That the PNG+RON are on disk and parse is what makes this pass;
        // a declaration alone would pass with no art published at all.
        {
            let mut demand =
                app.world_mut()
                    .resource_mut::<ambition_platformer2d::actors::character_runtime::CharacterLoadDemand>();
            for (character_id, _) in forms {
                demand.request(character_id);
            }
        }
        app.update();

        let states = app
            .world()
            .resource::<ambition_platformer2d::actors::character_runtime::CharacterLoadStates>(
        );
        for (character_id, sheet_stem) in forms {
            assert_eq!(
                states.outcome(character_id),
                Some(ambition_platformer2d::actors::character_runtime::CharacterLoadOutcome::Ready),
                "published {sheet_stem}.png + .ron must materialize for {character_id}"
            );
        }
        let assets = app
            .world()
            .resource::<ambition_platformer2d::sprite_sheet::game_assets::GameAssets>();
        for (character_id, _) in forms {
            assert!(
                assets.characters.sheet(character_id).is_some(),
                "{character_id} must resolve a decoded sheet after materialization"
            );
        }
    }
}
