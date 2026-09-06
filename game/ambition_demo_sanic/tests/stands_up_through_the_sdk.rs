//! Consumer-matrix row 4, standalone half: a module that PREDATES the SDK
//! stands up through it.
//!
//! Sanic was not.
//!
//! That distinction is the point. §4 authorises a decomposition on a SENTINEL
//! CONSUMER's capability footprint, and a footprint measured only against games
//! their own API author designed answers a much weaker question.
//!
//! Sanic mounts as a `capability`, not through `playable()`. `playable` does
//! not carry presentation profiles or a HUD declaration, and inventing those
//! parameters to make this test prettier would be designing an API from one
//! caller. The capability slot is the supported escape hatch for exactly this,
//! and using it here is evidence about where `playable` stops rather than a
//! workaround.

use ambition_platformer2d::app::prelude::*;
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

/// It boots, and it REACHES A RUNNING HOST.
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

/// Sanic standalone and Sanic embedded produce the SAME identities.
///
/// Two identities, because they can fail independently:
///
/// * the authored content registry's deterministic dump — what Sanic
///   declared;
/// * the rollback schema fingerprint — what a session would snapshot.
///
/// Compared for SANIC specifically, not for the whole composition. The
/// embedded app legitimately contains more (the other module's content, its
/// routes), so asserting whole-app equality would be asserting that embedding
/// changes nothing, which is false and uninteresting. What must not change is
/// Sanic's own identity — that is what "the same module" means.
#[test]
fn sanic_has_the_same_identities_standalone_and_embedded() {
    /// A second, unrelated module to embed Sanic alongside.
    struct Neighbour;

    impl GameModule for Neighbour {
        fn manifest(&self) -> ModuleManifest {
            ModuleManifest::new("neighbour")
        }
        fn define(&self, module: &mut ModuleDraft) {
            module
                .experience("neighbour")
                .launcher_route("neighbour/menu")
                .gameplay_route("neighbour/play")
                .characters(ambition_platformer2d::app::MINIMAL_CHARACTER_ROSTER_RON)
                .no_audio()
                // It must be PLAYABLE, or nothing registers its route and rule
                // 7 refuses the composition — which it did on the first
                // attempt, correctly, from the widened per-experience check.
                .playable(
                    "Neighbour",
                    "an unrelated module for Sanic to be embedded beside",
                    "my_hero",
                    "neighbour_room",
                    vec![neighbour_room()],
                );
        }
    }

    fn neighbour_room() -> ambition_platformer2d::world::prelude::RoomSpec {
        use ambition_platformer2d::world::prelude::*;
        let size = Vec2::new(640.0, 360.0);
        let world = AuthoredWorld::new(
            "Neighbour Room",
            size,
            Vec2::new(64.0, 256.0),
            // MIN corner, not a centre — see the note in
            // `fixtures/minimal_game`, which had this wrong and whose own tests
            // could not see it.
            vec![Block::solid("floor", Vec2::new(0.0, 320.0), Vec2::new(size.x, 40.0))],
        );
        RoomSpec::new("neighbour_room", world)
    }

    fn sanic_identity(app: &ambition_platformer2d::bevy::prelude::App) -> (String, String) {
        let authored = app
            .world()
            .get_resource::<ambition_platformer2d::character::PlatformerAuthoredCatalogRegistry>()
            .expect("the authored catalog registry exists once a provider registered")
            .deterministic_dump();
        // Only Sanic's rows. The embedded app also holds the neighbour's, and
        // including those would compare the COMPOSITION rather than the module.
        let sanic_rows: String = authored
            .lines()
            .filter(|line| line.contains(SANIC_EXPERIENCE))
            .collect::<Vec<_>>()
            .join("\n");
        let schema = format!(
            "{:?}",
            app.world()
                .get_resource::<ambition_platformer2d::rollback::RollbackRegistry>()
                .expect("the rollback registry exists")
                .schema_fingerprint()
        );
        (sanic_rows, schema)
    }

    let standalone = PlatformerApp::headless()
        .mount(SanicModule)
        .build();
    let embedded = PlatformerApp::headless()
        .mount(SanicModule)
        .mount(Neighbour)
        .build();

    let (standalone_content, standalone_schema) = sanic_identity(&standalone);
    let (embedded_content, embedded_schema) = sanic_identity(&embedded);

    // Non-vacuity: two empty strings are trivially equal, and that failure would
    // be green in the flattering direction.
    assert!(
        !standalone_content.is_empty(),
        "no Sanic rows in the authored registry — this is comparing nothing"
    );

    assert_eq!(
        standalone_content, embedded_content,
        "Sanic's authored content changed when it was embedded beside another \
         module, so 'the same module' is not the same content"
    );
    assert_eq!(
        standalone_schema, embedded_schema,
        "the rollback schema fingerprint changed when a second module was \
         mounted, so a save or a session from one composition would not load in \
         the other"
    );
}
