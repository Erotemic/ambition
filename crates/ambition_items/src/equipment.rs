//! Keep a body's live repertoire equal to what its durable facts say.
//!
//! The fold itself is `ambition_characters::repertoire::effective_repertoire`;
//! this is the system that re-runs it whenever one of its inputs changes, so
//! grants and held verbs are revocable regardless of which input moved.
//! The query is body-generic and does not branch on controller identity.

use bevy::prelude::*;

use ambition_characters::brain::action_set::{ActionSet, IdentityKit};
use ambition_characters::equipment::WornEquipment;
use ambition_characters::repertoire::{effective_repertoire, Hand};
use ambition_combat::held_items::HeldItem;
use ambition_combat::moveset::ActorMoveset;

/// The phase [`reconcile_effective_repertoire`] runs in, published so a consumer can
/// order against a PHASE rather than against this function's identity.
///
/// Two consumers in `actor_monolith/src/action_scheme.rs` —
/// `reconcile_moveset_routing_markers` and `reconcile_action_schemes` — need the
/// guarantee that a verb granted by a row routes through the move timeline on
/// the same tick it is granted. Naming a set instead of a function keeps that
/// edge from being private cross-crate ordering authority, and lets the set grow
/// a second member without touching a consumer.
///
/// The membership is the installer's to declare: this crate publishes the
/// vocabulary, the composition decides which phase it lives in (today
/// `PlayerInput`, after the persona set). Hence a bare marker with no
/// `configure_sets` beside it.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EffectiveRepertoireReconciled;

/// Re-fold a body's repertoire when identity, worn equipment or the hand moves.
pub fn reconcile_effective_repertoire(
    mut bodies: Query<
        (
            &IdentityKit,
            Option<&WornEquipment>,
            Option<&HeldItem>,
            &mut ActionSet,
            &mut ActorMoveset,
        ),
        Or<(
            Changed<WornEquipment>,
            Changed<IdentityKit>,
            Changed<HeldItem>,
        )>,
    >,
) {
    for (identity, worn, held, mut action_set, mut moveset) in &mut bodies {
        // Every input, every time. A fold that omits one of them repairs the
        // input that moved by discarding the ones that did not — the equipment
        // change that used to take a body's held weapon off it.
        let hand = held.map_or(Hand::Empty, |held| Hand::Holding(&held.spec));
        let rebuilt = effective_repertoire(identity, worn, hand);
        *action_set = rebuilt.action_set;
        *moveset = ActorMoveset(rebuilt.moveset);
    }
}

#[cfg(test)]
mod tests;
