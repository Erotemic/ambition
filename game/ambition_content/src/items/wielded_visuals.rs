//! Ambition's wielded item art: the pack's `data/wielded_item_art.ron`
//! (schema `wielded_item_art`, owned by the items capability), registered
//! into the render layer's over-hand catalog.

use ambition_render::rendering::{WieldedItemVisualAppExt, WieldedItemVisualSpec};
use bevy::math::{Rect, Vec2};
use bevy::prelude::App;

/// Register every row the selected pack states.
pub(super) fn register(app: &mut App) {
    let pack = crate::pack::select(app.world_mut());
    let rows = ambition_items::content_schema::wielded_item_art::lowered_wielded_item_art(&pack)
        .expect("Ambition's pack states its wielded item art (`data/wielded_item_art.ron`)");
    for (id, art) in rows {
        let source = art.source;
        app.register_wielded_item_visual(
            id.clone(),
            WieldedItemVisualSpec {
                texture_path: art.path.clone(),
                source_rect: Rect::from_corners(Vec2::new(source.x, source.y), Vec2::new(source.x + source.w, source.y + source.h)),
                grip_px: Vec2::new(art.grip.0, art.grip.1),
                width_per_wielder_height: art.width_per_wielder_height,
                z_bias: art.z_bias,
                entity_name: art.name.clone(),
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_render::rendering::WieldedItemVisualCatalog;

    /// A hostile body's item is drawn from this catalog alone: the coconut an
    /// African swallow carries needs its row.
    #[test]
    fn the_coconut_a_swallow_carries_has_a_picture() {
        let mut app = App::new();
        register(&mut app);
        assert!(app.world().resource::<WieldedItemVisualCatalog>().get(crate::primeval::COCONUT).is_some());
    }

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
