//! Outlander in a window — the "visibly" half of Phase 6's "runs visibly and
//! headlessly from the same content". The provider, routes, and session
//! lifecycle are byte-for-byte the headless binary's (`compose_outlander_shell`);
//! only the host face differs: `DefaultPlugins` with a window, the engine's
//! generic presentation plugin, and the standard input path.
//!
//! Two recorded SDK findings ride along (campaign doc, Phase-6 account):
//! - the AssetServer file root must be pointed at the ENGINE's asset tree via
//!   `actors_desktop_asset_root()` — consumer-owned art still has no home
//!   (leak #3), and a consumer that forgets this line gets bare boxes;
//! - the in-repo demo shells each hand-roll a standalone asset-resource
//!   install (`SandboxAssetCatalog` + `GameAssets`) that no umbrella helper
//!   offers, so this binary ships WITHOUT it and draws the world as colored
//!   primitives — a faithful record of what a third party gets today, not a
//!   bug in this fixture.

fn main() {
    use bevy::prelude::*;

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(bevy::asset::AssetPlugin {
                file_path: ambition::asset_manager::actors_desktop_asset_root(),
                ..Default::default()
            })
            .set(bevy::window::WindowPlugin {
                primary_window: Some(Window {
                    title: "Outlander — external consumer proof".into(),
                    ..Default::default()
                }),
                exit_condition: bevy::window::ExitCondition::OnAllClosed,
                close_when_requested: true,
                ..Default::default()
            }),
    );
    ambition::engine::init_engine_states(&mut app);
    app.add_plugins(ambition::engine::PlatformerEnginePlugins::fixed_tick());
    app.add_plugins(ambition::windowed_host::PlatformerHostPlugins);
    outlander::compose_outlander_shell(&mut app);
    app.add_plugins(ambition::presentation::PlatformerPresentationPlugin);
    app.run();
}
