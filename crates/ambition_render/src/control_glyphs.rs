//! Load published control pictures through the selected sprite content.

use ambition_asset_manager::platformer_assets::Platformer2dAssetCatalog;
use ambition_sprite_sheet::SpriteContent;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

/// The render-owned image inputs for a control presenter.
#[derive(SystemParam)]
pub struct ControlGlyphImages<'w> {
    catalog: Option<Res<'w, Platformer2dAssetCatalog>>,
    content: Option<Res<'w, SpriteContent>>,
    server: Option<Res<'w, AssetServer>>,
}

impl ControlGlyphImages<'_> {
    pub fn is_changed(&self) -> bool {
        self.catalog
            .as_ref()
            .is_some_and(|catalog| catalog.is_changed())
            || self
                .content
                .as_ref()
                .is_some_and(|content| content.is_changed())
    }

    /// Missing art leaves the caller's control label visible.
    pub fn load(&self, path: &str) -> Option<Handle<Image>> {
        let path = self.catalog.as_ref()?.try_image_path_for_load(path)?;
        Some(ambition_sprite_sheet::game_assets::load_sheet_image(
            self.content.as_deref(),
            self.server.as_deref()?,
            "control glyph",
            path,
        ))
    }

    /// Use the load state because the renderer can release the CPU image.
    pub fn is_ready(&self, image: &Handle<Image>) -> bool {
        self.server
            .as_ref()
            .is_some_and(|server| server.is_loaded_with_dependencies(image.id()))
    }
}
