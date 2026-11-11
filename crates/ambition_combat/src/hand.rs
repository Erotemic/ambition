//! What a body can do right now, kept equal to what its durable facts say.
//!
//! The fold is [`effective_repertoire`]: identity, worn equipment and the hand
//! in, `ActionSet` + `ActorMoveset` out. Nothing stores an earlier answer to
//! restore. A transition that changes the hand re-runs the fold with the hand it
//! just produced ([`RepertoireQueryItem::refold`]), and
//! [`reconcile_effective_repertoire`] re-runs it when identity, equipment or a
//! hand component changes in place.
//!
//! Removal is the reason transitions fold for themselves rather than leaving it
//! to the reconcile: a removed component satisfies no `Changed` filter, and
//! removal events are lost when the simulation schedule skips a frame.

use bevy::ecs::query::QueryData;
use bevy::prelude::*;

use ambition_characters::brain::action_set::{ActionSet, IdentityKit};
use ambition_characters::equipment::WornEquipment;
use ambition_characters::repertoire::{effective_repertoire, Hand};

use crate::held_items::HeldItem;
use crate::moveset::ActorMoveset;

/// What occupies a body's hand: the item it holds, if any.
pub fn hand(held: Option<&HeldItem>) -> Hand<'_> {
    held.map_or(Hand::Empty, |held| Hand::Holding(&held.spec))
}

/// A body's repertoire inputs — identity, worn equipment and the hand as it is
/// now — and its two derived outputs, borrowed together so that writing one
/// output without the other is not expressible.
#[derive(QueryData)]
#[query_data(mutable)]
pub struct RepertoireQuery {
    pub identity: &'static IdentityKit,
    pub worn: Option<&'static WornEquipment>,
    pub held: Option<&'static HeldItem>,
    pub action_set: &'static mut ActionSet,
    pub moveset: &'static mut ActorMoveset,
}

impl RepertoireQueryItem<'_, '_> {
    /// Re-derive `ActionSet` and `ActorMoveset` for `hand`.
    pub fn refold(&mut self, hand: Hand<'_>) {
        let rebuilt = effective_repertoire(self.identity, self.worn, hand);
        *self.action_set = rebuilt.action_set;
        *self.moveset = ActorMoveset(rebuilt.moveset);
    }

    /// Re-derive for the hand as the components say it is now.
    pub fn refold_current(&mut self) {
        let held = self.held;
        self.refold_with_held(held);
    }

    /// Re-derive for a hand whose held item is about to become `held`. Used
    /// where the change is still a queued command.
    pub fn refold_with_held(&mut self, held: Option<&HeldItem>) {
        self.refold(hand(held));
    }
}

/// [`RepertoireQueryItem::refold_with_held`] for a caller holding the whole
/// `World`. Returns `false` for a body with no repertoire to fold.
pub fn refold_in_world_with_held(world: &mut World, body: Entity, held: Option<&HeldItem>) -> bool {
    let Some(identity) = world.get::<IdentityKit>(body) else {
        return false;
    };
    let rebuilt = effective_repertoire(identity, world.get::<WornEquipment>(body), hand(held));
    let mut entity = world.entity_mut(body);
    if !entity.contains::<ActionSet>() || !entity.contains::<ActorMoveset>() {
        return false;
    }
    *entity.get_mut::<ActionSet>().expect("checked above") = rebuilt.action_set;
    *entity.get_mut::<ActorMoveset>().expect("checked above") = ActorMoveset(rebuilt.moveset);
    true
}

/// The phase [`reconcile_effective_repertoire`] runs in, published so a
/// consumer can order against a phase rather than against a function. The
/// composition decides where it lives.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EffectiveRepertoireReconciled;

type HandInputChanged = Or<(Changed<WornEquipment>, Changed<IdentityKit>, Changed<HeldItem>)>;

/// Re-fold a body whose identity, worn equipment or hand changed in place.
pub fn reconcile_effective_repertoire(mut bodies: Query<RepertoireQuery, HandInputChanged>) {
    for mut repertoire in &mut bodies {
        repertoire.refold_current();
    }
}

#[cfg(test)]
mod tests;
