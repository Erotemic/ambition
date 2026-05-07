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
    pub fn new(
        id: impl Into<String>,
        prompt: impl Into<String>,
        aabb: Aabb,
        kind: InteractionKind,
    ) -> Self {
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
    Door {
        target: Option<String>,
    },
    Npc {
        dialogue_id: Option<String>,
        /// Half-range of the NPC's patrol pace, in world pixels.
        /// `0.0` (the default) means the NPC stands still; values
        /// > 0 make the NPC pace between
        /// > `[spawn_x - patrol_radius, spawn_x + patrol_radius]`
        /// > at the sandbox's authored patrol speed. The NPC stops
        /// > inside the player's `talk_radius` so the player can
        /// > open dialog without chasing a moving target.
        ///
        /// Engine model only — the actual movement / gravity /
        /// collision lives in `ambition_sandbox::features::NpcRuntime`.
        patrol_radius: f32,
    },
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

/// What causes a breakable to break.
///
/// Replaces an earlier magic-string check that decided "stand-to-crumble" by
/// substring-matching on the entity name/id. Authors now pick the trigger
/// explicitly per LDtk entity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BreakableTrigger {
    /// Only player attacks deal damage (default; original behavior).
    #[default]
    OnHit,
    /// Crumbles after the player stands on it for a short window.
    /// Stand-to-crumble requires the breakable to contribute non-`None`
    /// collision while intact (see [`BreakableCollision`]).
    OnStand,
    /// Either trigger applies.
    Either,
}

impl BreakableTrigger {
    pub fn allows_hit(self) -> bool {
        matches!(self, BreakableTrigger::OnHit | BreakableTrigger::Either)
    }

    pub fn allows_stand(self) -> bool {
        matches!(self, BreakableTrigger::OnStand | BreakableTrigger::Either)
    }
}

/// What kind of collision a breakable contributes while it is still intact.
///
/// Replaces the older `solid: bool` knob with a typed shape so authoring
/// tooling (LDtk Surface) can compile down a single rectangular volume into
/// either a hard wall, a one-way landing, or a pure trigger volume.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BreakableCollision {
    /// Pure trigger volume: damage/contact events apply, but the player passes
    /// through it. Useful for breakable scenery that does not block movement.
    #[default]
    None,
    /// Hard collision on both axes while intact (legacy `solid: true`).
    Solid,
    /// One-way landing platform while intact: solid only when crossed from above.
    OneWayUp,
}

impl BreakableCollision {
    /// True if the breakable currently blocks movement on any axis.
    pub fn blocks_movement(self) -> bool {
        !matches!(self, BreakableCollision::None)
    }

    /// True if the breakable presents a hard wall while intact.
    pub fn is_solid(self) -> bool {
        matches!(self, BreakableCollision::Solid)
    }
}

/// Breakable wall/platform/object semantics.
#[derive(Clone, Debug, PartialEq)]
pub struct Breakable {
    pub id: String,
    pub state: BreakableState,
    pub health: Health,
    pub respawn: RespawnPolicy,
    /// Collision shape contributed while the breakable is intact.
    pub collision: BreakableCollision,
    pub trigger: BreakableTrigger,
    pub debris_cue: Option<String>,
    /// True for breakable pogo orbs: while intact the breakable contributes
    /// a `BlockKind::PogoOrb` to the collision world, and each successful
    /// pogo bounce damages it. Doesn't change collision/trigger semantics.
    pub pogo_refresh: bool,
}

impl Breakable {
    pub fn new(id: impl Into<String>, max_hp: i32) -> Self {
        Self {
            id: id.into(),
            state: BreakableState::Intact,
            health: Health::new(max_hp),
            respawn: RespawnPolicy::Never,
            collision: BreakableCollision::None,
            trigger: BreakableTrigger::OnHit,
            debris_cue: None,
            pogo_refresh: false,
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

    #[test]
    fn breakable_trigger_predicates() {
        assert!(BreakableTrigger::OnHit.allows_hit());
        assert!(!BreakableTrigger::OnHit.allows_stand());
        assert!(!BreakableTrigger::OnStand.allows_hit());
        assert!(BreakableTrigger::OnStand.allows_stand());
        assert!(BreakableTrigger::Either.allows_hit());
        assert!(BreakableTrigger::Either.allows_stand());
    }

    #[test]
    fn breakable_collision_predicates() {
        assert!(!BreakableCollision::None.blocks_movement());
        assert!(BreakableCollision::Solid.blocks_movement());
        assert!(BreakableCollision::OneWayUp.blocks_movement());
        assert!(!BreakableCollision::None.is_solid());
        assert!(BreakableCollision::Solid.is_solid());
        // OneWayUp is "blocks movement" but not strictly Solid in the
        // hard-wall sense — it only stops the falling player from above.
        assert!(!BreakableCollision::OneWayUp.is_solid());
    }

    #[test]
    fn breakable_default_state_is_intact() {
        let block = Breakable::new("test", 1);
        assert_eq!(block.state, BreakableState::Intact);
        assert_eq!(block.collision, BreakableCollision::None);
        assert_eq!(block.trigger, BreakableTrigger::OnHit);
        assert!(!block.pogo_refresh);
    }

    #[test]
    fn chest_default_state_is_closed_and_persistent() {
        // Chests default to Closed and persistent=true so the save
        // system records them automatically; reward is propagated as
        // given (None for empty chests / triggers).
        let chest = Chest::new("hub_chest", Some(PickupKind::Health { amount: 2 }));
        assert_eq!(chest.state, ChestState::Closed);
        assert!(chest.persistent);
        assert_eq!(chest.reward, Some(PickupKind::Health { amount: 2 }));

        let empty = Chest::new("decoration", None);
        assert_eq!(empty.state, ChestState::Closed);
        assert!(empty.reward.is_none());
    }
}
