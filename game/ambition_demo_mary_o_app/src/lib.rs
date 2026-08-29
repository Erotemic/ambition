//! See `main.rs` for the doctrine this file encodes.

use bevy::prelude::*;

/// Assemble the demo under a standalone shell host: foundation + engine + host +
/// the Mary-O experience. Zero engine edits, zero `ambition_app`.
///
/// The shell owns entry: `initial_route = mary_o_gameplay` (direct standalone
/// entry) and `home_route = mary_o_launcher`, so `QuitToHome` returns to a
/// Mary-O-only launcher and a relaunch rebuilds a fresh, scope-clean session. The
/// SAME [`MaryOExperiencePlugin`](ambition_demo_mary_o::MaryOExperiencePlugin) powers
/// direct entry and launcher relaunch.
pub fn build_demo_app() -> App {
    build_demo_app_with_home(ambition_demo_mary_o::MARY_O_LAUNCHER_ROUTE)
}

/// The same standalone host with an explicitly named home route — exposed so a
/// lifecycle test can build a SECOND host from the identical provider and prove
/// `QuitToHome` resolves relative to whichever home this host declared.
pub fn build_demo_app_with_home(home_route: &str) -> App {
    let mut app = App::new();
    ambition_platformer2d::engine::add_headless_foundation(&mut app);
    app.add_plugins(ambition_platformer2d::engine::PlatformerEnginePlugins::fixed_tick());
    app.add_plugins(ambition_platformer2d::windowed_host::PlatformerHostPlugins);
    compose_mary_o_shell(&mut app, home_route);
    // Pin the frame dt to the tick dt so one `update()` is exactly one sim tick.
    let timestep = app.world().resource::<Time<Fixed>>().timestep();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(timestep));
    app
}

/// Compose the Mary-O experience under a thin standalone host: session-scope +
/// minimal shell + the reusable provider + a launcher home. The provider is
/// host-independent — only these host lines are host-specific.
fn compose_mary_o_shell(app: &mut App, home_route: &str) {
    use ambition_demo_mary_o::{MaryOExperiencePlugin, MARY_O_GAMEPLAY_ROUTE};

    // The shell, the load coordinator, the loading presentation, the frontend
    // audio context and the two route registrations — the seven steps every
    // host of a platformer provider performs in the same order, none of which
    // is a decision this demo makes. Mary-O authors no frontend sound, so the
    // default profile keeps the launcher deliberately silent rather than
    // inheriting another provider's cached audio.
    ambition_platformer2d::provider::ShellComposition::new(
        ambition_demo_mary_o::MARY_O_EXPERIENCE,
        home_route,
        MARY_O_GAMEPLAY_ROUTE,
    )
    .install(app, MaryOExperiencePlugin);

    // The shell-gated simulation stays dormant until the provider publishes
    // its exact SessionRoot during activation. No process-resident bootstrap
    // world is installed here; loading and launcher frames have zero world
    // authority by construction.
}

/// The same demo, DRAWN — foundation swapped for `DefaultPlugins`, plus the
/// engine's generic presentation face (oracle-violation OV1).
///
/// The only difference from [`build_demo_app`] is the first call and one added
/// plugin. That is the claim the demos doctrine makes about a `<name>_app` shell,
/// and it is now true rather than aspirational.
///
/// `render` decides whether a rasterizer is created. `RenderMode::Headless` builds
/// the full render graph against no wgpu backend and opens no window — the
/// standard Bevy recipe for exercising presentation in CI. The entities, the
/// camera, and the schedule are identical either way, which is what makes
/// `tests/ov1_draws_the_world.rs` meaningful without a GPU.
#[cfg(feature = "visible")]
pub fn build_windowed_demo_app(render: RenderMode) -> App {
    build_windowed_demo_app_entering(
        render,
        ambition_demo_mary_o::MARY_O_LAUNCHER_ROUTE,
        ambition_demo_mary_o::LEVEL_1_1_ROOM_ID,
    )
}

/// The windowed host with an explicitly named home route AND entry room — the
/// sibling of [`build_demo_app_with_home`], and the reason it exists is the same.
///
/// There was no way to look at 1-2 at all, which is why three open observations about it could only
/// be argued.
///
/// the entry room is answered ONCE here, and the asset bind room is read off the same
/// session world rather than off `mary_o_session_world()`.
#[cfg(all(feature = "visible", not(target_arch = "wasm32")))]
pub fn build_windowed_demo_app_entering(
    render: RenderMode,
    home_route: &str,
    entry_room: &str,
) -> App {
    let mut app = App::new();
    // ⭐ THE ENGINE'S FACE, NOT A FOURTH COPY OF IT. This block used to hand-roll
    // `DefaultPlugins` — the asset root, the window/exit/close matrix and the
    // per-mode plugin disables — which is exactly what
    // `install_windowed_foundation` already does, and what D183 named as the
    // leak: "a consumer re-deriving the disables". `init_engine_states` is
    // called by the foundation, so it is no longer called again below.
    //
    // ⛔ ONE BEHAVIOURAL DIFFERENCE, AND IT IS THE WHOLE BLAST RADIUS OF THIS
    // MIGRATION: the hand-rolled `OffscreenGpu` arm also installed
    // `ScheduleRunnerPlugin`, because disabling `winit` removes the app RUNNER
    // and `run()` would otherwise perform ONE update and return. The engine's
    // `Display::Offscreen` deliberately does NOT install a runner — an offscreen
    // app is stepped by its CALLER, which is what a capture wants. ⇒ the runner
    // moved to `bin/capture_mary_o.rs`, the only consumer that calls `run()`;
    // the tests here drive `update()` themselves and never needed it.
    ambition_platformer2d::app::install_windowed_foundation(
        &mut app,
        "Super Mary-O — 1-1",
        render.into(),
    );
    ambition_platformer2d::engine::init_engine_states(&mut app);
    // WHICH ROOM, said once. The provider installs its world source as a SYSTEM,
    // so this resource is read on the update that prepares the session and
    // inserting it at build time is early enough — the same seam
    // `tests/course_playthrough.rs` uses to boot the fixture course.
    app.insert_resource(ambition_demo_mary_o::provider::MaryOEntryRoom(
        entry_room.to_string(),
    ));
    app.add_plugins(ambition_platformer2d::engine::PlatformerEnginePlugins::fixed_tick());
    app.add_plugins(ambition_platformer2d::windowed_host::PlatformerHostPlugins);
    // Visible and headless hosts share one provider/shell/session lifecycle.
    // The provider installs Mary-O's content definitions before the shared asset
    // catalog is assembled below.
    compose_mary_o_shell(&mut app, home_route);
    // Level 1-1 is authored in code rather than LDtk, so no world manifest: a
    // world-less catalog contributes no world rows and every other entry lands.
    app.add_plugins(
        ambition_platformer2d::game_assets::PlatformerAssetsPlugin::for_experience(
            ambition_demo_mary_o::MARY_O_EXPERIENCE,
        )
        // Startup asset binding precedes gameplay activation, so the theme comes from the
        // authored room rather than a session root that does not exist yet.
        .with_room(
            ambition_demo_mary_o::provider::mary_o_session_world_entering(entry_room)
                .metadata
                .0,
        ),
    );

    // OV1, closed: a camera, the room's static visuals, and the sprite/animation
    // chain. The minimal launcher/loading presentation is composed by the host.
    // Without the asset resources installed above this plugin has an empty
    // `GameAssets` to draw from and every actor and block renders as a colored
    // rectangle — the exact divergence that made this demo assetless standalone
    // while it rendered fine inside the hosted app.
    app.add_plugins(ambition_platformer2d::presentation::PlatformerPresentationPlugin);
    // The engine's opt-in F1 debug visualizations (collision blocks, surface
    // chains + normals, read-model body/feature boxes). Shapes only — no dev
    // HUD. Starts OFF; press F1 in-game.
    app.add_plugins(ambition_platformer2d::render::rendering::debug_viz::DebugVizPlugin::default());

    // The windowed host uses the physical Kira backend. Mary-O's provider authors
    // a run+jump SFX voice and the "Support Theme" music cue; this wires the same
    // shared audio face the hosted app uses so both are audible standalone.
    install_mary_o_audio(&mut app);
    app
}

/// Install the engine-owned audio runtime plus Mary-O's resident catalog cache.
/// The same selection, intent, director, SFX, and frontend-reset path is used by
/// the multi-game host; only provider-authored resources differ here.
#[cfg(feature = "visible")]
fn install_mary_o_audio(app: &mut App) {
    use bevy::prelude::IntoScheduleConfigs as _;

    // `SfxBankAssetPath` is published by `PlatformerAssetsPlugin`, which
    // resolves it from the same catalog it builds — one resolution rather than
    // an app-local repeat that could name a different path.

    // Use the same engine-owned audio runtime as the multi-game host. The
    // standalone app contributes only its provider catalogs and resident asset
    // library; selection, intent priority, playback state, channels, and the
    // director are installed once by `Platformer2dAudioPlugin`.
    app.add_plugins(ambition_platformer2d::actors::audio::Platformer2dAudioPlugin)
        .add_systems(
            Startup,
            setup_mary_o_audio_library
                .in_set(ambition_platformer2d::platformer::schedule::PresentationSetupSet),
        );
}

#[cfg(feature = "visible")]
fn setup_mary_o_audio_library(
    mut commands: Commands,
    catalogs: Res<ambition_platformer2d::audio::catalog::AudioCatalogRegistry>,
    mut audio_sources: ResMut<Assets<bevy_kira_audio::prelude::AudioSource>>,
) {
    let music = catalogs
        .music_for(ambition_demo_mary_o::MARY_O_EXPERIENCE)
        .expect("Mary-O provider registered its App-local music catalog");
    let sfx = catalogs
        .sfx_for(ambition_demo_mary_o::MARY_O_EXPERIENCE)
        .expect("Mary-O provider registered its App-local SFX catalog");
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
    /// No window and a REAL backend — the mode that can produce pixels
    /// without a display.
    ///
    /// `Headless` cannot be used for this: it sets `backends: None`, so there is no RenderApp
    /// and nothing to read a texture out of.
    ///
    /// Everything a window would have given us stays: `CorePipelinePlugin`, the
    /// gizmo pass, the real wgpu backend. Only `winit` and the window itself go.
    OffscreenGpu,
}
// Gated exactly as `RenderMode` is: the enum lives behind `visible`, so an impl
// mentioning it outside the gate is a compile error in the no-warnings job.
#[cfg(feature = "visible")]
impl From<RenderMode> for ambition_platformer2d::app::Display {
    /// The demo's three modes ARE the engine's three faces; this states the
    /// mapping once instead of letting each arm re-derive it.
    fn from(mode: RenderMode) -> Self {
        match mode {
            RenderMode::Windowed => Self::Window,
            RenderMode::Headless => Self::NoGpu,
            RenderMode::OffscreenGpu => Self::Offscreen,
        }
    }
}
