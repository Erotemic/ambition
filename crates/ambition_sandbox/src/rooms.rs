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
/// `visual_theme` (added by `tools/add_biome_level_fields.py`) land
/// here. Every field is optional so existing levels keep working
/// without a value. The first non-empty value among an active area's
/// member levels wins; future systems can refine this if needed
/// (e.g. dominant-vote, level-position weighted).
///
/// Consumers: room music selection, ambient layer selection,
/// renderer palette/theme variants. This struct is intentionally
/// non-exhaustive — adding a metadata seam is cheaper than adding a
/// new resource per consumer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RoomMetadata {
    pub biome: Option<String>,
    pub music_track: Option<String>,
    pub ambient_profile: Option<String>,
    pub visual_theme: Option<String>,
}

impl RoomMetadata {
    pub fn is_empty(&self) -> bool {
        self.biome.is_none()
            && self.music_track.is_none()
            && self.ambient_profile.is_none()
            && self.visual_theme.is_none()
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
    }
}

/// Complete room data used by the Bevy sandbox.
#[derive(Clone, Debug)]
pub struct RoomSpec {
    pub id: String,
    pub world: ae::World,
    pub loading_zones: Vec<LoadingZone>,
    pub metadata: RoomMetadata,
    /// LDtk-authored moving platforms for this area.
    ///
    /// Empty means the sandbox runtime uses the legacy
    /// `MovingPlatformState::time_reference` fallback for compatibility with
    /// unauthored/debug rooms. New gameplay rooms should place explicit
    /// `MovingPlatform` entities so the map remains the source of truth.
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
