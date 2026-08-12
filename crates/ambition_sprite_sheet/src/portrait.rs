//! Runtime vocabulary for separately published dialogue portrait sheets.
//!
//! Portrait sheets are presentation products rather than gameplay animation
//! sheets: they have named clips, a required default clip, and no collision or
//! actor geometry. The authoring implementation that produced the raster is
//! intentionally outside this schema.

use bevy::prelude::{App, Plugin, ResMut, Resource, Startup};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use tracing::{info, warn};

/// One rectangular portrait frame within the portrait image page.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct PortraitFrameRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// Named static or animated portrait clip.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct PortraitClipRecord {
    /// Milliseconds per frame. Static one-frame clips normally use zero.
    #[serde(default)]
    pub duration_ms: u32,
    /// Whether playback wraps after the final frame.
    #[serde(default)]
    pub looping: bool,
    pub frames: Vec<PortraitFrameRect>,
}

/// Manifest emitted next to `<target>_portraits.png`.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct PortraitSheetManifest {
    pub target: String,
    pub image: String,
    pub frame_width: u32,
    pub frame_height: u32,
    pub default_clip: String,
    pub clips: BTreeMap<String, PortraitClipRecord>,
}

impl PortraitSheetManifest {
    /// Validate the structural contract shared by authoring and runtime code.
    pub fn validate(&self) -> Result<(), String> {
        if self.target.trim().is_empty() {
            return Err("portrait manifest target is empty".to_string());
        }
        if self.image.trim().is_empty() {
            return Err(format!(
                "portrait manifest '{}' has an empty image path",
                self.target
            ));
        }
        if self.frame_width == 0 || self.frame_height == 0 {
            return Err(format!(
                "portrait manifest '{}' has a zero-sized logical frame",
                self.target
            ));
        }
        let Some(default) = self.clips.get(&self.default_clip) else {
            return Err(format!(
                "portrait manifest '{}' default clip '{}' is missing",
                self.target, self.default_clip
            ));
        };
        if default.frames.is_empty() {
            return Err(format!(
                "portrait manifest '{}' default clip '{}' has no frames",
                self.target, self.default_clip
            ));
        }
        for (name, clip) in &self.clips {
            if name.trim().is_empty() {
                return Err(format!(
                    "portrait manifest '{}' has an empty clip name",
                    self.target
                ));
            }
            if clip.frames.is_empty() {
                return Err(format!(
                    "portrait manifest '{}' clip '{}' has no frames",
                    self.target, name
                ));
            }
            if clip.frames.len() > 1 && clip.duration_ms == 0 {
                return Err(format!(
                    "portrait manifest '{}' animated clip '{}' has zero duration",
                    self.target, name
                ));
            }
            if let Some(frame) = clip
                .frames
                .iter()
                .find(|frame| frame.w == 0 || frame.h == 0)
            {
                return Err(format!(
                    "portrait manifest '{}' clip '{}' has a zero-sized frame at ({}, {})",
                    self.target, name, frame.x, frame.y
                ));
            }
        }
        Ok(())
    }
}

/// Runtime index of baked portrait manifests, keyed by the same asset-relative
/// manifest path stored in character-catalog rows.
///
/// ⭐ **and by TARGET**, which every manifest has carried since it was written
/// and which this index used to read and throw away. `target: "alice"` is a
/// NAME for a portrait product, and a name is what a character definition can
/// author — the path is what the CATALOG derives. Both roads end at the same
/// manifest; only one of them was addressable.
#[derive(Resource, Clone, Debug, Default)]
pub struct PortraitSheetRegistry {
    manifests: HashMap<String, PortraitSheetManifest>,
    /// `target` → the manifest path it was indexed under. A second map rather
    /// than a second copy: the manifest itself is owned once.
    by_target: BTreeMap<String, String>,
}

impl PortraitSheetRegistry {
    pub fn from_baked_table(table: &[(&str, &str)]) -> Self {
        let mut registry = Self::default();
        let mut loaded = 0usize;
        for (asset_path, text) in table {
            match parse_portrait_manifest(text) {
                Ok(manifest) => {
                    let path = normalize_manifest_path(asset_path);
                    let target = manifest.target.trim().to_string();
                    if !target.is_empty() {
                        // ⚠ REFUSED rather than last-writer-wins, exactly like
                        // `AuthoredSheets::insert_ron`. Two manifests claiming
                        // one target is an authoring mistake whose symptom
                        // would be a character wearing somebody else's face on
                        // some runs and not others, depending on bake order.
                        if let Some(existing) = registry.by_target.get(&target) {
                            warn!(
                                "PortraitSheetRegistry: target '{target}' is claimed by \
                                 both '{existing}' and '{path}'; keeping '{existing}'"
                            );
                        } else {
                            registry.by_target.insert(target, path.clone());
                        }
                    }
                    registry.manifests.insert(path, manifest);
                    loaded += 1;
                }
                Err(error) => {
                    warn!("PortraitSheetRegistry: failed to parse baked {asset_path}: {error}");
                }
            }
        }
        info!("PortraitSheetRegistry: loaded {loaded} baked portrait manifests");
        registry
    }

    pub fn get(&self, manifest_path: &str) -> Option<&PortraitSheetManifest> {
        self.manifests.get(&normalize_manifest_path(manifest_path))
    }

    /// **A portrait TARGET's manifest**, and the path it lives at.
    ///
    /// The path comes back with it because a manifest's own `image` field is a
    /// bare filename (`"alice_portraits.png"`) while everything that loads one
    /// speaks asset-relative paths (`"sprites/alice_portraits.png"`). ⛔ a
    /// resolver that returned the manifest alone would hand its caller a
    /// filename that resolves to nothing, silently — which is the exact failure
    /// `declared_art_resolves.rs` exists for.
    pub fn manifest_for_target(&self, target: &str) -> Option<(&str, &PortraitSheetManifest)> {
        let path = self.by_target.get(target.trim())?;
        Some((path.as_str(), self.manifests.get(path)?))
    }

    /// Every portrait target this registry can name, in stable order.
    ///
    /// `BTreeMap`, so a preparation-time "did you mean" list is the same on
    /// every machine.
    pub fn available_targets(&self) -> impl Iterator<Item = &str> {
        self.by_target.keys().map(String::as_str)
    }

    /// Resolve a requested clip with deterministic fallbacks: requested key,
    /// catalog-declared default, manifest default, then the conventional
    /// `default`. Returns the actual selected key beside its record.
    pub fn resolve_clip<'a>(
        &'a self,
        manifest_path: &str,
        requested: Option<&str>,
        catalog_default: &str,
    ) -> Option<(&'a str, &'a PortraitClipRecord)> {
        let manifest = self.get(manifest_path)?;
        let candidates = [
            requested.filter(|name| !name.trim().is_empty()),
            (!catalog_default.trim().is_empty()).then_some(catalog_default),
            Some(manifest.default_clip.as_str()),
            Some("default"),
        ];
        for candidate in candidates.into_iter().flatten() {
            if let Some((name, clip)) = manifest.clips.get_key_value(candidate) {
                return Some((name.as_str(), clip));
            }
        }
        None
    }

    pub fn len(&self) -> usize {
        self.manifests.len()
    }

    pub fn is_empty(&self) -> bool {
        self.manifests.is_empty()
    }
}

fn normalize_manifest_path(path: &str) -> String {
    path.trim_start_matches("./").replace('\\', "/")
}

pub fn baked_portrait_registry() -> PortraitSheetRegistry {
    PortraitSheetRegistry::from_baked_table(crate::baked_portrait_rons::BAKED_PORTRAIT_RONS)
}

/// **Every baked portrait target, sorted** — the vocabulary a character's
/// `portrait` reference resolves against at preparation.
///
/// ⭐ the exact twin of `character::sheets::available_targets`, and it exists for
/// the reason that one does: the engine always knows this vocabulary because it
/// is baked, so a provider should never have to hand it over just to have its
/// typo caught. [`PortraitSheetRegistry::available_targets`] already carried the
/// note *"so a preparation-time did-you-mean list is the same on every
/// machine"* — a doc naming the use nothing was connected to (ledger D106).
///
/// ⚠ **`OnceLock`, because [`baked_portrait_registry`] PARSES.** Preparation
/// runs per character, and calling the registry constructor there would re-parse
/// every baked portrait manifest once per registered character — the startup
/// decode storm §7.1 deleted, rebuilt from the other end. The sheet index is a
/// process-global `OnceLock` for the same reason and classifies itself as an
/// immutable asset cache; this is that.
pub fn available_portrait_targets() -> Vec<&'static str> {
    static INDEX: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    INDEX
        .get_or_init(|| {
            let mut out: Vec<String> = baked_portrait_registry()
                .available_targets()
                .map(str::to_owned)
                .collect();
            out.sort_unstable();
            out
        })
        .iter()
        .map(String::as_str)
        .collect()
}

/// Install the compile-time portrait manifest index. Presentation code consumes
/// this resource; simulation remains independent of portrait assets.
pub struct PortraitSheetRegistryPlugin;

impl Plugin for PortraitSheetRegistryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PortraitSheetRegistry>()
            .add_systems(Startup, init_portrait_registry);
    }
}

fn init_portrait_registry(mut registry: ResMut<PortraitSheetRegistry>) {
    if registry.is_empty() {
        *registry = baked_portrait_registry();
    }
}

/// Parse and validate one portrait manifest.
pub fn parse_portrait_manifest(text: &str) -> Result<PortraitSheetManifest, String> {
    let manifest: PortraitSheetManifest =
        ron::from_str(text).map_err(|err| format!("portrait manifest parse failed: {err}"))?;
    manifest.validate()?;
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renderer_portrait_manifest_round_trips() {
        let manifest = parse_portrait_manifest(
            r#"(
                target: "alice",
                image: "alice_portraits.png",
                frame_width: 256,
                frame_height: 320,
                default_clip: "default",
                clips: {
                    "default": (
                        duration_ms: 0,
                        looping: false,
                        frames: [(x: 0, y: 0, w: 256, h: 320)],
                    ),
                },
            )"#,
        )
        .expect("renderer portrait shape should parse");
        assert_eq!(manifest.target, "alice");
        assert_eq!(manifest.default_clip, "default");
        assert_eq!(manifest.clips["default"].frames[0].h, 320);
    }

    #[test]
    fn baked_registry_resolves_named_clips_and_falls_back_to_default() {
        let registry = PortraitSheetRegistry::from_baked_table(&[(
            "sprites/alice_portraits.ron",
            r#"(
                target: "alice",
                image: "alice_portraits.png",
                frame_width: 256,
                frame_height: 320,
                default_clip: "calm",
                clips: {
                    "calm": (frames: [(x: 0, y: 0, w: 256, h: 320)]),
                    "speaking": (
                        duration_ms: 90,
                        looping: true,
                        frames: [(x: 256, y: 0, w: 256, h: 320)],
                    ),
                },
            )"#,
        )]);
        let (name, clip) = registry
            .resolve_clip("sprites\\alice_portraits.ron", Some("speaking"), "calm")
            .expect("named clip resolves");
        assert_eq!(name, "speaking");
        assert!(clip.looping);

        let (name, _) = registry
            .resolve_clip("sprites/alice_portraits.ron", Some("missing"), "calm")
            .expect("missing expression falls back");
        assert_eq!(name, "calm");
    }

    #[test]
    fn animated_clip_requires_positive_frame_duration() {
        let error = parse_portrait_manifest(
            r#"(
                target: "alice",
                image: "alice_portraits.png",
                frame_width: 256,
                frame_height: 320,
                default_clip: "default",
                clips: {
                    "default": (frames: [(x: 0, y: 0, w: 256, h: 320)]),
                    "speaking": (
                        frames: [
                            (x: 256, y: 0, w: 256, h: 320),
                            (x: 512, y: 0, w: 256, h: 320),
                        ],
                    ),
                },
            )"#,
        )
        .expect_err("multi-frame clips need a playback duration");
        assert!(error.contains("animated clip 'speaking' has zero duration"));
    }

    #[test]
    fn plugin_preserves_an_explicitly_injected_registry() {
        let custom = PortraitSheetRegistry::from_baked_table(&[(
            "sprites/custom_portraits.ron",
            r#"(
                target: "custom",
                image: "custom_portraits.png",
                frame_width: 16,
                frame_height: 20,
                default_clip: "default",
                clips: {
                    "default": (frames: [(x: 0, y: 0, w: 16, h: 20)]),
                },
            )"#,
        )]);
        let mut app = App::new();
        app.insert_resource(custom);
        app.add_plugins(PortraitSheetRegistryPlugin);
        app.update();
        assert!(app
            .world()
            .resource::<PortraitSheetRegistry>()
            .get("sprites/custom_portraits.ron")
            .is_some());
    }

    #[test]
    fn missing_default_clip_is_rejected() {
        let error = parse_portrait_manifest(
            r#"(
                target: "alice",
                image: "alice_portraits.png",
                frame_width: 256,
                frame_height: 320,
                default_clip: "default",
                clips: {},
            )"#,
        )
        .expect_err("default clip is required");
        assert!(error.contains("default clip 'default' is missing"));
    }
}
