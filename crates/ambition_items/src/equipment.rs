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
