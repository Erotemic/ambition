//! Bevy `EmbeddedAssetRegistry` integration for platformer assets.
//!
//! This module couples embedded URL constants with their `include_bytes!`
//! registrations. Catalog tests verify each URL also has an authored candidate.
//! World JSON registrations come from installed `WorldManifest` rows.

use bevy::prelude::{App, Plugin};

use crate::AssetProfile;



/// Registers embedded catalog assets and installed world JSON with Bevy.
///
/// The registry is created by Bevy's `AssetPlugin`, so this plugin must run
/// after `DefaultPlugins`. World rows use the same `embedded_bevy_path` authored
/// by their `EmbeddedBinary` catalog candidate.
pub struct AmbitionAssetSourcePlugin {
    pub profile: AssetProfile,
    pub embedded_worlds: Vec<EmbeddedWorldAsset>,
}

impl AmbitionAssetSourcePlugin {
    pub fn for_profile(profile: AssetProfile) -> Self {
        Self {
            profile,
            embedded_worlds: Vec::new(),
        }
    }

    pub fn with_embedded_worlds(
        profile: AssetProfile,
        embedded_worlds: Vec<EmbeddedWorldAsset>,
    ) -> Self {
        Self {
            profile,
            embedded_worlds,
        }
    }
}

impl Plugin for AmbitionAssetSourcePlugin {
    fn build(&self, app: &mut App) {
        register_embedded_assets(app, &self.embedded_worlds);
        // Future profile-conditional source registration (HTTP / IPFS /
        // user-data) hooks here.
        let _ = self.profile;
    }
}

/// Embedded LDtk world bytes supplied by the game/content layer.
#[derive(Clone, Debug)]
pub struct EmbeddedWorldAsset {
    pub bevy_path: &'static str,
    pub text: &'static str,
}

/// Register the embedded worlds supplied by the game.
#[allow(unused_variables)]
fn register_embedded_assets(app: &mut App, embedded_worlds: &[EmbeddedWorldAsset]) {
    // World JSON: every installed WorldManifest row that carries embedded
    // text registers its bytes under the row's EmbeddedBinary URL — the
    // same text the serde loader parses, so the two embedded
    // representations can never diverge.
    {
        use bevy::asset::io::embedded::EmbeddedAssetRegistry;
        use std::path::{Path, PathBuf};

        let embedded = app.world_mut().resource_mut::<EmbeddedAssetRegistry>();
        for source in embedded_worlds {
            embedded.insert_asset(
                PathBuf::new(),
                Path::new(source.bevy_path),
                source.text.as_bytes(),
            );
        }
    }

}
