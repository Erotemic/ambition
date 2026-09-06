//! Runtime vocabulary for separately published dialogue portrait sheets.
//!
//! Portrait sheets are presentation products rather than gameplay animation
//! sheets: they have named clips, a required default clip, and no collision or
//! actor geometry. The authoring implementation that produced the raster is
//! intentionally outside this schema.

use bevy::prelude::{App, Plugin, Rect, ResMut, Resource, Startup};
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

impl From<PortraitFrameRect> for Rect {
    /// The sub-rectangle an `ImageNode` draws. Every still consumer needs this
    /// and none of them should spell it out; a hand-written conversion is where
    /// an off-by-a-width creeps in.
    fn from(frame: PortraitFrameRect) -> Self {
        Rect::new(
            frame.x as f32,
            frame.y as f32,
            (frame.x + frame.w) as f32,
            (frame.y + frame.h) as f32,
        )
    }
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
    /// The clip a STILL consumer should draw, when this target names one.
    ///
    /// A UI box wants a chosen pose; `default_clip` is the clip that PLAYS, and
    /// for a target whose default is a looping idle its first frame is wherever
    /// the loop happens to start. Empty means this target names no separate
    /// still, and a still request falls through to `default_clip`'s first frame.
    #[serde(default)]
    pub still_clip: String,
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
        if !self.still_clip.trim().is_empty() && !self.clips.contains_key(&self.still_clip) {
            return Err(format!(
                "portrait manifest '{}' still clip '{}' is missing",
                self.target, self.still_clip
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
/// Both roads end at the same manifest; only one of them was addressable.
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
                        // REFUSED rather than last-writer-wins, exactly like
                        // `AuthoredSheets::insert_ron`.
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

    /// A portrait TARGET's manifest, and the path it lives at.
    ///
    /// The path comes back with it because a manifest's own `image` field is a
    /// bare filename (`"alice_portraits.png"`) while everything that loads one
    /// speaks asset-relative paths (`"sprites/alice_portraits.png"`). a
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

    /// ONE frame to draw, for a consumer that wants a still portrait.
    ///
    /// A still and an animation are different requests, and a portrait sheet can
    /// answer either — so the caller says which. Selection walks requested key,
    /// the catalog's still override, the manifest's `still_clip`, its
    /// `default_clip`, then the conventional `default`; the chosen clip's FIRST
    /// frame is the still. Reducing an animated clip that way is the sanctioned
    /// degradation, not a guess: a target that wants a different still names one.
    pub fn resolve_still(
        &self,
        manifest_path: &str,
        requested: Option<&str>,
        catalog_still: Option<&str>,
    ) -> Option<(&str, PortraitFrameRect)> {
        let manifest = self.get(manifest_path)?;
        let (name, clip) = select_clip(
            manifest,
            [
                requested,
                catalog_still,
                Some(manifest.still_clip.as_str()),
                Some(manifest.default_clip.as_str()),
                Some("default"),
            ],
        )?;
        Some((name, *clip.frames.first()?))
    }

    /// A clip to PLAY, for a consumer that animates.
    ///
    /// Selection walks requested key, the catalog's declared default, the
    /// manifest default, then the conventional `default`. A one-frame clip comes
    /// back unchanged — a held still is a valid animation, and it is what a
    /// character who never authored motion has to give.
    pub fn resolve_animated<'a>(
        &'a self,
        manifest_path: &str,
        requested: Option<&str>,
        catalog_default: Option<&str>,
    ) -> Option<(&'a str, &'a PortraitClipRecord)> {
        let manifest = self.get(manifest_path)?;
        select_clip(
            manifest,
            [
                requested,
                catalog_default,
                Some(manifest.default_clip.as_str()),
                Some("default"),
                None,
            ],
        )
    }

    pub fn len(&self) -> usize {
        self.manifests.len()
    }

    pub fn is_empty(&self) -> bool {
        self.manifests.is_empty()
    }
}

/// First candidate naming a clip this manifest actually carries, with its key.
///
/// Blank candidates are skipped rather than matched: an unset override and an
/// empty string mean the same thing to every caller, and a clip name is never
/// empty (`validate` refuses one).
fn select_clip<'a>(
    manifest: &'a PortraitSheetManifest,
    candidates: [Option<&str>; 5],
) -> Option<(&'a str, &'a PortraitClipRecord)> {
    candidates
        .into_iter()
        .flatten()
        .filter(|name| !name.trim().is_empty())
        .find_map(|name| {
            manifest
                .clips
                .get_key_value(name)
                .map(|(name, clip)| (name.as_str(), clip))
        })
}

fn normalize_manifest_path(path: &str) -> String {
    path.trim_start_matches("./").replace('\\', "/")
}

pub fn baked_portrait_registry() -> PortraitSheetRegistry {
    PortraitSheetRegistry::from_baked_table(crate::baked_portrait_rons::BAKED_PORTRAIT_RONS)
}

/// Every baked portrait target, sorted — the vocabulary a character's
/// `portrait` reference resolves against at preparation.
///
/// the exact twin of `character::sheets::available_targets`, and it exists for
/// the reason that one does: the engine always knows this vocabulary because it
/// is baked, so a provider should never have to hand it over just to have its
/// typo caught. [`PortraitSheetRegistry::available_targets`] already carried the
/// note *"so a preparation-time did-you-mean list is the same on every
/// machine"* — a doc naming the use nothing was connected to.
///
/// `OnceLock`, because [`baked_portrait_registry`] PARSES. Preparation
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

    /// A target whose default MOVES and which names its own still.
    fn animated_registry() -> PortraitSheetRegistry {
        PortraitSheetRegistry::from_baked_table(&[(
            "sprites/alice_portraits.ron",
            r#"(
                target: "alice",
                image: "alice_portraits.png",
                frame_width: 256,
                frame_height: 320,
                default_clip: "idle",
                still_clip: "hero",
                clips: {
                    "idle": (
                        duration_ms: 140,
                        looping: true,
                        frames: [
                            (x: 0, y: 0, w: 256, h: 320),
                            (x: 256, y: 0, w: 256, h: 320),
                        ],
                    ),
                    "hero": (frames: [(x: 512, y: 0, w: 256, h: 320)]),
                    "speaking": (
                        duration_ms: 90,
                        looping: true,
                        frames: [(x: 768, y: 0, w: 256, h: 320)],
                    ),
                },
            )"#,
        )])
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
            .resolve_animated("sprites\\alice_portraits.ron", Some("speaking"), Some("calm"))
            .expect("named clip resolves");
        assert_eq!(name, "speaking");
        assert!(clip.looping);

        let (name, _) = registry
            .resolve_animated("sprites/alice_portraits.ron", Some("missing"), Some("calm"))
            .expect("missing expression falls back");
        assert_eq!(name, "calm");
    }

    /// The whole point of the split: one sheet, two questions, two answers.
    #[test]
    fn a_still_request_and_an_animated_request_disagree_on_purpose() {
        let registry = animated_registry();

        let (still_name, frame) = registry
            .resolve_still("sprites/alice_portraits.ron", None, None)
            .expect("a still resolves");
        assert_eq!(still_name, "hero", "a declared still outranks the playing default");
        assert_eq!(frame.x, 512);

        let (clip_name, clip) = registry
            .resolve_animated("sprites/alice_portraits.ron", None, None)
            .expect("an animation resolves");
        assert_eq!(clip_name, "idle");
        assert_eq!(clip.frames.len(), 2, "the animated road keeps every frame");
    }

    /// Jon's stated fallback: ask for a still, get the first frame of what moves.
    #[test]
    fn a_still_of_an_animated_clip_is_its_first_frame() {
        let registry = animated_registry();
        let (name, frame) = registry
            .resolve_still("sprites/alice_portraits.ron", Some("speaking"), None)
            .expect("an animated clip still yields a still");
        assert_eq!(name, "speaking");
        assert_eq!(frame.x, 768);
    }

    /// And the other direction: a character who authored no motion still answers.
    #[test]
    fn an_animated_request_holds_a_one_frame_clip() {
        let registry = animated_registry();
        let (name, clip) = registry
            .resolve_animated("sprites/alice_portraits.ron", Some("hero"), None)
            .expect("a still clip is a held animation");
        assert_eq!(name, "hero");
        assert_eq!(clip.frames.len(), 1);
        assert!(!clip.looping);
    }

    /// Without a declared still, the still road lands on the default's frame 0.
    #[test]
    fn an_undeclared_still_falls_through_to_the_default_clip() {
        let registry = PortraitSheetRegistry::from_baked_table(&[(
            "sprites/victor_portraits.ron",
            r#"(
                target: "victor",
                image: "victor_portraits.png",
                frame_width: 256,
                frame_height: 320,
                default_clip: "idle",
                clips: {
                    "idle": (
                        duration_ms: 140,
                        looping: true,
                        frames: [
                            (x: 0, y: 0, w: 256, h: 320),
                            (x: 256, y: 0, w: 256, h: 320),
                        ],
                    ),
                },
            )"#,
        )]);
        let (name, frame) = registry
            .resolve_still("sprites/victor_portraits.ron", None, None)
            .expect("a still resolves without a declared one");
        assert_eq!(name, "idle");
        assert_eq!(frame.x, 0);
    }

    #[test]
    fn a_still_clip_naming_nothing_is_rejected() {
        let error = parse_portrait_manifest(
            r#"(
                target: "alice",
                image: "alice_portraits.png",
                frame_width: 256,
                frame_height: 320,
                default_clip: "default",
                still_clip: "hero",
                clips: {
                    "default": (frames: [(x: 0, y: 0, w: 256, h: 320)]),
                },
            )"#,
        )
        .expect_err("a declared still must name a clip that exists");
        assert!(error.contains("still clip 'hero' is missing"));
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
