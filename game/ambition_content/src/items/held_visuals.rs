//! Ambition's held item art: the pack's `data/held_item_art.ron` (schema
//! `held_item_art`, owned by the items capability).
//!
//! Every ground-pickup and in-hand picture Ambition's held items use (the axe,
//! the javelin, the gun-sword, the wielded-gauntlet ability props) is a row of
//! that file, contributed to the render layer's
//! [`HeldItemArtManifest`](ambition_platformer2d_shared_tangle::held_item_art::HeldItemArtManifest).
//! The renderer names none of these paths; it resolves the merged manifest
//! into image handles at startup.

use ambition_platformer2d_shared_tangle::held_item_art::{HeldItemArtAppExt, HeldItemArtEntry};
use bevy::math::Vec2;
use bevy::prelude::{App, Resource};

/// The held item art ids Ambition's pack registered: the rows a reload
/// restates, and no other provider's. A composition that draws no item art
/// has no record, and a reload restates nothing there.
#[derive(Resource, Clone, Debug, Default)]
pub(crate) struct AmbitionHeldItemArt(pub(crate) Vec<String>);

/// The manifest entries `pack` states.
pub(crate) fn entries(pack: &ambition_content_pack::PreparedContentPack) -> Vec<HeldItemArtEntry> {
    let rows = ambition_items::content_schema::held_item_art::lowered_held_item_art(pack)
        .expect("Ambition's pack states its held item art (`data/held_item_art.ron`)");
    rows.iter()
        .map(|(id, art)| {
            let entry = HeldItemArtEntry::new(id.clone(), art.path.clone(), Vec2::new(art.size.0, art.size.1));
            match art.grip {
                Some(grip) => entry.gripped_at(grip),
                None => entry,
            }
        })
        .collect()
}

/// Contribute the selected pack's held item art to the render manifest.
pub(crate) fn register(app: &mut App) {
    let pack = crate::pack::select(app.world_mut());
    let entries = entries(&pack);
    let ids = entries.iter().map(|entry| entry.item_id.clone()).collect();
    app.register_held_item_art(entries);
    app.insert_resource(AmbitionHeldItemArt(ids));
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::held_item_art::HeldItemArtManifest;

    /// The manifest is the selected pack's file: an edit to the axe's row is
    /// the size it is drawn at, with no Rust build. The javelin is the control.
    #[test]
    fn the_held_item_art_is_the_pack_file() {
        // The anchor stops at the size: the row goes on (its `grip`).
        const AXE: &str = r#""axe": (path: "sprites/props/axe.png", size: (40.0, 18.0)"#;
        let pack = crate::pack::compile_pack_with(|path, text| {
            if path != "data/held_item_art.ron" {
                return text;
            }
            assert_eq!(text.matches(AXE).count(), 1, "the edit anchor");
            text.replace(AXE, r#""axe": (path: "sprites/props/axe.png", size: (50.0, 20.0)"#)
        })
        .unwrap_or_else(|failure| panic!("{}", failure.render()));
        let mut app = App::new();
        crate::pack::select_pack(&mut app, std::sync::Arc::new(pack)).expect("a fresh App takes the pack");
        register(&mut app);
        let manifest = app.world().resource::<HeldItemArtManifest>();
        let size = |id: &str| manifest.effective().into_iter().find(|entry| entry.item_id == id).map(|entry| entry.size);
        assert_eq!(size("javelin").map(|s| (s.x, s.y)), Some((58.0, 7.0)), "control: the javelin is the shipped row");
        assert_eq!(size("axe").map(|s| (s.x, s.y)), Some((50.0, 20.0)), "the axe is not drawn at the edited size");
    }

    #[test]
    fn ambition_registers_its_held_item_props() {
        let mut app = App::new();
        register(&mut app);
        let manifest = app.world().resource::<HeldItemArtManifest>();
        // The Polygon's three are named explicitly, because a missing art
        // registration logs an `ERROR` per spawn and draws nothing (see above).
        for id in [
            "axe",
            "javelin",
            "coconut",
            "gun_sword",
            "gun_sword_heavy",
            "blink",
            "polygon_bomb",
            "polygon_ponytail",
            "polygon_mine",
        ] {
            assert!(
                manifest.0.iter().any(|e| e.item_id == id),
                "{id} art must be registered"
            );
        }
    }
}
