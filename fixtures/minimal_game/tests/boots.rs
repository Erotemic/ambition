//! **The smallest game stands up — on both faces, from one module.**
//!
//! Consumer-matrix row 2. Outlander already proves external composition, so
//! what these add is the part one consumer cannot: that the API works for a
//! game which asked for almost nothing.
//!
//! Each test below corresponds to a slice-B exit criterion, and none of them
//! asserts "it compiled" — the campaign's rule is that a proof is a test that
//! runs.

use ambition::app::prelude::*;
use minimal_game::MinimalModule;

/// One module, so a passing pair cannot secretly be two different games.
fn the_one_module() -> MinimalModule {
    MinimalModule
}

/// **It boots headless.**
#[test]
fn the_minimal_game_boots_headless() {
    let app = PlatformerApp::headless()
        .mount(the_one_module())
        .try_build()
        .expect("the smallest game composes headless");
    assert!(
        app.get_schedule(ambition::bevy::prelude::FixedUpdate)
            .is_some(),
        "a composed app runs a fixed-step simulation; its absence means no engine \
         was installed at all, which a did-it-panic test cannot distinguish from \
         success"
    );
}

/// **The SAME module reaches the windowed face.**
///
/// This is the slice-B leak, as a test. Before slice B the visible face
/// installed `PlatformerAssetsPlugin`, which panics without a
/// `CharacterCatalog`, and a minimal module had no way to supply one — so a
/// game that booted headless could not boot windowed, while
/// `api-prototype.md` §2b claimed the two faces differed only in policy. The
/// 2026-07-30 blind agent hit exactly this and recorded that the document
/// "actively told me the opposite would be true".
#[test]
fn the_minimal_game_boots_windowed() {
    let app = PlatformerApp::windowed(minimal_game::MINIMAL_WINDOW_TITLE)
        .without_gpu()
        .mount(the_one_module())
        .try_build()
        .expect("the smallest game composes windowed — this is slice B's leak");
    assert!(
        app.get_schedule(ambition::bevy::prelude::FixedUpdate)
            .is_some(),
        "the windowed face runs the same simulation; only the face differs"
    );
}

/// **A composition that prepares art and declares no cast is REFUSED.**
///
/// The counterpart to the test above, and the reason slice B did not simply
/// make the engine invent an empty catalog. `PlatformerAssetsPlugin`'s refusal
/// is deliberate — *"silently substituting an empty catalog is how a game ships
/// with its bosses drawn as the fallback body and nobody notices"* — so the fix
/// had to make the true answer SAYABLE, not make the demand disappear.
///
/// Saying nothing must therefore still fail, and fail where the consumer can
/// read it: a structured `CompositionError` naming both fixes, rather than a
/// panic from inside a plugin three installs later.
#[test]
fn preparing_art_with_no_declared_cast_is_refused_and_names_both_fixes() {
    struct Silent;

    impl GameModule for Silent {
        fn manifest(&self) -> ModuleManifest {
            ModuleManifest::new("silent")
        }

        fn define(&self, module: &mut ModuleDraft) {
            module
                .experience(minimal_game::MINIMAL_EXPERIENCE)
                .launcher_route(minimal_game::MINIMAL_LAUNCHER_ROUTE)
                .gameplay_route(minimal_game::MINIMAL_GAMEPLAY_ROUTE)
                .capability(minimal_game::MinimalExperiencePlugin);
            // and says nothing at all about characters
        }
    }

    let error = PlatformerApp::windowed("silent")
        .without_gpu()
        .mount(Silent)
        .try_build()
        .expect_err("a composition that prepares art with no declared cast must refuse");
    let reported = error.to_string();
    assert!(
        reported.contains("no_characters"),
        "the refusal must name the way to say 'this game has no cast'; got {reported:?}"
    );
    assert!(
        reported.contains("characters("),
        "the refusal must also name the way to declare one; got {reported:?}"
    );
}

/// **A game with genuinely no cast can say so, and it composes.**
///
/// An empty roster is valid — ADR 0032: *"an installed schema with zero
/// authored instances is VALID; a capability installed and unused is
/// ordinary."* What was missing was a word for it.
#[test]
fn a_game_can_declare_that_it_has_no_cast() {
    struct Castless;

    impl GameModule for Castless {
        fn manifest(&self) -> ModuleManifest {
            ModuleManifest::new("castless")
        }

        fn define(&self, module: &mut ModuleDraft) {
            module
                .experience(minimal_game::MINIMAL_EXPERIENCE)
                .launcher_route(minimal_game::MINIMAL_LAUNCHER_ROUTE)
                .gameplay_route(minimal_game::MINIMAL_GAMEPLAY_ROUTE)
                .no_characters()
                .capability(minimal_game::MinimalExperiencePlugin);
        }
    }

    let _ = PlatformerApp::headless()
        .with_game_assets()
        .mount(Castless)
        .try_build()
        .expect("declaring an empty cast is a legitimate thing for a game to do");
}

/// **The game reports that it started — without counting raw Bevy entities.**
///
/// The affordance the 2026-07-30 blind agent went looking for and did not find.
/// It fell back to `app.world().entities().len()`, which is raw Bevy and says
/// nothing about routes; every consumer would have invented that same smoke
/// test, badly. Four of the eight ordering rules `ambition::app` owns fail
/// silently, so an API with no assertion surface makes each of those a debugging
/// session rather than a message.
#[test]
fn the_minimal_game_reports_that_it_started() {
    let mut app = PlatformerApp::headless()
        .mount(the_one_module())
        .build();

    assert_eq!(
        host_status(&app),
        HostStatus::Initializing,
        "before any update the router has not initialized; a read-model that \
         claimed otherwise would be describing something it cannot see yet"
    );

    let mut status = host_status(&app);
    for _ in 0..600 {
        app.update();
        status = host_status(&app);
        if status.is_running() {
            break;
        }
    }

    assert!(
        status.is_running(),
        "the minimal game never reached a running host in 600 ticks; status is \
         {status:?}"
    );
    assert_eq!(
        status.route(),
        Some(minimal_game::MINIMAL_GAMEPLAY_ROUTE),
        "the host activated a route this game did not declare"
    );
}

/// **`is_running` is not satisfied by a route with nothing behind it.**
///
/// The distinction the read-model exists for. "A route is active" and "a
/// session was prepared for it" are different facts, and the gap between them
/// IS the empty host — an earlier draft of Outlander's headless binary "ran"
/// 120 ticks of exactly that. A status type that collapsed them would agree
/// with the bug it is supposed to expose.
#[test]
fn a_route_with_no_prepared_session_does_not_count_as_running() {
    let live = HostStatus::Running {
        route: "r".into(),
        experience: "e".into(),
        prepared: true,
    };
    let hollow = HostStatus::Running {
        route: "r".into(),
        experience: "e".into(),
        prepared: false,
    };
    assert!(live.is_running());
    assert!(
        !hollow.is_running(),
        "a route with no prepared session behind it is the empty host, and \
         `is_running` must not call it started"
    );
    // Both still report the route: a diagnosis needs to know WHICH route is
    // hollow, so the distinction must not cost the caller that information.
    assert_eq!(hollow.route(), Some("r"));
}
