//! Synthetic-LDtk tests for the level-metadata reader and the
//! `RoomMetadata::merge` policy.

use serde_json::Value;

use super::super::project::*;

#[test]
fn level_metadata_reads_optional_biome_fields() {
    // Build a synthetic level whose fieldInstances declare every
    // optional metadata + visual-profile field. The reader should pick
    // them up and produce a RoomMetadata with each Some(...).
    fn field(name: &str, value: &str) -> LdtkFieldInstance {
        LdtkFieldInstance {
            identifier: name.into(),
            value: Value::String(value.into()),
            real_editor_values: vec![],
        }
    }
    let level = LdtkLevel {
        iid: "level-iid".into(),
        identifier: "metadata_level".into(),
        world_x: 0,
        world_y: 0,
        px_wid: 256,
        px_hei: 256,
        field_instances: vec![
            field("activeArea", "metadata_area"),
            field("biome", "cave"),
            field("music_track", "loop_a"),
            field("ambient_profile", "damp"),
            field("visual_theme", "blue"),
            field("visual_profile", "intro_wakeup_room"),
            field("parallax_theme", "basement"),
            field("palette", "warm_terminal"),
            field("lighting_hint", "low_key"),
            field("foreground_treatment", "dusty_edges"),
        ],
        layer_instances: Vec::new(),
    };
    let meta = level.level_metadata();
    assert_eq!(meta.biome.as_deref(), Some("cave"));
    assert_eq!(meta.music_track.as_deref(), Some("loop_a"));
    assert_eq!(meta.ambient_profile.as_deref(), Some("damp"));
    assert_eq!(meta.visual_theme.as_deref(), Some("blue"));
    assert_eq!(meta.visual_profile.id.as_deref(), Some("intro_wakeup_room"));
    assert_eq!(
        meta.visual_profile.parallax_theme.as_deref(),
        Some("basement")
    );
    assert_eq!(
        meta.visual_profile.palette.as_deref(),
        Some("warm_terminal")
    );
    assert_eq!(
        meta.visual_profile.lighting_hint.as_deref(),
        Some("low_key")
    );
    assert_eq!(
        meta.visual_profile.foreground_treatment.as_deref(),
        Some("dusty_edges")
    );
}

#[test]
fn level_metadata_skips_blank_strings() {
    fn field(name: &str, value: &str) -> LdtkFieldInstance {
        LdtkFieldInstance {
            identifier: name.into(),
            value: Value::String(value.into()),
            real_editor_values: vec![],
        }
    }
    let level = LdtkLevel {
        iid: "level-iid".into(),
        identifier: "blank_level".into(),
        world_x: 0,
        world_y: 0,
        px_wid: 256,
        px_hei: 256,
        field_instances: vec![
            field("activeArea", "blank_area"),
            field("biome", "   "),
            field("music_track", ""),
        ],
        layer_instances: Vec::new(),
    };
    let meta = level.level_metadata();
    assert!(
        meta.biome.is_none(),
        "whitespace-only must be treated as None"
    );
    assert!(meta.music_track.is_none());
}

#[test]
fn room_metadata_merge_first_non_empty_wins() {
    use crate::rooms::RoomMetadata;
    let mut a = RoomMetadata {
        biome: Some("hub".into()),
        music_track: None,
        ambient_profile: None,
        visual_theme: None,
        visual_profile: Default::default(),
    };
    let b = RoomMetadata {
        biome: Some("basement".into()),
        music_track: Some("dark_loop".into()),
        ambient_profile: Some("bass".into()),
        visual_theme: None,
        visual_profile: Default::default(),
    };
    a.merge(b);
    assert_eq!(a.biome.as_deref(), Some("hub"), "first non-empty wins");
    assert_eq!(
        a.music_track.as_deref(),
        Some("dark_loop"),
        "later levels fill in missing fields"
    );
    assert_eq!(a.ambient_profile.as_deref(), Some("bass"));
    assert_eq!(a.visual_theme, None);
}
