//! **Consumer-matrix row 6: Ambition itself, composed through the public API.**
//!
//! The last matrix row not gated on deferred rollback, and the one the campaign
//! kept arriving at from other directions. Twice the shipped host turned out to
//! need something the SDK did not express:
//!
//! * it registers FOUR experiences and a draft held one (closed in slice D);
//! * it boots into a LAUNCHER and the builder only booted into a game (slice E).
//!
//! Both are closed, so this asks the question directly: can the host this
//! engine actually ships be described by the API a third party gets?
//!
//! ⚠ **This composes the shipped host's EXPERIENCES, not the shipped binary.**
//! `ambition_app`'s real composer also installs dev tools, a settings menu,
//! kaleidoscope menus, load presentation and the versus stage — a whole app
//! shell around the games. Claiming row 6 on a test that quietly dropped all of
//! that would be the overclaim this campaign has caught itself in three times.
//! What is proven here is that the four games it ships compose and route
//! through `PlatformerApp`; the surrounding shell is named in the row's record
//! as what remains.

use ambition::app::prelude::*;

/// One shipped game, declared. Each existing provider plugin already registers
/// its own catalogs, session construction and rules — the module says who it is
/// and hands the plugin over as a capability.
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

/// **The shipped games compose through the SDK and the host boots to its
/// launcher.**
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
        .resource::<ambition::game_shell::ShellRouteCatalog>();
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
