use super::*;
use crate::encounter::{EncounterPhase, EncounterRegistry, EncounterRun};

fn binding(encounter_id: &str, cue_id: &str) -> EncounterMusicBinding {
    EncounterMusicBinding {
        encounter_id: encounter_id.to_string(),
        cue_id: cue_id.to_string(),
        starting_state: "intro".to_string(),
        wave_states: vec!["wave1".into(), "wave2".into(), "wave3".into()],
        wave2_reinforced_state: Some("wave2_brute".into()),
        cleared_state: "outro".to_string(),
    }
}

fn director_with_active_cue(cue_id: Option<&str>) -> MusicDirectorState {
    let mut s = MusicDirectorState::default();
    s.active_cue_id = cue_id.map(|c| c.to_string());
    // AdaptiveLoop: a non-Idle / non-Finished mode so the
    // Inactive-with-active-cue branch can fire.
    s.mode = MusicDirectorMode::AdaptiveLoop;
    s
}

fn registry_with_phase(encounter_id: &str, phase: EncounterPhase) -> EncounterRegistry {
    let mut registry = EncounterRegistry::default();
    let state = registry.ensure(encounter_id);
    state.phase = phase;
    registry
}

#[test]
fn unknown_encounter_with_inactive_cue_returns_none() {
    let registry = EncounterRegistry::default();
    let director = director_with_active_cue(None);
    let bind = binding("nonexistent", "first_goblin_tune_v2");
    assert!(resolve_directive_for_binding(&bind, &registry, &director).is_none());
}

#[test]
fn unknown_encounter_with_active_cue_returns_stop_now() {
    let registry = EncounterRegistry::default();
    // The cue is currently playing for an encounter that no longer
    // exists in the registry — the resolver should stop it.
    let director = director_with_active_cue(Some("first_goblin_tune_v2"));
    let bind = binding("nonexistent", "first_goblin_tune_v2");
    assert_eq!(
        resolve_directive_for_binding(&bind, &registry, &director),
        Some(AdaptiveCueDirective::StopNow)
    );
}

#[test]
fn starting_phase_returns_starting_state_play() {
    let registry = registry_with_phase(
        "goblin_encounter",
        EncounterPhase::Starting { remaining: 1.0 },
    );
    let director = director_with_active_cue(None);
    let bind = binding("goblin_encounter", "first_goblin_tune_v2");
    assert_eq!(
        resolve_directive_for_binding(&bind, &registry, &director),
        Some(AdaptiveCueDirective::Play {
            cue_id: "first_goblin_tune_v2".into(),
            state_id: "intro".into(),
        })
    );
}

#[test]
fn active_phase_uses_wave_state_by_index() {
    let registry = registry_with_phase(
        "goblin_encounter",
        EncounterPhase::Active {
            wave_index: 2,
            remaining_mobs: 1,
        },
    );
    let director = director_with_active_cue(None);
    let bind = binding("goblin_encounter", "first_goblin_tune_v2");
    assert_eq!(
        resolve_directive_for_binding(&bind, &registry, &director),
        Some(AdaptiveCueDirective::Play {
            cue_id: "first_goblin_tune_v2".into(),
            state_id: "wave3".into(),
        })
    );
}

#[test]
fn cleared_phase_returns_cleared_state_play() {
    let registry = registry_with_phase("goblin_encounter", EncounterPhase::Cleared);
    let director = director_with_active_cue(None);
    let bind = binding("goblin_encounter", "first_goblin_tune_v2");
    assert_eq!(
        resolve_directive_for_binding(&bind, &registry, &director),
        Some(AdaptiveCueDirective::Play {
            cue_id: "first_goblin_tune_v2".into(),
            state_id: "outro".into(),
        })
    );
}

#[test]
fn failed_phase_returns_stop_now() {
    let registry = registry_with_phase("goblin_encounter", EncounterPhase::Failed);
    let director = director_with_active_cue(None);
    let bind = binding("goblin_encounter", "first_goblin_tune_v2");
    assert_eq!(
        resolve_directive_for_binding(&bind, &registry, &director),
        Some(AdaptiveCueDirective::StopNow)
    );
}

#[test]
fn active_phase_wave2_promotes_to_reinforced_after_brute_delay() {
    let mut registry = EncounterRegistry::default();
    let state = registry.ensure("goblin_encounter");
    state.phase = EncounterPhase::Active {
        wave_index: 1,
        remaining_mobs: 3,
    };
    // Simulate enough wave_elapsed time to trigger the
    // wave2_reinforced_state promotion (LARGE_BRUTE_DELAY_SECONDS
    // is the threshold).
    state.run = EncounterRun {
        wave_elapsed: LARGE_BRUTE_DELAY_SECONDS + 0.1,
        ..Default::default()
    };
    let director = director_with_active_cue(None);
    let bind = binding("goblin_encounter", "first_goblin_tune_v2");
    assert_eq!(
        resolve_directive_for_binding(&bind, &registry, &director),
        Some(AdaptiveCueDirective::Play {
            cue_id: "first_goblin_tune_v2".into(),
            state_id: "wave2_brute".into(),
        })
    );
}

#[test]
fn resolver_iterates_multiple_bindings() {
    // Catalog with two bindings; only the second is in flight. Build a minimal
    // base catalog inline (the authored goblin catalog is content now); this
    // resolver unit test only needs a catalog carrying the goblin binding.
    let mut catalog = MusicCueCatalog::from_parts(
        Vec::new(),
        vec![EncounterMusicBinding {
            encounter_id: "goblin_encounter".into(),
            cue_id: "first_goblin_tune_v2".into(),
            starting_state: "intro".into(),
            wave_states: vec!["wave1".into()],
            wave2_reinforced_state: None,
            cleared_state: "outro".into(),
        }],
    );
    catalog.add_encounter_binding(EncounterMusicBinding {
        encounter_id: "imaginary_arena".into(),
        cue_id: "imaginary_cue".into(),
        starting_state: "intro".into(),
        wave_states: vec!["w1".into()],
        wave2_reinforced_state: None,
        cleared_state: "outro".into(),
    });
    let mut registry = EncounterRegistry::default();
    let state = registry.ensure("imaginary_arena");
    state.phase = EncounterPhase::Cleared;
    let director = MusicDirectorState::default();
    // goblin_encounter binding has no encounter; imaginary_arena binding
    // is Cleared. The resolver iterates and returns the second
    // binding's Play directive.
    let result = resolve_adaptive_directive(&catalog, &registry, &director);
    assert!(matches!(result, Some(AdaptiveCueDirective::Play { .. })));
}

// ---- should_restart_adaptive: encounter-restart-during-outro race ----
//
// Jon's 2026-05-09 report: "started the goblin encounter, beat it,
// but also died at the same time, which reset me back to the start.
// I reset and restarted the goblin encounter, so maybe the timed
// trigger to restart the lofi music happened and then the trigger
// to start the goblin music happened (because i reset the
// encounter), so they both played at the same time."
//
// The race is in `drive_adaptive_cue_state`: when the encounter
// restarts during the outro overlap window (after `drive_outro_tail`
// has started the base lofi channel for the return-to-room overlap
// but before the outro's full duration expires), the director still
// has `active_cue_id = Some(goblin)` and the cue id matches the
// new directive. The pre-fix predicate skipped the
// stop-base-channel + restart-adaptive path on a same-cue match,
// leaving lofi playing alongside the rebuilt adaptive layers.
//
// The fix preserves the invariant
//   `simple base track playing ⇒ no adaptive layers audible`
// by additionally restarting the adaptive cue from its intro when
// the mode says a simple base track is currently audible OR the
// director is in `AdaptiveOutro` and the directive's target state
// is no longer the outro section.
