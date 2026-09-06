//! Generic room-object label for debug overlays and editor selection.
//!
//! Rendering lives in the Bevy adapter; the label meaning stays with authored
//! room data so debug overlays, inspectors, and editor tools share one source.

use ambition_platformer2d_core::Vec2;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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
}
