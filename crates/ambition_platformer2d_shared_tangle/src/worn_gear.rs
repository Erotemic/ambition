//! Provider-contributed declarations of worn gear: an item a body wears on an
//! anchor of its art (a space helmet on its head).
//!
//! A gear is not a part of one character's art. It is an item with its own
//! pictures, and it fits each wearer sheet by a [`GearFit`]. Three readers
//! use one declaration:
//!
//! - the simulation puts a gear on a body as an equipment row (its id and its
//!   exclusive slot) and refuses a gear id that no provider declared;
//! - the observation boundary publishes the gear each body wears;
//! - the render layer draws the pictures on the wearer's anchor part.
//!
//! So the declaration is pure DATA here, as the held-item art is
//! (`held_item_art`): a game contributes [`WornGearEntry`] rows through
//! [`WornGearAppExt`], and the manifest is a MERGE target that a host with
//! several providers unions.

use std::collections::BTreeMap;

use ambition_platformer2d_core as ae;
use bevy::prelude::{App, Resource};

/// The part of a wearer that a gear attaches to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GearAnchor {
    /// The head.
    #[default]
    Head,
}

/// How a gear sits on one wearer sheet.
#[derive(Clone, Debug, PartialEq)]
pub struct GearFit {
    /// The track of the sheet's part flipbook that is the anchor part.
    pub track: String,
    /// The size of the pictures, as a part of [`WornGearEntry::size`].
    pub scale: f32,
    /// The move of the pictures from the anchor's pivot, in world units, in
    /// the frame of the body: +x is the way it faces, +y is up.
    pub offset: ae::Vec2,
    /// The turn of the pictures on the anchor, in degrees. A positive turn
    /// lifts the side of the pictures that is to the way the body faces.
    pub turn_degrees: f32,
}

/// One provider's declaration of one gear.
#[derive(Clone, Debug, PartialEq)]
pub struct WornGearEntry {
    /// The gear id. It is also the id of the equipment row a wearer wears.
    pub id: String,
    /// The part of the wearer that the gear attaches to.
    pub anchor: GearAnchor,
    /// The exclusive equipment slot: a body wears one gear in a slot.
    pub slot: String,
    /// The asset path of the picture drawn under the body.
    pub back: Option<String>,
    /// The asset path of the picture drawn over the anchor part.
    pub front: Option<String>,
    /// The size of the pictures in world units, at a fit scale of 1.
    pub size: ae::Vec2,
    /// The point of the pictures that is on the anchor's pivot, from 0 to 1
    /// on each axis, from the top left.
    pub socket: ae::Vec2,
    /// The fit for each wearer, by the wearer's sheet target.
    pub fits: BTreeMap<String, GearFit>,
}

/// Every provider's [`WornGearEntry`]. Contributors EXTEND it. When two
/// entries have one id, the LAST registered entry is the gear.
#[derive(Resource, Default, Debug)]
pub struct WornGearManifest(pub Vec<WornGearEntry>);

impl WornGearManifest {
    /// The gear with this id, the last registration winning.
    pub fn get(&self, id: &str) -> Option<&WornGearEntry> {
        self.0.iter().rev().find(|entry| entry.id == id)
    }

    /// Replace the entries one provider stated (`previous`, the ids it
    /// registered) with `entries`, as a content reload does. The ids it
    /// registered now are returned, for the next reload.
    pub fn restate<'a>(&mut self, previous: impl IntoIterator<Item = &'a str>, entries: Vec<WornGearEntry>) -> Vec<String> {
        let previous: Vec<&str> = previous.into_iter().collect();
        self.0.retain(|entry| !previous.contains(&entry.id.as_str()));
        let ids = entries.iter().map(|entry| entry.id.clone()).collect();
        self.0.extend(entries);
        ids
    }
}

/// Register a game's worn gear (data only). Each call appends.
pub trait WornGearAppExt {
    /// Contribute the declarations of this game's gear.
    fn register_worn_gear(&mut self, entries: impl IntoIterator<Item = WornGearEntry>) -> &mut Self;
}

impl WornGearAppExt for App {
    fn register_worn_gear(&mut self, entries: impl IntoIterator<Item = WornGearEntry>) -> &mut Self {
        self.init_resource::<WornGearManifest>();
        self.world_mut().resource_mut::<WornGearManifest>().0.extend(entries);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn helmet(slot: &str) -> WornGearEntry {
        WornGearEntry {
            id: "space_helmet".into(),
            anchor: GearAnchor::Head,
            slot: slot.into(),
            back: None,
            front: Some("sprites/space_helmet_front.png".into()),
            size: ae::Vec2::splat(64.0),
            socket: ae::Vec2::new(0.5, 0.58),
            fits: BTreeMap::new(),
        }
    }

    /// Two providers union their gear; a second entry for one id wins, and a
    /// restate withdraws only the ids of its provider.
    #[test]
    fn providers_union_their_gear_and_the_last_entry_of_an_id_wins() {
        let mut app = App::new();
        app.register_worn_gear([helmet("head")]);
        app.register_worn_gear([WornGearEntry { id: "crown".into(), ..helmet("head") }, helmet("hat")]);
        let mut manifest = app.world_mut().resource_mut::<WornGearManifest>();
        assert_eq!(manifest.get("space_helmet").map(|gear| gear.slot.as_str()), Some("hat"));
        assert!(manifest.get("crown").is_some());
        let ids = manifest.restate(["crown"], Vec::new());
        assert!(ids.is_empty());
        assert!(manifest.get("crown").is_none(), "the withdrawn gear is gone");
        assert!(manifest.get("space_helmet").is_some(), "the other ids stay");
    }
}
