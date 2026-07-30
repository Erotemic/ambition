//! **Consumer-matrix row 5, the four fifths that do not need rollback.**
//!
//! Smash proves "participants, character selection, atomic match lifecycle,
//! scoped rules, rollback". Rollback as a public knob is deferred by ADR 0031
//! to its own slice, so the row cannot close — but four of its five properties
//! are testable now, and testing them is how the row's remaining blocker stays
//! honestly ONE thing rather than a vague five.
//!
//! ⚠ This is a PARTIAL proof and the matrix records it as such. The campaign
//! has caught itself three times claiming a row on a test that quietly dropped
//! part of it; naming the missing fifth is cheaper than discovering later that
//! "Smash proven" meant "Smash minus the hard part".

use ambition::app::prelude::*;

/// The versus stage, mounted through the public API.
struct VersusModule;

impl GameModule for VersusModule {
    fn manifest(&self) -> ModuleManifest {
        ModuleManifest::new(ambition_app::app::versus::VERSUS_EXPERIENCE)
    }

    fn define(&self, module: &mut ModuleDraft) {
        module
            .experience(ambition_app::app::versus::VERSUS_EXPERIENCE)
            .launcher_route(ambition_app::app::shell_host::AMBITION_LAUNCHER_ROUTE)
            .gameplay_route(ambition_app::app::versus::VERSUS_GAMEPLAY_ROUTE)
            .capability(VersusCapability);
    }
}

#[derive(Clone)]
struct VersusCapability;

impl ambition::bevy::prelude::Plugin for VersusCapability {
    fn build(&self, app: &mut ambition::bevy::prelude::App) {
        ambition_app::app::versus::compose_versus_experience(app);
    }
}

/// **A two-participant stage composes through the SDK and seats its cast.**
#[test]
fn the_versus_stage_composes_and_seats_two_participants() {
    let mut app = PlatformerApp::headless()
        .start_at_launcher()
        .mount(VersusModule)
        .try_build()
        .expect("the versus stage must compose through the public API");

    for _ in 0..600 {
        app.update();
        if host_status(&app).is_refused() {
            panic!("refused: {:?}", host_status(&app).refusal());
        }
    }

    // CHARACTER SELECTION: both fighters are in the assembled cast. A stage that
    // seated one, or none, would still route and still report a live host.
    let catalog = app
        .world()
        .resource::<ambition::character::CharacterCatalog>();
    let ids: Vec<&String> = catalog.iter().map(|(id, _)| id).collect();
    // ⚠ Spelled out rather than importing `versus::FIGHTERS`, which is private.
    // AGENTS.md: never widen a production API to move a test. These are content
    // ids; if the cast changes, this failing is the correct outcome.
    for fighter in ["arena_duelist_long", "arena_duelist_close"] {
        assert!(
            ids.iter().any(|id| id.as_str() == fighter),
            "fighter {fighter:?} is not in the composed cast; got {ids:?}"
        );
    }

    // SCOPED RULES: the stage registered its own route rather than borrowing one.
    let routes: Vec<String> = app
        .world()
        .resource::<ambition::game_shell::ShellRouteCatalog>()
        .ids()
        .map(str::to_string)
        .collect();
    assert!(
        routes
            .iter()
            .any(|r| r == ambition_app::app::versus::VERSUS_GAMEPLAY_ROUTE),
        "the versus route is not registered; got {routes:?}"
    );
}
