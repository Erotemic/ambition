//! Outlander in a window — the "visibly" half of Phase 6's "runs visibly and
//! headlessly from the same content". The provider, routes, and session
//! lifecycle are byte-for-byte the headless binary's — both mount the same
//! `OutlanderModule`; only the host FACE differs, and that is now one builder
//! call rather than a composition this binary had to spell out.
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
//! (`Platformer2dAssetCatalog` + `GameAssets`) that no umbrella helper offers, so
//! this binary ships WITHOUT it and draws the world as colored primitives — a
//! faithful record of what a third party gets today". The helper exists now
//! (`ambition_platformer2d::game_assets::PlatformerAssetsPlugin`) and this binary is its
//! first external caller, which is the point of the fixture: the gap it
//! recorded is the gap it now proves is gone.

fn main() {
    // The composition lives in the lib so the headless render test builds the
    // SAME app (queue T2). A `main` a test cannot call is a composition nothing
    // verifies.
    outlander::build_windowed_app(true).run();
}
