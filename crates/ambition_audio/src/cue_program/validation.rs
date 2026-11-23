use std::collections::BTreeSet;

use crate::cue_spec::*;
use super::{DAY_PHASE_KEYS, references::cue_reference_errors};

fn check_ids<'a>(label: &str, ids: impl Iterator<Item = &'a str>, errors: &mut Vec<String>) {
    let mut seen = BTreeSet::new();
    for id in ids {
        if id.is_empty() || id.trim() != id {
            errors.push(format!(
                "{label}: id {id:?} is empty or has surrounding whitespace"
            ));
        }
        if !seen.insert(id) {
            errors.push(format!("{label}: duplicate id {id:?}"));
        }
    }
}

fn check_level(label: &str, value: f32, master: f32, errors: &mut Vec<String>) {
    if !value.is_finite() || value < 0.0 || !(value * master).is_finite() {
        errors.push(format!(
            "{label}: gain {value} must be finite, not negative, and finite after cue volume"
        ));
    }
}

pub(super) fn validation_errors(authored: &AuthoredMusicCues) -> Vec<String> {
    let mut errors = cue_reference_errors(
        &authored.cues,
        &authored.encounter_bindings,
        &authored.room_bindings,
        &authored.boss_bindings,
    );
    check_ids(
        "cues",
        authored.cues.iter().map(|cue| cue.id.as_str()),
        &mut errors,
    );
    check_ids(
        "encounter bindings",
        authored
            .encounter_bindings
            .iter()
            .map(|b| b.encounter_id.as_str()),
        &mut errors,
    );
    check_ids(
        "room bindings",
        authored.room_bindings.iter().map(|b| b.room_id.as_str()),
        &mut errors,
    );
    check_ids(
        "boss bindings",
        authored.boss_bindings.iter().map(|b| b.boss_id.as_str()),
        &mut errors,
    );
    for binding in &authored.boss_bindings {
        check_ids(
            &format!("boss {:?} phases", binding.boss_id),
            binding.phase_states.iter().map(|row| row.phase.as_str()),
            &mut errors,
        );
    }
    let mut phases = BTreeSet::new();
    for row in &authored.room_phase_tracks {
        if row.room_id.trim().is_empty()
            || row.room_id.trim() != row.room_id
            || row.track.trim().is_empty()
            || row.track.trim() != row.track
        {
            errors.push(format!("room phase track must name a room and track without surrounding whitespace: {row:?}"));
        }
        if !DAY_PHASE_KEYS.contains(&row.phase.as_str()) {
            errors.push(format!(
                "room {:?} names unknown day phase {:?}",
                row.room_id, row.phase
            ));
        }
        if !phases.insert((&row.room_id, &row.phase)) {
            errors.push(format!(
                "room {:?} has duplicate phase {:?}",
                row.room_id, row.phase
            ));
        }
    }
    for cue in &authored.cues {
        let label = format!("cue {:?}", cue.id);
        if !cue.bpm.is_finite()
            || cue.bpm <= 0.0
            || !cue.beats_per_bar.is_finite()
            || cue.beats_per_bar <= 0.0
            || !(cue.beats_per_bar * 60.0 / cue.bpm).is_finite()
            || cue.beats_per_bar * 60.0 / cue.bpm <= 0.0
        {
            errors.push(format!(
                "{label}: tempo and bar length must be finite and above zero"
            ));
        }
        check_level(&label, cue.relative_volume, 1.0, &mut errors);
        for (kind, empty) in [
            ("sections", cue.sections.is_empty()),
            ("layers", cue.layers.is_empty()),
            ("states", cue.states.is_empty()),
        ] {
            if empty {
                errors.push(format!("{label}: no {kind}"));
            }
        }
        check_ids(
            &format!("{label} sections"),
            cue.sections.iter().map(|section| section.id.as_str()),
            &mut errors,
        );
        check_ids(
            &format!("{label} layers"),
            cue.layers.iter().map(|layer| layer.id.as_str()),
            &mut errors,
        );
        check_ids(
            &format!("{label} states"),
            cue.states.iter().map(|state| state.id.as_str()),
            &mut errors,
        );
        let mut slots = BTreeSet::new();
        for layer in &cue.layers {
            if layer.slot >= MAX_MUSIC_LAYERS || !slots.insert(layer.slot) {
                errors.push(format!(
                    "{label}: layer {:?} has duplicate or out-of-range slot {}",
                    layer.id, layer.slot
                ));
            }
        }
        for section in &cue.sections {
            let label = format!("{label} section {:?}", section.id);
            let seconds = section.duration_beats * 60.0 / cue.bpm;
            if !section.duration_beats.is_finite()
                || section.duration_beats <= 0.0
                || !seconds.is_finite()
                || seconds <= 0.0
            {
                errors.push(format!("{label}: duration must be finite and above zero"));
            }
            if section.sources.is_empty() {
                errors.push(format!("{label}: no playable sources"));
            }
            check_ids(
                &format!("{label} sources"),
                section
                    .sources
                    .iter()
                    .map(|source| source.layer_id.as_str()),
                &mut errors,
            );
        }
        check_ids(
            &format!("{label} balance overrides"),
            cue.runtime_balance_overrides
                .iter()
                .map(|balance| balance.state_id.as_str()),
            &mut errors,
        );
        for balance in &cue.runtime_balance_overrides {
            if !cue.states.iter().any(|state| state.id == balance.state_id) {
                errors.push(format!(
                    "{label}: balance override references unknown state {:?}",
                    balance.state_id
                ));
            }
            check_ids(
                &format!("{label} balance {:?}", balance.state_id),
                balance.layer_gains.iter().map(|(layer, _)| layer.as_str()),
                &mut errors,
            );
            for (layer, gain) in &balance.layer_gains {
                if !cue.layers.iter().any(|known| known.id == *layer) {
                    errors.push(format!(
                        "{label}: balance override references unknown layer {layer:?}"
                    ));
                }
                check_level(&label, *gain, cue.relative_volume, &mut errors);
            }
        }
        for state in &cue.states {
            let label = format!("{label} state {:?}", state.id);
            check_ids(
                &format!("{label} gains"),
                state.gains.iter().map(|gain| gain.layer_id.as_str()),
                &mut errors,
            );
            for gain in &state.gains {
                check_level(&label, gain.gain, cue.relative_volume, &mut errors);
            }
            if let Some(section) = cue
                .sections
                .iter()
                .find(|section| section.id == state.section_id)
            {
                if section.looped && state.next_state.is_some() {
                    errors.push(format!("{label}: a loop cannot reach next_state"));
                }
                let full = cue.layers.len() == 1 && cue.layers[0].id == "full";
                let balance = cue
                    .runtime_balance_overrides
                    .iter()
                    .find(|balance| balance.state_id == state.id)
                    .filter(|_| !full);
                for layer in &cue.layers {
                    let gain = balance
                        .and_then(|balance| {
                            balance
                                .layer_gains
                                .iter()
                                .find(|(id, _)| *id == layer.id)
                                .map(|(_, gain)| *gain)
                        })
                        .or_else(|| {
                            state
                                .gains
                                .iter()
                                .find(|gain| gain.layer_id == layer.id)
                                .map(|gain| gain.gain)
                        })
                        .unwrap_or(0.0);
                    if gain > 0.0
                        && !section
                            .sources
                            .iter()
                            .any(|source| source.layer_id == layer.id)
                    {
                        errors.push(format!(
                            "{label}: audible layer {:?} has no source in section {:?}",
                            layer.id, section.id
                        ));
                    }
                }
            }
        }
    }
    errors
}
