//! GNU-ton arena exit gating.
//!
//! One current boss-alive check controls both sides of the retreat path: while
//! alive, the ladder is carved out and `ladder_floor_gate` remains solid; after
//! defeat, the ladder reappears and the floor gate is removed. The rule derives
//! from current ECS boss state each frame, so room resets recompute it naturally.

use ambition_platformer2d_core as ae;
use bevy::prelude::*;

use ambition_boss_encounter::BossClusterRef;
use ambition_platformer2d_core::RoomGeometry;
use ambition_platformer2d::world::FeatureEcsWorldOverlay;

/// LDtk level identifier of the arena room whose ladder this system
/// gates. Held as a constant so it's grep-able alongside the matching
/// yaml at `tools/ambition_ldtk_tools/specs/gnu_ton_arena_area.yaml`.
const ARENA_ROOM_NAME: &str = "gnu_ton_arena";

/// Authored name of the named Solid block that fills the gap above
/// the ladder while the fight is live. Defined in the LDtk file as a
/// `Solid` entity with `fields.name = "ladder_floor_gate"`. Must
/// match `specs/gnu_ton/add_ladder_floor_gate.yaml`.
const FLOOR_GATE_BLOCK_NAME: &str = "ladder_floor_gate";

/// Recognize the encounter rider by authored id/name. The `giant_gnu` mount is
/// excluded because its death is a phase transition, not encounter completion.
fn boss_is_gnu_ton(
    boss: &ambition_boss_encounter::BossRef<'_>,
) -> bool {
    boss.config.behavior.id == "gnu_ton_rider"
        || boss.config.name.eq_ignore_ascii_case("gnu_ton")
        || boss.config.name.eq_ignore_ascii_case("gnu-ton")
}

/// Derive GNU-ton arena collision overlays from current boss state.
///
/// The authored `RoomGeometry` remains immutable: while the boss is alive the
/// overlay hides ladder regions; after defeat it exposes those regions and removes
/// the named floor gate. `WorldPrep` rebuilds this overlay from scratch each
/// frame.
pub fn gate_gnu_ton_arena_ladder(
    world: ambition_platformer2d::platformer::lifecycle::SessionWorldRef<RoomGeometry>,
    bosses: Query<(BossClusterRef, &ambition_characters::actor::BodyHealth)>,
    mut overlay: ResMut<FeatureEcsWorldOverlay>,
) {
    if world.0.name != ARENA_ROOM_NAME {
        return;
    }
    // Defeat = an ECS gnu_ton boss observed `alive = false`. An empty query
    // (boss not yet spawned) is NOT defeat — the ladder stays hidden.
    let boss_defeated = bosses.iter().any(|(feature, health)| {
        let boss = feature.as_boss_ref();
        boss_is_gnu_ton(&boss) && !health.alive()
    });

    if boss_defeated {
        // Open the gap above the ladder so the player can climb back to the exit.
        overlay
            .removed_block_names
            .push(FLOOR_GATE_BLOCK_NAME.to_string());
        // Ladders: contribute no carve → they reappear from the immutable base.
    } else {
        // Hide every authored Ladder region while the fight is live.
        for region in &world.0.climbable_regions {
            if region.kind == ae::ClimbableKind::Ladder {
                overlay.climbable_carves.push(region.aabb);
            }
        }
    }
}

#[cfg(test)]
mod tests;
