//! Player gameplay events / messages.

use bevy::prelude::Message;

/// Typed heal request message for gameplay heal events.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayerHealRequested {
    pub amount: i32,
}

impl PlayerHealRequested {
    pub fn new(amount: i32) -> Self {
        Self { amount }
    }
}

/// Damage already travels through the feature-domain rich message. This alias
/// documents that the same message is the player damage request seam.
pub type PlayerDamageRequested = crate::features::PlayerDamageEvent;
