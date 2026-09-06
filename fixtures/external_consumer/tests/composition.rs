//! The other test files in this fixture assert what Outlander DOES — it
//! activates, it walks, its gate transits a body, its character is drawn. This
//! one asserts only that the composition RESOLVES, on each host face, from one
//! mounted module. That is a different claim and it belongs in its own file:
//! `gameplay.rs` going green tells you the headless face composes and says
//! nothing whatsoever about the windowed one, which is exactly the asymmetry
//! that let three hand-ordered builders drift apart before A4 deleted them.
//!
//! Not "both faces work" — both faces worked before, via separate builders that
//! each restated the engine's ordering rules and disagreed about four of them.
//! The claim is that ONE mounted [`OutlanderModule`] reaches both faces, so the
//! only difference between a windowed Outlander and a headless one is the
//! builder call that selects the face. A test that constructed a different
//! module per face would pass while proving nothing.
//!
//! # Scope discipline
//!
//! This file is inside the `include_tests` scope of BOTH slice-A contracts
//! (`outlander-names-only-the-public-sdk` and
//! `outlander-does-not-hand-order-its-own-composition`) — the fixture's tests
//! ARE the consumer. So it names `ambition_platformer2d::app` and nothing else, and it must
//! not reach for a plugin group to check its own work. Everything asserted below
//! is observable through the public surface or through the `App` itself.

use ambition_platformer2d::app::prelude::*;
use outlander::OutlanderModule;

/// Both faces come from ONE module, so the test cannot accidentally prove that
/// two different games compose.
fn the_one_module() -> OutlanderModule {
    OutlanderModule
}

/// Headless composes, and one update is one sim tick.
///
/// `try_build` rather than `build`: a panic tells a reader that something failed,
/// while a [`CompositionError`] tells them WHICH rule — and A3's whole subject is
/// whether the builder can state its own failures.
#[test]
fn platformer_app_composes_the_fixture_headless() {
    let app = PlatformerApp::headless()
        .mount(the_one_module())
        .try_build()
        .expect("the headless face composes the fixture's one module");
    // A composed app is not an empty one.
    assert!(
        app.get_schedule(bevy::prelude::FixedUpdate).is_some(),
        "the headless face pins the sim to a fixed step (rule 8), so FixedUpdate \
         must exist in the composed app"
    );
}

/// The windowed face composes the same module, with no GPU.
///
/// `without_gpu` is the engine owning rule 3 — the five plugin disables a
/// display-less window needs. A consumer re-deriving them was recorded leak
/// material; that it is one builder call is the thing under test.
///
/// Deliberately NOT gated on the `visible` feature. The face selection lives in
/// `ambition_platformer2d::app` with no `cfg` on it, so if this ever stops compiling under
/// default features that is a real change in what a consumer can reach for, and a
/// gate here would hide it.
#[test]
fn platformer_app_composes_the_fixture_windowed_without_a_gpu() {
    let app = PlatformerApp::windowed("Outlander — composition proof")
        .without_gpu()
        .mount(the_one_module())
        .try_build()
        .expect("the windowed face composes the fixture's one module");
    assert!(
        app.get_schedule(bevy::prelude::FixedUpdate).is_some(),
        "the windowed face runs the same fixed-step simulation; only the face differs"
    );
}

/// `without_gpu` on a headless face is a stated conflict, not a silent no-op.
///
/// The builder collects reasons and reports all of them, which is the affordance
/// that makes `try_build` worth having over a panic. A headless face has no
/// render graph to build against no backend, and answering that with silence is
/// how a consumer ends up debugging an absence.
#[test]
fn a_face_that_cannot_honor_a_request_says_so() {
    let error = PlatformerApp::headless()
        .without_gpu()
        .mount(the_one_module())
        .try_build()
        .expect_err("`without_gpu` is meaningless on a headless face");
    let reported = error.to_string();
    assert!(
        reported.contains("without_gpu"),
        "the error must NAME the request it could not honor; got {reported:?}"
    );
}

/// A declared route nothing registers is REFUSED, not silently empty.
///
/// It declared a gameplay route no experience registered and got a host that built clean, ran 60
/// ticks, and spawned zero entities — while `ambition_platformer2d::app`'s own module docs claimed
/// rule 7 was enforced "by TYPE, so the empty host is unreachable rather than merely documented".
/// What was enforced was that a *string* had been supplied.
///
/// An overclaimed guarantee is worse than an absent one, because it tells a
/// consumer to stop looking. The refusal must also NAME the registered routes:
/// "unknown route" is a puzzle, "unknown route, here are the ones that exist"
/// is a typo somebody fixes without a debugger.
#[test]
fn a_declared_route_no_capability_registers_is_refused() {
    struct GhostRoute;

    impl GameModule for GhostRoute {
        fn manifest(&self) -> ModuleManifest {
            ModuleManifest::new("ghost")
        }

        fn define(&self, module: &mut ModuleDraft) {
            module
                .experience(outlander::OUTLANDER_EXPERIENCE)
                .launcher_route(outlander::OUTLANDER_LAUNCHER_ROUTE)
                // Registered by nobody.
                .gameplay_route("ghost/gameplay")
                .capability(outlander::OutlanderExperiencePlugin);
        }
    }

    let error = PlatformerApp::headless()
        .mount(GhostRoute)
        .try_build()
        .expect_err("a gameplay route no capability registers must not compose");
    let reported = error.to_string();
    assert!(
        reported.contains("ghost/gameplay"),
        "the refusal must name the route that does not exist; got {reported:?}"
    );
    // The route named as available is the LAUNCHER, not the gameplay route,
    // and that changed under this test rather than being got wrong once.
    // Since slice C, `playable()` is what registers a gameplay route; this
    // module never calls it, so the only registered route is the one
    // `ShellComposition` installs. Asserting on the gameplay route here would
    // be asserting that a thing this module never declared exists anyway.
    assert!(
        reported.contains(outlander::OUTLANDER_LAUNCHER_ROUTE),
        "the refusal must list the routes that DO exist, or it is a puzzle \
         rather than a fix; got {reported:?}"
    );
}

/// WHAT THE DIAGNOSTICS ACTUALLY SAY, printed rather than described.
///
/// ⭐⭐ THE PLAN ASKED FOR THIS IN THESE WORDS: *"measure first-room workflow and
/// deliberate-error diagnostics rather than only describing them qualitatively"*
/// (`engine/immutable-content-and-transactional-construction.md`). The two arms
/// above assert a SUBSTRING each, which proves the message names the thing — it
/// does not show whether the rest of the message helps somebody who has never
/// seen this engine.
///
/// ⛔ PRINT-ONLY, and deliberately not an assertion. Pinning the exact wording
/// would make every improvement to an error message a red test, which is how a
/// diagnostic stops improving. The measurement belongs in the plan doc, dated;
/// this is how it gets taken again.
#[test]
#[ignore = "PROBE, print-only: what a consumer is actually told"]
fn probe_what_the_refusals_tell_a_stranger() {
    let gpu = PlatformerApp::headless()
        .without_gpu()
        .mount(the_one_module())
        .try_build()
        .expect_err("`without_gpu` is meaningless on a headless face");
    println!("--- without_gpu on a headless face ---\n{gpu}");

    struct GhostRoute;
    impl GameModule for GhostRoute {
        fn manifest(&self) -> ModuleManifest {
            ModuleManifest::new("ghost")
        }
        fn define(&self, module: &mut ModuleDraft) {
            module
                .experience(outlander::OUTLANDER_EXPERIENCE)
                .launcher_route(outlander::OUTLANDER_LAUNCHER_ROUTE)
                .gameplay_route("ghost/gameplay")
                .capability(outlander::OutlanderExperiencePlugin);
        }
    }
    let route = PlatformerApp::headless()
        .mount(GhostRoute)
        .try_build()
        .expect_err("a gameplay route no capability registers must not compose");
    println!("--- a gameplay route nobody registers ---\n{route}");
}
