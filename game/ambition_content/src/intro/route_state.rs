//! Derived narrative save flags for the intro route.
//!
//! After flag effects are applied, this system emits missing target flags from
//! the static chain table. Targets then flow through the ordinary flag-effect
//! path on the next frame, including quest notifications. The operation is
//! idempotent because an already-present target is skipped.

use bevy::prelude::*;

use ambition_combat::SetFlagRequested;

/// `(trigger_flag, target_flag)` pairs for derived intro save state.
pub const INTRO_FLAG_CHAINS: &[(&str, &str)] = &[
    // Bob's survey reveals private map marks.
    ("bob_field_survey_received", "map_private_marks_unlocked"),
    // The system-boss reward records route memory.
    ("intro_p5_route_memory_received", "route_memory_received"),
    // Alice's route note enables basic map awareness.
    ("alice_route_note_carried", "map_basic_unlocked"),
    // Reporting Alice's route note records both the report and its consequence.
    (
        "switch_gate_official_report_used",
        "alice_route_note_reported",
    ),
    ("alice_route_note_reported", "private_routes_compromised"),
];

/// Emit target flags whose triggers are present and whose targets are absent.
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

// Flag-gated walls are authored through LDtk `gated_by`; do not duplicate
// those relationships in this narrative chain table.

#[cfg(test)]
mod tests;
