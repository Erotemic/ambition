//! Register physical container state for rollback.

use ambition_platformer2d_core::snapshot::RollbackRegistrar;
const OWNER: &str = env!("CARGO_PKG_NAME");
pub fn register_rollback_state<R: RollbackRegistrar>(registrar: &mut R) {    // `RoomScopedEntity` governs lifetime, not rewindability; moving world
    // items therefore require their own rollback registration.
    registrar.require_rollback::<crate::world_item::WorldItem>(
        OWNER,
        "entity:world_item",
    );
    registrar.rollback_component_clone_probed::<crate::world_item::WorldItem>(
        OWNER,
        "item.world_item",
        |item| {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            // Bit patterns: this is a checksum, not an arithmetic comparison.
            item.pos.x.to_bits().hash(&mut hasher);
            item.pos.y.to_bits().hash(&mut hasher);
            item.half_extent.x.to_bits().hash(&mut hasher);
            item.half_extent.y.to_bits().hash(&mut hasher);
            item.sprite.hash(&mut hasher);
            // WHAT COLLECTING IT DOES, which the box and the sprite do not say.
            // Two pickups can sit in the same place looking the same and equip
            // different rows, and only this tells them apart.
            //
            // the ROW IDENTITY, not the row's authored numbers. `EquipmentRow`
            // carries modifiers, grants and an on-hit rule, and those are
            // CONTENT — read from the same files, identical for a given id in
            // one build, and therefore incapable of differing between two
            // timelines of one session. The id and the exclusive slot are what
            // a divergent spawn would change.
            match &item.payload {
                crate::world_item::WorldItemPayload::Equip(row) => {
                    row.id.hash(&mut hasher);
                    row.exclusive_slot.hash(&mut hasher);
                }
            }
            hasher.finish()
        },
    );
    // The motion PLAN and its cursor travel together — `ItemMotion`'s own doc
    // says a cursor without its plan is meaningless — so one registration
    // restores both halves of where the pickup is in its arc.
    registrar.rollback_component_clone_probed::<crate::item_motion::ItemMotion>(
        OWNER,
        "item.motion",
        |motion| {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            // WHERE IT IS IN ITS ARC: how far the rise has got, how fast it is
            // travelling, and which way it turned last.
            motion.emerged_for().to_bits().hash(&mut hasher);
            motion.velocity().x.to_bits().hash(&mut hasher);
            motion.velocity().y.to_bits().hash(&mut hasher);
            motion.facing().to_bits().hash(&mut hasher);
            // AND THE PLAN IT IS FOLLOWING. The cursor above is meaningless
            // without it — `emerged_for` is read against `emerge.seconds`, and
            // the same cursor under a different plan is a different pickup
            // mid-arc. The plan is authored and does not normally change, which
            // is exactly why a timeline where it DID is worth catching.
            let plan = &motion.plan;
            plan.emerge
                .map(|e| (e.distance.to_bits(), e.seconds.to_bits()))
                .hash(&mut hasher);
            plan.speed.to_bits().hash(&mut hasher);
            plan.facing.to_bits().hash(&mut hasher);
            plan.gravity.to_bits().hash(&mut hasher);
            plan.bounce.to_bits().hash(&mut hasher);
            plan.turns_at_walls.hash(&mut hasher);
            hasher.finish()
        },
    );

    registrar.rollback_component_clone_probed::<crate::containers::FallingChest>(
        OWNER,
        "feature.falling_chest",
        |chest| chest.vel_y.to_bits() as u64,
    );
    registrar.rollback_component_clone::<crate::containers::ChestFeature>(OWNER, "feature.chest");
    registrar.rollback_component_clone::<crate::containers::Opened>(OWNER, "feature.opened");
}
