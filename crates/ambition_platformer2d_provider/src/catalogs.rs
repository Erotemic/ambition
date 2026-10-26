//! Provider catalog declarations, registration conflicts and readiness facts.

use ambition_audio::catalog::AudioCatalogRegistry;
use ambition_characters::actor::character_catalog::CharacterCatalog;
use ambition_game_shell::{PREPARE_AUDIO_WORK_ID, PREPARE_CATALOGS_WORK_ID};
use bevy::prelude::Resource;
use std::collections::BTreeMap;

/// The catalog identity a provider authors: its starting character, its audio
/// provider id, and which audio fragments a prepared session must find.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthoredCatalogFragments {
    pub starting_character: String,
    pub audio_provider: String,
    pub expects_music: bool,
    pub expects_procedural_sfx: bool,
    pub expects_adaptive_cues: bool,
    pub expects_packed_sfx: bool,
}

impl AuthoredCatalogFragments {
    pub fn new(starting_character: impl Into<String>, audio_provider: impl Into<String>) -> Self {
        Self {
            starting_character: starting_character.into(),
            audio_provider: audio_provider.into(),
            expects_music: false,
            expects_procedural_sfx: false,
            expects_adaptive_cues: false,
            expects_packed_sfx: false,
        }
    }

    pub fn with_music(mut self) -> Self {
        self.expects_music = true;
        self
    }

    pub fn with_procedural_sfx(mut self) -> Self {
        self.expects_procedural_sfx = true;
        self
    }

    pub fn with_adaptive_cues(mut self) -> Self {
        self.expects_adaptive_cues = true;
        self
    }

    pub fn with_packed_sfx(mut self) -> Self {
        self.expects_packed_sfx = true;
        self
    }

    pub fn validate(
        &self,
        character_catalog: &CharacterCatalog,
        audio_catalogs: &AudioCatalogRegistry,
    ) -> Option<(&'static str, ambition_load::LoadFailure)> {
        if character_catalog
            .get(self.starting_character.as_str())
            .is_none()
        {
            return Some((
                PREPARE_CATALOGS_WORK_ID,
                ambition_load::LoadFailure::new(
                    "Starting character data is unavailable",
                    format!("character catalog has no '{}' row", self.starting_character),
                )
                .retryable(true),
            ));
        }
        if !audio_catalogs.has_provider(self.audio_provider.as_str()) {
            return Some((
                PREPARE_AUDIO_WORK_ID,
                ambition_load::LoadFailure::new(
                    "Provider audio intent is unavailable",
                    format!(
                        "provider '{}' registered no explicit audio fragment",
                        self.audio_provider
                    ),
                )
                .retryable(true),
            ));
        }
        None
    }
}

/// App-local map from experience id to its authored catalog fragments — the
/// authority the shared preparation systems validate against.
#[derive(Resource, Default)]
pub struct PlatformerAuthoredCatalogRegistry {
    by_experience: BTreeMap<String, AuthoredCatalogFragments>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlatformerAuthoringRegistrationError {
    EmptyExperienceId,
    Conflict {
        experience_id: String,
        existing: AuthoredCatalogFragments,
        candidate: AuthoredCatalogFragments,
    },
}

impl std::fmt::Display for PlatformerAuthoringRegistrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyExperienceId => write!(f, "platformer experience id must not be empty"),
            Self::Conflict { experience_id, .. } => write!(
                f,
                "platformer experience '{experience_id}' registered conflicting authored catalogs"
            ),
        }
    }
}

impl std::error::Error for PlatformerAuthoringRegistrationError {}

impl PlatformerAuthoredCatalogRegistry {
    pub fn get(&self, experience_id: &str) -> Option<&AuthoredCatalogFragments> {
        self.by_experience.get(experience_id)
    }

    pub fn try_register(
        &mut self,
        experience_id: &str,
        fragments: AuthoredCatalogFragments,
    ) -> Result<(), PlatformerAuthoringRegistrationError> {
        if experience_id.trim().is_empty() {
            return Err(PlatformerAuthoringRegistrationError::EmptyExperienceId);
        }
        if let Some(existing) = self.by_experience.get(experience_id) {
            if existing == &fragments {
                return Ok(());
            }
            return Err(PlatformerAuthoringRegistrationError::Conflict {
                experience_id: experience_id.to_owned(),
                existing: existing.clone(),
                candidate: fragments,
            });
        }
        self.by_experience
            .insert(experience_id.to_owned(), fragments);
        Ok(())
    }

    pub fn deterministic_dump(&self) -> String {
        let mut out = String::new();
        for (experience, fragment) in &self.by_experience {
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                experience,
                fragment.starting_character,
                fragment.audio_provider,
                fragment.expects_music,
                fragment.expects_procedural_sfx,
                fragment.expects_adaptive_cues,
                fragment.expects_packed_sfx,
            ));
        }
        out
    }
}
