//! The `wielded_item_art` schema: how an item is drawn over the hand of the
//! body that wields it, as data in a game's pack.
//!
//! A hostile body's item is drawn by this table and not by the in-hand one
//! (`held_item_art`), so an item a hostile carries needs a row here. A row
//! names the picture, the part of it to draw, the pixel the hand holds, the
//! width as a part of the wielder's height, the depth over the wielder and
//! the debug name of the drawn entity.

use std::collections::BTreeMap;

use ambition_content_pack::{lowered_rules, rules_schema, PreparedContentPack, SchemaRegistration};
use serde::Deserialize;

/// The authored FILE kind.
pub const WIELDED_ITEM_ART_SCHEMA: &str = "wielded_item_art";

/// One wielded item's picture.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoredWieldedItemArt {
    /// The image, as the asset server names it.
    pub path: String,
    /// The part of the image to draw, in its pixels: the corner and the size.
    pub source: SourceRect,
    /// The pixel of that part the hand holds (x, y).
    pub grip: (f32, f32),
    /// The width it is drawn at, as a part of the wielder's height.
    pub width_per_wielder_height: f32,
    /// How far over the wielder it is drawn.
    pub z_bias: f32,
    /// The name of the drawn entity, for debug views.
    pub name: String,
}

/// A rectangle of an image, in its pixels.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// A game's wielded item art, by held item id.
pub type WieldedItemArtRows = BTreeMap<String, AuthoredWieldedItemArt>;

/// Each row the renderer cannot draw, named by its id and field.
fn problems(rows: &WieldedItemArtRows) -> Vec<String> {
    let mut problems = Vec::new();
    for (id, art) in rows {
        if art.path.trim().is_empty() {
            problems.push(format!("`{id}`: the `path` is empty"));
        }
        if art.name.trim().is_empty() {
            problems.push(format!("`{id}`: the `name` is empty; debug views read it"));
        }
        let SourceRect { x, y, w, h } = art.source;
        if ![x, y, w, h].iter().all(|v| v.is_finite()) || w <= 0.0 || h <= 0.0 {
            problems.push(format!("`{id}`: `source` is {:?}; it is finite, and its size is more than 0", art.source));
        }
        if !(art.grip.0.is_finite() && art.grip.1.is_finite()) {
            problems.push(format!("`{id}`: `grip` is {:?}; it is a finite pixel", art.grip));
        }
        if !(art.width_per_wielder_height.is_finite() && art.width_per_wielder_height > 0.0) {
            problems.push(format!(
                "`{id}`: `width_per_wielder_height` is {}; it is finite and more than 0",
                art.width_per_wielder_height
            ));
        }
        if !art.z_bias.is_finite() {
            problems.push(format!("`{id}`: `z_bias` is {}; it is finite", art.z_bias));
        }
    }
    problems
}

/// The schema.
pub fn wielded_item_art_schema() -> SchemaRegistration {
    rules_schema::<WieldedItemArtRows>(
        WIELDED_ITEM_ART_SCHEMA,
        1,
        super::ITEMS_CAPABILITY,
        "How an item is drawn over the hand of the body that wields it: the picture, \
         the part of it, the pixel the hand holds, the width by the wielder's height, \
         the depth and the debug name.",
        problems,
    )
}

/// The rows the pack states, or `None` when it has no `wielded_item_art` source.
pub fn lowered_wielded_item_art(pack: &PreparedContentPack) -> Option<&WieldedItemArtRows> {
    lowered_rules::<WieldedItemArtRows>(pack, WIELDED_ITEM_ART_SCHEMA)
}
