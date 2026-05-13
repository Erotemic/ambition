//! Data-driven sandbox room-set and loading-zone graph.
//!
//! Rooms are runtime graph nodes built from LDtk-authored runtime data. This
//! module owns transition graph assembly and arrival validation, while LDtk owns
//! sandbox world authoring.
//! Loading-zone links point at destination zones by name, so authoring no longer
//! requires brittle hand-written spawn coordinates.

use ambition_engine as ae;
use bevy::prelude::Resource;
use petgraph::graph::{Graph, NodeIndex};

mod graph;
mod spawn;
mod systems;
#[cfg(test)]
mod tests;

pub use spawn::validated_spawn;
pub use systems::{sync_active_room_metadata, sync_room_music_request};

/// How a loading zone should be activated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadingZoneActivation {
    EdgeExit,
    Door,
}

impl LoadingZoneActivation {
    pub fn label(self) -> &'static str {
        match self {
            Self::EdgeExit => "edge exit",
            Self::Door => "door",
        }
    }
}

/// A non-colliding rectangular trigger that swaps the active room.
#[derive(Clone, Debug)]
pub struct LoadingZone {
    pub id: String,
    pub name: String,
    pub activation: LoadingZoneActivation,
    pub aabb: ae::Aabb,
}

impl LoadingZone {
    pub fn is_ready(&self, wants_interact: bool) -> bool {
        match self.activation {
            LoadingZoneActivation::EdgeExit => true,
            LoadingZoneActivation::Door => wants_interact,
        }
    }

    pub fn hint(&self, _flying: bool) -> String {
        match self.activation {
            LoadingZoneActivation::EdgeExit => {
                format!("{}: {}", self.activation.label(), self.name)
            }
            LoadingZoneActivation::Door => {
                format!(
                    "{}: {} (Interact / double-tap up)",
                    self.activation.label(),
                    self.name
                )
            }
        }
    }
}

/// Track the music identifier the active room would like to play.
///
/// Written by `sync_room_music_request` from `ActiveRoomMetadata`,
/// consumed by `audio::apply_encounter_music` as the "default track"
/// when no encounter override is active. The encounter system retains
/// priority — `EncounterMusicRequest::desired_track = Some(...)`
/// overrides this resource the same way it overrides the sandbox-wide
/// default music track. Empty/absent room music falls back to
/// `sandbox_data.audio.default_music_track`.
#[derive(Resource, Clone, Debug, Default)]
pub struct RoomMusicRequest {
    pub desired_track: Option<String>,
}

/// Mirrors `RoomSet::active_metadata()` as a standalone Bevy resource.
///
/// Synced by `sync_active_room_metadata` each frame the active room
/// changes. Consumers (room music selection, ambient layer selection,
/// renderer palette swaps) can subscribe via `Res<ActiveRoomMetadata>`
/// + change detection without importing the larger `RoomSet` type.
#[derive(Resource, Clone, Debug, Default)]
pub struct ActiveRoomMetadata(pub RoomMetadata);

/// Optional declarative room metadata authored on LDtk levels.
///
/// LDtk level fields `biome` / `music_track` / `ambient_profile` /
/// `visual_theme` plus explicit room-visual-profile fields land here.
/// Every field is optional so existing levels keep working
/// without a value. The first non-empty value among an active area's
/// member levels wins; future systems can refine this if needed
/// (e.g. dominant-vote, level-position weighted).
///
/// Consumers: room music selection, ambient layer selection,
/// renderer palette/theme variants. This struct is intentionally
/// non-exhaustive — adding a metadata seam is cheaper than adding a
/// new resource per consumer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RoomVisualProfile {
    /// Stable authored profile id (for example `intro_wakeup_room`).
    pub id: Option<String>,
    /// Explicit parallax/background theme. Prefer this over inferring from
    /// biome, music, or loose color-theme strings.
    pub parallax_theme: Option<String>,
    /// Palette / color-grading hint for future renderer passes.
    pub palette: Option<String>,
    /// Lighting mood hint for future post-process / shader passes.
    pub lighting_hint: Option<String>,
    /// Foreground treatment hint for generated atmosphere layers.
    pub foreground_treatment: Option<String>,
}

impl RoomVisualProfile {
    pub fn is_empty(&self) -> bool {
        self.id.is_none()
            && self.parallax_theme.is_none()
            && self.palette.is_none()
            && self.lighting_hint.is_none()
            && self.foreground_treatment.is_none()
    }

    pub fn merge(&mut self, other: RoomVisualProfile) {
        if self.id.is_none() {
            self.id = other.id;
        }
        if self.parallax_theme.is_none() {
            self.parallax_theme = other.parallax_theme;
        }
        if self.palette.is_none() {
            self.palette = other.palette;
        }
        if self.lighting_hint.is_none() {
            self.lighting_hint = other.lighting_hint;
        }
        if self.foreground_treatment.is_none() {
            self.foreground_treatment = other.foreground_treatment;
        }
    }

    pub fn label(&self) -> Option<&str> {
        self.id
            .as_deref()
            .or(self.parallax_theme.as_deref())
            .or(self.palette.as_deref())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RoomMetadata {
    pub biome: Option<String>,
    pub music_track: Option<String>,
    pub ambient_profile: Option<String>,
    pub visual_theme: Option<String>,
    pub visual_profile: RoomVisualProfile,
}

impl RoomMetadata {
    pub fn is_empty(&self) -> bool {
        self.biome.is_none()
            && self.music_track.is_none()
            && self.ambient_profile.is_none()
            && self.visual_theme.is_none()
            && self.visual_profile.is_empty()
    }

    /// Fold `other` into `self`, preferring values already set.
    /// LDtk active areas can span multiple levels; the first level
    /// with a non-empty value wins so author intent is predictable.
    pub fn merge(&mut self, other: RoomMetadata) {
        if self.biome.is_none() {
            self.biome = other.biome;
        }
        if self.music_track.is_none() {
            self.music_track = other.music_track;
        }
        if self.ambient_profile.is_none() {
            self.ambient_profile = other.ambient_profile;
        }
        if self.visual_theme.is_none() {
            self.visual_theme = other.visual_theme;
        }
        self.visual_profile.merge(other.visual_profile);
    }
}

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
    pub path: ae::KinematicPath,
}

impl KinematicPathSpec {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        aabb: ae::Aabb,
        path: ae::KinematicPath,
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
        self.aliases().any(|alias| alias == query)
    }
}

/// Complete room data used by the Bevy sandbox.
#[derive(Clone, Debug)]
pub struct RoomSpec {
    pub id: String,
    pub world: ae::World,
    pub loading_zones: Vec<LoadingZone>,
    pub metadata: RoomMetadata,
    pub camera_zones: Vec<CameraZoneSpec>,
    /// LDtk-authored path index for platforms, hazards, NPC patrols, camera
    /// rails, and future scripted room beats. `World::objects` still mirrors
    /// these as `RoomObjectKind::KinematicPath` for older consumers, but new
    /// systems should use this typed area-local index.
    pub kinematic_paths: Vec<KinematicPathSpec>,
    /// LDtk-authored moving platforms for this area. This is the complete
    /// platform set for gameplay: if the vector is empty, the room has no
    /// moving platforms.
    pub moving_platforms: Vec<crate::platforms::MovingPlatformState>,
}

#[derive(Clone, Debug)]
struct TransitionEdge {
    from_zone: String,
    to_zone: String,
}

/// Authored directed connection between loading zones in runtime rooms.
///
/// This is intentionally independent from the retired RON world manifest so
/// LDtk and future generators can build `RoomSet` directly from runtime room
/// data.
#[derive(Clone, Debug)]
pub struct RoomLink {
    pub from_room: String,
    pub from_zone: String,
    pub to_room: String,
    pub to_zone: String,
    pub bidirectional: bool,
}

/// Resolved transition from the active room to a graph-linked destination room.
#[derive(Clone, Debug)]
pub struct RoomTransition {
    pub zone: LoadingZone,
    pub target_room: usize,
    pub arrival: ae::Vec2,
}

/// Small room graph for early loading-zone tests.
#[derive(Resource, Clone, Debug)]
pub struct RoomSet {
    pub rooms: Vec<RoomSpec>,
    pub active: usize,
    /// Index of the room the player starts in on a fresh sandbox.
    /// Captured at `from_parts` time so the "reset sandbox" flow can
    /// warp the player back without round-tripping through LDtk.
    pub start: usize,
    graph: Graph<String, TransitionEdge>,
    room_nodes: Vec<NodeIndex>,
}
