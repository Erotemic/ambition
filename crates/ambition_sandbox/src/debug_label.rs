//! Generic room-object label for debug overlays and editor selection.
//!
//! Moved from `crate::engine_core::debug` to the sandbox in Phase 3d of
//! the player-ecs-bandaid plan (the broader goal: removing the
//! `crate::engine_core` crate entirely). DebugLabel is a sandbox concern
//! — rendering of labels lives in the Bevy adapter, and the meaning
//! is authored with the room data so debug overlays, inspectors, and
//! future editor tools share the same source of truth.

use crate::engine_core::Vec2;

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
