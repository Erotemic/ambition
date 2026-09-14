//! Canonical registry of schemas owned by the engine.
//!
//! Both runtime composition and lightweight tooling consume this registry so the
//! schema set has one owner without requiring tools to link the public game facade.

pub use ambition_content_pack::SchemaRegistry;

/// Return the engine-owned schema set. Consumers may add capability-specific
/// schemas without knowing the internal crates that own the built-in entries.
/// Registration order has no semantic meaning; duplicate IDs are refused.
pub fn engine_schemas() -> SchemaRegistry {
    let mut registry = SchemaRegistry::new();
    for schema in [
        ambition_characters::actor::character_catalog::character_catalog_schema(),
        // Schema ownership follows the capability, not the crate containing its types.
        ambition_characters::smash_fighter::content_schema::smash_fighter_schema(),
        // Move tables belong with the character capability whose catalog they reference.
        ambition_characters::moveset_content_schema::moveset_schema(),
        ambition_combat::brain::fighter::content_schema::fighter_brain_ladder_schema(),
        // Capability schemas remain owned by the capability that can interpret them.
        ambition_items::content_schema::item_catalog_schema(),
        ambition_encounter::content_schema::encounter_waves_schema(),
        // Boss schemas stay with the boss capability.
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
