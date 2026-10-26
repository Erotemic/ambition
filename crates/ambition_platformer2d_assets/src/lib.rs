//! Asset catalog assembly, character materialization, and startup asset loading.
//!
//! Live body mechanics remain at the actor kernel. This crate resolves asset
//! demands and publishes readiness and presentation source facts.

pub mod character_sprites;
pub mod gameplay_defaults;
#[cfg(feature = "ldtk_runtime")]
pub mod loading;
pub mod materialization;
pub mod platformer_assets;

#[cfg(test)]
mod character_roster {
    use ambition_characters::actor::character_catalog::{parse_catalog, CharacterCatalog};
    pub(crate) fn catalog() -> CharacterCatalog {
        CharacterCatalog::from_data(parse_catalog(include_str!(
            "../../../game/ambition_content/assets/data/character_catalog.ron"
        )))
    }
}
