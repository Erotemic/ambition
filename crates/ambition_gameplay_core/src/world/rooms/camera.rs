//! Camera zones, clamp modes, and kinematic path specs.
//!
//! Split out of the former 823-line `rooms/mod.rs` (2026-06-15); the
//! parent re-exports every type so `rooms::*` paths are unchanged.

use super::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CameraClampMode {
    #[default]
    RoomBounds,
    ZoneBounds,
    None,
}

impl CameraClampMode {
    pub fn from_author_value(value: Option<&str>) -> Self {
        match value
            .map(str::trim)
            .map(|value| value.to_ascii_lowercase().replace('-', "_"))
            .as_deref()
        {
            Some("zone") | Some("zone_bounds") | Some("camera_zone") => Self::ZoneBounds,
            Some("none") | Some("unclamped") | Some("free") => Self::None,
            _ => Self::RoomBounds,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::RoomBounds => "room_bounds",
            Self::ZoneBounds => "zone_bounds",
            Self::None => "none",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CameraZoneSpec {
    pub id: String,
    pub name: String,
    pub aabb: ae::Aabb,
    pub priority: i32,
    /// Requested zoom multiplier while the player overlaps the zone.
    /// `None` preserves the legacy camera-zone breath-out default.
    pub zoom: Option<f32>,
    /// World-space target offset applied after normal look-ahead framing.
    pub target_offset: ae::Vec2,
    /// Optional target-easing override, in hertz.
    pub easing_hz: Option<f32>,
    /// When true, target the zone center instead of the player.
    pub cinematic_lock: bool,
    pub clamp_mode: CameraClampMode,
}

impl CameraZoneSpec {
    pub const LEGACY_BREATH_ZOOM: f32 = 1.15;

    pub fn effective_zoom(&self) -> f32 {
        self.zoom.unwrap_or(Self::LEGACY_BREATH_ZOOM).max(1.0)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct KinematicPathSpec {
    /// Stable authored lookup id. LDtk may not have an explicit `id` field yet,
    /// so conversion falls back to the entity `name` and finally the LDtk iid.
    pub id: String,
    pub name: String,
    pub aabb: ae::Aabb,
    pub path: ambition_characters::actor::KinematicPath,
}

impl KinematicPathSpec {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        aabb: ae::Aabb,
        path: ambition_characters::actor::KinematicPath,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            aabb,
            path,
        }
    }

    pub fn aliases(&self) -> impl Iterator<Item = &str> {
        [self.id.as_str(), self.name.as_str()].into_iter()
    }

    pub fn matches_id(&self, query: &str) -> bool {
        if self.aliases().any(|alias| alias == query) {
            return true;
        }
        // Tolerate `compact_path_name`-style normalization (used by
        // the LDtk path-lookup-id derivation): if the query, after
        // slugifying the spec's `name` field with the same rules
        // *minus* the "_path_" stripping, matches, accept it. This
        // resolves the latent mismatch between authors who reference
        // a path by its raw name-slug (e.g. `enemy_patrol_path_a`)
        // and the runtime id derived from the same name with
        // `_path_` collapsed away (`enemy_patrol_a`). See the comment
        // on `validate_patrol_brain_paths` in `content_validation.rs`.
        if let Some(slug) = name_slug(&self.name) {
            if slug == query {
                return true;
            }
        }
        false
    }
}

fn name_slug(name: &str) -> Option<String> {
    let mut slug = String::new();
    let mut previous_was_sep = false;
    for ch in name.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            previous_was_sep = false;
        } else if !previous_was_sep && !slug.is_empty() {
            slug.push('_');
            previous_was_sep = true;
        }
    }
    while slug.ends_with('_') {
        slug.pop();
    }
    if slug.is_empty() {
        None
    } else {
        Some(slug)
    }
}
