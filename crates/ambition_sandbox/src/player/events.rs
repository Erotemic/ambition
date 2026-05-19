//! Player gameplay events / messages.

use bevy::prelude::{Entity, Message};

/// Typed heal request message for gameplay heal events.
///
/// `target` carries the player entity the heal should land on. `None`
/// keeps the legacy behavior (route to the primary player) — useful
/// for cutscene / quest / dev-tool heals that aren't tied to a
/// specific player. A concrete `Some(entity)` is what per-player
/// pickup collection (#17.6 bridge) sets so a non-primary player's
/// pickup heals them, not the primary.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayerHealRequested {
    pub amount: i32,
    pub target: Option<Entity>,
}

impl PlayerHealRequested {
    /// Heal request without a specific player target — applies to the
    /// primary player.
    pub fn new(amount: i32) -> Self {
        Self {
            amount,
            target: None,
        }
    }

    /// Heal request targeting a specific player entity. Use when the
    /// producer already knows which player to heal (pickup overlap,
    /// per-player ability, etc.).
    pub fn for_target(amount: i32, target: Entity) -> Self {
        Self {
            amount,
            target: Some(target),
        }
    }
}

/// Damage already travels through the feature-domain rich message. This alias
/// documents that the same message is the player damage request seam.
pub type PlayerDamageRequested = crate::features::PlayerDamageEvent;
