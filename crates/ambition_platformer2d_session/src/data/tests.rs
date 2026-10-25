use super::*;
use ambition_audio::spec::MusicRegistry;
use ambition_audio::spec::MusicTrack;

#[test]
fn embedded_sandbox_data_parses() {
    let _spec = Platformer2dGameplayDefaults::load();
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

/// The sandbox world this test's catalog carries, as a plain value. The
/// `Once` guard died with the install seam — a manifest is now just an
/// argument, so repeated construction is free of cross-test coupling.
fn test_world_manifest() -> ambition_platformer2d_world::world_manifest::WorldManifest {
    use ambition_asset_manager::AssetId;
    use ambition_platformer2d_world::world_manifest::{WorldManifest, WorldSource};
    let worlds_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../game/ambition_content/assets/worlds");
    WorldManifest {
        entry_room: "central_hub_complex".to_string(),
        ron_rooms: Vec::new(),
        worlds: vec![WorldSource {
            id: AssetId::new("world.sandbox_ldtk"),
            asset_path: "game://worlds/sandbox.ldtk".to_string(),
            loose_path: Some(worlds_dir.join("sandbox.ldtk")),
            embedded_text: None,
            embedded_bevy_path: Some("ambition_content/worlds/sandbox.ldtk"),
            required: true,
        }],
    }
}

/// Live-runtime guardrail: every music track in the embedded sandbox
/// spec must have a `WebServedAssets`-resolvable catalog path. This
/// pins the "music works on the served-web profile" contract — if a
/// new track lands without an `asset_path`, the catalog drops it and
/// this test fails loudly instead of letting the radio silently lose
/// an entry on web.
#[test]
fn every_live_music_track_resolves_under_web_served_assets() {
    use ambition_sprite_sheet::game_assets::GameAssetConfig;
    use ambition_asset_manager::AssetProfile;

    let manifest = test_world_manifest();
    let music = fixture_music_registry().clone();
    let mut config = GameAssetConfig::default();
    config.asset_profile = AssetProfile::WebServedAssets;
    let character_catalog =
        ambition_characters::actor::character_catalog::CharacterCatalog::from_data(
            ambition_characters::actor::character_catalog::parse_catalog(include_str!(
                "../../../../game/ambition_content/assets/data/character_catalog.ron"
            )),
        );
    let boss_catalog = ambition_boss_encounter::BossCatalog::default();
    let catalog = ambition_platformer2d_actor_monolith::assets::platformer_assets::build_platformer2d_asset_catalog(
        &config,
        &character_catalog,
        &boss_catalog,
        &music,
        &manifest,
    );

    let mut missing: Vec<String> = Vec::new();
    for track in &music.tracks {
        let id = ambition_platformer2d_actor_monolith::assets::platformer_assets::ids::music_track(&track.id);
        let path = catalog.path_for(&id);
        if path.is_none() {
            missing.push(track.id.clone());
        }
    }
    assert!(
        missing.is_empty(),
        "music tracks without a WebServedAssets-resolvable path: {missing:?}. \
         Either add a pre-rendered OGG or remove the track from \
         music_registry.ron — the procedural fallback is retired."
    );
}

