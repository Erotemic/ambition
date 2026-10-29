//! Authored audio and tuning fixtures for session tests.

use ambition_platformer2d_assets::gameplay_defaults::Platformer2dGameplayDefaults;
use ambition_audio::spec::{MusicRegistry, SfxRegistry};
use std::collections::HashSet;

/// Test-only handle to the game's authored music registry. Not a runtime seam.
#[cfg(test)]
pub(crate) fn fixture_music_registry() -> &'static MusicRegistry {
    &TEST_FIXTURE_MUSIC_REGISTRY
}

/// Test-only handle to the game's authored SFX registry. Not a runtime seam.
#[cfg(test)]
pub(crate) fn fixture_sfx_registry() -> &'static SfxRegistry {
    &TEST_FIXTURE_SFX_REGISTRY
}

/// Test fixture = the game's REAL authored registries, read cross-crate
/// from `ambition_content` as an explicit cross-crate test fixture so
/// core's catalog/audio tests exercise real data without core embedding it.
#[cfg(test)]
static TEST_FIXTURE_MUSIC_REGISTRY: std::sync::LazyLock<MusicRegistry> =
    std::sync::LazyLock::new(|| {
        ron::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../game/ambition_content_library/generated/runtime/full/audio/music_registry.ron")).expect("shared audio fixture"))
        .expect("ambition_content music_registry.ron should parse")
    });

#[cfg(test)]
static TEST_FIXTURE_SFX_REGISTRY: std::sync::LazyLock<SfxRegistry> =
    std::sync::LazyLock::new(|| {
        ron::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../game/ambition_content_library/authored/runtime/audio/sfx_registry.ron")).expect("shared audio fixture"))
        .expect("ambition_content sfx_registry.ron should parse")
    });

#[cfg(test)]
mod tests;
