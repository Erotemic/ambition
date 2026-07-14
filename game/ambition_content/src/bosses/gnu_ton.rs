//! GNU-ton arena environment gating.
//!
//! Two arena hooks live here, both driven by the same "is the GNU-ton
//! boss alive?" check so they stay in lockstep:
//!
//! 1. **Ladder reveal.** The arena's retreat ladder is authored as a
//!    Climbable IntGrid column in `gnu_ton_arena_area.yaml`, so by
//!    default it's painted into `world.climbable_regions` the moment
//!    the room loads — which would let the player skip the fight by
//!    climbing right back out. This module hides the ladder while the
//!    boss is alive and re-adds it the frame the boss is defeated.
//!
//! 2. **Floor-gate above the ladder.** The entry ledge has a 48-px
//!    gap punched out above the ladder column; a named Solid
//!    (`ladder_floor_gate`) authored in LDtk fills that gap while the
//!    boss is alive and is removed from `world.blocks` on defeat, so
//!    the player can climb up the ladder and walk back to the exit
//!    door. The floor-gate uses the *opposite* polarity from the
//!    ladder (present-when-alive instead of absent-when-alive); both
//!    are intentionally driven from the same boss-alive check so a
//!    single gating system maintains both invariants.
//!
//! Gating is current-state-driven (any ECS boss with `is_gnu_ton() &&
//! !alive`) rather than persisted-encounter-driven. Dying mid-fight
//! resets the boss to alive, which correctly re-hides the ladder for
//! the next attempt. Cross-session persistence inherits whatever
//! state the boss runtime restores on respawn — no extra hookup here.

use ambition_engine_core as ae;
use bevy::prelude::*;

use ambition_actors::features::{BossClusterRef, FeatureEcsWorldOverlay};
use ambition_engine_core::RoomGeometry;

/// LDtk level identifier of the arena room whose ladder this system
/// gates. Held as a constant so it's grep-able alongside the matching
/// yaml at `tools/ambition_ldtk_tools/specs/gnu_ton_arena_area.yaml`.
const ARENA_ROOM_NAME: &str = "gnu_ton_arena";

/// Authored name of the named Solid block that fills the gap above
/// the ladder while the fight is live. Defined in the LDtk file as a
/// `Solid` entity with `fields.name = "ladder_floor_gate"`. Must
/// match `specs/gnu_ton/add_ladder_floor_gate.yaml`.
const FLOOR_GATE_BLOCK_NAME: &str = "ladder_floor_gate";

/// GNU-ton recognizer (id or authored display name). Lives content-side:
/// the generic cluster views no longer carry named-boss predicates.
///
/// The arena (ADR 0020 / G4) spawns the SPLIT pair: the encounter boss is the
/// `gnu_ton_rider` scholar riding the `giant_gnu` mount. The fused `gnu_ton`
/// profile it replaced was torn down in the E6 teardown (`refactor-chain.md`
/// R2), so the rider id — plus the display name a room author writes — is the
/// whole recognizer. Note the MOUNT is deliberately not matched: the giant dying
/// is a phase trigger, not the encounter ending.
fn boss_is_gnu_ton(boss: &ambition_actors::features::BossRef<'_>) -> bool {
    boss.config.behavior.id == "gnu_ton_rider"
        || boss.config.name.eq_ignore_ascii_case("gnu_ton")
        || boss.config.name.eq_ignore_ascii_case("gnu-ton")
}

/// Stateless arena-gate contributor. The authored base ALWAYS carries the
/// retreat ladders + the `ladder_floor_gate` Solid (immutable mid-room); this
/// system derives, each frame, which of them the collision *view* should hide,
/// from the current boss state — instead of mutating `RoomGeometry`:
///
/// - **Boss alive (or not yet spawned):** carve out the arena's Ladder regions
///   (so the player can't climb back out and skip the fight) and leave the
///   floor-gate solid.
/// - **Boss defeated:** stop carving the ladders (they reappear from the base)
///   and add the floor-gate block to `removed_block_names` so the gap opens and
///   the player can climb up to the exit.
///
/// Runs in `WorldPrep` after `rebuild_feature_ecs_world_overlay` clears the
/// overlay (same clean-slate-per-frame contract as the encounter / intro lock
/// walls). No per-visit `Local` state: a fresh room load swaps the immutable
/// base and the derive recomputes from scratch; dying mid-fight (boss back to
/// alive) re-hides the ladders automatically.
pub fn gate_gnu_ton_arena_ladder(
    world: ambition::platformer::lifecycle::SessionWorldRef<RoomGeometry>,
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
