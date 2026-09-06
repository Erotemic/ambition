//! Ambition-owned over-hand item presentation registrations.

use ambition_render::rendering::{WieldedItemVisualAppExt, WieldedItemVisualSpec};
use bevy::math::{Rect, Vec2};
use bevy::prelude::App;

const GUN_SWORD_SHEET_PATH: &str = "sprites/lasersword_with_guns_spritesheet.png";
const GUN_SWORD_FRAME_W: f32 = 177.0;
const GUN_SWORD_FRAME_H: f32 = 46.0;
const GUN_SWORD_IDLE_FRAME_X: f32 = 118.0;
const GUN_SWORD_IDLE_FRAME_Y: f32 = 0.0;
const GUN_SWORD_GRIP_X_PX: f32 = 36.45;
const GUN_SWORD_GRIP_Y_PX: f32 = 23.8;
const GUN_SWORD_WIDTH_PER_WIELDER_HEIGHT: f32 = 64.0 / 72.0;

fn gun_sword_visual(entity_name: &str) -> WieldedItemVisualSpec {
    WieldedItemVisualSpec {
        texture_path: GUN_SWORD_SHEET_PATH.into(),
        source_rect: Rect::from_corners(
            Vec2::new(GUN_SWORD_IDLE_FRAME_X, GUN_SWORD_IDLE_FRAME_Y),
            Vec2::new(
                GUN_SWORD_IDLE_FRAME_X + GUN_SWORD_FRAME_W,
                GUN_SWORD_IDLE_FRAME_Y + GUN_SWORD_FRAME_H,
            ),
        ),
        grip_px: Vec2::new(GUN_SWORD_GRIP_X_PX, GUN_SWORD_GRIP_Y_PX),
        width_per_wielder_height: GUN_SWORD_WIDTH_PER_WIELDER_HEIGHT,
        z_bias: 0.7,
        entity_name: entity_name.into(),
    }
}

pub(super) fn register(app: &mut App) {
    app.register_wielded_item_visual("gun_sword", gun_sword_visual("Pirate gun-sword"));
    // The heavy archetype has always authored a distinct mechanical item id,
    // but uses the same visible weapon sheet and anchor.
    app.register_wielded_item_visual(
        "gun_sword_heavy",
        gun_sword_visual("Heavy pirate gun-sword"),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_render::rendering::WieldedItemVisualCatalog;

    #[test]
    fn ambition_registers_both_gun_sword_item_ids() {
        let mut app = App::new();
        register(&mut app);
        let catalog = app.world().resource::<WieldedItemVisualCatalog>();
        assert!(catalog.get("gun_sword").is_some());
        assert!(catalog.get("gun_sword_heavy").is_some());
        assert!(catalog.get("unrelated_item").is_none());
    }
}
