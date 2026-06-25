//! Lock-wall sync: insert/remove the solid blocks that seal an arena's exits
//! while an encounter is in flight. `sync_lock_walls` reconciles `RoomGeometry`
//! blocks named `lockwall:<encounter_id>` against the live phase of each
//! registered encounter (Starting/Active want a wall; everything else removes
//! it). The wall geometry comes from `EncounterSpec::lock_wall`.

use ambition_engine_core as ae;

use super::{EncounterPhase, EncounterRegistry};

/// Insert / remove the encounter lock wall solid blocks based on
/// the live phase of each encounter. Block name format is
/// `lockwall:<encounter_id>` so the system can find and remove only
/// the blocks it owns.
pub(in crate::encounter) fn sync_lock_walls(world: &mut ae::World, registry: &EncounterRegistry) {
    // Collect the desired (min, size) of each lock-wall block (one
    // per Starting/Active encounter that has an authored LockWall).
    let mut desired: std::collections::HashMap<String, (ae::Vec2, ae::Vec2)> =
        std::collections::HashMap::new();
    for (id, state) in &registry.encounters {
        if !matches!(
            state.phase,
            EncounterPhase::Starting { .. } | EncounterPhase::Active { .. }
        ) {
            continue;
        }
        let Some(spec) = state.spec.as_ref() else {
            continue;
        };
        let Some(wall) = spec.lock_wall.as_ref() else {
            continue;
        };
        desired.insert(
            id.clone(),
            (
                ae::Vec2::new(wall.min[0], wall.min[1]),
                ae::Vec2::new(wall.size[0], wall.size[1]),
            ),
        );
    }

    // Drop any present-but-unwanted lock walls.
    world.blocks.retain(|b| {
        if let Some(stripped) = b.name.strip_prefix("lockwall:") {
            desired.contains_key(stripped)
        } else {
            true
        }
    });

    // Insert any wanted-but-missing lock walls.
    for (id, (min, size)) in desired {
        let name = format!("lockwall:{id}");
        if !world.blocks.iter().any(|b| b.name == name) {
            world.blocks.push(ae::Block::solid(name, min, size));
        }
    }
}
