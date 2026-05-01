//! Engine-owned debug metadata.
//!
//! Rendering of labels stays in the Bevy adapter, but the meaning of labels is
//! authored with the room data so debug overlays, inspectors, and future editor
//! tools can share the same source of truth.

use crate::Vec2;

/// Generic room-object label for debug overlays and editor selection.
#[derive(Clone, Debug, PartialEq)]
pub struct DebugLabel {
    pub text: String,
    pub position: Vec2,
    pub category: DebugLabelKind,
}

impl DebugLabel {
    pub fn new(text: impl Into<String>, position: Vec2, category: DebugLabelKind) -> Self {
        Self {
            text: text.into(),
            position,
            category,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DebugLabelKind {
    Room,
    LoadingZone,
    Hazard,
    Enemy,
    Boss,
    Interactable,
    Pickup,
    Custom,
}

/// Specialized label for loading-zone destinations.
#[derive(Clone, Debug, PartialEq)]
pub struct DestinationLabel {
    pub zone_id: String,
    pub destination_room: String,
    pub destination_zone: String,
    pub position: Vec2,
}

impl DestinationLabel {
    pub fn text(&self) -> String {
        format!("{} -> {}:{}", self.zone_id, self.destination_room, self.destination_zone)
    }
}
