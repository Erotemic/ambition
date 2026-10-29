//! The authored gameplay defaults format. Callers supply its source text.

use bevy::prelude::Resource;
use bevy::asset::Asset;
use bevy::reflect::TypePath;
use serde::Deserialize;

use ambition_platformer2d_core as ae;

#[derive(Clone, Debug, Deserialize, Asset, TypePath, Resource)]
pub struct Platformer2dGameplayDefaults {
    pub abilities: ae::AbilitySet,
    pub tuning: ae::MovementTuning,
    /// The combat and time feel values this game changes. A field the file
    /// does not state keeps `Platformer2dFeelTuningMonolith::default()`.
    #[serde(default)]
    pub feel: ambition_combat::feel::Platformer2dFeelTuningMonolith,
}

impl Platformer2dGameplayDefaults {
    pub fn parse(text: &str) -> Result<Self, ron::error::SpannedError> {
        ron::from_str(text)
    }
}
