//! The schemas the ENGINE itself owns — the one list, in one place.
//!
//! ⛔⛔ THERE WERE TWO HAND-KEPT COPIES AND A TEST HOLDING THEM EQUAL.
//! `ambition_platformer2d::content::engine_schemas()` and
//! `ambition_content_cli::default_registry()` each spelled the same twelve
//! registrations, in different orders, in different crates, and
//! `content_pack_registry::the_tools_composition_and_the_games_composition_are_the_same_set`
//! asserted they matched. The CLI's own comment stated the rule that test
//! enforced — *"A schema added to one belongs in the other in the same commit"* —
//! which is a rule about a list that should not have been two lists.
//!
//! ⭐ THE OBVIOUS COLLAPSE WAS THE WRONG ONE. The CLI cannot call
//! `engine_schemas()` where it lived, because that is the `ambition_platformer2d`
//! facade and the CLI is a lightweight validator that deliberately does not link
//! it. MEASURED 2026-09-11: the facade is absent from the CLI's 324-crate
//! closure, and this crate's six dependencies were ALL already in it — so the
//! one list moved DOWN to a crate both sides can reach. Measured by member: the
//! CLI goes 324 -> 325 crates and the only addition is this one.
//!
//! ⚠ WHAT WOULD HAVE MADE THIS NOT WORTH DOING, checked first: a schema owned by
//! a crate the CLI does not link. None is. All twelve belong to the six
//! capability crates the CLI already declared by name in its own manifest, and
//! none of those six is an optional edge of the facade — so the two lists were
//! the same fact in every configuration, not merely in the default one.

pub use ambition_content_pack::SchemaRegistry;

/// The schemas the ENGINE itself owns, ready for a consumer to add to.
///
/// A consumer cannot assemble this for itself without knowing which crates own
/// which schemas — exactly the internal topology the SDK is supposed to hide. A
/// capability's own schema is added on top; nobody has to know that the
/// character catalog lives in `ambition_characters`.
///
/// ⚠ ORDER IS NOT MEANING. `register` refuses a DUPLICATE id and nothing else,
/// so this is a SET. The two copies this replaced listed the same twelve in
/// different orders, which is how a reader could believe they had diverged when
/// they had not — and how a real divergence could hide.
pub fn engine_schemas() -> SchemaRegistry {
    let mut registry = SchemaRegistry::new();
    for schema in [
        ambition_characters::actor::character_catalog::character_catalog_schema(),
        // The capability owns the schema even though its types still live in
        // `ambition_characters` — which is the point of registering it, since a
        // registration is where ownership becomes something a tool can print
        // rather than a comment.
        ambition_characters::smash_fighter::content_schema::smash_fighter_schema(),
        // ⭐⭐ MOVE TABLES ARE CONTENT (fast-iteration I2). The same capability
        // owns them as owns the catalog they key against: a move table names a
        // character, and a composition that installs one without the other could
        // admit a file describing a fighter it cannot build.
        ambition_characters::moveset_content_schema::moveset_schema(),
        ambition_combat::brain::fighter::content_schema::fighter_brain_ladder_schema(),
        // a capability's schema follows the CAPABILITY: a composition without
        // `ambition_items` must not claim to own `item_catalog`, which is what
        // makes "uninstalled capability" a real refusal rather than a
        // hypothetical one.
        ambition_items::content_schema::item_catalog_schema(),
        // Same rule, same reason, for `encounter_waves`.
        ambition_encounter::content_schema::encounter_waves_schema(),
        // The boss schemas moved with the boss's thinking (D168, 2026-08-27).
        ambition_boss_encounter::pattern::content_schema::boss_seed_library_schema(),
        ambition_boss_encounter::pattern::content_schema::boss_validator_bands_schema(),
        ambition_boss_encounter::pattern::content_schema::boss_profiles_schema(),
        ambition_boss_encounter::pattern::content_schema::boss_encounter_schema(),
        ambition_audio::content_schema::music_registry_schema(),
        ambition_audio::content_schema::sfx_registry_schema(),
    ] {
        registry
            .register(schema)
            .expect("the engine's own schemas are registered once");
    }
    registry
}
