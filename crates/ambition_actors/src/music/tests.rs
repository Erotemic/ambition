//! Unit tests for the encounter→adaptive-cue resolver (`intent.rs`):
//! per-binding directive resolution across encounter phases (starting /
//! active-wave / wave-2 reinforced promotion / cleared / failed / unknown),
//! multi-binding iteration, and notes on the outro/restart race fix.

use super::*;
use crate::encounter::{EncounterPhase, EncounterRun, EncounterState};
use std::collections::HashMap;

/// A live encounter state fixture at `phase` (E1: the resolvers read encounter
/// entities via an `id -> &EncounterState` lookup, not the registry).
fn state_with_phase(phase: EncounterPhase) -> EncounterState {
    EncounterState {
        phase,
        ..Default::default()
    }
}

/// A single-entry `id -> &EncounterState` lookup, matching what
/// `compute_music_intent` builds from the encounter entities.
fn lookup<'a>(id: &'a str, state: &'a EncounterState) -> HashMap<&'a str, &'a EncounterState> {
    HashMap::from([(id, state)])
}

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

#[test]
fn unknown_encounter_with_inactive_cue_returns_none() {
    let states: HashMap<&str, &EncounterState> = HashMap::new();
    let director = director_with_active_cue(None);
    let bind = binding("nonexistent", "first_goblin_tune_v2");
    assert!(resolve_directive_for_binding(&bind, &states, &director).is_none());
}

#[test]
fn unknown_encounter_with_active_cue_returns_stop_now() {
    let states: HashMap<&str, &EncounterState> = HashMap::new();
    // The cue is currently playing for an encounter that no longer
    // exists — the resolver should stop it.
    let director = director_with_active_cue(Some("first_goblin_tune_v2"));
    let bind = binding("nonexistent", "first_goblin_tune_v2");
    assert_eq!(
        resolve_directive_for_binding(&bind, &states, &director),
        Some(AdaptiveCueDirective::StopNow)
    );
}

#[test]
fn starting_phase_returns_starting_state_play() {
    let state = state_with_phase(EncounterPhase::Starting { remaining: 1.0 });
    let states = lookup("goblin_encounter", &state);
    let director = director_with_active_cue(None);
    let bind = binding("goblin_encounter", "first_goblin_tune_v2");
    assert_eq!(
        resolve_directive_for_binding(&bind, &states, &director),
        Some(AdaptiveCueDirective::Play {
            cue_id: "first_goblin_tune_v2".into(),
            state_id: "intro".into(),
        })
    );
}

#[test]
fn active_phase_uses_wave_state_by_index() {
    let state = state_with_phase(EncounterPhase::Active {
        wave_index: 2,
        remaining_mobs: 1,
    });
    let states = lookup("goblin_encounter", &state);
    let director = director_with_active_cue(None);
    let bind = binding("goblin_encounter", "first_goblin_tune_v2");
    assert_eq!(
        resolve_directive_for_binding(&bind, &states, &director),
        Some(AdaptiveCueDirective::Play {
            cue_id: "first_goblin_tune_v2".into(),
            state_id: "wave3".into(),
        })
    );
}

#[test]
fn cleared_phase_returns_cleared_state_play() {
    let state = state_with_phase(EncounterPhase::Cleared);
    let states = lookup("goblin_encounter", &state);
    let director = director_with_active_cue(None);
    let bind = binding("goblin_encounter", "first_goblin_tune_v2");
    assert_eq!(
        resolve_directive_for_binding(&bind, &states, &director),
        Some(AdaptiveCueDirective::Play {
            cue_id: "first_goblin_tune_v2".into(),
            state_id: "outro".into(),
        })
    );
}

#[test]
fn failed_phase_returns_stop_now() {
    let state = state_with_phase(EncounterPhase::Failed);
    let states = lookup("goblin_encounter", &state);
    let director = director_with_active_cue(None);
    let bind = binding("goblin_encounter", "first_goblin_tune_v2");
    assert_eq!(
        resolve_directive_for_binding(&bind, &states, &director),
        Some(AdaptiveCueDirective::StopNow)
    );
}

#[test]
fn active_phase_wave2_promotes_to_reinforced_after_brute_delay() {
    let state = EncounterState {
        phase: EncounterPhase::Active {
            wave_index: 1,
            remaining_mobs: 3,
        },
        // Simulate enough wave_elapsed time to trigger the
        // wave2_reinforced_state promotion (LARGE_BRUTE_DELAY_SECONDS
        // is the threshold).
        run: EncounterRun {
            wave_elapsed: LARGE_BRUTE_DELAY_SECONDS + 0.1,
            ..Default::default()
        },
        ..Default::default()
    };
    let states = lookup("goblin_encounter", &state);
    let director = director_with_active_cue(None);
    let bind = binding("goblin_encounter", "first_goblin_tune_v2");
    assert_eq!(
        resolve_directive_for_binding(&bind, &states, &director),
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
    let state = state_with_phase(EncounterPhase::Cleared);
    let states = lookup("imaginary_arena", &state);
    let director = MusicDirectorState::default();
    // goblin_encounter binding has no encounter; imaginary_arena binding
    // is Cleared. The resolver iterates and returns the second
    // binding's Play directive.
    let result = resolve_adaptive_directive(&catalog, &states, &director);
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
