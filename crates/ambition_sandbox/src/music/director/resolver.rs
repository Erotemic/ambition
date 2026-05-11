use super::*;

pub(in crate::music) fn resolve_adaptive_directive(
    catalog: &MusicCueCatalog,
    encounters: &EncounterRegistry,
    director: &MusicDirectorState,
) -> Option<AdaptiveCueDirective> {
    // First binding with a live directive wins. Iterating the catalog
    // (rather than hardcoding mob_lab) lets future encounter cues
    // (boss / mini-boss) drop in by adding a binding without touching
    // this resolver.
    for binding in &catalog.encounter_bindings {
        if let Some(directive) = resolve_directive_for_binding(binding, encounters, director) {
            return Some(directive);
        }
    }
    None
}

/// Resolve a single encounter binding's adaptive cue directive.
/// Returns:
/// - `Some(StopNow)` when the encounter no longer exists but the
///   binding's cue is still active.
/// - `Some(Play { cue_id, state_id })` when the encounter is in a
///   phase mapped to a cue state.
/// - `None` when the encounter is unknown / inactive AND its cue is
///   not playing — the binding doesn't claim audio this frame.
pub(in crate::music) fn resolve_directive_for_binding(
    binding: &EncounterMusicBinding,
    encounters: &EncounterRegistry,
    director: &MusicDirectorState,
) -> Option<AdaptiveCueDirective> {
    let cue_active = director.active_cue_id.as_deref() == Some(binding.cue_id.as_str());
    let Some(encounter) = encounters.get(&binding.encounter_id) else {
        if cue_active {
            return Some(AdaptiveCueDirective::StopNow);
        }
        return None;
    };

    match encounter.phase {
        EncounterPhase::Starting { .. } => Some(AdaptiveCueDirective::Play {
            cue_id: binding.cue_id.clone(),
            state_id: binding.starting_state.clone(),
        }),
        EncounterPhase::Active { wave_index, .. } => {
            let mut state_id = binding
                .wave_states
                .get(wave_index)
                .or_else(|| binding.wave_states.last())
                .cloned()
                .unwrap_or_else(|| binding.starting_state.clone());
            if wave_index == 1 && encounter.run.wave_elapsed >= LARGE_BRUTE_DELAY_SECONDS {
                if let Some(reinforced) = &binding.wave2_reinforced_state {
                    state_id = reinforced.clone();
                }
            }
            Some(AdaptiveCueDirective::Play {
                cue_id: binding.cue_id.clone(),
                state_id,
            })
        }
        EncounterPhase::Cleared => Some(AdaptiveCueDirective::Play {
            cue_id: binding.cue_id.clone(),
            state_id: binding.cleared_state.clone(),
        }),
        EncounterPhase::Inactive => {
            // The encounter often resets to Inactive immediately after clear;
            // if this adaptive cue is already active, continue into its outro
            // instead of hard-cutting to room music.
            if cue_active
                && director.mode != MusicDirectorMode::AdaptiveFinished
                && director.mode != MusicDirectorMode::Idle
            {
                Some(AdaptiveCueDirective::Play {
                    cue_id: binding.cue_id.clone(),
                    state_id: binding.cleared_state.clone(),
                })
            } else {
                None
            }
        }
        EncounterPhase::Failed => Some(AdaptiveCueDirective::StopNow),
    }
}
