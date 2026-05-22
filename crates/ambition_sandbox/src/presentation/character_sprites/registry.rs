//! Runtime-loaded sprite-sheet metadata, deserialized from per-sheet
//! `*_spritesheet.ron` manifests emitted by the procedural sprite
//! generators.
//!
//! # Why this exists
//!
//! Historically every sprite-sheet's dimensions, foot anchor, and
//! animation row layout were hand-typed into [`super::sheets`] as
//! `CharacterSheetSpec` const tables. Whenever a generator was
//! re-run, those consts silently drifted from the regenerated YAML
//! manifests until something visibly misaligned in-game. The
//! 2026-05-22 "pirate floats above its collision box" report was
//! exactly this drift. See `TODO.md` ("Sprite frame dimensions read
//! from YAML at runtime…") and the `feedback-no-drop-shadows-on-sprites`
//! agent memory for the full history.
//!
//! This module is the long-term fix: drive sheet metadata from a
//! machine-readable manifest that the generator itself owns. The
//! Python `pirates/common::build_sheet` helper writes both:
//!
//!   * `*_spritesheet.yaml` — sidecar for human inspection and the
//!     existing preview / audit tooling. Unchanged.
//!   * `*_spritesheet.ron`  — canonical machine-readable manifest
//!     that this loader consumes. The shape is intentionally a
//!     direct serde mirror of [`SheetRecord`].
//!
//! Authoring-time tools that want to read the metadata (audits,
//! debuggers, atlas viewers) should prefer the YAML; runtime
//! consumers should prefer the RON via [`SheetRegistry`].
//!
//! # Iteration vs ship builds
//!
//! By default the registry reads every `*_spritesheet.ron` from
//! `crates/ambition_sandbox/assets/sprites` at startup using
//! `std::fs::read_to_string`. That keeps the iteration loop tight:
//! edit a generator → run `./regen_sprites.sh` → restart the
//! sandbox → see the new shape. No `cargo build` required.
//!
//! For ship builds (and for non-desktop platforms where the project
//! root isn't readable at runtime), enable the **`baked_sheets`**
//! Cargo feature. That swaps the loader for an `include_str!`-driven
//! variant that embeds every RON byte in the binary at compile time.
//! The baked path is currently a `todo!()` stub — see [`init_baked`]
//! for the migration recipe.

use std::collections::HashMap;

use bevy::prelude::*;
use serde::Deserialize;

/// One sprite-sheet's metadata as serialized by the generator. Field
/// names mirror the RON shape exactly; reorder cautiously.
///
/// The RON file shape is always `[SheetRecord, SheetRecord, …]` — a
/// list, even for single-target sheets. Most lists have length 1, but
/// shared PNGs (e.g. `creator_lab_props_spritesheet.png` packs 8 props
/// into one image) carry one record per sub-target, each with a
/// distinct `y_offset`. The list shape is uniform so loaders and the
/// generator emitters don't branch.
#[derive(Debug, Clone, Deserialize)]
pub struct SheetRecord {
    /// Unique target id (matches the generator's `TARGET_NAME`, the
    /// YAML's `target` field, and the PNG filename root). Use this as
    /// the key when looking up a sheet.
    pub target: String,
    /// PNG filename, relative to the sprites asset dir. May be shared
    /// across multiple records when several targets pack onto the same
    /// sheet image (in which case `y_offset` selects each target's row
    /// band).
    pub image: String,
    pub label_width: u32,
    pub frame_width: u32,
    pub frame_height: u32,
    /// Pixel offset from the top of the shared sheet PNG before this
    /// target's first row. `0` for sheets whose row 0 starts at the
    /// top of the image (the common case). Lab-prop entries on the
    /// shared `creator_lab_props_spritesheet.png` set this to
    /// `prop_index * frame_height` so each prop addresses its own row
    /// band of the packed image.
    #[serde(default)]
    pub y_offset: u32,
    /// Derived geometry the generator computed from the rendered art:
    /// alpha-bbox of the body, foot pixel, and the normalized foot
    /// anchor (`feet_anchor_norm.y` is what
    /// `CharacterSheetSpec::feet_anchor_y` mirrors today).
    #[serde(default)]
    pub body_metrics: Option<BodyMetrics>,
    pub rows: Vec<SheetRow>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct BodyMetrics {
    #[serde(default)]
    pub body_pixel_bbox: Option<PixelRect>,
    #[serde(default)]
    pub feet_pixel: Option<PixelPoint>,
    #[serde(default)]
    pub feet_anchor_norm: Option<NormPoint>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct PixelRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct PixelPoint {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct NormPoint {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SheetRow {
    pub animation: String,
    pub row_index: u32,
    pub frame_count: u32,
    pub duration_ms: u32,
    pub duration_secs: f32,
    #[serde(default)]
    pub rects: Vec<FrameRect>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FrameRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    /// Per-frame named anchors emitted by `frame_meta_fn` (e.g.
    /// `hand_anchor`, `muzzle_anchor`). Generators that don't use
    /// `frame_meta_fn` leave this empty.
    #[serde(default)]
    pub anchors: HashMap<String, NormPoint>,
}

/// Resource looked up by sprite target id. Populated at startup by
/// [`init_sheet_registry`].
#[derive(Resource, Debug, Default)]
pub struct SheetRegistry {
    sheets: HashMap<String, SheetRecord>,
}

impl SheetRegistry {
    pub fn get(&self, target: &str) -> Option<&SheetRecord> {
        self.sheets.get(target)
    }

    pub fn len(&self) -> usize {
        self.sheets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sheets.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &SheetRecord)> {
        self.sheets.iter().map(|(k, v)| (k.as_str(), v))
    }
}

/// Bevy plugin that installs the registry resource and a Startup
/// system to populate it.
pub struct SheetRegistryPlugin;

impl Plugin for SheetRegistryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SheetRegistry>()
            .add_systems(Startup, init_sheet_registry);
    }
}

#[cfg(not(feature = "baked_sheets"))]
fn init_sheet_registry(mut registry: ResMut<SheetRegistry>) {
    init_runtime(&mut registry);
}

#[cfg(feature = "baked_sheets")]
fn init_sheet_registry(mut registry: ResMut<SheetRegistry>) {
    init_baked(&mut registry);
}

/// Default loader: read every `*_spritesheet.ron` from
/// `<manifest>/assets/sprites/` at startup. Native-only — wasm and
/// android builds should enable the `baked_sheets` Cargo feature.
///
/// Each file is a list `[SheetRecord, …]`. Most lists are length 1;
/// shared-PNG sheets (lab props) carry multiple records, one per
/// sub-target.
#[cfg(not(feature = "baked_sheets"))]
fn init_runtime(registry: &mut SheetRegistry) {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/sprites");
    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(err) => {
            warn!(
                "SheetRegistry: cannot read {}: {err}; registry will be empty",
                dir.display()
            );
            return;
        }
    };

    let mut loaded = 0usize;
    let mut failed: Vec<(String, String)> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !name.ends_with("_spritesheet.ron") {
            continue;
        }
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(err) => {
                failed.push((name.to_owned(), err.to_string()));
                continue;
            }
        };
        match ron::from_str::<Vec<SheetRecord>>(&text) {
            Ok(records) => {
                for record in records {
                    registry.sheets.insert(record.target.clone(), record);
                    loaded += 1;
                }
            }
            Err(err) => {
                failed.push((name.to_owned(), err.to_string()));
            }
        }
    }

    info!(
        "SheetRegistry: loaded {loaded} sheets from {} ({} failed)",
        dir.display(),
        failed.len()
    );
    for (file, err) in failed {
        warn!("SheetRegistry: failed to load {file}: {err}");
    }
}

/// Compile-time-baked loader. Gated behind the `baked_sheets` Cargo
/// feature so iteration builds don't pay the rebuild cost when a
/// sprite RON changes.
///
/// To fill this in:
///
/// ```ignore
/// fn init_baked(registry: &mut SheetRegistry) {
///     macro_rules! load {
///         ($name:literal) => {{
///             let text = include_str!(concat!(
///                 "../../../assets/sprites/",
///                 $name,
///                 "_spritesheet.ron",
///             ));
///             let records: Vec<SheetRecord> = ron::from_str(text)
///                 .expect(concat!("baked: ", $name, " parse"));
///             for record in records {
///                 registry.sheets.insert(record.target.clone(), record);
///             }
///         }};
///     }
///     load!("pirate_admiral");
///     load!("pirate_raider");
///     /* …one per sheet filename, not per target id… */
/// }
/// ```
///
/// Sketch only; the actual list of targets should match the runtime
/// loader's directory enumeration. If we ever profile and find the
/// startup parse expensive enough to matter, the next step beyond
/// this is a `build.rs` that emits a `static SHEETS: &[(&str,
/// SheetRecord)] = &[...]` table directly.
#[cfg(feature = "baked_sheets")]
fn init_baked(_registry: &mut SheetRegistry) {
    todo!(
        "baked_sheets: replace this with the include_str! pattern in the doc above. \
         The runtime loader in this file should be the reference for what to deserialize."
    );
}
