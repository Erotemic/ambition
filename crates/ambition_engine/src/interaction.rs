//! Interaction, pickup, chest, and breakable building blocks.
//!
//! These are reusable mechanics/data components. The Bevy sandbox can render
//! prompts and play animations, but the identity and gameplay semantics belong
//! in the engine so later story crates can share them.

use crate::actor::{Health, RespawnPolicy};
use crate::geometry::Aabb;

/// A player-facing interaction trigger.
#[derive(Clone, Debug, PartialEq)]
pub struct Interactable {
    pub id: String,
    pub prompt: String,
    pub aabb: Aabb,
    pub kind: InteractionKind,
    pub requires_facing: bool,
    pub enabled: bool,
}

impl Interactable {
    pub fn new(id: impl Into<String>, prompt: impl Into<String>, aabb: Aabb, kind: InteractionKind) -> Self {
        Self {
            id: id.into(),
            prompt: prompt.into(),
            aabb,
            kind,
            requires_facing: false,
            enabled: true,
        }
    }
}

/// What an interactable does when activated.
#[derive(Clone, Debug, PartialEq)]
pub enum InteractionKind {
    Door { target: Option<String> },
    Npc { dialogue_id: Option<String> },
    Chest,
    Pickup,
    Breakable,
    Custom(String),
}

/// Collectible object semantics.
#[derive(Clone, Debug, PartialEq)]
pub struct Pickup {
    pub id: String,
    pub kind: PickupKind,
    pub respawn: RespawnPolicy,
    pub collected: bool,
}

impl Pickup {
    pub fn new(id: impl Into<String>, kind: PickupKind) -> Self {
        Self {
            id: id.into(),
            kind,
            respawn: RespawnPolicy::Never,
            collected: false,
        }
    }
}

/// The reward/effect represented by a pickup or chest.
#[derive(Clone, Debug, PartialEq)]
pub enum PickupKind {
    Health { amount: i32 },
    Currency { amount: i32 },
    Ability { ability_id: String },
    StoryFlag { flag: String },
    Custom(String),
}

/// Treasure chest state and reward. Chests are interactables plus persistence.
#[derive(Clone, Debug, PartialEq)]
pub struct Chest {
    pub id: String,
    pub state: ChestState,
    pub reward: Option<PickupKind>,
    pub persistent: bool,
}

impl Chest {
    pub fn new(id: impl Into<String>, reward: Option<PickupKind>) -> Self {
        Self {
            id: id.into(),
            state: ChestState::Closed,
            reward,
            persistent: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChestState {
    Closed,
    Opening,
    Opened,
}

/// Breakable wall/platform/object semantics.
#[derive(Clone, Debug, PartialEq)]
pub struct Breakable {
    pub id: String,
    pub state: BreakableState,
    pub health: Health,
    pub respawn: RespawnPolicy,
    pub debris_cue: Option<String>,
}

impl Breakable {
    pub fn new(id: impl Into<String>, max_hp: i32) -> Self {
        Self {
            id: id.into(),
            state: BreakableState::Intact,
            health: Health::new(max_hp),
            respawn: RespawnPolicy::Never,
            debris_cue: None,
        }
    }

    pub fn apply_damage(&mut self, amount: i32) -> bool {
        let broke = self.health.damage(amount);
        if broke {
            self.state = BreakableState::Broken;
        } else if self.health.current < self.health.max {
            self.state = BreakableState::Cracking;
        }
        broke
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BreakableState {
    Intact,
    Cracking,
    Broken,
    Respawning,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breakable_moves_through_cracking_to_broken() {
        let mut block = Breakable::new("test", 4);
        assert!(!block.apply_damage(1));
        assert_eq!(block.state, BreakableState::Cracking);
        assert!(block.apply_damage(3));
        assert_eq!(block.state, BreakableState::Broken);
    }
}
