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
        r#"(id: "t", room_id: "r", waves: [], trigger_min: (0.0, 0.0), trigger_size: (10.0, 10.0),
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
    // The clearing frame: the fight's own cue is the one playing.
    let director = director_with_active_cue(Some("first_goblin_tune_v2"));
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
    let director = director_with_active_cue(Some("imaginary_cue"));
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

fn narrative(track: &str) -> ambition_conversation::NarrativeMusicRequest {
    let mut request = ambition_conversation::NarrativeMusicRequest::default();
    request.request(track);
    request
}

fn room(track: &str) -> Option<&str> {
    Some(track)
}

/// A conversation outranks the room it happens in — the point of the command.
#[test]
fn a_conversations_track_beats_the_rooms_own() {
    let candidates = simple_track_candidates(
        room("for_emmy_forever_ago"),
        None,
        Some(&narrative("super_smash_siblings_theme")),
        None,
        &ActiveAudioSelection::default(),
        None,
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
    encounter.claim_priority(None, "test_boss", "you_are_too_slow");
    let candidates = simple_track_candidates(
        room("for_emmy_forever_ago"),
        None,
        Some(&narrative("super_smash_siblings_theme")),
        None,
        &ActiveAudioSelection::default(),
        encounter.desired_track(None),
    );
    assert_eq!(
        candidates.first().map(String::as_str),
        Some("you_are_too_slow")
    );
}

/// A room says what its fights sound like. The room's fight track outranks
/// the boss's own, only while a fight is on, and the boss's track stays a
/// candidate underneath (the director plays the first id it has).
#[test]
fn a_rooms_fight_track_beats_the_fights_own_only_during_a_fight() {
    let mut fight = EncounterMusicRequest::default();
    fight.claim_priority(None, "test_boss", "flying_spaghetti_monster_roots_boss_choir_backing");
    let candidates = simple_track_candidates(
        room("for_emmy_forever_ago"),
        room("crooked_ascent_boss"),
        None,
        None,
        &ActiveAudioSelection::default(),
        fight.desired_track(None),
    );
    assert_eq!(
        candidates.iter().map(String::as_str).take(2).collect::<Vec<_>>(),
        ["crooked_ascent_boss", "flying_spaghetti_monster_roots_boss_choir_backing"],
        "the room's fight track must come first, with the boss's own as the fallback"
    );

    let quiet = simple_track_candidates(
        room("for_emmy_forever_ago"),
        room("crooked_ascent_boss"),
        None,
        None,
        &ActiveAudioSelection::default(),
        None,
    );
    assert_eq!(
        quiet.first().map(String::as_str),
        Some("for_emmy_forever_ago"),
        "with no fight on, the room plays its own music, not its fight track"
    );
    assert!(
        !quiet.contains(&"crooked_ascent_boss".to_string()),
        "the fight track is a candidate only during a fight: {quiet:?}"
    );
}

/// An empty id hands the room its own music back without a second command.
#[test]
fn an_empty_id_is_not_a_claim() {
    let candidates = simple_track_candidates(
        room("for_emmy_forever_ago"),
        None,
        Some(&narrative("")),
        None,
        &ActiveAudioSelection::default(),
        None,
    );
    assert_eq!(
        candidates.first().map(String::as_str),
        Some("for_emmy_forever_ago")
    );
}


/// ⛔⛔ A CLEARED ENCOUNTER MUST NOT LOOP ITS OWN OUTRO — Jon, 2026-09-06, with
/// logs showing a 7.30s cycle against a 7.285s outro:
///
/// ```text
/// finish_adaptive_outro cue=first_goblin_tune_v2 t=7.285
/// start_adaptive_state  cue=first_goblin_tune_v2 state=outro   <- same frame
/// resume_simple_music   target=long_lofi_drift                 <- loses the fight
/// ```
///
/// `EncounterPhase::Completed` returned `Play { cleared_state }` UNCONDITIONALLY
/// while `Inactive` already refused to re-request a finished cue. Completed is the
/// phase an encounter SITS in, so it is the arm that loops.
///
/// ⭐ AND THE DIRECTOR CANNOT DEFEND ITSELF, which is why the fix belongs here:
/// `finish_adaptive_outro` sets `AdaptiveFinished` and CLEARS `active_cue_id`, so a
/// standing request satisfies BOTH of `should_restart_adaptive`'s conditions at
/// once — `!same_cue` because the id is gone, and `mode_lost_adaptive` because
/// `AdaptiveFinished` is in its list.
#[test]
fn a_cleared_encounter_stops_asking_once_its_outro_has_run_out() {
    let catalog = MusicCueCatalog::from_parts(
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
    let waves = waves_fixture(EncounterRun::default());
    let states = lookup("goblin_encounter", EncounterPhase::Completed, &waves);

    // ⚠ THE PREMISE FIRST: while the outro has NOT run out, a cleared encounter
    // must still ask for it — or this test would pass on a resolver that never
    // plays an outro at all.
    let mut director = MusicDirectorState::default();
    director.mode = MusicDirectorMode::AdaptiveOutro;
    director.active_cue_id = Some("first_goblin_tune_v2".into());
    assert!(
        matches!(
            resolve_adaptive_directive(&catalog, &states, &director),
            Some(AdaptiveCueDirective::Play { .. })
        ),
        "a cleared encounter mid-outro must keep asking for it"
    );

    // Exactly what `finish_adaptive_outro` leaves behind.
    director.mode = MusicDirectorMode::AdaptiveFinished;
    director.active_cue_id = None;
    assert_eq!(
        resolve_adaptive_directive(&catalog, &states, &director),
        None,
        "the outro has already run to completion and the encounter is still \
         Completed, so asking again restarts it — the 7.3s loop Jon heard"
    );

    // ⛔ AND A DOOR DOES NOT BRING IT BACK. A room-track switch sets
    // `SimpleTrack`; asking only "has the mode finished" replayed the outro at
    // every door whose room changed the track, for the rest of the session.
    director.mode = MusicDirectorMode::SimpleTrack;
    assert_eq!(
        resolve_adaptive_directive(&catalog, &states, &director),
        None,
        "a door after the outro replayed the goblin fight's outro"
    );
    // Nor does a fresh director: a save loaded with the fight already cleared.
    assert_eq!(
        resolve_adaptive_directive(&catalog, &states, &MusicDirectorState::default()),
        None,
        "a session starting on a cleared fight played its outro"
    );
}

// ── The room the music plays for ───────────────────────────────────────────

/// The music intent's candidates with two live rooms (`hall`, the activation
/// room, and `chapel`), the primary body in `chapel`, and a fight track
/// claimed in `fight_in`.
fn candidates_with_two_rooms(
    fight_in: ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance,
) -> Vec<String> {
    use ambition_platformer2d_shared_tangle::lifecycle::{
        insert_session_world_component, session_world_component, InRoomInstance,
        LiveRoomInstance, RoomInstanceRoot,
    };
    use bevy::prelude::*;
    let mut app = App::new();
    app.init_resource::<ambition_audio::music::AdaptiveMusicCatalogRegistry>();
    app.init_resource::<ActiveAudioSelection>();
    app.init_resource::<ambition_audio::music::MusicIntent>();
    let room = |id: &str, track: &str| {
        let mut spec = ambition_platformer2d_world::rooms::RoomSpec::new(
            id,
            ambition_platformer2d_core::World::new(id, Vec2::new(800.0, 600.0), Vec2::new(16.0, 16.0), Vec::new()),
        );
        spec.metadata.music_track = Some(track.to_string());
        spec
    };
    ambition_platformer2d_world::rooms::insert_room_set(
        app.world_mut(),
        ambition_platformer2d_world::rooms::RoomSet::from_parts_or_panic(
            "hall",
            vec![room("hall", "hall_theme"), room("chapel", "chapel_theme")],
            Vec::new(),
        ),
    );
    let chapel = session_world_component::<ambition_platformer2d_world::rooms::RoomSet>(app.world())
        .and_then(|rooms| rooms.definition_by_id("chapel"))
        .expect("the set has `chapel`");
    let second = LiveRoomInstance::ACTIVATION.next();
    app.world_mut().spawn((RoomInstanceRoot, second, chapel));
    app.world_mut()
        .spawn((ambition_platformer2d_shared_tangle::body::PrimaryBody, InRoomInstance(second)));
    let mut music = EncounterMusicRequest::default();
    music.claim_priority(Some(fight_in), "test_boss", "fight_theme");
    insert_session_world_component(app.world_mut(), music);
    app.add_systems(Update, super::intent::compute_music_intent);
    app.update();
    app.world().resource::<ambition_audio::music::MusicIntent>().simple_track_candidates.clone()
}

/// Q150 (a): the music plays for the primary seat's room. Two live rooms;
/// the primary body is in `chapel`. The intent offers chapel's music and not
/// the hall's, and a fight is heard only when it is in chapel (the fight in
/// chapel is the control). Before, the intent read the sole live room, so
/// with two rooms it did not run and the music froze. Poisons: read the sole
/// room (no candidates at all), hear every room's fight (the hall's fight is
/// heard).
#[test]
fn the_music_plays_for_the_primary_seats_room() {
    use ambition_platformer2d_shared_tangle::lifecycle::LiveRoomInstance;
    let hall = LiveRoomInstance::ACTIVATION;
    let chapel = hall.next();
    let in_chapel = candidates_with_two_rooms(chapel);
    let in_hall = candidates_with_two_rooms(hall);
    assert_eq!(
        (
            in_chapel.first().map(String::as_str),
            in_chapel.contains(&"chapel_theme".to_string()),
            in_chapel.contains(&"hall_theme".to_string()),
        ),
        (Some("fight_theme"), true, false),
        "a fight in the primary seat's room, over its own music: {in_chapel:?}"
    );
    assert_eq!(
        in_hall.first().map(String::as_str),
        Some("chapel_theme"),
        "a fight in the other room is not heard: {in_hall:?}"
    );
}
