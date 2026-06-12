//! Runtime sprite-sheet metadata registry.
//!
//! Procedural generators emit `*_spritesheet.ron` manifests alongside the YAML
//! audit sidecars. Runtime code reads the baked RON table through [`SheetRegistry`]
//! so sprite dimensions, row layout, and body metrics stay aligned with generated
//! sheets.
//!
//! Authoring tools may keep using YAML for inspection; runtime consumers should use
//! the RON data embedded by `build.rs` through
//! [`super::baked_sheet_rons::BAKED_SHEET_RONS`]. Re-running sprite generation and
//! then building is enough to refresh the baked table for desktop, Android, wasm,
//! and other targets.

// SheetRecord / SheetRow / BodyMetrics / FrameRect / PixelRect /
// PixelPoint / NormPoint carry the full generator-emitted schema.
// Several fields are diagnostic or reserved for future consumers
// (atlas viewer, per-frame anchor probes) — silence the unused-field
// warnings at the module level so the schema stays whole.
#![allow(
    dead_code,
    reason = "deserialize surface that mirrors the on-disk RON schema; not every field is queried at runtime yet"
)]

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
    /// Per-target gameplay tuning authored alongside the sheet. When absent,
    /// callers use their Rust fallback tuning.
    #[serde(default)]
    pub tuning: Option<SheetTuningSpec>,
    pub rows: Vec<SheetRow>,
}

/// Per-target gameplay-tuning fields embedded in the spritesheet manifest.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct SheetTuningSpec {
    /// Multiplier on the actor's collision AABB when computing the
    /// rendered sprite size. `1.0` = sprite exactly fills the AABB;
    /// `2.1` (the robot's tuning) = sprite is much larger than the
    /// hitbox. Authored per-character to compensate for the fraction
    /// of each frame the actual character art occupies after
    /// auto-crop.
    pub collision_scale: f32,
    /// Inset (pixels) applied to each frame rect when sampling the
    /// atlas. `1` (the common case) trims one pixel from every edge
    /// to avoid bilinear bleed from neighboring frames. `0` for
    /// pixel-perfect sheets that don't need the inset.
    pub frame_sample_inset: u32,
}

/// Body / hurtbox metadata emitted alongside the sprite sheet.
///
/// `body_pixel_bbox` is the single overall bbox (alpha-bbox of the
/// idle/rest frame) — the common case for single-piece characters
/// (player, goblins, small bosses).
///
/// `body_pixel_parts` is the multi-rect representation for
/// **disjointed-piece characters** — giant bosses with head + body
/// + arms + legs that the gameplay code wants to address
/// individually. Each part carries a `name` so consumers can target
/// "head" vs "left_hand" by string. Defaults to empty.
///
/// `animations` carries **per-animation** hurtbox + hitbox data
/// keyed by animation name (e.g. `"floor_slam"`, `"side_sweep"`).
/// Each entry overrides the static body bbox for that animation
/// so a boss whose arms extend out only during attack frames gets
/// the right hurtbox during those frames, and so attack
/// hitboxes are positioned where the sprite author intended.
///
/// Consumer rule (hurtbox): when the current animation has a
/// `AnimationMetrics::hurtbox`, use it. Else when
/// `body_pixel_parts` is non-empty, prefer it. Else fall back to a
/// single-element list built from `body_pixel_bbox`. See
/// [`super::super::super::content::features::boss_attack_geometry::world_space_body_aabbs_from_metrics`]
/// for the canonical derivation.
#[derive(Debug, Clone, Deserialize)]
pub struct BodyMetrics {
    #[serde(default)]
    pub body_pixel_bbox: Option<PixelRect>,
    /// Multi-rect hurtbox metadata. Each entry is a named pixel
    /// rectangle in sprite-frame space. Empty = use `body_pixel_bbox`
    /// as the single body.
    #[serde(default)]
    pub body_pixel_parts: Vec<NamedPixelRect>,
    /// Per-animation hurtbox + hitbox overrides. Keyed by the same
    /// animation name the spritesheet rows use (`"rest"`,
    /// `"floor_slam"`, `"side_sweep"`, …). The renderer emits one
    /// entry per animation in the sheet; consumers look up by the
    /// boss's currently-playing animation name.
    #[serde(default)]
    pub animations: std::collections::HashMap<String, AnimationMetrics>,
    #[serde(default)]
    pub feet_pixel: Option<PixelPoint>,
    #[serde(default)]
    pub feet_anchor_norm: Option<NormPoint>,
}

/// Per-animation authored / derived hit + hurt box data. The
/// renderer fills `hurtbox` from each animation's alpha-bbox by
/// default; adapters declare `hitbox` rectangles explicitly for
/// each attack animation. Either may be `None` (meaning "fall
/// back to the static `body_pixel_bbox`" or "this animation has
/// no attack hitbox").
#[derive(Debug, Clone, Deserialize, Default)]
pub struct AnimationMetrics {
    /// Optional frame duration for sampling `AnimationBox::frames`.
    /// Uses the same units as `SheetRow::duration_secs`. Generators
    /// only need to fill this when they emit per-frame gameplay boxes.
    #[serde(default)]
    pub frame_duration_secs: Option<f32>,
    /// Hurtbox for this animation (where the *player's* attacks
    /// register hits on this actor). Multi-rect if the sprite has
    /// disjoint body parts; single-rect via `bbox` for simple
    /// bodies. `None` = fall back to `BodyMetrics::body_pixel_bbox`.
    #[serde(default)]
    pub hurtbox: Option<AnimationBox>,
    /// Hitbox for this animation (where *this actor's* attack
    /// damages the player). Non-attack animations leave this `None`.
    /// Attack-flavored animations (`floor_slam`, `side_sweep`,
    /// `spike_halo`, etc.) author one or more rects.
    #[serde(default)]
    pub hitbox: Option<AnimationBox>,
}

/// One animation's hit-or-hurt box, expressed as multi-rect parts
/// + an optional fallback single bbox. Mirrors the
/// `body_pixel_parts` / `body_pixel_bbox` split.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct AnimationBox {
    /// Multi-rect representation. Use `parts` when the sprite has
    /// disjoint pieces (head + arms + body). Empty = fall back to
    /// `bbox`.
    #[serde(default)]
    pub parts: Vec<NamedPixelRect>,
    /// Single-rect fallback. Most attack hitboxes are one box;
    /// most hurtboxes derived from alpha bounds are one box.
    #[serde(default)]
    pub bbox: Option<PixelRect>,
    /// Optional per-frame boxes for animation rows whose gameplay
    /// geometry should move with the drawn pose. When populated,
    /// consumers sample this by the current animation elapsed time
    /// before falling back to the coarse per-animation `parts`/`bbox`.
    #[serde(default)]
    pub frames: Vec<AnimationBoxFrame>,
}

impl AnimationBox {
    /// True iff this box has at least one rectangle (either parts,
    /// bbox, or per-frame data populated). Used by consumers as the
    /// "should I use this or fall back?" gate.
    pub fn is_populated(&self) -> bool {
        !self.parts.is_empty()
            || self.bbox.is_some()
            || self.frames.iter().any(AnimationBoxFrame::is_populated)
    }
}

/// One sampled frame of an [`AnimationBox`]. Same rectangle shape as
/// the coarse box, but indexed by animation time. This is intentionally
/// optional so old manifests keep deserializing unchanged.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct AnimationBoxFrame {
    #[serde(default)]
    pub parts: Vec<NamedPixelRect>,
    #[serde(default)]
    pub bbox: Option<PixelRect>,
}

impl AnimationBoxFrame {
    pub fn is_populated(&self) -> bool {
        !self.parts.is_empty() || self.bbox.is_some()
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
pub struct PixelRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// A named pixel rectangle in sprite-frame space, used for
/// multi-part body / hurtbox metadata. The `name` lets gameplay
/// code address parts individually (`head`, `body`, `left_hand`,
/// `right_hand`, …). For single-piece characters, leave
/// `body_pixel_parts` empty and use `body_pixel_bbox` instead.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct NamedPixelRect {
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl NamedPixelRect {
    pub fn rect(&self) -> PixelRect {
        PixelRect {
            x: self.x,
            y: self.y,
            w: self.w,
            h: self.h,
        }
    }
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

    /// Look up the body metrics + frame size for a sprite target.
    /// Used by gameplay code (boss combat_size derivation, hurtbox
    /// math) so the sprite RON is the single source of truth for
    /// where the visible body sits inside the frame.
    ///
    /// Returns `(metrics, frame_width, frame_height)` when the
    /// target exists *and* has body_metrics; `None` otherwise.
    pub fn body_metrics(&self, target: &str) -> Option<(&BodyMetrics, u32, u32)> {
        let record = self.sheets.get(target)?;
        let metrics = record.body_metrics.as_ref()?;
        Some((metrics, record.frame_width, record.frame_height))
    }

    /// Build a fully-populated registry from the compile-time baked
    /// sheet RONs with no Bevy `App` / `Startup` schedule. Lets headless
    /// tools and tests obtain sprite metrics (boss hurtbox math, the
    /// geometry-debug renderer) without spinning up `SheetRegistryPlugin`.
    pub fn from_baked() -> Self {
        let mut registry = Self::default();
        init_from_baked(&mut registry);
        registry
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

fn init_sheet_registry(mut registry: ResMut<SheetRegistry>) {
    *registry = SheetRegistry::from_baked();
}

/// Build the runtime `SheetRegistry` from baked `*_spritesheet.ron` text.
/// Most files are a length-1 list; shared-PNG sheets (lab props)
/// carry multiple records, one per sub-target.
fn init_from_baked(registry: &mut SheetRegistry) {
    let mut loaded = 0usize;
    let mut failed: Vec<(String, String)> = Vec::new();
    for (filename_root, text) in super::baked_sheet_rons::BAKED_SHEET_RONS {
        match ron::from_str::<Vec<SheetRecord>>(text) {
            Ok(records) => {
                for record in records {
                    registry.sheets.insert(record.target.clone(), record);
                    loaded += 1;
                }
            }
            Err(err) => {
                failed.push(((*filename_root).to_owned(), err.to_string()));
            }
        }
    }

    info!(
        "SheetRegistry: loaded {loaded} sheets from baked table ({} failed)",
        failed.len()
    );
    for (file, err) in failed {
        warn!("SheetRegistry: failed to parse baked {file}: {err}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Python renderer emits `body_metrics.animations` as a
    /// map keyed by animation name. This test pins that the
    /// Rust deserializer reads it back — regressing this would
    /// silently fall back to the legacy `body_pixel_bbox`
    /// (cyan box stays at idle-pose size during attacks).
    #[test]
    fn body_metrics_animations_round_trip_from_renderer_emit() {
        // Matches the shape emitted by
        // `_ron_anim_metrics_map` in `sheet.py` for the boss.
        let ron_text = r#"
        (
            body_pixel_bbox: Some((x: 8, y: 5, w: 106, h: 83)),
            feet_pixel: Some((x: 60.5, y: 87.0)),
            feet_anchor_norm: Some((x: -0.02734375, y: -0.1796875)),
            animations: {
                "rest": (hurtbox: Some((bbox: Some((x: 8, y: 4, w: 106, h: 84))))),
                "floor_slam": (
                    hurtbox: Some((bbox: Some((x: 5, y: 0, w: 111, h: 110)))),
                    hitbox: Some((bbox: Some((x: 4, y: 88, w: 120, h: 30))))
                ),
                "side_sweep": (
                    hurtbox: Some((bbox: Some((x: 1, y: 5, w: 127, h: 86)))),
                    hitbox: Some((parts: [
                        (name: "left", x: 0, y: 40, w: 32, h: 50),
                        (name: "right", x: 96, y: 40, w: 32, h: 50)
                    ]))
                )
            }
        )
        "#;
        let metrics: BodyMetrics = ron::from_str(ron_text)
            .expect("BodyMetrics should deserialize from renderer-emitted RON");

        assert_eq!(metrics.animations.len(), 3);
        let rest = metrics.animations.get("rest").expect("`rest` present");
        let rest_hurt = rest.hurtbox.as_ref().expect("`rest` hurtbox");
        assert!(rest_hurt.bbox.is_some(), "rest hurtbox has bbox");
        assert!(rest.hitbox.is_none(), "rest has no hitbox (idle pose)");

        let floor = metrics
            .animations
            .get("floor_slam")
            .expect("`floor_slam` present");
        let floor_hit = floor.hitbox.as_ref().expect("`floor_slam` hitbox");
        let bbox = floor_hit.bbox.expect("floor_slam hitbox bbox");
        assert_eq!(bbox.w, 120);
        assert_eq!(bbox.h, 30);

        let sweep = metrics
            .animations
            .get("side_sweep")
            .expect("`side_sweep` present");
        let sweep_hit = sweep.hitbox.as_ref().expect("`side_sweep` hitbox");
        assert_eq!(
            sweep_hit.parts.len(),
            2,
            "side_sweep has left + right parts"
        );
        assert_eq!(sweep_hit.parts[0].name, "left");
        assert_eq!(sweep_hit.parts[1].name, "right");
    }

    /// Verify the actual on-disk boss sheet RON parses. If the
    /// Python renderer + Rust schema ever drift this test catches
    /// it on the spot rather than at runtime via a silent
    /// "animations: empty" fallback.
    #[test]
    fn live_boss_spritesheet_ron_round_trips() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("assets/sprites/boss_spritesheet.ron");
        if !path.exists() {
            // Sprites are gitignored; if a clean checkout hasn't
            // regenerated yet, skip rather than fail.
            return;
        }
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let records: Vec<SheetRecord> =
            ron::from_str(&text).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));
        let record = records
            .into_iter()
            .find(|r| r.target == "boss")
            .expect("boss record");
        let metrics = record.body_metrics.expect("body_metrics");
        assert!(
            !metrics.animations.is_empty(),
            "expected per-animation metadata in boss_spritesheet.ron — \
             check that the Python renderer emitted `animations:` and that \
             this test is reading the regenerated file"
        );
        // Spot-check the floor_slam hitbox (adapter-declared) so a
        // future renderer change that drops author-declared hitboxes
        // trips this guard.
        let floor_slam = metrics
            .animations
            .get("floor_slam")
            .expect("floor_slam animation present");
        assert!(
            floor_slam.hitbox.is_some(),
            "floor_slam should have an authored hitbox (boss adapter declares it)"
        );
        // The boss hurtbox is split into head + body parts so the
        // player must aim at the central body (not extended arms).
        // Pin both parts come through so a renderer regression that
        // drops `hurtbox_parts` reverts to the loose single-bbox
        // alpha hurtbox.
        let rest = metrics.animations.get("rest").expect("rest animation");
        let rest_hurt = rest.hurtbox.as_ref().expect("rest hurtbox");
        assert!(
            !rest_hurt.parts.is_empty(),
            "rest hurtbox must be the multi-part head + body override (parts empty implies the adapter's hurtbox_parts was lost)"
        );
        let part_names: Vec<&str> = rest_hurt.parts.iter().map(|p| p.name.as_str()).collect();
        assert!(
            part_names.contains(&"head") && part_names.contains(&"body"),
            "rest hurtbox parts must include 'head' and 'body'; got {part_names:?}"
        );
        // SideSweep should also have head + body hurtbox parts (not
        // a single bbox that would include the extended arms).
        let sweep = metrics
            .animations
            .get("side_sweep")
            .expect("side_sweep animation");
        let sweep_hurt = sweep.hurtbox.as_ref().expect("side_sweep hurtbox");
        assert!(
            sweep_hurt.parts.len() >= 2,
            "side_sweep hurtbox must be multi-part; got {} parts",
            sweep_hurt.parts.len()
        );
    }
}
