use super::*;
use ambition_audio::spec::MusicRegistry;
use ambition_audio::spec::MusicTrack;

#[test]
fn embedded_sandbox_data_parses() {
    let _spec = Platformer2dGameplayDefaults::load_embedded();
}

#[test]
fn embedded_registries_parse_and_validate() {
    fixture_sfx_registry()
        .clone()
        .validate()
        .expect("embedded sfx registry validates");
    fixture_music_registry()
        .clone()
        .validate()
        .expect("embedded music registry validates");
}

#[test]
fn embedded_music_tracks_are_unique_and_default_resolves() {
    let music = fixture_music_registry().clone();
    let mut ids = HashSet::new();
    for track in &music.tracks {
        assert!(ids.insert(track.id.as_str()), "duplicate id {}", track.id);
    }
    assert!(music.track(&music.default_track).is_some());
}

fn synthetic_track(id: &str) -> MusicTrack {
    MusicTrack {
        id: id.into(),
        display_name: format!("{id} display"),
        asset_path: None,
        one_shot: false,
    }
}

fn synthetic_music(tracks: Vec<MusicTrack>, default: &str) -> MusicRegistry {
    MusicRegistry {
        default_track: default.into(),
        tracks,
    }
}

/// A track id with no explicit `asset_path` derives the conventional
/// generated path from its id — that's what lets the registry be
/// generated from ids alone.
#[test]
fn track_path_defaults_to_generated_full_ogg() {
    let track = synthetic_track("hawks_of_the_high_aerie");
    assert_eq!(
        track.resolved_asset_path(),
        "audio/music/generated/hawks_of_the_high_aerie/full.ogg"
    );
}

/// Duplicate track ids must be rejected — the audio system uses
/// the id as a switch key, so a duplicate would shadow whichever
/// track the player asked for at runtime.
#[test]
fn validate_rejects_duplicate_track_ids() {
    let music = synthetic_music(
        vec![synthetic_track("alpha"), synthetic_track("alpha")],
        "alpha",
    );
    let err = music.validate().expect_err("duplicate ids must fail");
    assert!(err.contains("duplicate"), "got: {err}");
}

/// An empty track list must fail — nothing to play.
#[test]
fn validate_rejects_empty_tracks() {
    let music = synthetic_music(Vec::new(), "alpha");
    let err = music.validate().expect_err("empty tracks must fail");
    assert!(err.contains("at least one"), "got: {err}");
}

/// Missing default_track id (no track matches) must fail — the audio
/// system would otherwise try to play a non-existent track at startup.
#[test]
fn validate_rejects_missing_default_track() {
    let music = synthetic_music(vec![synthetic_track("alpha")], "ghost");
    let err = music.validate().expect_err("missing default must fail");
    assert!(err.contains("default_track"), "got: {err}");
}

/// Empty display_name must fail — the music selector surfaces it in the
/// UI; an empty value would render as a blank line.
#[test]
fn validate_rejects_empty_display_name() {
    let mut track = synthetic_track("alpha");
    track.display_name = String::new();
    let music = synthetic_music(vec![track], "alpha");
    let err = music.validate().expect_err("empty display_name must fail");
    assert!(err.contains("display_name"), "got: {err}");
}

/// Empty track id must fail — id is used as a switch key in the audio
/// system; an empty key collides with "no track selected".
#[test]
fn validate_rejects_empty_track_id() {
    let track = synthetic_track("");
    let music = synthetic_music(vec![track], "");
    let err = music.validate().expect_err("empty id must fail");
    assert!(err.contains("id"), "got: {err}");
}

#[test]
fn embedded_music_includes_original_and_long_default() {
    let music = fixture_music_registry().clone();
    assert_eq!(music.default_track, "long_lofi_drift");
    assert!(
        music.track("original_lofi_loop").is_some(),
        "original_lofi_loop present"
    );
    assert!(
        music.track("long_lofi_drift").is_some(),
        "long_lofi_drift present"
    );
    // The FSM radio entry is the choir-backing roots boss — the only FSM cue
    // that has a score and therefore the only one a checkout can render.
    //
    // This previously pinned `flying_spaghetti_monster_roots_boss`, which has
    // never had a score in this repo. The registry is a PROJECTION of the
    // rendered-OGG tree, so regenerating it correctly dropped that id, this
    // assertion went red, and 38d15197a put seven unbuildable ids back into
    // shipped data to make it green again. That left the registry
    // unregeneratable for a month: every honest regen wanted to drop the same
    // seven, and the removal guard stopped it — which also silently kept 16
    // cues that DO exist off the radio.
    //
    // Pin something the build can actually produce.
    assert!(
        music
            .track("flying_spaghetti_monster_roots_boss_choir_backing")
            .is_some(),
        "roots boss (choir backing) registered"
    );
    assert!(
        music.track("flying_spaghetti_monster_fight").is_none(),
        "old FSM fight is retired from the radio"
    );
}
