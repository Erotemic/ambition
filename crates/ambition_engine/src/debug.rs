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
        format!(
            "{} -> {}:{}",
            self.zone_id, self.destination_room, self.destination_zone
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_label_new_clones_text() {
        let label = DebugLabel::new("hello", Vec2::new(10.0, 20.0), DebugLabelKind::Hazard);
        assert_eq!(label.text, "hello");
        assert_eq!(label.position, Vec2::new(10.0, 20.0));
        assert_eq!(label.category, DebugLabelKind::Hazard);
    }

    #[test]
    fn destination_label_text_uses_arrow_and_colon() {
        let dest = DestinationLabel {
            zone_id: "east_exit".into(),
            destination_room: "scroll_lab".into(),
            destination_zone: "lab_entry".into(),
            position: Vec2::ZERO,
        };
        assert_eq!(dest.text(), "east_exit -> scroll_lab:lab_entry");
    }
}
