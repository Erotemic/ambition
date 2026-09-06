//! Provider-contributed art declarations for walk-into world items.
//!
//! A `WorldItem` (in `ambition_platformer2d_actor_monolith`) carries a presentation `sprite` id (an art
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

use ambition_platformer2d_core as ae;
use bevy::prelude::{App, Resource};

use crate::binding::{Namespace, Ref, Resolver};

/// The world-item art ids every provider registered.
///
/// This namespace is why a spark blossom fell through the world and never drew: the item carried a
/// `sprite` id with no matching entry, and the renderer's map lookup simply missed.
pub struct WorldItemSprite;

impl Namespace for WorldItemSprite {
    const NAME: &'static str = "world item sprite";
}

/// An authored world-item art reference.
pub type WorldItemSpriteRef = Ref<WorldItemSprite>;

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

impl WorldItemArtManifest {
    /// The entries that actually bind: one per `sprite_id`, the LAST registration
    /// winning, ordered by id.
    ///
    /// Last-wins is the behaviour of the map-insert loop this replaced, and it is
    /// the useful one for a merge target — a host composing providers lets the
    /// later one override. Resolve and load from THIS list, not from `self.0`, so
    /// a binding's `slot()` indexes the art that actually won.
    pub fn effective(&self) -> Vec<&WorldItemArtEntry> {
        let mut winners: std::collections::BTreeMap<&str, &WorldItemArtEntry> =
            std::collections::BTreeMap::new();
        for entry in &self.0 {
            winners.insert(entry.sprite_id.as_str(), entry);
        }
        winners.into_values().collect()
    }

    /// The art ids this manifest binds. Slots index [`Self::effective`].
    pub fn sprite_ids(&self) -> Resolver<WorldItemSprite> {
        Resolver::new(
            self.effective()
                .into_iter()
                .map(|entry| entry.sprite_id.as_str()),
        )
    }
}

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

    /// A later registration overrides an earlier one for the same id (the merge
    /// rule a host composing providers relies on), and a resolved id's `slot()`
    /// indexes the winner — the alignment the render layer loads handles against.
    #[test]
    fn a_later_registration_wins_and_its_slot_addresses_it() {
        let manifest = WorldItemArtManifest(vec![
            WorldItemArtEntry::new("milk", "base/milk.png", ae::Vec2::new(24.0, 28.0)),
            WorldItemArtEntry::new("ring", "base/ring.png", ae::Vec2::new(16.0, 16.0)),
            WorldItemArtEntry::new("milk", "override/milk.png", ae::Vec2::new(24.0, 28.0)),
        ]);

        let effective = manifest.effective();
        assert_eq!(effective.len(), 2, "one entry per id");

        let bound = manifest
            .sprite_ids()
            .resolve(&WorldItemSpriteRef::new("milk"), "test")
            .expect("milk is registered");
        assert_eq!(effective[bound.slot()].asset_path, "override/milk.png");
    }
}
