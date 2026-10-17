//! The `prop_sheets` schema: the prop sheets a game draws, as data in its
//! pack. Also `platform_looks`: how a moving platform drawn as one of those
//! sheets is laid on its platform.
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
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoredPropSheet {
    /// The prop's box is multiplied by this to give the size it is drawn at.
    #[serde(default = "one")]
    pub scale: f32,
    /// Pixels trimmed from each edge of a frame when it is sampled.
    #[serde(default)]
    pub frame_sample_inset: u32,
    /// The sheet record the prop draws (its target). `None`: the record of
    /// the prop's own kind. A record can share its page image with others
    /// (the creator lab's props are one sheet).
    #[serde(default)]
    pub sheet: Option<String>,
    /// Draw from the shared sprite pack of the sheet first, at the quality
    /// tier of the game, and from the sheet's own page when the pack does
    /// not have it.
    #[serde(default)]
    pub packed: bool,
}

fn one() -> f32 {
    1.0
}

impl AuthoredPropSheet {
    /// The tuning the sheet loader reads.
    pub fn tuning(&self) -> SheetTuning {
        SheetTuning::new(self.scale, self.frame_sample_inset)
    }

    /// The sheet record the prop of `kind` draws.
    pub fn target<'a>(&'a self, kind: &'a str) -> &'a str {
        self.sheet.as_deref().unwrap_or(kind)
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
        if row.sheet.as_deref().is_some_and(|sheet| sheet.trim().is_empty() || sheet.contains('/')) {
            problems.push(format!("`{kind}.sheet` is {:?}; it is a sheet target, a name with no `/`", row.sheet));
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

/// The authored FILE kind of the platform looks.
pub const PLATFORM_LOOKS_SCHEMA: &str = "platform_looks";

/// How a moving platform whose `visual` is a prop kind is drawn on it: the
/// row it loops, the span of the frame (in frame pixels) its width covers,
/// and the frame row (pixels from the top) its top surface is. The
/// Mockingbird's sharks: the back of the shark is the platform, so a body
/// stands on the saddle, not on the fin.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformLook {
    /// The prop sheet this look draws, when it is not the sheet of its own
    /// kind: a kind that is one part of the picture of another (the ramp of
    /// the cart is a row of the sheet of the cart).
    #[serde(default)]
    pub sheet: Option<String>,
    pub row: String,
    /// The frame `span_px` and `top_px` were measured in: the frame of the
    /// full-size sheet. A reduced copy of the sheet (a lower quality tier, a
    /// pack) has a smaller frame, and the platform is drawn at the same size
    /// from it. Measured 2026-10-10: the placement was read against the frame
    /// of the copy in use, so a half-size sheet was drawn at twice the size.
    pub frame_px: [f32; 2],
    pub span_px: [f32; 2],
    pub top_px: f32,
    /// The sheet is drawn facing right (it is flipped to fly left).
    #[serde(default = "yes")]
    pub faces_right: bool,
    /// A second row of the same frame, drawn in front of each body: the near
    /// side of a thing a body stands IN. The cart of the intro: its near rail
    /// is this row, so a body on its bed is in the cart and not on it.
    #[serde(default)]
    pub front_row: Option<String>,
}

fn yes() -> bool {
    true
}

/// A game's platform looks, by the platform's `visual` kind.
pub type PlatformLookRows = BTreeMap<String, PlatformLook>;

/// Each look the renderer cannot place, named by its kind and field.
#[cfg(feature = "content_pack")]
fn look_problems(rows: &PlatformLookRows) -> Vec<String> {
    let mut problems = Vec::new();
    for (kind, look) in rows {
        let [width, height] = look.frame_px;
        if !(width.is_finite() && width > 0.0 && height.is_finite() && height > 0.0) {
            problems.push(format!("`{kind}.frame_px` is {:?}; each side is finite and more than 0", look.frame_px));
            continue;
        }
        let [left, right] = look.span_px;
        if !(0.0 <= left && left < right && right <= width) {
            problems.push(format!(
                "`{kind}.span_px` is {:?}; it is a part of the frame's width {width}, left to right",
                look.span_px
            ));
        }
        if !(0.0 <= look.top_px && look.top_px <= height) {
            problems.push(format!("`{kind}.top_px` is {}; it is a row of the frame's height {height}", look.top_px));
        }
        for (field, row) in [("row", Some(&look.row)), ("front_row", look.front_row.as_ref())] {
            if row.is_some_and(|row| row.trim().is_empty()) {
                problems.push(format!("`{kind}.{field}` is empty; it names a row of the sheet"));
            }
        }
    }
    problems
}

/// The platform looks schema.
#[cfg(feature = "content_pack")]
pub fn platform_looks_schema() -> ambition_content_pack::SchemaRegistration {
    ambition_content_pack::rules_schema::<PlatformLookRows>(
        PLATFORM_LOOKS_SCHEMA,
        1,
        SPRITES_CAPABILITY,
        "How a moving platform drawn as a prop sheet is laid on its platform: \
         the row it loops, the span of the frame its width covers and the row \
         its top surface is, in frame pixels.",
        look_problems,
    )
}

/// The looks the pack states, or `None` when it has no `platform_looks`
/// source.
#[cfg(feature = "content_pack")]
pub fn lowered_platform_looks(pack: &ambition_content_pack::PreparedContentPack) -> Option<&PlatformLookRows> {
    ambition_content_pack::lowered_rules::<PlatformLookRows>(pack, PLATFORM_LOOKS_SCHEMA)
}

#[cfg(all(test, feature = "content_pack"))]
mod tests {
    use super::*;

    #[test]
    fn a_row_the_loader_cannot_draw_is_named_by_its_kind_and_field() {
        let row = |scale| AuthoredPropSheet { scale, frame_sample_inset: 0, sheet: None, packed: false };
        let good = PropSheetRows::from([("coconut_palm".to_string(), row(1.4))]);
        assert!(problems(&good).is_empty(), "{:?}", problems(&good));
        let bad = PropSheetRows::from([
            ("coconut_palm".to_string(), row(0.0)),
            ("props/palm".to_string(), row(1.0)),
            ("lab_vat".to_string(), AuthoredPropSheet { sheet: Some("lab/vat".into()), ..row(1.0) }),
        ]);
        let problems = problems(&bad);
        assert!(problems.iter().any(|problem| problem.starts_with("`coconut_palm.scale` is 0")), "{problems:?}");
        assert!(problems.iter().any(|problem| problem.starts_with("`props/palm` is not a sheet target")), "{problems:?}");
        assert!(problems.iter().any(|problem| problem.starts_with("`lab_vat.sheet`")), "{problems:?}");
        assert_eq!(problems.len(), 3, "{problems:?}");
    }

    #[test]
    fn a_look_the_renderer_cannot_place_is_named_by_its_kind_and_field() {
        let look = |span_px: [f32; 2], top_px: f32| PlatformLook {
            sheet: None,
            row: "idle".into(),
            frame_px: [192.0, 72.0],
            span_px,
            top_px,
            faces_right: true,
            front_row: None,
        };
        let good = PlatformLookRows::from([("lift".to_string(), look([0.0, 192.0], 4.0))]);
        assert!(look_problems(&good).is_empty(), "{:?}", look_problems(&good));
        let bad = PlatformLookRows::from([
            ("wide".to_string(), look([0.0, 300.0], 4.0)),
            ("low".to_string(), look([0.0, 192.0], 80.0)),
        ]);
        let problems = look_problems(&bad);
        assert!(problems.iter().any(|problem| problem.starts_with("`wide.span_px`")), "{problems:?}");
        assert!(problems.iter().any(|problem| problem.starts_with("`low.top_px` is 80")), "{problems:?}");
        assert_eq!(problems.len(), 2, "{problems:?}");
    }
}
