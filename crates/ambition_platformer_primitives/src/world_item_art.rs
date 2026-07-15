//! Provider-contributed art declarations for walk-into world items.
//!
//! A `WorldItem` (in `ambition_actors`) carries a presentation `sprite` id (an art
//! key, deliberately separate from the equipment it grants).
//! The render layer draws that id as a real sprite through its `WorldItemArt`
//! handle map — but a gameplay PROVIDER crate (which owns the asset knowledge:
//! "the milk pickup is `sprites/props/super_mary_o_milk_carton.png`") must not
//! depend on the renderer. So the contribution is split, exactly like the audio /
//! character catalog fragments:
//!
//! - the game contributes pure DATA here ([`WorldItemArtEntry`]: id → path + size),
//!   registered on the `App` at plugin-build time via [`WorldItemArtAppExt`];
//! - the render layer resolves each path string into a loaded image handle at
//!   startup, filling its `WorldItemArt` resource.
//!
//! Because the manifest is a MERGE target (contributors extend it), a multi-game
//! host that composes several providers unions their pickup art rather than one
//! provider's `insert_resource` clobbering another's.

use ambition_engine_core as ae;
use bevy::prelude::{App, Resource};

/// One game's declaration of art for a walk-into world item: the presentation
/// `sprite` id → the asset path to draw and its on-screen size. Pure data (no
/// render types), so a provider crate contributes it without a render dependency.
#[derive(Clone, Debug, PartialEq)]
pub struct WorldItemArtEntry {
    /// The `sprite` id a `WorldItem` carries (the render lookup key).
    pub sprite_id: String,
    /// Asset-server path to the image (e.g. `sprites/props/milk_carton.png`).
    pub asset_path: String,
    /// On-screen display size, world units.
    pub size: ae::Vec2,
}

impl WorldItemArtEntry {
    /// Declare `sprite_id` draws `asset_path` at `size`.
    pub fn new(
        sprite_id: impl Into<String>,
        asset_path: impl Into<String>,
        size: ae::Vec2,
    ) -> Self {
        Self {
            sprite_id: sprite_id.into(),
            asset_path: asset_path.into(),
            size,
        }
    }
}

/// Accumulates every provider's [`WorldItemArtEntry`] before the render layer
/// resolves them into loaded handles. Contributors EXTEND it (never replace), so
/// composing several games unions their pickup art.
#[derive(Resource, Default, Debug)]
pub struct WorldItemArtManifest(pub Vec<WorldItemArtEntry>);

/// Register a game's walk-into pickup art (data only). The render layer's startup
/// loader turns these into real image handles; a headless app simply never reads
/// the manifest. Idempotent resource init; each call appends.
pub trait WorldItemArtAppExt {
    /// Contribute art declarations for this game's world items.
    fn register_world_item_art(
        &mut self,
        entries: impl IntoIterator<Item = WorldItemArtEntry>,
    ) -> &mut Self;
}

impl WorldItemArtAppExt for App {
    fn register_world_item_art(
        &mut self,
        entries: impl IntoIterator<Item = WorldItemArtEntry>,
    ) -> &mut Self {
        self.init_resource::<WorldItemArtManifest>();
        self.world_mut()
            .resource_mut::<WorldItemArtManifest>()
            .0
            .extend(entries);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two providers registering their own pickup art UNION into one manifest —
    /// the second's contribution does not clobber the first's (the multi-game host
    /// invariant this seam exists to hold).
    #[test]
    fn registering_two_providers_unions_their_art() {
        let mut app = App::new();
        app.register_world_item_art([WorldItemArtEntry::new(
            "milk",
            "sprites/props/milk.png",
            ae::Vec2::new(24.0, 28.0),
        )]);
        app.register_world_item_art([WorldItemArtEntry::new(
            "ring",
            "sprites/props/ring.png",
            ae::Vec2::new(16.0, 16.0),
        )]);

        let manifest = app.world().resource::<WorldItemArtManifest>();
        assert_eq!(manifest.0.len(), 2, "both providers' entries survive");
        assert!(manifest.0.iter().any(|e| e.sprite_id == "milk"));
        assert!(manifest.0.iter().any(|e| e.sprite_id == "ring"));
    }
}
