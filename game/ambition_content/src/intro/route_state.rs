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
    save: Res<ambition_persistence::save::AmbitionGameSave>,
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

// The pair now lives on the LDtk entity as a `gated_by` field, and the system
// that reads it is `ambition_platformer2d_actor_monolith::world::gated_lock_walls`
// — an ENGINE system, so every game gets flag-gated walls without Rust. It asks
// the `world.flag_set` condition through the shared catalog rather than reading
// the save, which is what lets a later wall be gated on something that is not a
// flag at all.
//
// do not reintroduce a table here. Adding a gated wall is an LDtk edit.

#[cfg(test)]
mod tests;
