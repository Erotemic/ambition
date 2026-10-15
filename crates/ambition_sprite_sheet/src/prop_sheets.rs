//! The `prop_sheets` schema: the prop sheets a game draws, as data in its
//! pack.
//!
//! A row is keyed by the `kind` a room's prop names, which is also the sheet
//! target: the row loads `<sprite folder>/<kind>_spritesheet.png` with the
//! rest of the art. The loader is the actor layer's `PropSheets`; this crate
//! owns only the row and its check, so an edit to the file is drawn at the
//! next start with no Rust build.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::character::SheetTuning;

/// The authored FILE kind.
pub const PROP_SHEETS_SCHEMA: &str = "prop_sheets";

/// The capability that owns the schema.
pub const SPRITES_CAPABILITY: &str = "sprites";

/// How one prop kind's sheet is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoredPropSheet {
    /// The prop's box is multiplied by this to give the size it is drawn at.
    #[serde(default = "one")]
    pub scale: f32,
    /// Pixels trimmed from each edge of a frame when it is sampled.
    #[serde(default)]
    pub frame_sample_inset: u32,
}

fn one() -> f32 {
    1.0
}

impl AuthoredPropSheet {
    /// The tuning the sheet loader reads.
    pub fn tuning(&self) -> SheetTuning {
        SheetTuning::new(self.scale, self.frame_sample_inset)
    }
}

/// A game's prop sheets, by prop kind.
pub type PropSheetRows = BTreeMap<String, AuthoredPropSheet>;

/// Each row the loader cannot draw, named by its kind and field.
#[cfg(feature = "content_pack")]
fn problems(rows: &PropSheetRows) -> Vec<String> {
    let mut problems = Vec::new();
    for (kind, row) in rows {
        if kind.trim().is_empty() || kind.contains('/') {
            problems.push(format!("`{kind}` is not a sheet target; it is a name with no `/`"));
        }
        if !(row.scale.is_finite() && row.scale > 0.0) {
            problems.push(format!("`{kind}.scale` is {}; it is finite and more than 0", row.scale));
        }
    }
    problems
}

/// The schema.
#[cfg(feature = "content_pack")]
pub fn prop_sheets_schema() -> ambition_content_pack::SchemaRegistration {
    ambition_content_pack::rules_schema::<PropSheetRows>(
        PROP_SHEETS_SCHEMA,
        1,
        SPRITES_CAPABILITY,
        "The prop sheets a game draws, by the kind a room's prop names: the \
         size the sheet is drawn at and the pixels trimmed from each frame.",
        problems,
    )
}

/// The rows the pack states, or `None` when it has no `prop_sheets` source.
#[cfg(feature = "content_pack")]
pub fn lowered_prop_sheets(pack: &ambition_content_pack::PreparedContentPack) -> Option<&PropSheetRows> {
    ambition_content_pack::lowered_rules::<PropSheetRows>(pack, PROP_SHEETS_SCHEMA)
}

#[cfg(all(test, feature = "content_pack"))]
mod tests {
    use super::*;

    #[test]
    fn a_row_the_loader_cannot_draw_is_named_by_its_kind_and_field() {
        let row = |scale| AuthoredPropSheet { scale, frame_sample_inset: 0 };
        let good = PropSheetRows::from([("coconut_palm".to_string(), row(1.4))]);
        assert!(problems(&good).is_empty(), "{:?}", problems(&good));
        let bad = PropSheetRows::from([
            ("coconut_palm".to_string(), row(0.0)),
            ("props/palm".to_string(), row(1.0)),
        ]);
        let problems = problems(&bad);
        assert!(problems.iter().any(|problem| problem.starts_with("`coconut_palm.scale` is 0")), "{problems:?}");
        assert!(problems.iter().any(|problem| problem.starts_with("`props/palm` is not a sheet target")), "{problems:?}");
        assert_eq!(problems.len(), 2, "{problems:?}");
    }
}
