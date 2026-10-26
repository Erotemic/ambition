//! Rollback declarations for state owned by this crate.

use ambition_platformer2d_core::snapshot::RollbackRegistrar;
const OWNER: &str = env!("CARGO_PKG_NAME");

/// Register this domain's rollback state.
pub fn register_rollback_state<R: RollbackRegistrar>(registrar: &mut R) {
    registrar.require_rollback::<crate::GroundItem>(OWNER, "entity:ground_item");
    registrar.require_rollback::<crate::SettledItem>(OWNER, "entity:settled_item");
    registrar
        .rollback_component_clone::<crate::GroundItem>(OWNER, "item.ground_item");
    // ⭐ SLEEP IS SIMULATION STATE, and this one is presence/absence rather than
    // a value. `ground_item_physics` skips a settled object entirely, so a
    // rewind past the frame an object landed must also un-settle it — otherwise
    // the replay steps a resting item on one peer and not the other, and the
    // divergence is silent until the positions have drifted apart.
    //
    // ⭐⭐ PROBED, NOT PRESENCE-ONLY, since it stopped being a marker. It carries
    // `impact_speed` now — the speed the step that settled it was travelling —
    // and the Smash bomb detonates on that value. A presence probe satisfies the
    // coverage test while seeing NOTHING of it, so a rewind that restored the
    // component with a stale speed would be invisible: the item is settled on
    // both peers and the bomb goes off on one. `rollback_exit_oracle` is what
    // said so, on the run after the field was added.
    // ⛔⛔ IT DECIDES WHETHER AN OBJECT IN THE WORLD IS GOING TO EXPLODE, and it
    // lives on a `GroundItem`, which is already an anchor. A rewind that lost it
    // disarms a live bomb; a rewind that restored it onto a caught bomb re-arms
    // one in a hand. It replaced a velocity heuristic that WAS rollback state by
    // accident (`GroundItem` carries `vel`), so this is the same coverage moved
    // to the fact that actually decides.
    registrar.rollback_component_clone_probed::<crate::ReleasedAs>(
        OWNER,
        "item.released_as",
        |released| match released.0 {
            crate::Release::Throw => 1,
            crate::Release::Drop => 2,
        },
    );
    registrar.rollback_component_clone_probed::<crate::SettledItem>(
        OWNER,
        "item.settled_item",
        |settled| settled.impact_speed.to_bits() as u64,
    );
    // CUSTODY IS SIMULATION STATE, not a cache. It decides on every later
    // frame whether the item is drawn, stepped by `ground_item_physics`, and
    // grabbable — so a rewind that restored the wrong value leaves the same axe
    // both in a hand and on the floor, or makes a carried axe fall out of it.
    // It replaced a despawn/spawn pair, which GGRS reproduced through the
    // entity anchor; the state that took over the same job owes the same
    // coverage.
    //
    // `_entity_set` rather than `_entity_ref`: `InWorld` names no body at all,
    // so the handle is a zero-or-one set. The probe therefore measures WHICH
    // body is holding it through that body's stable identity — a restore that
    // hands the item to the wrong holder changes this census and would not
    // change a presence count.
    registrar.rollback_component_clone_entity_set::<crate::ItemCustody>(
        OWNER,
        "item.item_custody",
        |custody| match custody {
            crate::ItemCustody::InWorld => Vec::new(),
            crate::ItemCustody::Held { holder } => {
                vec![*holder]
            }
        },
    );
    registrar.rollback_map_entities::<crate::ItemCustody>(OWNER, "map.item_custody");

}
