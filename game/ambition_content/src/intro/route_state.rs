//! Intro-v1 route-state chained flags.
//!
//! When the player picks up certain narrative pickups (Bob's field survey,
//! the system boss's P5 reward, etc.) the slice wants secondary flags to
//! flip too — `map_private_marks_unlocked`, `route_memory_received`, and
//! similar map-layer hooks that downstream listeners can subscribe to
//! without watching for the specific source flag.
//!
//! Implemented as a tiny system that runs after [`apply_flag_effects`] each
//! frame: it reads the save layer, walks the static [`INTRO_FLAG_CHAINS`]
//! table, and emits a fresh `GameplayEffect::SetFlag` for any chained flag
//! whose trigger is set but whose target is still missing. The chained
//! emission then flows through `apply_flag_effects` next frame, which
//! writes it to save and pushes a `QuestAdvanceEvent::FlagSet` so quest
//! steps that listen on the chained flag advance automatically.
//!
//! Keeping the chain as a const data table (not a switch arm in
//! `apply_flag_effects`) means new intro chains are one-line edits and the
//! bus stays generic.
//!
//! The system is idempotent: the second time it observes a trigger that
//! has already set its target it sees the target flag present and skips.

use bevy::prelude::*;

use ambition_combat::SetFlagRequested;

/// `(trigger_flag, target_flag)` — when the trigger lands in the save
/// layer, the system emits a SetFlag for the target. Targets are listed
/// in playtest-handoff.md §"What remains placeholder" so the next agent
/// can grep both ends in one read.
pub const INTRO_FLAG_CHAINS: &[(&str, &str)] = &[
    // Bob's field survey reveals private map marks the player can read
    // back. Wired here so Task 04's narrative beat surfaces a concrete
    // downstream flag without the cartography quest having to carry the
    // entire reveal payload.
    ("bob_field_survey_received", "map_private_marks_unlocked"),
    // The P5 reward (collected in first_system_boss) imprints route
    // memory: the world remembers which routes the player cleared,
    // which Task 09+ visualizations / dialogue branches can consume.
    ("intro_p5_route_memory_received", "route_memory_received"),
    // Picking up Alice's sealed route note also turns on basic map
    // awareness so a future minimap layer has a flag to gate on.
    ("alice_route_note_carried", "map_basic_unlocked"),
    // Evil/lawful report route (Script C in playtest-handoff.md).
    // Activating the `gate_official_report` Switch in
    // gate_stack_lower sets `switch_gate_official_report_used` (the
    // standard interact-system pattern). This chain promotes that to
    // the canonical `alice_route_note_reported` and then to
    // `private_routes_compromised` so a single Switch toggle
    // produces a coherent save-state record of the report path.
    (
        "switch_gate_official_report_used",
        "alice_route_note_reported",
    ),
    ("alice_route_note_reported", "private_routes_compromised"),
];

/// Watches the save layer for any chained trigger and emits the target
/// flag through the standard `GameplayEffect::SetFlag` bus. Runs every
/// frame; cost is O(chains × set-flag-lookups) and the chain table is
/// expected to stay under a few dozen entries.
pub fn emit_intro_flag_chains(
    save: Res<ambition_persistence::save::SandboxSave>,
    mut effects: MessageWriter<SetFlagRequested>,
) {
    let data = save.data();
    for (trigger, target) in INTRO_FLAG_CHAINS.iter().copied() {
        if data.flag(trigger) && !data.flag(target) {
            effects.write(SetFlagRequested {
                id: target.to_string(),
                on: true,
            });
        }
    }
}

/// LockWalls in the intro slice whose collision should be removed
/// once the named flag is set in save. Each entry is
/// `(lock_id_on_LockWall_entity, unlock_flag)`.
///
/// LockWalls without an associated EncounterTrigger are inert in the
/// stock runtime — the entity exists in LDtk but no system inserts a
/// blocking solid into the engine's `world.blocks`. The system below
/// reads from this table to provide that wiring for the cartography
/// route: while the unlock flag is clear, an `intro_lock:<id>` solid
/// block is inserted in the active room; once the flag flips, the
/// block is removed and the player can walk through.
pub const INTRO_FLAG_GATED_LOCK_WALLS: &[(&str, &str)] = &[
    ("alice_private_return_lock", "bob_field_survey_received"),
    ("gate_alice_private_lock", "bob_field_survey_received"),
];

// The intro-specific dialog redirect table moved into the unified
// data-driven registry at `assets/data/dialogue/registry.ron`
// alongside the sandbox redirects. The boss/flag gate predicates are
// the same shape; `dialog::redirect_post_quest_dialog` now walks both
// families in one pass. Adding a new intro flag swap is a one-row
// edit in the RON file.

/// Pure computational core of [`sync_intro_flag_gated_lock_walls`].
/// Given the LDtk project, the active room id, and a save snapshot,
/// returns the (lock_id, min, size) triples that should be present
/// as `intro_lock:<id>` blocks this frame. Extracted so the Bevy
/// system can be tested without spinning up a full ECS world.
pub fn compute_intro_flag_gated_lock_walls(
    project: &ambition_actors::world::ldtk_world::LdtkProject,
    active_room_id: &str,
    save: &ambition_persistence::save_data::SandboxSaveData,
) -> Vec<(
    String,
    ambition_engine_core::Vec2,
    ambition_engine_core::Vec2,
)> {
    let mut out: Vec<(
        String,
        ambition_engine_core::Vec2,
        ambition_engine_core::Vec2,
    )> = Vec::new();
    for level in &project.levels {
        if level.active_area() != active_room_id {
            continue;
        }
        for entity in level.all_entity_instances() {
            if entity.identifier != "LockWall" {
                continue;
            }
            let Some(id) = ambition_actors::world::ldtk_world::field_string(entity, "id") else {
                continue;
            };
            let id_trim = id.trim();
            let Some((_, flag)) = INTRO_FLAG_GATED_LOCK_WALLS
                .iter()
                .find(|(lock, _)| *lock == id_trim)
            else {
                continue;
            };
            if save.flag(flag) {
                continue;
            }
            let min = ambition_engine_core::Vec2::new(entity.px[0] as f32, entity.px[1] as f32);
            let size = ambition_engine_core::Vec2::new(entity.width as f32, entity.height as f32);
            out.push((id_trim.to_string(), min, size));
        }
    }
    out
}

/// Per-frame contribution of the intro flag-gated lock walls onto the
/// collision overlay's `gate_solids`. Mirrors the encounter system's
/// `contribute_encounter_lock_walls` but driven by the save layer rather than
/// encounter phase: the walls are derived each frame and folded into the
/// per-frame overlay, never mutated into the authored `RoomGeometry` base (the
/// resolved authored-base model). Delegates the LDtk-walking policy to
/// [`compute_intro_flag_gated_lock_walls`] so it stays testable in isolation.
///
/// Runs in `WorldPrep` after the overlay rebuild clears `gate_solids` (so the
/// contribution is a clean per-frame derive) and before the WorldPrep collision
/// consumers. The `intro_lock:<id>` block name lets the render layer surface
/// each wall as a `LockWallVisual`, same as encounter lock walls.
pub fn sync_intro_flag_gated_lock_walls(
    project: Option<Res<ambition_actors::world::ldtk_world::SandboxLdtkProject>>,
    room_set: Option<ambition::platformer::lifecycle::SessionWorldRef<ambition_actors::rooms::RoomSet>>,
    save: Option<Res<ambition_persistence::save::SandboxSave>>,
    overlay: Option<
        ResMut<ambition_platformer_primitives::feature_overlay::FeatureEcsWorldOverlay>,
    >,
) {
    let (Some(project), Some(room_set), Some(save), Some(mut overlay)) =
        (project, room_set, save, overlay)
    else {
        return;
    };
    let active_room_id = room_set.active_spec().id.clone();
    let desired = compute_intro_flag_gated_lock_walls(&project.0, &active_room_id, save.data());
    for (id, min, size) in desired {
        overlay.gate_solids.push(ambition_engine_core::Block::solid(
            format!("intro_lock:{id}"),
            min,
            size,
        ));
    }
}

#[cfg(test)]
mod tests;
