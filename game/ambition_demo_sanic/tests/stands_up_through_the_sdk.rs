//! **Consumer-matrix row 4, standalone half: a module that PREDATES the SDK
//! stands up through it.**
//!
//! Every consumer the campaign has proven so far — Outlander and the minimal
//! game — was written or rewritten against `ambition::app`. Sanic was not. It
//! is an in-tree demo with presentation profiles, a declared HUD, music,
//! procedural SFX and packed SFX, written before `PlatformerApp` existed, and
//! it is the closest thing available to an independent consumer.
//!
//! That distinction is the point. §4 authorises a decomposition on a SENTINEL
//! CONSUMER's capability footprint, and a footprint measured only against games
//! their own API author designed answers a much weaker question.
//!
//! ⚠ Sanic mounts as a `capability`, not through `playable()`. `playable` does
//! not carry presentation profiles or a HUD declaration, and inventing those
//! parameters to make this test prettier would be designing an API from one
//! caller. The capability slot is the supported escape hatch for exactly this,
//! and using it here is evidence about where `playable` stops rather than a
//! workaround.

use ambition::app::prelude::*;
use ambition_demo_sanic::provider::{
    SanicExperiencePlugin, SANIC_EXPERIENCE, SANIC_GAMEPLAY_ROUTE, SANIC_LAUNCHER_ROUTE,
};

/// Sanic, declared. Everything the engine needs; nothing about how it is
/// assembled.
struct SanicModule;

impl GameModule for SanicModule {
    fn manifest(&self) -> ModuleManifest {
        ModuleManifest::new(SANIC_EXPERIENCE)
    }

    fn define(&self, module: &mut ModuleDraft) {
        module
            .experience(SANIC_EXPERIENCE)
            .launcher_route(SANIC_LAUNCHER_ROUTE)
            .gameplay_route(SANIC_GAMEPLAY_ROUTE)
            .capability(SanicExperiencePlugin);
    }
}

/// **It boots, and it REACHES A RUNNING HOST.**
///
/// Not "it composes". Slice B learned that lesson expensively: its first boot
/// tests asserted `try_build` succeeded and never ran a tick, and the host they
/// blessed had never started.
#[test]
fn sanic_stands_up_standalone_through_the_public_api() {
    let mut app = PlatformerApp::headless()
        .mount(SanicModule)
        .try_build()
        .expect("an in-tree module composes through the public API");

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
        "the host refused Sanic: {:?}",
        status.refusal()
    );
    assert!(
        status.is_running(),
        "Sanic never reached a running host; status is {status:?}"
    );
    assert_eq!(
        status.route(),
        Some(SANIC_GAMEPLAY_ROUTE),
        "the host activated a route Sanic did not declare"
    );
}
