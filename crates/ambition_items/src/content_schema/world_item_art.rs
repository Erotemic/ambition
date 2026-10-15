//! The `world_item_art` schema: how each walk-into world item is drawn, as
//! data in a game's pack.
//!
//! A row is keyed by the `sprite` id a `WorldItem` carries (Mary-O's star
//! wand, her cinder beacon, the quasar) and names the picture and its size in
//! the world. The render layer resolves the rows into images at startup, so
//! an edit here is drawn at the next start with no Rust build.

use std::collections::BTreeMap;

use ambition_content_pack::{lowered_rules, rules_schema, PreparedContentPack, SchemaRegistration};
use serde::Deserialize;

/// The authored FILE kind.
pub const WORLD_ITEM_ART_SCHEMA: &str = "world_item_art";

/// One world item's picture.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoredWorldItemArt {
    /// The image, as the asset server names it (`sprites/props/axe.png`).
    pub path: String,
    /// The size it is drawn at, in world units (width, height).
    pub size: (f32, f32),
}

/// A game's world item art, by `sprite` id.
pub type WorldItemArtRows = BTreeMap<String, AuthoredWorldItemArt>;

/// Each row the renderer cannot draw, named by its id and field.
fn problems(rows: &WorldItemArtRows) -> Vec<String> {
    let mut problems = Vec::new();
    for (id, art) in rows {
        if art.path.trim().is_empty() {
            problems.push(format!("`{id}.path` is empty"));
        }
        let (width, height) = art.size;
        if !(width.is_finite() && width > 0.0 && height.is_finite() && height > 0.0) {
            problems.push(format!("`{id}.size` is {:?}; each side is finite and more than 0", art.size));
        }
    }
    problems
}

/// The schema.
pub fn world_item_art_schema() -> SchemaRegistration {
    rules_schema::<WorldItemArtRows>(
        WORLD_ITEM_ART_SCHEMA,
        1,
        super::ITEMS_CAPABILITY,
        "How each walk-into world item is drawn, by the sprite id it carries: \
         the picture and its size in the world.",
        problems,
    )
}

/// The rows the pack states, or `None` when it has no `world_item_art`
/// source.
pub fn lowered_world_item_art(pack: &PreparedContentPack) -> Option<&WorldItemArtRows> {
    lowered_rules::<WorldItemArtRows>(pack, WORLD_ITEM_ART_SCHEMA)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_row_the_renderer_cannot_draw_is_named_by_its_id_and_field() {
        let row = |path: &str, size| AuthoredWorldItemArt { path: path.into(), size };
        let good = WorldItemArtRows::from([("star_wand".to_string(), row("sprites/props/wand.png", (24.0, 31.0)))]);
        assert!(problems(&good).is_empty(), "{:?}", problems(&good));
        let bad = WorldItemArtRows::from([
            ("flat".to_string(), row("sprites/props/flat.png", (24.0, 0.0))),
            ("blank".to_string(), row(" ", (24.0, 31.0))),
        ]);
        let problems = problems(&bad);
        assert!(problems.iter().any(|problem| problem.starts_with("`flat.size`")), "{problems:?}");
        assert!(problems.iter().any(|problem| problem.starts_with("`blank.path` is empty")), "{problems:?}");
        assert_eq!(problems.len(), 2, "{problems:?}");
    }
}
