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

use crate::features::GameplayEffect;

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
    (
        "alice_route_note_reported",
        "private_routes_compromised",
    ),
];

/// Watches the save layer for any chained trigger and emits the target
/// flag through the standard `GameplayEffect::SetFlag` bus. Runs every
/// frame; cost is O(chains × set-flag-lookups) and the chain table is
/// expected to stay under a few dozen entries.
pub fn emit_intro_flag_chains(
    save: Res<crate::persistence::save::SandboxSave>,
    mut effects: MessageWriter<GameplayEffect>,
) {
    let data = save.data();
    for (trigger, target) in INTRO_FLAG_CHAINS.iter().copied() {
        if data.flag(trigger) && !data.flag(target) {
            effects.write(GameplayEffect::SetFlag {
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

/// Per-frame dialog redirector. Mirrors the pirate-cove pattern
/// (`dialog::redirect_post_quest_dialog`) but for intro NPCs whose
/// post-state lines are swapped in based on save flags rather than
/// boss death. Wired into `IntroPlugin::build` to run alongside the
/// flag-chain and lock-wall syncs.
pub fn redirect_post_intro_dialog(
    mut dialogue: ResMut<crate::dialog::DialogState>,
    save: Option<Res<crate::persistence::save::SandboxSave>>,
) {
    if !dialogue.active() {
        return;
    }
    let Some(save) = save else { return };
    let data = save.data();
    use crate::dialog::DialogMode;
    use crate::intro::dialog::IntroDialog;
    let new_intro = match dialogue.mode() {
        DialogMode::Intro(IntroDialog::OilerIntro) if data.flag("p1_stabilizer_received") => {
            Some(IntroDialog::OilerPostStabilizer)
        }
        DialogMode::Intro(IntroDialog::AliceIntroStub)
            if data.flag("bob_field_survey_received") =>
        {
            Some(IntroDialog::AliceAfterBobSurvey)
        }
        DialogMode::Intro(IntroDialog::BobIntroStub)
            if data.flag("alice_route_note_reported") =>
        {
            Some(IntroDialog::BobAfterReport)
        }
        _ => None,
    };
    if let Some(intro) = new_intro {
        dialogue.set_mode(DialogMode::Intro(intro));
    }
}

/// Per-frame sync of the intro flag-gated lock walls. Mirrors the
/// encounter system's `sync_lock_walls` but driven by the save layer
/// rather than encounter phase.
pub fn sync_intro_flag_gated_lock_walls(
    project: Option<Res<crate::world::ldtk_world::SandboxLdtkProject>>,
    room_set: Option<Res<crate::rooms::RoomSet>>,
    save: Option<Res<crate::persistence::save::SandboxSave>>,
    world: Option<ResMut<crate::GameWorld>>,
) {
    let (Some(project), Some(room_set), Some(save), Some(mut world)) =
        (project, room_set, save, world)
    else {
        return;
    };
    let active_room_id = room_set.active_spec().id.clone();
    let save_data = save.data();

    let mut desired: std::collections::BTreeMap<
        String,
        (ambition_engine::Vec2, ambition_engine::Vec2),
    > = std::collections::BTreeMap::new();
    for level in &project.0.levels {
        if level.active_area() != active_room_id {
            continue;
        }
        let Some(layer) = level.ambition_layer() else {
            continue;
        };
        for entity in &layer.entity_instances {
            if entity.identifier != "LockWall" {
                continue;
            }
            let Some(id) = crate::world::ldtk_world::field_string(entity, "id") else {
                continue;
            };
            let id_trim = id.trim();
            let Some((_, flag)) = INTRO_FLAG_GATED_LOCK_WALLS
                .iter()
                .find(|(lock, _)| *lock == id_trim)
            else {
                continue;
            };
            if save_data.flag(flag) {
                continue;
            }
            let min = ambition_engine::Vec2::new(entity.px[0] as f32, entity.px[1] as f32);
            let size = ambition_engine::Vec2::new(entity.width as f32, entity.height as f32);
            desired.insert(id_trim.to_string(), (min, size));
        }
    }

    world.0.blocks.retain(|b| {
        if let Some(stripped) = b.name.strip_prefix("intro_lock:") {
            desired.contains_key(stripped)
        } else {
            true
        }
    });

    for (id, (min, size)) in desired {
        let name = format!("intro_lock:{id}");
        if !world.0.blocks.iter().any(|b| b.name == name) {
            world.0.blocks.push(ambition_engine::Block::solid(name, min, size));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_table_has_no_duplicate_triggers() {
        // Two chains with the same trigger would emit redundant SetFlag
        // effects every frame. Forbid that at compile-time-style check.
        let mut triggers = std::collections::BTreeSet::new();
        for (trigger, _target) in INTRO_FLAG_CHAINS.iter().copied() {
            assert!(
                triggers.insert(trigger),
                "duplicate trigger in INTRO_FLAG_CHAINS: {trigger}"
            );
        }
    }

    #[test]
    fn chain_table_has_no_trigger_equals_target() {
        for (trigger, target) in INTRO_FLAG_CHAINS.iter().copied() {
            assert_ne!(trigger, target, "chain trigger == target: {trigger}");
        }
    }

    #[test]
    fn flag_gated_lock_walls_have_unique_ids() {
        let mut ids = std::collections::BTreeSet::new();
        for (lock_id, _flag) in INTRO_FLAG_GATED_LOCK_WALLS.iter().copied() {
            assert!(
                ids.insert(lock_id),
                "duplicate LockWall id in INTRO_FLAG_GATED_LOCK_WALLS: {lock_id}"
            );
        }
    }
}
