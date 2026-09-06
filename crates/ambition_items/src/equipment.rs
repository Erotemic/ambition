//! Derive granted actions and moves from identity plus worn equipment.
//!
//! `ActionSet` and `ActorMoveset` are rebuilt whenever either input changes, so
//! grants are revocable regardless of how equipment changed. The query is body-
//! generic and does not branch on controller identity.

use bevy::prelude::*;

use ambition_characters::brain::action_set::{ActionSet, IdentityKit};
use ambition_characters::equipment::{apply_equipment_grants, WornEquipment};
use ambition_combat::moveset::{build_actor_moveset, ActorMoveset};

/// Reconcile granted actions and moves when identity or equipment changes.
///
/// Specials are identity policy and are not re-folded here. Equipment grants
/// overlay melee/ranged verbs onto the identity's authoritative baseline.
/// The set [`reconcile_equipment_grants`] runs in — **published so a consumer can
/// order against a PHASE instead of against this function's identity.**
///
/// ⭐⭐ IT EXISTS BECAUSE TWO CONSUMERS WERE NAMING THE SYSTEM.
/// `actor_monolith/src/action_scheme.rs` hangs both
/// `reconcile_moveset_routing_markers` and `reconcile_action_schemes` off this
/// function by name, to get the real guarantee its own comment states — *"a ranged
/// move granted by a row routes through the move timeline on the same tick it is
/// granted."* The edge is genuine; expressing it required another crate to name a
/// function in this one, which is private cross-domain ordering authority.
///
/// ⛔ THE MEMBERSHIP IS THE INSTALLER'S TO DECLARE. This crate publishes the
/// vocabulary; the composition decides which phase it lives in — today
/// `PlayerInput`, after the persona set — and what else shares it. Hence a bare
/// marker with no `configure_sets` beside it.
///
/// ⚠ ONE MEMBER TODAY, so `.after(EquipmentGrantsReconciled)` is exactly
/// `.after(reconcile_equipment_grants)`, which is the point: the same guarantee,
/// in vocabulary that can grow a second member without touching a consumer.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EquipmentGrantsReconciled;

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
        // Rebuild from identity so revoked grants disappear.
        let mut derived = identity.action_set.clone();
        apply_equipment_grants(&mut derived, worn);

        // Rebuild moves from the same authoritative identity baseline.
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
