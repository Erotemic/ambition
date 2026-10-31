//! Prepare room assets and covers for visible platformer hosts.

mod first_room_art;
mod first_room_cover;
mod parallax_residency;
mod room_transition_assets;
mod room_transition_presentation;

pub use room_transition_assets::{
    build_loaded_room_asset_manifest, demand_room_character_sheets, inspect_demanded_characters,
    inspect_room_asset_manifest, realized_character_count, room_character_tokens, RoomAssetManifest,
    RoomAssetReadiness, RoomResidencyOwners,
};
pub use room_transition_presentation::RoomTransitionCoverSet;

/// Install the asset barrier and cover for each visible session.
pub struct RoomLoadingPresentationPlugin;

impl bevy::prelude::Plugin for RoomLoadingPresentationPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugins(crate::load::AmbitionLoadPlugin);
        app.add_plugins(crate::render::rendering::room_transition_cover::RoomTransitionCoverPlugin);
        if !app.is_plugin_added::<crate::load_presentation::AmbitionLoadPresentationPlugin>() {
            app.add_plugins(crate::load_presentation::MinimalLoadPresentationPlugins);
        }
        room_transition_presentation::install_room_transition_presentation(app);
    }
}

/// Count the room preparations made by the neighbor cache.
pub fn prefetch_preparations(world: &bevy::prelude::World) -> u64 {
    world.get_resource::<room_transition_assets::RoomPreparationPrefetchState>()
        .map_or(0, |cache| cache.preparations)
}
