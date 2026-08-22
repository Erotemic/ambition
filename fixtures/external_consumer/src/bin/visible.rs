//! Outlander in a window — the "visibly" half of Phase 6's "runs visibly and
//! headlessly from the same content". The provider, routes, and session
//! lifecycle are byte-for-byte the headless binary's — both mount the same
//! `OutlanderModule`; only the host FACE differs, and that is now one builder
//! call rather than a composition this binary had to spell out.
//!
//! Leak #3 read: "the AssetServer file root must be pointed at the ENGINE's
//! asset tree via `actors_desktop_asset_root()` — consumer-owned art still has
//! no home, and a consumer that forgets this line gets bare boxes." A consumer
//! now registers its OWN `game://` source layered over the engine's tree
//! (`ambition_asset_manager::consumer_source`), so its art has somewhere to
//! live and anything it did not author still resolves.

fn main() {
    // The composition lives in the lib so the headless render test builds the
    // SAME app. A `main` a test cannot call is a composition nothing
    // verifies.
    outlander::build_windowed_app(true).run();
}
