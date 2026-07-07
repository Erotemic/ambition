//! Asset-loading foundation for the sandbox.
//!
//! The sandbox still keeps the embedded RON fallback so iteration is robust when
//! Bevy asset loading fails. This module introduces `bevy_asset_loader` as the
//! central place for future manifest/audio/dialogue/visual asset collections.

use bevy::prelude::*;
use bevy_asset_loader::prelude::AssetCollection;

use crate::session::data::SandboxDataSpec;

/// First asset collection for the sandbox.
///
/// `bevy_asset_loader` inserts this resource immediately through
/// `init_collection` in this conservative first pass. A later patch can promote
/// it into a real `BootState::Loading -> Ready` loading state once startup has
/// enough assets to justify the state transition.
#[derive(AssetCollection, Resource, Clone)]
pub struct SandboxAssetCollection {
    #[asset(path = "ambition/sandbox.ron")]
    pub sandbox_data: Handle<SandboxDataSpec>,

    // Worlds moved to the content crate's `assets/` tree (R3.2), served via the
    // `game://` asset source the app registers — NOT the default (actor-sim)
    // source. The canonical world path is `game://worlds/<file>` (see
    // `world_manifest`/`world_bevy_asset_path`); loading it from the default
    // source here 404'd ("ambition/worlds/sandbox.ldtk" no longer exists).
    #[asset(path = "game://worlds/sandbox.ldtk")]
    pub ldtk_project: Handle<bevy_ecs_ldtk::assets::LdtkProject>,
}
