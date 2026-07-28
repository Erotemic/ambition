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
    // The composition lives in the lib so the headless render test builds the
    // SAME app (queue T2). A `main` a test cannot call is a composition nothing
    // verifies.
    outlander::build_windowed_app(outlander::RenderMode::Windowed).run();
}
