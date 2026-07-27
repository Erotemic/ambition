//! Outlander in a window — the "visibly" half of Phase 6's "runs visibly and
//! headlessly from the same content". The provider, routes, and session
//! lifecycle are byte-for-byte the headless binary's (`compose_outlander_shell`);
//! only the host face differs: `DefaultPlugins` with a window, the engine's
//! generic presentation plugin, and the standard input path.
//!
//! BOTH recorded SDK findings are CLOSED (2026-07-27), and this binary is the
//! caller that proves each one.
//!
//! Leak #3 read: "the AssetServer file root must be pointed at the ENGINE's
//! asset tree via `actors_desktop_asset_root()` — consumer-owned art still has
//! no home, and a consumer that forgets this line gets bare boxes." A consumer
//! now registers its OWN `game://` source layered over the engine's tree
//! (`ambition_asset_manager::consumer_source`), so its art has somewhere to
//! live and anything it did not author still resolves.
//!
//! The SECOND recorded finding is CLOSED (2026-07-27). It read: "the in-repo
//! demo shells each hand-roll a standalone asset-resource install
//! (`SandboxAssetCatalog` + `GameAssets`) that no umbrella helper offers, so
//! this binary ships WITHOUT it and draws the world as colored primitives — a
//! faithful record of what a third party gets today". The helper exists now
//! (`ambition::game_assets::PlatformerAssetsPlugin`) and this binary is its
//! first external caller, which is the point of the fixture: the gap it
//! recorded is the gap it now proves is gone.

fn main() {
    use bevy::prelude::*;

    let mut app = App::new();
    // BEFORE `DefaultPlugins`: Bevy seals its asset sources when `AssetPlugin`
    // builds, so a consumer's own tree has to be registered first.
    outlander::register_outlander_asset_source(&mut app);
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
    // AFTER the content, which registers the catalogs this reads, and BEFORE the
    // presentation, which draws from what it installs.
    app.add_plugins(
        ambition::game_assets::PlatformerAssetsPlugin::for_experience(
            outlander::OUTLANDER_EXPERIENCE,
        )
        .with_room(outlander::outlander_room().metadata),
    );
    app.add_plugins(ambition::presentation::PlatformerPresentationPlugin);
    app.run();
}
