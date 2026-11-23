use std::collections::{BTreeMap, BTreeSet};

use super::media::LinkedMusicCue;
use super::{DAY_PHASE_KEYS, references::cue_reference_errors};
use crate::cue_spec::*;

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

pub(super) fn validation_errors(
    authored: &AuthoredMusicCues,
    linked: &[LinkedMusicCue],
) -> Vec<String> {
    let mut errors = cue_reference_errors(
        &authored.cues,
        linked,
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
        let linked = linked.iter().find(|linked| linked.spec.id == cue.id);
        let label = format!("cue {:?}", cue.id);
        check_level(&label, cue.relative_volume, 1.0, &mut errors);
        for (kind, empty) in [
            (
                "sections",
                linked.is_some_and(|linked| linked.sections.is_empty()),
            ),
            ("layers", cue.layers.is_empty()),
            ("states", cue.states.is_empty()),
        ] {
            if empty {
                errors.push(format!("{label}: no {kind}"));
            }
        }
        check_ids(
            &format!("{label} sections"),
            linked
                .into_iter()
                .flat_map(|linked| &linked.sections)
                .map(|section| section.id.as_str()),
            &mut errors,
        );
        let policy = cue.playback_policy();
        if policy.timing != crate::music_policy::MusicTimingRequirement::FrameDriven {
            errors.push(format!(
                "{label}: native music supports FrameDriven timing only; {:?} is not supported",
                policy.timing
            ));
        }
        for (field, value) in [
            (
                "section_crossfade_seconds",
                policy.section_crossfade_seconds,
            ),
            ("ending_crossfade_seconds", policy.ending_crossfade_seconds),
            (
                "continuation_crossfade_seconds",
                policy.continuation_crossfade_seconds,
            ),
            ("return_overlap_seconds", policy.return_overlap_seconds),
            (
                "ending_media_timeout_seconds",
                policy.ending_media_timeout_seconds,
            ),
        ] {
            if !value.is_finite() || value < 0.0 {
                errors.push(format!(
                    "{label}: playback.{field} must be finite and not negative"
                ));
            }
        }
        for (field, rule) in [
            ("section_exit", &policy.section_exit),
            ("ending_exit", &policy.ending_exit),
        ] {
            if field == "ending_exit" && cue.outro_state.is_none() {
                continue;
            }
            use crate::music_policy::MusicExitRule;
            if matches!(rule, MusicExitRule::Bars(0)) {
                errors.push(format!(
                    "{label}: playback.{field} bar spacing must be above zero"
                ));
            }
            if let MusicExitRule::Markers(ids) = rule {
                check_ids(
                    &format!("{label} playback.{field}"),
                    ids.iter().map(String::as_str),
                    &mut errors,
                );
                if ids.is_empty() {
                    errors.push(format!("{label}: playback.{field} needs an exit marker"));
                }
                if let Some(linked) = linked {
                    for id in ids {
                        if !linked
                            .sections
                            .iter()
                            .flat_map(|section| &section.markers)
                            .any(|marker| {
                                marker.id == *id
                                    && marker.role == crate::music_media::MusicMarkerRole::Exit
                            })
                        {
                            errors.push(format!(
                                "{label}: playback.{field} references unknown exit marker {id:?}"
                            ));
                        }
                    }
                }
            }
            for section in linked.into_iter().flat_map(|linked| &linked.sections) {
                let supported = match rule {
                    MusicExitRule::Immediate | MusicExitRule::AssetEnd => true,
                    MusicExitRule::MusicalEnd => section.musical_duration_seconds.is_some(),
                    MusicExitRule::Bars(bars) => {
                        *bars > 0
                            && !matches!(
                                section.clock,
                                crate::music_media::PreparedMusicClock::Unclocked
                            )
                    }
                    MusicExitRule::Markers(ids) => section.markers.iter().any(|marker| {
                        ids.contains(&marker.id)
                            && marker.role == crate::music_media::MusicMarkerRole::Exit
                    }),
                };
                if !supported {
                    errors.push(format!(
                        "{label} section {:?}: playback.{field} {rule:?} has no supported exit",
                        section.id
                    ));
                }
            }
        }
        check_ids(
            &format!("{label} states"),
            cue.states.iter().map(|state| state.id.as_str()),
            &mut errors,
        );
        for section in linked.into_iter().flat_map(|linked| &linked.sections) {
            let label = format!("{label} section {:?}", section.id);
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
            if let Some(section) = linked
                .into_iter()
                .flat_map(|linked| &linked.sections)
                .find(|section| section.id == state.section_id)
            {
                if section.looped && state.next_state.is_some() {
                    errors.push(format!("{label}: a loop cannot reach next_state"));
                }
                let full = linked.expect("linked section").mastered_full;
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

pub(super) fn layer_slots(layers: &[MusicLayerSpec]) -> Result<BTreeMap<&str, usize>, String> {
    let mut slots = BTreeMap::new();
    let mut occupied = BTreeSet::new();
    for layer in layers {
        if layer.id.is_empty()
            || layer.id.trim() != layer.id
            || slots.contains_key(layer.id.as_str())
        {
            return Err(format!("invalid or duplicate layer id {:?}", layer.id));
        }
        if layer.slot >= MAX_MUSIC_LAYERS || !occupied.insert(layer.slot) {
            return Err(format!(
                "layer {:?} has duplicate or out-of-range slot {}",
                layer.id, layer.slot
            ));
        }
        slots.insert(layer.id.as_str(), layer.slot);
    }
    Ok(slots)
}
