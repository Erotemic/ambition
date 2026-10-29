//! Startup assets selected by the Ambition game.

use bevy::prelude::*;
use bevy_asset_loader::prelude::AssetCollection;
use ambition_platformer2d_assets::gameplay_defaults::Platformer2dGameplayDefaults;

#[derive(AssetCollection, Resource, Clone)]
pub struct AmbitionStartupAssets {
    #[asset(path = "game://ambition/platformer_defaults.ron")]
    pub gameplay_defaults: Handle<Platformer2dGameplayDefaults>,
    #[asset(path = "game://worlds/sandbox.ldtk")]
    pub ldtk_project: Handle<bevy_ecs_ldtk::assets::LdtkProject>,
}
