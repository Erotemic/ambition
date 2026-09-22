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

#[cfg(feature = "portal")]
pub use ambition_portal2d::PortalGun;

/// What occupies a body's hand. An active portal gun outranks a held item, the
/// same precedence the inventory's "what is equipped" projection reads.
pub fn hand<'a>(held: Option<&'a HeldItem>, gun_active: bool) -> Hand<'a> {
    if gun_active {
        Hand::PortalGun
    } else {
        held.map_or(Hand::Empty, |held| Hand::Holding(&held.spec))
    }
}

/// The portal-gun half of a hand, as a query member: absent from a composition
/// without portals.
#[cfg(feature = "portal")]
pub type GunSlot = Option<&'static PortalGun>;
#[cfg(not(feature = "portal"))]
pub type GunSlot = ();

/// A body's repertoire inputs — identity, worn equipment and the hand as it is
/// now — and its two derived outputs, borrowed together so that writing one
/// output without the other is not expressible.
#[derive(QueryData)]
#[query_data(mutable)]
pub struct RepertoireQuery {
    pub identity: &'static IdentityKit,
    pub worn: Option<&'static WornEquipment>,
    pub held: Option<&'static HeldItem>,
    pub gun: GunSlot,
    pub action_set: &'static mut ActionSet,
    /// `Option` because a body with no move timelines carries none; its
    /// `ActionSet` is then the whole derivation.
    pub moveset: Option<&'static mut ActorMoveset>,
}

impl RepertoireQueryItem<'_, '_> {
    /// Re-derive `ActionSet` and `ActorMoveset` for `hand`.
    pub fn refold(&mut self, hand: Hand<'_>) {
        let rebuilt = effective_repertoire(self.identity, self.worn, hand);
        *self.action_set = rebuilt.action_set;
        if let Some(moveset) = self.moveset.as_deref_mut() {
            *moveset = ActorMoveset(rebuilt.moveset);
        }
    }

    /// Re-derive for the hand as the components say it is now.
    pub fn refold_current(&mut self) {
        let held = self.held;
        self.refold_with_held(held);
    }

    /// Re-derive for a hand whose held item is about to become `held`, with the
    /// portal gun as it is. Used where the change is still a queued command.
    pub fn refold_with_held(&mut self, held: Option<&HeldItem>) {
        let gun_active = self.gun_active();
        self.refold(hand(held, gun_active));
    }

    /// Re-derive for a hand whose portal gun is about to be `active` (or gone),
    /// with the held item as it is.
    pub fn refold_with_gun(&mut self, active: bool) {
        let held = self.held;
        self.refold(hand(held, active));
    }

    fn gun_active(&self) -> bool {
        #[cfg(feature = "portal")]
        return self.gun.is_some_and(|gun| gun.active);
        #[cfg(not(feature = "portal"))]
        false
    }
}

/// [`RepertoireQueryItem::refold_with_held`] for a caller holding the whole
/// `World`. Returns `false` for a body with no repertoire to fold.
pub fn refold_in_world_with_held(world: &mut World, body: Entity, held: Option<&HeldItem>) -> bool {
    let Some(identity) = world.get::<IdentityKit>(body) else {
        return false;
    };
    #[cfg(feature = "portal")]
    let gun_active = world.get::<PortalGun>(body).is_some_and(|gun| gun.active);
    #[cfg(not(feature = "portal"))]
    let gun_active = false;
    let rebuilt = effective_repertoire(
        identity,
        world.get::<WornEquipment>(body),
        hand(held, gun_active),
    );
    let mut entity = world.entity_mut(body);
    let Some(mut action_set) = entity.get_mut::<ActionSet>() else {
        return false;
    };
    *action_set = rebuilt.action_set;
    if let Some(mut moveset) = entity.get_mut::<ActorMoveset>() {
        *moveset = ActorMoveset(rebuilt.moveset);
    }
    true
}

/// The phase [`reconcile_effective_repertoire`] runs in, published so a
/// consumer can order against a phase rather than against a function. The
/// composition decides where it lives.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EffectiveRepertoireReconciled;

#[cfg(feature = "portal")]
type HandInputChanged = Or<(
    Changed<WornEquipment>,
    Changed<IdentityKit>,
    Changed<HeldItem>,
    Changed<PortalGun>,
)>;
#[cfg(not(feature = "portal"))]
type HandInputChanged = Or<(Changed<WornEquipment>, Changed<IdentityKit>, Changed<HeldItem>)>;

/// Re-fold a body whose identity, worn equipment or hand changed in place.
pub fn reconcile_effective_repertoire(mut bodies: Query<RepertoireQuery, HandInputChanged>) {
    for mut repertoire in &mut bodies {
        repertoire.refold_current();
    }
}

#[cfg(test)]
mod tests;
