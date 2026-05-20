//! Catalog identity tests: id presence, kind, preload group, and
//! uniqueness across the manifest.

use super::super::*;
use crate::content::data::SandboxDataSpec;
use std::collections::HashSet;

use super::fixture_catalog;

#[test]
fn every_well_known_id_resolves_to_an_entry() {
    let catalog = fixture_catalog();
    let inner = catalog.catalog();
    for id_str in [
        ids::SANDBOX_LDTK,
        ids::SANDBOX_DATA,
        ids::SFX_BANK,
        ids::FONT_DIALOG_REGULAR,
        ids::FONT_DIALOG_SEMIBOLD,
        ids::FONT_DEBUG_MONO,
    ] {
        assert!(
            inner.manifest().get(&AssetId::new(id_str)).is_some(),
            "manifest missing well-known id `{id_str}`",
        );
    }
}

#[test]
fn sandbox_ldtk_is_required_and_bootstrap() {
    let catalog = fixture_catalog();
    let entry = catalog
        .catalog()
        .manifest()
        .get(&ids::sandbox_ldtk())
        .unwrap();
    assert_eq!(entry.kind, AssetKind::LdtkProject);
    assert_eq!(entry.missing_policy, MissingAssetPolicy::Error);
    assert_eq!(entry.preload_group, Some(PreloadGroup::Bootstrap));
}

#[test]
fn sandbox_data_is_required_and_bootstrap() {
    let catalog = fixture_catalog();
    let entry = catalog
        .catalog()
        .manifest()
        .get(&ids::sandbox_data())
        .unwrap();
    assert_eq!(entry.kind, AssetKind::RonData);
    assert_eq!(entry.missing_policy, MissingAssetPolicy::Error);
    assert_eq!(entry.preload_group, Some(PreloadGroup::Bootstrap));
}

#[test]
fn music_track_ids_match_audio_spec() {
    let catalog = fixture_catalog();
    let spec = SandboxDataSpec::load_embedded();
    for track in &spec.audio.music_tracks {
        let id = ids::music_track(&track.id);
        if track.asset_path.is_some() {
            let entry = catalog
                .catalog()
                .manifest()
                .get(&id)
                .unwrap_or_else(|| panic!("missing music catalog entry for {id}"));
            assert_eq!(entry.kind, AssetKind::AudioClip);
        }
    }
}

#[test]
fn all_catalog_ids_are_unique() {
    let catalog = fixture_catalog();
    let mut seen = HashSet::new();
    for (id, _) in catalog.catalog().manifest().iter() {
        assert!(seen.insert(id.clone()), "duplicate id: {id}");
    }
}

/// Intro NPC + prop catalog entries exist in the prebuilt
/// catalog. The intro plugin's load systems query them via
/// `try_path_for_load`; missing entries would silently fall
/// through to colored rectangles.
#[test]
fn intro_npc_and_prop_sprite_ids_resolve_through_the_catalog() {
    use crate::intro::sprites::{
        intro_npc_asset_id, intro_npc_sprite_rows, intro_prop_asset_id, intro_prop_sprite_rows,
    };

    let mut config = GameAssetConfig::default();
    config.asset_profile = AssetProfile::DesktopDevLoose;
    let spec = SandboxDataSpec::load_embedded();
    let catalog = build_sandbox_catalog(&config, &spec.audio);

    for (name, filename, _spec) in intro_npc_sprite_rows() {
        let id = intro_npc_asset_id(name);
        let resolved = catalog.resolve(&id).unwrap_or_else(|err| {
            panic!("intro NPC `{name}` (id {id}) missing from catalog: {err}")
        });
        assert_eq!(resolved.kind, AssetKind::Image);
        // The logical path should end with the registered filename.
        assert!(
            resolved
                .bevy_asset_path()
                .map(|p| p.ends_with(filename))
                .unwrap_or(false),
            "intro NPC `{name}` resolved to path that doesn't end with {filename}",
        );
    }
    for (kind, filename, _spec) in intro_prop_sprite_rows() {
        let id = intro_prop_asset_id(kind);
        let resolved = catalog.resolve(&id).unwrap_or_else(|err| {
            panic!("intro prop `{kind}` (id {id}) missing from catalog: {err}")
        });
        assert!(resolved
            .bevy_asset_path()
            .map(|p| p.ends_with(filename))
            .unwrap_or(false));
    }
}

#[test]
fn intro_ldtk_is_in_the_catalog_under_world_namespace() {
    let mut config = GameAssetConfig::default();
    config.asset_profile = AssetProfile::DesktopDevLoose;
    let spec = SandboxDataSpec::load_embedded();
    let catalog = build_sandbox_catalog(&config, &spec.audio);
    let entry = catalog
        .catalog()
        .manifest()
        .get(&ids::intro_ldtk())
        .expect("world.intro_ldtk catalog entry missing");
    assert_eq!(entry.kind, AssetKind::LdtkProject);
    let r_desktop = catalog.resolve(&ids::intro_ldtk()).unwrap();
    assert!(r_desktop.location.as_local_path().is_some());

    config.asset_profile = AssetProfile::WebStatic;
    let catalog = build_sandbox_catalog(&config, &spec.audio);
    let path = catalog.try_path_for_load(&ids::intro_ldtk()).unwrap();
    assert_eq!(path, format!("embedded://{EMBEDDED_INTRO_LDTK_ASSET_PATH}"));
}
