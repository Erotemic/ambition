//! **Worn equipment → granted actions**, reconciled continuously.
//!
//! A row may GRANT an action verb ([`EquipmentGrant`]), and a grant has to be
//! revocable: a row that is consumed, downgraded, or unequipped must take its verb
//! with it. Equip used to apply grants one-shot at the pickup site, which made
//! revocation impossible to express — there was no un-granted baseline to subtract
//! against, so the A3 contract documented "a grant-bearing downgrade is out of
//! scope" and the victim-side hit resolver was forbidden from touching a granting
//! row at all.
//!
//! This module removes that limitation by inverting the relationship. A body's
//! live [`ActionSet`] and [`ActorMoveset`] are no longer WRITTEN by whoever
//! happened to change the equipment; they are DERIVED — a pure function of
//!
//! ```text
//! IdentityKit (what the body's worn identity grants)  +  WornEquipment (what it carries)
//! ```
//!
//! recomputed by [`reconcile_equipment_grants`] whenever either input changes.
//! Every mutation therefore reconciles through one derivation, no matter its
//! cause: a pickup, a menu equip, an unequip, or a hit that spends armor and
//! splices in a downgrade row deep inside the shared damage resolver. The resolver
//! does not need an `ActionSet` in its signature and no caller has to remember to
//! rebuild anything — a mutation that forgets to reconcile is now unrepresentable.
//!
//! It is body-generic by construction: the query names no controller and no
//! marker, so a possessed actor, an NPC, or an enemy wearing a granting row
//! reconciles on the identical path the player does.

use bevy::prelude::*;

use ambition_characters::brain::action_set::{ActionSet, IdentityKit};
use ambition_characters::equipment::{apply_equipment_grants, WornEquipment};
use ambition_combat::moveset::{build_actor_moveset, ActorMoveset};

/// Re-derive a body's granted actions and moveset from its identity plus its worn
/// equipment, whenever either side changes.
///
/// Runs only on an actual change to [`WornEquipment`] or [`IdentityKit`]. Writing
/// [`ActionSet`] / [`ActorMoveset`] marks neither of those, so the derivation
/// settles in one pass and cannot re-trigger itself.
///
/// `special` is deliberately NOT re-folded here: whether a body's special becomes
/// a moveset move is an IDENTITY policy (authored personas drive theirs through
/// their own authored path, and folding a generic shell move over it would make a
/// possessed boss fire the wrong special). [`IdentityKit::moveset`] already
/// embodies whatever that identity decided, and this overlays onto it — so a
/// grant adds its verb without re-litigating a decision that is not equipment's to
/// make. Grants only ever carry melee/ranged, so nothing is lost.
pub fn reconcile_equipment_grants(
    mut bodies: Query<
        (
            &IdentityKit,
            &WornEquipment,
            &mut ActionSet,
            &mut ActorMoveset,
        ),
        Or<(Changed<WornEquipment>, Changed<IdentityKit>)>,
    >,
) {
    for (identity, worn, mut action_set, mut moveset) in &mut bodies {
        // The un-granted baseline, then this tick's rows overlaid onto it. Starting
        // from identity — rather than mutating the live set — is what makes a
        // REVOKED grant disappear: the verb is simply never re-applied.
        let mut derived = identity.action_set.clone();
        apply_equipment_grants(&mut derived, worn);

        // Same story for the moveset: re-derive from the identity's contract so a
        // revoked verb's MOVE goes with it, instead of lingering in an
        // overlay-only rebuild that can add but never remove.
        let rebuilt = build_actor_moveset(
            Some(&identity.moveset),
            derived.melee.as_ref(),
            derived.ranged.as_ref(),
            None,
        )
        .unwrap_or_else(|| identity.moveset.clone());

        *action_set = derived;
        *moveset = ActorMoveset(rebuilt);
    }
}

#[cfg(test)]
mod tests;
