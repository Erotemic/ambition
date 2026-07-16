//! Lock-wall contribution: the solid blocks that seal an arena's exits while an
//! encounter is in flight. The walls are NOT mutated into the authored
//! [`ambition_engine_core::RoomGeometry`] base — that would break the resolved authored-base
//! model (the base is swapped at room boundaries, never edited mid-room).
//! Instead [`contribute_encounter_lock_walls`] derives the live wall set every
//! frame and pushes it onto [`FeatureEcsWorldOverlay::gate_solids`], the overlay
//! category composited into every collision read-path and surfaced to the render
//! layer — so a lock wall collides and draws exactly as it did when it lived in
//! the base, while the base stays immutable.

use ambition_engine_core as ae;
use bevy::prelude::*;

use super::{Encounter, EncounterLifecycle, EncounterPhase, EncounterWaves};
use crate::features::FeatureEcsWorldOverlay;

/// The lock-wall solid blocks wanted THIS frame: one per in-flight encounter
/// that has an authored `LockWall`. Block name format is
/// `lockwall:<encounter_id>` so the render layer can surface them as
/// `LockWallVisual` sprites (and a future per-id query can find them).
pub(in crate::encounter) fn desired_lock_wall_blocks<'a>(
    encounters: impl IntoIterator<Item = (&'a str, EncounterPhase, &'a super::EncounterSpec)>,
) -> Vec<ae::Block> {
    let mut blocks = Vec::new();
    for (id, phase, spec) in encounters {
        if !phase.locks_exits() {
            continue;
        }
        let Some(wall) = spec.lock_wall.as_ref() else {
            continue;
        };
        blocks.push(ae::Block::solid(
            format!("lockwall:{id}"),
            ae::Vec2::new(wall.min[0], wall.min[1]),
            ae::Vec2::new(wall.size[0], wall.size[1]),
        ));
    }
    blocks
}

/// Contribute the encounter lock walls to the per-frame collision overlay.
/// Runs in `WorldPrep` after [`crate::features::rebuild_feature_ecs_world_overlay`]
/// has cleared `gate_solids`, so the contribution is a clean per-frame derive of
/// the encounter entities' live phase — no base mutation, no reconcile.
pub fn contribute_encounter_lock_walls(
    encounters: Query<(&Encounter, &EncounterLifecycle, &EncounterWaves)>,
    mut overlay: ResMut<FeatureEcsWorldOverlay>,
) {
    overlay.gate_solids.extend(desired_lock_wall_blocks(
        encounters
            .iter()
            .map(|(enc, lifecycle, waves)| (enc.id.as_str(), lifecycle.phase, &waves.spec)),
    ));
}
