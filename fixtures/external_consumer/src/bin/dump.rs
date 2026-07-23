//! Developer-readable dumps through the PUBLIC surface only (Phase-6 task 6):
//! prepared-content identity/fingerprints/owners, the construction registry's
//! recipes and relation kinds — printed from a composed headless app. That
//! this binary lives in the CONSUMER crate is the point: inspectability does
//! not require engine tooling. (That no engine-side dump CLI exists is
//! recorded leak #4.)

use bevy::prelude::*;

fn main() {
    let mut app = App::new();
    ambition::engine::add_headless_foundation(&mut app);
    app.add_plugins(ambition::engine::PlatformerEnginePlugins::fixed_tick());
    app.add_plugins(ambition::windowed_host::PlatformerHostPlugins);
    app.add_plugins(ambition::game_shell::MinimalShellPlugins);
    app.add_plugins(ambition::load::AmbitionLoadPlugin);
    app.add_plugins(outlander::OutlanderExperiencePlugin);

    let world = app.world_mut();

    println!("== rollback registration schema ==");
    if let Some(registry) = world.get_resource::<ambition::runtime::rollback::RollbackRegistry>() {
        println!("{}", registry.deterministic_dump());
        println!("schema fingerprint: {:?}", registry.schema_fingerprint());
    }

    println!("== construction registry ==");
    if let Some(registry) =
        world.get_resource::<ambition::runtime::demo_fixture::ActorConstructionRegistry>()
    {
        println!("{}", registry.deterministic_dump());
    }

    println!("== prepared content ==");
    // `PreparedContent` is not a bare resource — the provider lifecycle owns
    // it per prepared route, so a dump BEFORE launch reports the authored
    // catalog registry instead (the pre-preparation truth).
    if let Some(authored) =
        world.get_resource::<ambition::provider::PlatformerAuthoredCatalogRegistry>()
    {
        println!("{}", authored.deterministic_dump());
    }
}
