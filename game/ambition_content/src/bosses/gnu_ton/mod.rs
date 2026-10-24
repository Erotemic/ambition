//! GNU-ton: the scholar who stands on the shoulders of a giant.
//!
//! * His conductor — it performs his live move with the gnu's fists — is the
//!   procedural module `ambition_content_modules::gnu_ton::conductor`, a
//!   conducted boss on the extension host (`ambition.boss.conduct`).
//! * [`conductor`] — what the game keeps here: his id, his birth, and a view
//!   of the module's record for tests and inspectors.
//! * [`duel`] — he fights on the giant's back with the moves of the fighter
//!   `npc_gnu_ton`: it plays the move his conductor chose on his body.
//! * [`portals`] — the portals of the fight, opened where his conductor's
//!   record says each line is.
//! * This file — the arena's exit gating.
//!
//! ## Exit gating
//!
//! The arena is one encounter. Its ladder and its `ladder_floor_gate` follow
//! the RECORDED OUTCOME: the ladder opens when every GNU-ton of the arena is
//! `Cleared` in the save (`boss.cleared`, the boss domain's one answer), and
//! not when a body is found dead. The members are the GNU-ton placements the
//! room authors, and any GNU-ton body that is live in the room. A corpse that
//! goes, a room that is entered again (a cleared placement is not built
//! again), and a rewind before the defeat each read the same record.

use ambition_platformer2d_core as ae;
use bevy::prelude::*;

use ambition_boss_encounter::BossClusterRef;
use ambition_platformer2d_core::RoomGeometry;

pub mod conductor;
pub mod duel;
pub mod prelude;
#[cfg(feature = "portal_render")]
pub mod fist_lines;

pub use conductor::{ascent_of, birth, conductor_of, Arm, Blow, Exit, GnuTonView, Move, GNU_TON_ID};

/// LDtk level identifier of the arena room whose ladder this system
/// gates. Held as a constant so it's grep-able alongside the matching
/// spec at `tools/ambition_ldtk_tools/specs/gnu_ton_arena_area.ron`.
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
    boss.config.behavior.id == GNU_TON_ID
        || boss.config.name.eq_ignore_ascii_case("gnu_ton")
        || boss.config.name.eq_ignore_ascii_case("gnu-ton")
}

/// Whether an authored boss placement is GNU-ton (the rider; the `giant_gnu`
/// mount is an actor, and its death is a phase of the fight).
fn placement_is_gnu_ton(
    spawn: &ambition_platformer2d_world::rooms::Authored<ambition_entity_catalog::placements::BossBrain>,
) -> bool {
    let scripted = matches!(
        &spawn.payload,
        ambition_entity_catalog::placements::BossBrain::PhaseScript { script_id } if script_id == GNU_TON_ID
    );
    scripted || spawn.name.eq_ignore_ascii_case("gnu_ton") || spawn.name.eq_ignore_ascii_case("gnu-ton")
}

/// Derive GNU-ton arena collision overlays from the recorded outcome, in
/// each live room that is the arena. Each live arena reads its own members.
/// Two live instances of the arena share its placements, so a defeat
/// recorded in one is the outcome of both: it is a fact of the world.
/// Run condition of [`gate_gnu_ton_arena_ladder`]: GNU-ton's arena is a live
/// room. The overlay is rebuilt each tick, so with no arena live the ladder
/// contributes nothing and need not run.
pub fn the_gnu_ton_arena_is_live(
    arenas: Query<
        &ambition_platformer2d_world::rooms::LiveRoomDefinition,
        With<ambition_platformer2d::platformer::lifecycle::RoomInstanceRoot>,
    >,
    room_set: Option<
        Single<
            &ambition_platformer2d_world::rooms::RoomSet,
            With<ambition_platformer2d_shared_tangle::lifecycle::SessionRoot>,
        >,
    >,
) -> bool {
    room_set.is_some_and(|rooms| arenas.iter().any(|definition| rooms.spec(*definition).id == ARENA_ROOM_NAME))
}

pub fn gate_gnu_ton_arena_ladder(
    arenas: Query<
        (
            &ambition_platformer2d::platformer::lifecycle::LiveRoomInstance,
            &RoomGeometry,
            &ambition_platformer2d_world::rooms::LiveRoomDefinition,
        ),
        With<ambition_platformer2d::platformer::lifecycle::RoomInstanceRoot>,
    >,
    room_set: Option<
        Single<
            &ambition_platformer2d_world::rooms::RoomSet,
            With<ambition_platformer2d_shared_tangle::lifecycle::SessionRoot>,
        >,
    >,
    save: Option<Res<ambition_persistence::save::AmbitionGameSave>>,
    live: ambition_platformer2d::platformer::lifecycle::LiveRooms,
    bosses: Query<(Entity, BossClusterRef)>,
    mut overlays: ambition_platformer2d::world::RoomOverlays,
) {
    let Some(rooms) = room_set.as_deref() else {
        return;
    };
    for (room, world, definition) in &arenas {
        // The room's identity is its spec id (the LDtk level identifier).
        // The geometry's name is a display name ("Ambition: gnu ton arena"):
        // a gate that compared it never ran in the shipped game.
        let spec = rooms.spec(*definition);
        if spec.id != ARENA_ROOM_NAME {
            continue;
        }
        let mut members: Vec<String> =
            spec.boss_spawns.iter().filter(|spawn| placement_is_gnu_ton(spawn)).map(|spawn| spawn.id.clone()).collect();
        for (entity, feature) in &bosses {
            let boss = feature.as_boss_ref();
            if live.of(entity) == Some(*room) && boss_is_gnu_ton(&boss) {
                members.push(boss.config.id.clone());
            }
        }
        members.sort_unstable();
        members.dedup();
        // No member is no outcome: a fight that is not built yet is not won.
        let completed = !members.is_empty()
            && save.as_deref().is_some_and(|save| {
                members.iter().all(|id| ambition_boss_encounter::placement_is_cleared(save.data(), id))
            });
        let Some(overlay) = overlays.for_room(Some(&ambition_platformer2d::platformer::lifecycle::InRoomInstance(*room))) else {
            continue;
        };
        gate_one_arena(world, completed, overlay);
    }
}

fn gate_one_arena(
    world: &RoomGeometry,
    completed: bool,
    mut overlay: Mut<ambition_platformer2d::world::FeatureEcsWorldOverlay>,
) {
    if completed {
        // Open the gap above the ladder so the player can climb back to the exit.
        overlay
            .removed_block_names
            .push(FLOOR_GATE_BLOCK_NAME.to_string());
        // Ladders: contribute no carve → they reappear from the immutable base.
    } else {
        // Hide every authored Ladder region while the fight is not won.
        for region in &world.0.climbable_regions {
            if region.kind == ae::ClimbableKind::Ladder {
                overlay.climbable_carves.push(region.aabb);
            }
        }
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
pub(crate) mod conductor_reference_tests;
#[cfg(test)]
mod conductor_parity_tests;
