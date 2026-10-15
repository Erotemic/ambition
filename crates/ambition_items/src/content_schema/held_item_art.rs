//! The `held_item_art` schema: how each held item is drawn on the ground and
//! in the hand, as data in a game's pack.
//!
//! A row is keyed by the held item's id (`axe`, `gun_sword`) and names the
//! picture, its size in the world and where the hand holds it. The render
//! layer resolves the rows into images at startup, so an edit here is drawn at
//! the next start with no Rust build.

use std::collections::BTreeMap;

use ambition_content_pack::{lowered_rules, rules_schema, PreparedContentPack, SchemaRegistration};
use serde::Deserialize;

/// The authored FILE kind.
pub const HELD_ITEM_ART_SCHEMA: &str = "held_item_art";

/// One held item's picture.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoredHeldItemArt {
    /// The image, as the asset server names it (`sprites/props/axe.png`).
    pub path: String,
    /// The size it is drawn at, in world units (width, height).
    pub size: (f32, f32),
    /// Where the hand holds it along its picture, from 0 (its near end) to 1
    /// (its far end). Absent: it is held by its middle. An item with a grip
    /// points out of the hand, and turns about the grip when it is swung.
    #[serde(default)]
    pub grip: Option<f32>,
}

/// A game's held item art, by held item id.
pub type HeldItemArtRows = BTreeMap<String, AuthoredHeldItemArt>;

/// Each row the renderer cannot draw, named by its id and field.
fn problems(rows: &HeldItemArtRows) -> Vec<String> {
    let mut problems = Vec::new();
    for (id, art) in rows {
        if art.path.trim().is_empty() {
            problems.push(format!("`{id}`: the `path` is empty"));
        }
        let (width, height) = art.size;
        if !(width.is_finite() && width > 0.0 && height.is_finite() && height > 0.0) {
            problems.push(format!("`{id}`: `size` is {:?}; each side is finite and more than 0", art.size));
        }
        if let Some(grip) = art.grip.filter(|grip| !(0.0..=1.0).contains(grip)) {
            problems.push(format!("`{id}`: `grip` is {grip}; it is a part of the picture's length, from 0 to 1"));
        }
    }
    problems
}

/// The schema.
pub fn held_item_art_schema() -> SchemaRegistration {
    rules_schema::<HeldItemArtRows>(
        HELD_ITEM_ART_SCHEMA,
        1,
        super::ITEMS_CAPABILITY,
        "How each held item is drawn on the ground and in the hand: the picture, \
         its size in the world and where the hand holds it.",
        problems,
    )
}

/// The rows the pack states, or `None` when it has no `held_item_art` source.
pub fn lowered_held_item_art(pack: &PreparedContentPack) -> Option<&HeldItemArtRows> {
    lowered_rules::<HeldItemArtRows>(pack, HELD_ITEM_ART_SCHEMA)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(size: (f32, f32), grip: Option<f32>) -> AuthoredHeldItemArt {
        AuthoredHeldItemArt { path: "sprites/props/axe.png".into(), size, grip }
    }

    #[test]
    fn a_row_the_renderer_cannot_draw_is_named_by_its_id_and_field() {
        let good = HeldItemArtRows::from([("axe".to_string(), row((40.0, 18.0), Some(0.2)))]);
        assert!(problems(&good).is_empty(), "{:?}", problems(&good));
        let flat = HeldItemArtRows::from([("axe".to_string(), row((40.0, 0.0), None))]);
        assert!(problems(&flat).iter().any(|problem| problem.contains("`axe`: `size`")), "{:?}", problems(&flat));
        let past_the_end = HeldItemArtRows::from([("axe".to_string(), row((40.0, 18.0), Some(1.5)))]);
        assert!(problems(&past_the_end).iter().any(|problem| problem.contains("`axe`: `grip`")), "{:?}", problems(&past_the_end));
    }
}
