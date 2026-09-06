//! Developer-readable dumps through the PUBLIC surface only (Phase-6 task 6):
//! prepared-content identity/fingerprints/owners, the construction registry's
//! recipes and relation kinds — printed from a composed headless app. That
//! this binary lives in the CONSUMER crate is the point: inspectability does
//! not require engine tooling. (That no engine-side dump CLI exists is
//! recorded leak #4.)
//!
//! # Composed by the builder, not by hand
//!
//! This binary was the LAST hand-ordered composition in the fixture. It named
//! `add_headless_foundation`, `PlatformerEnginePlugins::fixed_tick`,
//! `PlatformerHostPlugins` and `MinimalShellPlugins` in a specific order — four
//! engine rules restated by a third party, and one of them wrong: it installed
//! the WINDOWED host in a headless dump, which nothing noticed because the
//! registries it prints do not come from the host.
//!
//! It now calls [`outlander::build_outlander_app`], the same builder the
//! headless binary and the gameplay tests use. That is what "a slice ends with
//! ONE path" means here: not a second composition that happens to agree, but no
//! second composition. Retiring it is what closes `ambition_platformer2d::engine` and
//! `ambition_platformer2d::windowed_host` on the A1 ratchet.

fn main() {
    // The same composition the walkthrough runs. A dump of a DIFFERENT app than
    // the one under test would be a document about a hypothetical build.
    let mut app = outlander::build_outlander_app();
    let world = app.world_mut();
    // Absence is REPORTED, not skipped. Each of these is inserted while plugins
    // build, so a missing one means the builder no longer composes what this
    // dump is about — and a silent `if let Some` turned that into a shorter
    // report nobody would read as a failure. The exit code is what a migration
    // can actually be trusted against.
    let mut missing: Vec<&str> = Vec::new();

    println!("== rollback registration schema ==");
    match world.get_resource::<ambition_platformer2d::rollback::RollbackRegistry>() {
        Some(registry) => {
            println!("{}", registry.deterministic_dump());
            println!("schema fingerprint: {:?}", registry.schema_fingerprint());
        }
        None => missing.push("RollbackRegistry"),
    }

    println!("== construction registry ==");
    match world.get_resource::<ambition_platformer2d::actor::ActorConstructionRegistry>() {
        Some(registry) => println!("{}", registry.deterministic_dump()),
        None => missing.push("ActorConstructionRegistry"),
    }

    println!("== prepared content ==");
    // `PreparedContent` is not a bare resource — the provider lifecycle owns
    // it per prepared route, so a dump BEFORE launch reports the authored
    // catalog registry instead (the pre-preparation truth).
    match world.get_resource::<ambition_platformer2d::character::PlatformerAuthoredCatalogRegistry>() {
        Some(authored) => println!("{}", authored.deterministic_dump()),
        None => missing.push("PlatformerAuthoredCatalogRegistry"),
    }

    if !missing.is_empty() {
        eprintln!(
            "dump: FAILED — the composed app is missing {}. \
             These are inserted at plugin-build time, so this is a composition \
             regression, not an empty world.",
            missing.join(", "),
        );
        std::process::exit(1);
    }
}
