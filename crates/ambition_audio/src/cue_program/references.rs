use crate::cue_spec::*;

/// Every reference in `cues` and `bindings` that names nothing: a state's
/// section, a gain's or a source's layer, a cue's outro and bridge states,
/// and a binding's cue and states. Also an empty source path, and a room
/// binding's entry state on a section that loops (the director would never
/// leave it). In the order given, so one input gives one report.
pub(super) fn cue_reference_errors<'a>(
    cues: impl IntoIterator<Item = &'a MusicCueSpec>,
    bindings: &[EncounterMusicBinding],
    room_bindings: &[RoomMusicBinding],
    boss_bindings: &[BossMusicBinding],
) -> Vec<String> {
    let cues: Vec<&MusicCueSpec> = cues.into_iter().collect();
    let mut errors = Vec::new();
    for cue in &cues {
        let sections = cue
            .sections
            .iter()
            .map(|section| section.id.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        let layers = cue
            .layers
            .iter()
            .map(|layer| layer.id.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        let states = cue
            .states
            .iter()
            .map(|state| state.id.as_str())
            .collect::<std::collections::BTreeSet<_>>();

        for state in &cue.states {
            if !sections.contains(state.section_id.as_str()) {
                errors.push(format!(
                    "cue '{}' state '{}' references unknown section '{}'",
                    cue.id, state.id, state.section_id
                ));
            }
            if let Some(next) = &state.next_state {
                if !states.contains(next.as_str()) {
                    errors.push(format!(
                        "cue '{}' state '{}' references unknown next state '{}'",
                        cue.id, state.id, next
                    ));
                }
            }

            for gain in &state.gains {
                if !layers.contains(gain.layer_id.as_str()) {
                    errors.push(format!(
                        "cue '{}' state '{}' references unknown layer '{}'",
                        cue.id, state.id, gain.layer_id
                    ));
                }
            }
        }

        for section in &cue.sections {
            for source in &section.sources {
                if !layers.contains(source.layer_id.as_str()) {
                    errors.push(format!(
                        "cue '{}' section '{}' references unknown layer '{}'",
                        cue.id, section.id, source.layer_id
                    ));
                }
                if source.path.trim().is_empty() {
                    errors.push(format!(
                        "cue '{}' section '{}' has an empty source path for layer '{}'",
                        cue.id, section.id, source.layer_id
                    ));
                }
            }
        }

        for (field, value) in [
            ("outro_state", cue.outro_state.as_ref()),
            (
                "post_clear_bridge_state",
                cue.post_clear_bridge_state.as_ref(),
            ),
        ] {
            if let Some(state_id) = value {
                if !states.contains(state_id.as_str()) {
                    errors.push(format!(
                        "cue '{}' {field} references unknown state '{}'",
                        cue.id, state_id
                    ));
                }
            }
        }
    }

    for binding in bindings {
        let Some(cue) = cues.iter().find(|cue| cue.id == binding.cue_id) else {
            errors.push(format!(
                "encounter binding '{}' references unknown cue '{}'",
                binding.encounter_id, binding.cue_id
            ));
            continue;
        };
        let states = cue
            .states
            .iter()
            .map(|state| state.id.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        for (field, state_id) in [
            ("starting_state", binding.starting_state.as_str()),
            ("cleared_state", binding.cleared_state.as_str()),
        ] {
            if !states.contains(state_id) {
                errors.push(format!(
                    "encounter binding '{}' {field} references unknown state '{}' on cue '{}'",
                    binding.encounter_id, state_id, binding.cue_id
                ));
            }
        }
        for state_id in &binding.wave_states {
            if !states.contains(state_id.as_str()) {
                errors.push(format!(
                    "encounter binding '{}' wave_states references unknown state '{}' on cue '{}'",
                    binding.encounter_id, state_id, binding.cue_id
                ));
            }
        }
        if let Some(state_id) = &binding.wave2_reinforced_state {
            if !states.contains(state_id.as_str()) {
                errors.push(format!(
                    "encounter binding '{}' wave2_reinforced_state references unknown state '{}' on cue '{}'",
                    binding.encounter_id, state_id, binding.cue_id
                ));
            }
        }
    }

    for binding in room_bindings {
        let Some(cue) = cues.iter().find(|cue| cue.id == binding.cue_id) else {
            errors.push(format!(
                "room binding '{}' references unknown cue '{}'",
                binding.room_id, binding.cue_id
            ));
            continue;
        };
        for (field, state_id) in [
            ("state", Some(&binding.state)),
            ("entry_state", binding.entry_state.as_ref()),
            ("fight_state", binding.fight_state.as_ref()),
        ] {
            let Some(state_id) = state_id else { continue };
            let Some(state) = cue.states.iter().find(|state| &state.id == state_id) else {
                errors.push(format!(
                    "room binding '{}' {field} references unknown state '{}' on cue '{}'",
                    binding.room_id, state_id, binding.cue_id
                ));
                continue;
            };
            let loops = cue
                .sections
                .iter()
                .any(|section| section.id == state.section_id && section.looped);
            if field == "entry_state" && loops {
                errors.push(format!(
                    "room binding '{}' entry_state '{}' is on a section of cue '{}' that loops",
                    binding.room_id, state_id, binding.cue_id
                ));
            }
        }
    }

    for binding in boss_bindings {
        let Some(cue) = cues.iter().find(|cue| cue.id == binding.cue_id) else {
            errors.push(format!(
                "boss binding '{}' references unknown cue '{}'",
                binding.boss_id, binding.cue_id
            ));
            continue;
        };
        for row in &binding.phase_states {
            if !BOSS_PHASE_KEYS.contains(&row.phase.as_str()) {
                errors.push(format!(
                    "boss binding '{}' names unknown phase '{}' (one of {:?})",
                    binding.boss_id, row.phase, BOSS_PHASE_KEYS
                ));
            }
            if !cue.states.iter().any(|state| state.id == row.state) {
                errors.push(format!(
                    "boss binding '{}' phase '{}' references unknown state '{}' on cue '{}'",
                    binding.boss_id, row.phase, row.state, binding.cue_id
                ));
            }
        }
    }
    errors
}
