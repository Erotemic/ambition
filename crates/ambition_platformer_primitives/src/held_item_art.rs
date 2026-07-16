//! Provider-contributed art declarations for inventory/held items (the ground
//! pickup + in-hand icon of a `HeldItem`: an axe, a javelin, a gun-sword, a
//! wielded-gauntlet ability prop).
//!
//! A held item is identified by its spec id (`"axe"`, `"gun_sword"`,
//! `"shockwave"`, …). The render layer draws that id as a real sprite through its
//! `HeldItemArt` handle map — but a gameplay PROVIDER crate (which owns the asset
//! knowledge: "the axe is `sprites/props/axe.png`") must not depend on the
//! renderer. So the contribution is split, exactly like the world-item /
//! audio / character catalog fragments:
//!
//! - the game contributes pure DATA here ([`HeldItemArtEntry`]: id → path + size),
//!   registered on the `App` at plugin-build time via [`HeldItemArtAppExt`];
//! - the render layer resolves each path string into a loaded image handle at
//!   startup, filling its `HeldItemArt` resource.
//!
//! Because the manifest is a MERGE target (contributors extend it), a multi-game
//! host that composes several providers unions their held-item art rather than one
//! provider's `insert_resource` clobbering another's.

use ambition_engine_core as ae;
use bevy::prelude::{App, Resource};

/// One game's declaration of art for a held/inventory item: the held-item spec
/// id → the asset path to draw and its on-screen size. Pure data (no render
/// types), so a provider crate contributes it without a render dependency.
#[derive(Clone, Debug, PartialEq)]
pub struct HeldItemArtEntry {
    /// The held-item spec id (the render lookup key, e.g. `"axe"`, `"gun_sword"`).
    pub item_id: String,
    /// Asset-server path to the image (e.g. `sprites/props/axe.png`).
    pub asset_path: String,
    /// On-screen display size, world units.
    pub size: ae::Vec2,
}

impl HeldItemArtEntry {
    /// Declare `item_id` draws `asset_path` at `size`.
    pub fn new(item_id: impl Into<String>, asset_path: impl Into<String>, size: ae::Vec2) -> Self {
        Self {
            item_id: item_id.into(),
            asset_path: asset_path.into(),
            size,
        }
    }
}

/// Accumulates every provider's [`HeldItemArtEntry`] before the render layer
/// resolves them into loaded handles. Contributors EXTEND it (never replace), so
/// composing several games unions their held-item art. If two contributions name
/// the same `item_id`, the LAST-registered entry wins at resolution (the resolver
/// folds the `Vec` in registration order), which is deterministic plugin-add order.
#[derive(Resource, Default, Debug)]
pub struct HeldItemArtManifest(pub Vec<HeldItemArtEntry>);

/// Register a game's held/inventory item art (data only). The render layer's
/// startup loader turns these into real image handles; a headless app simply
/// never reads the manifest. Idempotent resource init; each call appends.
pub trait HeldItemArtAppExt {
    /// Contribute art declarations for this game's held items.
    fn register_held_item_art(
        &mut self,
        entries: impl IntoIterator<Item = HeldItemArtEntry>,
    ) -> &mut Self;
}

impl HeldItemArtAppExt for App {
    fn register_held_item_art(
        &mut self,
        entries: impl IntoIterator<Item = HeldItemArtEntry>,
    ) -> &mut Self {
        self.init_resource::<HeldItemArtManifest>();
        self.world_mut()
            .resource_mut::<HeldItemArtManifest>()
            .0
            .extend(entries);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two providers registering their own held-item art UNION into one manifest —
    /// the second's contribution does not clobber the first's (the multi-game host
    /// invariant this seam exists to hold).
    #[test]
    fn registering_two_providers_unions_their_art() {
        let mut app = App::new();
        app.register_held_item_art([HeldItemArtEntry::new(
            "axe",
            "sprites/props/axe.png",
            ae::Vec2::new(40.0, 18.0),
        )]);
        app.register_held_item_art([HeldItemArtEntry::new(
            "hammer",
            "sprites/props/hammer.png",
            ae::Vec2::new(30.0, 30.0),
        )]);

        let manifest = app.world().resource::<HeldItemArtManifest>();
        assert_eq!(manifest.0.len(), 2, "both providers' entries survive");
        assert!(manifest.0.iter().any(|e| e.item_id == "axe"));
        assert!(manifest.0.iter().any(|e| e.item_id == "hammer"));
    }
}
