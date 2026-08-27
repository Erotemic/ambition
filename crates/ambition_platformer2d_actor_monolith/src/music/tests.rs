//! Unit tests for the encounter→adaptive-cue resolver (`intent.rs`):
//! per-binding directive resolution across encounter phases (starting /
//! active-wave / wave-2 reinforced promotion / cleared / failed / unknown),
//! multi-binding iteration, and notes on the outro/restart race fix.

use super::*;
use ambition_encounter::{EncounterPhase, EncounterRun, EncounterWaves};
use std::collections::HashMap;
use ambition_audio::music::state::AdaptiveCueDirective;
use ambition_audio::music::catalog::MusicCueCatalog;
use ambition_audio::music::state::MusicDirectorMode;
use ambition_audio::music::state::MusicDirectorState;

/// A wave-policy fixture: the resolver keys adaptive states off the wave
/// index/clock (`EncounterWaves.run`); the generic lifecycle supplies the
/// phase separately.
fn waves_fixture(run: EncounterRun) -> EncounterWaves {
    let spec: ambition_encounter::EncounterSpec = ron::from_str(
        r#"(id: "t", waves: [], trigger_min: (0.0, 0.0), trigger_size: (10.0, 10.0),
            camera_zoom: 1.0, lock_wall: None, intro_seconds: 0.0, music_track: "")"#,
    )
    .expect("minimal spec");
    let mut waves = EncounterWaves::new(spec);
    waves.run = run;
    waves
}

/// A single-entry `id -> (phase, &EncounterWaves)` lookup, matching what
/// `compute_music_intent` builds from the encounter entities.
fn lookup<'a>(
    id: &'a str,
    phase: EncounterPhase,
    waves: &'a EncounterWaves,
) -> HashMap<&'a str, (EncounterPhase, &'a EncounterWaves)> {
    HashMap::from([(id, (phase, waves))])
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
    let states: HashMap<&str, (EncounterPhase, &EncounterWaves)> = HashMap::new();
    let director = director_with_active_cue(None);
    let bind = binding("nonexistent", "first_goblin_tune_v2");
    assert!(resolve_directive_for_binding(&bind, &states, &director).is_none());
}

#[test]
fn unknown_encounter_with_active_cue_returns_stop_now() {
    let states: HashMap<&str, (EncounterPhase, &EncounterWaves)> = HashMap::new();
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
    let waves = waves_fixture(EncounterRun::default());
    let states = lookup(
        "goblin_encounter",
        EncounterPhase::Starting { remaining: 1.0 },
        &waves,
    );
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
    let waves = waves_fixture(EncounterRun {
        wave_index: Some(2),
        ..Default::default()
    });
    let states = lookup("goblin_encounter", EncounterPhase::Active, &waves);
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
    let waves = waves_fixture(EncounterRun::default());
    let states = lookup("goblin_encounter", EncounterPhase::Completed, &waves);
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
    let waves = waves_fixture(EncounterRun::default());
    let states = lookup("goblin_encounter", EncounterPhase::Failed, &waves);
    let director = director_with_active_cue(None);
    let bind = binding("goblin_encounter", "first_goblin_tune_v2");
    assert_eq!(
        resolve_directive_for_binding(&bind, &states, &director),
        Some(AdaptiveCueDirective::StopNow)
    );
}

#[test]
fn active_phase_wave2_promotes_to_reinforced_after_brute_delay() {
    // Simulate enough wave_elapsed time to trigger the
    // wave2_reinforced_state promotion (LARGE_BRUTE_DELAY_SECONDS
    // is the threshold).
    let waves = waves_fixture(EncounterRun {
        wave_index: Some(1),
        wave_elapsed: LARGE_BRUTE_DELAY_SECONDS + 0.1,
        ..Default::default()
    });
    let states = lookup("goblin_encounter", EncounterPhase::Active, &waves);
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
    let waves = waves_fixture(EncounterRun::default());
    let states = lookup("imaginary_arena", EncounterPhase::Completed, &waves);
    let director = MusicDirectorState::default();
    // goblin_encounter binding has no encounter; imaginary_arena binding
    // is Cleared. The resolver iterates and returns the second
    // binding's Play directive.
    let result = resolve_adaptive_directive(&catalog, &states, &director);
    assert!(matches!(result, Some(AdaptiveCueDirective::Play { .. })));
}

// ---- should_restart_adaptive: encounter-restart-during-outro race ----
//
// but also died at the same time, which reset me back to the start.
// I reset and restarted the goblin encounter, so maybe the timed
// trigger to restart the lofi music happened and then the trigger
// to start the goblin music happened (because i reset the
// encounter), so they both played at the same time."

// ── The simple-track priority list ────────────────────────────────────────

use super::intent::simple_track_candidates;
use ambition_audio::selection::ActiveAudioSelection;
use ambition_encounter::EncounterMusicRequest;
use ambition_platformer2d_world::rooms::RoomMusicRequest;

fn narrative(track: &str) -> ambition_conversation::NarrativeMusicRequest {
    let mut request = ambition_conversation::NarrativeMusicRequest::default();
    request.request(track);
    request
}

fn room(track: &str) -> RoomMusicRequest {
    RoomMusicRequest {
        desired_track: Some(track.to_string()),
    }
}

/// A conversation outranks the room it happens in — the point of the command.
#[test]
fn a_conversations_track_beats_the_rooms_own() {
    let candidates = simple_track_candidates(
        &room("for_emmy_forever_ago"),
        Some(&narrative("super_smash_siblings_theme")),
        None,
        &ActiveAudioSelection::default(),
        &EncounterMusicRequest::default(),
    );
    assert_eq!(
        candidates.first().map(String::as_str),
        Some("super_smash_siblings_theme")
    );
    assert!(
        candidates.contains(&"for_emmy_forever_ago".to_string()),
        "the room stays a candidate underneath, so releasing the claim restores it: {candidates:?}"
    );
}

/// ...but a live fight scores itself. An encounter is the more specific claim.
#[test]
fn a_fight_outranks_a_conversations_track() {
    let mut encounter = EncounterMusicRequest::default();
    encounter.claim_priority("test_boss", "you_are_too_slow");
    let candidates = simple_track_candidates(
        &room("for_emmy_forever_ago"),
        Some(&narrative("super_smash_siblings_theme")),
        None,
        &ActiveAudioSelection::default(),
        &encounter,
    );
    assert_eq!(
        candidates.first().map(String::as_str),
        Some("you_are_too_slow")
    );
}

/// An empty id hands the room its own music back without a second command.
#[test]
fn an_empty_id_is_not_a_claim() {
    let candidates = simple_track_candidates(
        &room("for_emmy_forever_ago"),
        Some(&narrative("")),
        None,
        &ActiveAudioSelection::default(),
        &EncounterMusicRequest::default(),
    );
    assert_eq!(
        candidates.first().map(String::as_str),
        Some("for_emmy_forever_ago")
    );
}
