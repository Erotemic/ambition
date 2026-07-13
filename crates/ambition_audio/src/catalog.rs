//! App-local authored-audio catalogs contributed by experience providers.
//!
//! Music and SFX remain separate authored concerns, while one provider id ties
//! them to the experience that owns their defaults. The registry is a Bevy
//! resource, so independent `App`s in one process may compose different games.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use bevy::prelude::{App, Resource};

use crate::spec::{MusicRegistry, MusicTrack, SfxRegistry};

/// One provider's immutable authored-audio definitions.
#[derive(Clone, Debug, PartialEq)]
pub struct AudioCatalogFragment {
    provider_id: String,
    music: Option<MusicRegistry>,
    sfx: Option<SfxRegistry>,
}

impl AudioCatalogFragment {
    pub fn new(
        provider_id: impl Into<String>,
        music: Option<MusicRegistry>,
        sfx: Option<SfxRegistry>,
    ) -> Result<Self, AudioCatalogError> {
        let provider_id = provider_id.into();
        if provider_id.trim().is_empty() {
            return Err(AudioCatalogError::EmptyProviderId);
        }
        if let Some(music) = &music {
            music
                .validate()
                .map_err(|message| AudioCatalogError::InvalidMusic {
                    provider_id: provider_id.clone(),
                    message,
                })?;
        }
        if let Some(sfx) = &sfx {
            sfx.validate()
                .map_err(|message| AudioCatalogError::InvalidSfx {
                    provider_id: provider_id.clone(),
                    message,
                })?;
        }
        Ok(Self {
            provider_id,
            music,
            sfx,
        })
    }

    pub fn provider_id(&self) -> &str {
        &self.provider_id
    }

    pub fn music(&self) -> Option<&MusicRegistry> {
        self.music.as_ref()
    }

    pub fn sfx(&self) -> Option<&SfxRegistry> {
        self.sfx.as_ref()
    }

    fn validate(&self) -> Result<(), AudioCatalogError> {
        if self.provider_id.trim().is_empty() {
            return Err(AudioCatalogError::EmptyProviderId);
        }
        if let Some(music) = &self.music {
            music
                .validate()
                .map_err(|message| AudioCatalogError::InvalidMusic {
                    provider_id: self.provider_id.clone(),
                    message,
                })?;
        }
        if let Some(sfx) = &self.sfx {
            sfx.validate()
                .map_err(|message| AudioCatalogError::InvalidSfx {
                    provider_id: self.provider_id.clone(),
                    message,
                })?;
        }
        Ok(())
    }
}

/// Provider-indexed authored audio for one Bevy `App`.
#[derive(Resource, Clone, Debug, Default)]
pub struct AudioCatalogRegistry {
    fragments: BTreeMap<String, AudioCatalogFragment>,
}

impl AudioCatalogRegistry {
    pub fn register(&mut self, fragment: AudioCatalogFragment) -> Result<(), AudioCatalogError> {
        fragment.validate()?;
        if let Some(existing) = self.fragments.get(&fragment.provider_id) {
            if existing == &fragment {
                return Ok(());
            }
            return Err(AudioCatalogError::DuplicateProvider {
                provider_id: fragment.provider_id,
            });
        }
        self.fragments
            .insert(fragment.provider_id.clone(), fragment);
        Ok(())
    }

    pub fn providers(&self) -> impl Iterator<Item = &str> {
        self.fragments.keys().map(String::as_str)
    }

    /// Whether `provider_id` registered an audio fragment (music, SFX, or an
    /// explicitly-empty one for deliberate silence). A registered-but-empty
    /// fragment is how a silent provider declares intent; absence is a
    /// composition error the session bridge refuses to treat as silence.
    pub fn has_provider(&self, provider_id: &str) -> bool {
        self.fragments.contains_key(provider_id)
    }

    pub fn music_for(&self, provider_id: &str) -> Option<&MusicRegistry> {
        self.fragments.get(provider_id)?.music.as_ref()
    }

    pub fn sfx_for(&self, provider_id: &str) -> Option<&SfxRegistry> {
        self.fragments.get(provider_id)?.sfx.as_ref()
    }

    /// Build the App-visible music asset index while preserving a selected
    /// provider's default track. Track ids are global asset identities and must
    /// therefore be unique across linked providers.
    pub fn combined_music_registry(
        &self,
        default_provider: &str,
    ) -> Result<MusicRegistry, AudioCatalogError> {
        let default_track = self
            .music_for(default_provider)
            .ok_or_else(|| AudioCatalogError::MissingMusicProvider {
                provider_id: default_provider.to_string(),
            })?
            .default_track
            .clone();
        // id -> (first provider, its resolved asset path). Two providers naming
        // the SAME id for the SAME underlying asset (a shared track in the
        // common asset tree) is a benign duplicate — dedup it. Two providers
        // naming one id for DIFFERENT assets is a genuine conflict.
        let mut seen = BTreeMap::<String, (String, String)>::new();
        let mut tracks = Vec::<MusicTrack>::new();
        for (provider_id, fragment) in &self.fragments {
            let Some(music) = &fragment.music else {
                continue;
            };
            for track in &music.tracks {
                let resolved = track.resolved_asset_path();
                if let Some((first_provider, first_path)) = seen.get(&track.id) {
                    if first_path == &resolved {
                        continue;
                    }
                    return Err(AudioCatalogError::DuplicateMusicTrack {
                        track_id: track.id.clone(),
                        first_provider: first_provider.clone(),
                        second_provider: provider_id.clone(),
                    });
                }
                seen.insert(track.id.clone(), (provider_id.clone(), resolved));
                tracks.push(track.clone());
            }
        }
        tracks.sort_by(|a, b| a.id.cmp(&b.id));
        let combined = MusicRegistry {
            default_track,
            tracks,
        };
        combined
            .validate()
            .map_err(AudioCatalogError::InvalidCombinedMusic)?;
        Ok(combined)
    }

    /// A shared track (same id, same resolved asset path) may legitimately
    /// appear in more than one provider's registry — Ambition owns a superset
    /// of the asset tree and a demo carries a small subset that points at the
    /// SAME files. That is benign. Only a genuine collision — one id mapped to
    /// two DIFFERENT assets — is an error.
    pub fn validate_global_music_ids(&self) -> Result<(), AudioCatalogError> {
        let mut seen = BTreeMap::<String, (String, String)>::new();
        for (provider_id, fragment) in &self.fragments {
            let Some(music) = &fragment.music else {
                continue;
            };
            for track in &music.tracks {
                let resolved = track.resolved_asset_path();
                if let Some((first_provider, first_path)) = seen.get(&track.id) {
                    if first_path == &resolved {
                        continue;
                    }
                    return Err(AudioCatalogError::DuplicateMusicTrack {
                        track_id: track.id.clone(),
                        first_provider: first_provider.clone(),
                        second_provider: provider_id.clone(),
                    });
                }
                seen.insert(track.id.clone(), (provider_id.clone(), resolved));
            }
        }
        Ok(())
    }

    pub fn all_music_track_ids(&self) -> BTreeSet<&str> {
        self.fragments
            .values()
            .filter_map(|fragment| fragment.music.as_ref())
            .flat_map(|music| music.tracks.iter().map(|track| track.id.as_str()))
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AudioCatalogError {
    EmptyProviderId,
    DuplicateProvider {
        provider_id: String,
    },
    InvalidMusic {
        provider_id: String,
        message: String,
    },
    InvalidSfx {
        provider_id: String,
        message: String,
    },
    MissingMusicProvider {
        provider_id: String,
    },
    DuplicateMusicTrack {
        track_id: String,
        first_provider: String,
        second_provider: String,
    },
    InvalidCombinedMusic(String),
}

impl fmt::Display for AudioCatalogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyProviderId => write!(f, "audio catalog provider id must not be empty"),
            Self::DuplicateProvider { provider_id } => {
                write!(f, "audio catalog provider '{provider_id}' registered twice")
            }
            Self::InvalidMusic {
                provider_id,
                message,
            } => write!(f, "music catalog '{provider_id}' is invalid: {message}"),
            Self::InvalidSfx {
                provider_id,
                message,
            } => write!(f, "SFX catalog '{provider_id}' is invalid: {message}"),
            Self::MissingMusicProvider { provider_id } => {
                write!(f, "provider '{provider_id}' has no music catalog")
            }
            Self::DuplicateMusicTrack {
                track_id,
                first_provider,
                second_provider,
            } => write!(
                f,
                "music track id '{track_id}' is authored by both '{first_provider}' and '{second_provider}'"
            ),
            Self::InvalidCombinedMusic(message) => {
                write!(f, "combined music catalog is invalid: {message}")
            }
        }
    }
}

impl std::error::Error for AudioCatalogError {}

pub trait AudioCatalogAppExt {
    fn try_register_audio_catalog_fragment(
        &mut self,
        fragment: AudioCatalogFragment,
    ) -> Result<&mut Self, AudioCatalogError>;

    fn register_audio_catalog_fragment(&mut self, fragment: AudioCatalogFragment) -> &mut Self {
        self.try_register_audio_catalog_fragment(fragment)
            .unwrap_or_else(|error| panic!("{error}"))
    }
}

impl AudioCatalogAppExt for App {
    fn try_register_audio_catalog_fragment(
        &mut self,
        fragment: AudioCatalogFragment,
    ) -> Result<&mut Self, AudioCatalogError> {
        let registry = {
            let mut candidate = self
                .world()
                .get_resource::<AudioCatalogRegistry>()
                .cloned()
                .unwrap_or_default();
            candidate.register(fragment)?;
            candidate.validate_global_music_ids()?;
            candidate
        };
        self.insert_resource(registry);
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn music(id: &str) -> MusicRegistry {
        MusicRegistry {
            default_track: id.to_string(),
            tracks: vec![MusicTrack {
                id: id.to_string(),
                display_name: id.to_string(),
                asset_path: None,
            }],
        }
    }

    /// A track whose id collides with another provider's but points at a
    /// DIFFERENT asset — a genuine conflict.
    fn music_at(id: &str, asset_path: &str) -> MusicRegistry {
        MusicRegistry {
            default_track: id.to_string(),
            tracks: vec![MusicTrack {
                id: id.to_string(),
                display_name: id.to_string(),
                asset_path: Some(asset_path.to_string()),
            }],
        }
    }

    #[test]
    fn registration_order_does_not_change_provider_or_track_order() {
        let a = AudioCatalogFragment::new("a", Some(music("alpha")), None).unwrap();
        let b = AudioCatalogFragment::new("b", Some(music("beta")), None).unwrap();
        let mut first = AudioCatalogRegistry::default();
        first.register(a.clone()).unwrap();
        first.register(b.clone()).unwrap();
        let mut second = AudioCatalogRegistry::default();
        second.register(b).unwrap();
        second.register(a).unwrap();

        assert_eq!(first.providers().collect::<Vec<_>>(), vec!["a", "b"]);
        assert_eq!(second.providers().collect::<Vec<_>>(), vec!["a", "b"]);
        assert_eq!(
            first
                .combined_music_registry("a")
                .unwrap()
                .tracks
                .iter()
                .map(|track| track.id.as_str())
                .collect::<Vec<_>>(),
            vec!["alpha", "beta"]
        );
        assert_eq!(
            second
                .combined_music_registry("a")
                .unwrap()
                .tracks
                .iter()
                .map(|track| track.id.as_str())
                .collect::<Vec<_>>(),
            vec!["alpha", "beta"]
        );
    }

    #[test]
    fn duplicate_music_ids_for_different_assets_report_both_providers() {
        let mut registry = AudioCatalogRegistry::default();
        registry
            .register(
                AudioCatalogFragment::new("a", Some(music_at("same", "a/x.ogg")), None).unwrap(),
            )
            .unwrap();
        // Same id, DIFFERENT asset — a genuine conflict.
        registry
            .register(
                AudioCatalogFragment::new("b", Some(music_at("same", "b/y.ogg")), None).unwrap(),
            )
            .unwrap();
        assert_eq!(
            registry.validate_global_music_ids().unwrap_err(),
            AudioCatalogError::DuplicateMusicTrack {
                track_id: "same".to_string(),
                first_provider: "a".to_string(),
                second_provider: "b".to_string(),
            }
        );
    }

    #[test]
    fn shared_track_across_providers_is_deduped_not_a_conflict() {
        // Ambition owns the superset; a demo carries a small subset pointing at
        // the SAME asset. Same id + same resolved path is benign.
        let mut registry = AudioCatalogRegistry::default();
        registry
            .register(AudioCatalogFragment::new("ambition", Some(music("shared")), None).unwrap())
            .unwrap();
        registry
            .register(
                AudioCatalogFragment::new(
                    "sanic",
                    Some(music_at("shared", "audio/music/generated/shared/full.ogg")),
                    None,
                )
                .unwrap(),
            )
            .unwrap();
        // `music("shared")` resolves to the conventional
        // `audio/music/generated/shared/full.ogg` — the same asset the demo
        // names explicitly, so validation and combination both accept it.
        registry.validate_global_music_ids().unwrap();
        let combined = registry.combined_music_registry("ambition").unwrap();
        assert_eq!(
            combined.tracks.iter().filter(|t| t.id == "shared").count(),
            1,
            "the shared track appears exactly once in the combined registry"
        );
    }

    #[test]
    fn failed_registration_leaves_the_previous_registry_intact() {
        let mut app = App::new();
        app.register_audio_catalog_fragment(
            AudioCatalogFragment::new("a", Some(music_at("same", "a/x.ogg")), None).unwrap(),
        );
        let error = app
            .try_register_audio_catalog_fragment(
                AudioCatalogFragment::new("b", Some(music_at("same", "b/y.ogg")), None).unwrap(),
            )
            .err()
            .expect("registration should fail");
        assert!(matches!(
            error,
            AudioCatalogError::DuplicateMusicTrack { .. }
        ));
        let registry = app.world().resource::<AudioCatalogRegistry>();
        assert_eq!(registry.providers().collect::<Vec<_>>(), vec!["a"]);
        assert!(registry.music_for("b").is_none());
    }

    #[test]
    fn separate_apps_hold_independent_audio_catalogs() {
        let mut app_a = App::new();
        app_a.register_audio_catalog_fragment(
            AudioCatalogFragment::new("a", Some(music("alpha")), None).unwrap(),
        );
        let mut app_b = App::new();
        app_b.register_audio_catalog_fragment(
            AudioCatalogFragment::new("b", Some(music("beta")), None).unwrap(),
        );
        assert!(app_a
            .world()
            .resource::<AudioCatalogRegistry>()
            .music_for("a")
            .is_some());
        assert!(app_a
            .world()
            .resource::<AudioCatalogRegistry>()
            .music_for("b")
            .is_none());
        assert!(app_b
            .world()
            .resource::<AudioCatalogRegistry>()
            .music_for("b")
            .is_some());
    }
}
