//! The feature-visual TAXONOMY shared by the sim (which stamps it at spawn)
//! and every read-model/presentation consumer.
//!
//! Actors|props is the whole taxonomy (see `feedback: actors vs props`): one
//! actor kind covers enemy/NPC/boss/sandbag — those were never render *types*,
//! only states of one actor. Lives in the foundation crate so the render layer
//! and the sprite resolvers can name a kind without depending on the combat
//! model, and the combat/actor sim can stamp it without knowing presentation.
//! (Moved from `ambition_combat::events` — recon C2: it was the only reason
//! the renderer depended on the combat crate.)

use ambition_engine_core as ae;
use bevy::prelude::Component;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FeatureVisualKind {
    /// Any brain-carrying body — enemy, NPC, boss, sandbag. There is ONE actor
    /// kind: "enemy vs NPC vs boss vs training-dummy" was never a render *type*,
    /// only a STATE of one actor (see the view row's `fighting` for the combat
    /// state and the sandbag/name fallback in the actor sprite-upgrade system for
    /// the depiction). The taxonomy is actors|props; this is the actor arm.
    Actor,
    Hazard,
    Breakable,
    Chest,
    Pickup,
    /// Latched switch. Renders as a colored block whose color depends
    /// on the view row's `switch_on` (red = off, green = on).
    Switch,
}

/// Marker binding a feature visual to its kind + collision size (kept out of
/// the render layer so the mount gameplay can remove it without importing
/// presentation).
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct BoundFeatureKind {
    pub kind: FeatureVisualKind,
    pub collision_size: ae::Vec2,
}

impl BoundFeatureKind {
    pub fn new(kind: FeatureVisualKind, collision: bevy::math::Vec2) -> Self {
        Self {
            kind,
            collision_size: ae::Vec2::new(collision.x, collision.y),
        }
    }

    pub fn matches(&self, kind: FeatureVisualKind, collision_size: ae::Vec2) -> bool {
        self.kind == kind && (self.collision_size - collision_size).length_squared() <= 0.25
    }
}
