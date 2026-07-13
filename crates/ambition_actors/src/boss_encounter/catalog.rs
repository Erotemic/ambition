//! App-local composition of provider-authored boss data.
//!
//! A boss is assembled from five authored surfaces that must agree: behavior
//! profiles, encounter specs, sprite-sheet overrides, provider-owned sprite
//! filenames, and special-attack telegraph rows. Providers contribute immutable fragments; a Bevy [`App`]
//! assembles one deterministic [`BossCatalog`] resource. Runtime systems and
//! pure spawn helpers receive that catalog explicitly, so two Apps in one
//! process may host different boss sets without first-install-wins state.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use bevy::prelude::{App, Resource};

use super::behavior::BossBehaviorProfile;
use super::BossEncounterSpec;
use ambition_sprite_sheet::boss::BossSheetSpec;

/// One App's complete authored boss authority.
#[derive(Resource, Clone, Debug, Default)]
pub struct BossCatalog {
    behaviors: BTreeMap<String, BossBehaviorProfile>,
    encounters: BTreeMap<String, BossEncounterSpec>,
    sheets: BTreeMap<String, BossSheetSpec>,
    sprite_filenames: BTreeMap<String, String>,
    special_anim_keys: BTreeMap<String, Vec<String>>,
    fallback_boss_ids: BTreeMap<String, String>,
    fallback_sheet_keys: BTreeMap<String, String>,
}

impl BossCatalog {
    pub fn is_empty(&self) -> bool {
        self.behaviors.is_empty()
            && self.encounters.is_empty()
            && self.sheets.is_empty()
            && self.sprite_filenames.is_empty()
            && self.special_anim_keys.is_empty()
            && self.fallback_boss_ids.is_empty()
            && self.fallback_sheet_keys.is_empty()
    }

    pub fn behavior(&self, id: &str) -> Option<&BossBehaviorProfile> {
        self.behaviors.get(id)
    }

    pub fn encounter(&self, id: &str) -> Option<&BossEncounterSpec> {
        self.encounters.get(id)
    }

    pub fn encounter_specs(&self) -> impl Iterator<Item = &BossEncounterSpec> {
        self.encounters.values()
    }

    pub fn authored_sheet_keys(&self) -> impl Iterator<Item = &str> {
        self.sheets.keys().map(String::as_str)
    }

    pub fn has_authored_sheet(&self, key: &str) -> bool {
        self.sheets.contains_key(key)
    }

    pub fn sprite_filenames(&self) -> impl Iterator<Item = (&str, &str)> {
        self.sprite_filenames
            .iter()
            .map(|(key, filename)| (key.as_str(), filename.as_str()))
    }

    pub fn fallback_behavior(&self) -> Option<&BossBehaviorProfile> {
        let id = self.fallback_boss_id()?;
        self.behaviors.get(id)
    }

    /// The sole linked provider fallback, when unambiguous. Multiple games may
    /// each contribute a default; session authority must then choose one.
    pub fn fallback_boss_id(&self) -> Option<&str> {
        if self.fallback_boss_ids.len() == 1 {
            self.fallback_boss_ids.values().next().map(String::as_str)
        } else {
            None
        }
    }

    pub fn fallback_boss_id_for_provider(&self, provider_id: &str) -> Option<&str> {
        self.fallback_boss_ids.get(provider_id).map(String::as_str)
    }

    pub fn fallback_behavior_for_provider(
        &self,
        provider_id: &str,
    ) -> Option<&BossBehaviorProfile> {
        self.fallback_boss_id_for_provider(provider_id)
            .and_then(|id| self.behaviors.get(id))
    }

    /// The sole linked provider's default visual sheet, when unambiguous.
    /// Multi-game hosts choose a provider through active-session authority.
    pub fn fallback_sheet_key(&self) -> Option<&str> {
        if self.fallback_sheet_keys.len() == 1 {
            self.fallback_sheet_keys.values().next().map(String::as_str)
        } else {
            None
        }
    }

    pub fn fallback_sheet_key_for_provider(&self, provider_id: &str) -> Option<&str> {
        self.fallback_sheet_keys
            .get(provider_id)
            .map(String::as_str)
    }

    /// Resolve a content-authored sheet, then an engine built-in sheet, then
    /// the generic built-in boss sheet.
    pub fn sheet_for_key(&self, key: &str) -> BossSheetSpec {
        self.sheets
            .get(key)
            .cloned()
            .or_else(|| {
                ambition_sprite_sheet::boss::builtin_boss_sheets()
                    .get(key)
                    .cloned()
            })
            .unwrap_or_else(|| (*ambition_sprite_sheet::boss::BOSS_SHEET).clone())
    }

    /// Resolve render geometry for a live behavior. Providers usually key a
    /// sheet by behavior id; composite actors may instead point at another
    /// authored sheet through `sprite_target` (for example a rider borrowing
    /// its mount's geometry). A target is used only when it is an actual sheet
    /// key, so generator record names do not accidentally replace behavior ids.
    pub fn sheet_for_behavior(&self, behavior: &BossBehaviorProfile) -> BossSheetSpec {
        let builtins = ambition_sprite_sheet::boss::builtin_boss_sheets();
        let key = behavior
            .sprite_target
            .as_deref()
            .filter(|target| self.sheets.contains_key(*target) || builtins.contains_key(*target))
            .or_else(|| {
                (self.sheets.contains_key(&behavior.id) || builtins.contains_key(&behavior.id))
                    .then_some(behavior.id.as_str())
            })
            .or_else(|| self.fallback_sheet_key());
        key.map_or_else(
            || (*ambition_sprite_sheet::boss::BOSS_SHEET).clone(),
            |key| self.sheet_for_key(key),
        )
    }

    pub fn special_animation_keys(&self, key: &str) -> &[String] {
        self.special_anim_keys
            .get(key)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
}

/// One provider's immutable boss definitions.
#[derive(Clone, Debug, PartialEq)]
pub struct BossCatalogFragment {
    provider_id: String,
    fallback_boss_id: Option<String>,
    fallback_sheet_key: Option<String>,
    behaviors: BTreeMap<String, BossBehaviorProfile>,
    encounters: BTreeMap<String, BossEncounterSpec>,
    sheets: BTreeMap<String, BossSheetSpec>,
    sprite_filenames: BTreeMap<String, String>,
    special_anim_keys: BTreeMap<String, Vec<String>>,
}

impl BossCatalogFragment {
    #[allow(clippy::too_many_arguments)]
    pub fn from_ron(
        provider_id: impl Into<String>,
        fallback_boss_id: Option<impl Into<String>>,
        fallback_sheet_key: Option<impl Into<String>>,
        behavior_profiles_ron: &str,
        encounter_rons: &[&str],
        boss_sheets_ron: &str,
        sprite_filenames: BTreeMap<String, String>,
        special_anim_keys: BTreeMap<String, Vec<String>>,
    ) -> Result<Self, BossCatalogAssemblyError> {
        let provider_id = provider_id.into();
        let behaviors =
            ron::from_str::<BTreeMap<String, BossBehaviorProfile>>(behavior_profiles_ron).map_err(
                |error| BossCatalogAssemblyError::MalformedBehaviorProfiles {
                    provider_id: provider_id.clone(),
                    message: error.to_string(),
                },
            )?;
        let sheets =
            ron::from_str::<BTreeMap<String, BossSheetSpec>>(boss_sheets_ron).map_err(|error| {
                BossCatalogAssemblyError::MalformedSheets {
                    provider_id: provider_id.clone(),
                    message: error.to_string(),
                }
            })?;
        let mut encounters = BTreeMap::new();
        for encounter_ron in encounter_rons {
            let spec = ron::from_str::<BossEncounterSpec>(encounter_ron).map_err(|error| {
                BossCatalogAssemblyError::MalformedEncounter {
                    provider_id: provider_id.clone(),
                    message: error.to_string(),
                }
            })?;
            if encounters.insert(spec.id.clone(), spec).is_some() {
                return Err(BossCatalogAssemblyError::DuplicateEncounterInFragment { provider_id });
            }
        }
        let fragment = Self {
            provider_id,
            fallback_boss_id: fallback_boss_id.map(Into::into),
            fallback_sheet_key: fallback_sheet_key.map(Into::into),
            behaviors,
            encounters,
            sheets,
            sprite_filenames,
            special_anim_keys,
        };
        fragment.validate()?;
        Ok(fragment)
    }

    pub fn provider_id(&self) -> &str {
        &self.provider_id
    }

    pub fn fallback_boss_id(&self) -> Option<&str> {
        self.fallback_boss_id.as_deref()
    }

    pub fn fallback_sheet_key(&self) -> Option<&str> {
        self.fallback_sheet_key.as_deref()
    }

    fn validate(&self) -> Result<(), BossCatalogAssemblyError> {
        if self.provider_id.trim().is_empty() {
            return Err(BossCatalogAssemblyError::EmptyProviderId);
        }
        for (id, behavior) in &self.behaviors {
            if id.trim().is_empty() || behavior.id.trim().is_empty() {
                return Err(BossCatalogAssemblyError::EmptyBossId {
                    provider_id: self.provider_id.clone(),
                });
            }
            if behavior.id != *id {
                return Err(BossCatalogAssemblyError::BehaviorIdMismatch {
                    provider_id: self.provider_id.clone(),
                    map_id: id.clone(),
                    profile_id: behavior.id.clone(),
                });
            }
        }
        for id in self.encounters.keys() {
            if id.trim().is_empty() {
                return Err(BossCatalogAssemblyError::EmptyBossId {
                    provider_id: self.provider_id.clone(),
                });
            }
            if !self.behaviors.contains_key(id) {
                return Err(BossCatalogAssemblyError::MissingBehavior {
                    provider_id: self.provider_id.clone(),
                    boss_id: id.clone(),
                });
            }
        }
        let missing_encounters: BTreeSet<&str> = self
            .behaviors
            .keys()
            .filter(|id| !self.encounters.contains_key(*id))
            .map(String::as_str)
            .collect();
        if let Some(id) = missing_encounters.first() {
            return Err(BossCatalogAssemblyError::MissingEncounter {
                provider_id: self.provider_id.clone(),
                boss_id: (*id).to_string(),
            });
        }
        if let Some(fallback) = self.fallback_boss_id.as_deref() {
            if fallback.trim().is_empty() || !self.behaviors.contains_key(fallback) {
                return Err(BossCatalogAssemblyError::MissingFallbackBoss {
                    provider_id: self.provider_id.clone(),
                    boss_id: fallback.to_string(),
                });
            }
        }
        if let Some(sheet_key) = self.fallback_sheet_key.as_deref() {
            if sheet_key.trim().is_empty()
                || !self.sheets.contains_key(sheet_key)
                || !self.sprite_filenames.contains_key(sheet_key)
            {
                return Err(BossCatalogAssemblyError::MissingFallbackSheet {
                    provider_id: self.provider_id.clone(),
                    sheet_key: sheet_key.to_string(),
                });
            }
        }
        if let Some(key) = self.sheets.keys().find(|key| key.trim().is_empty()) {
            return Err(BossCatalogAssemblyError::EmptySheetKey {
                provider_id: self.provider_id.clone(),
                sheet_key: key.clone(),
            });
        }
        for (key, filename) in &self.sprite_filenames {
            if key.trim().is_empty() || filename.trim().is_empty() {
                return Err(BossCatalogAssemblyError::InvalidSpriteFilename {
                    provider_id: self.provider_id.clone(),
                    sheet_key: key.clone(),
                    filename: filename.clone(),
                });
            }
        }
        for key in self.sheets.keys() {
            if !self.sprite_filenames.contains_key(key) {
                return Err(BossCatalogAssemblyError::MissingSpriteFilename {
                    provider_id: self.provider_id.clone(),
                    sheet_key: key.clone(),
                });
            }
        }
        for (special, rows) in &self.special_anim_keys {
            if special.trim().is_empty() || rows.iter().any(|row| row.trim().is_empty()) {
                return Err(BossCatalogAssemblyError::InvalidSpecialAnimation {
                    provider_id: self.provider_id.clone(),
                    special: special.clone(),
                });
            }
        }
        Ok(())
    }
}

/// Provider fragments linked into one App.
#[derive(Resource, Clone, Debug, Default)]
pub struct BossCatalogRegistry {
    fragments: BTreeMap<String, BossCatalogFragment>,
}

impl BossCatalogRegistry {
    pub fn providers(&self) -> impl Iterator<Item = &str> {
        self.fragments.keys().map(String::as_str)
    }

    pub fn register(
        &mut self,
        fragment: BossCatalogFragment,
    ) -> Result<(), BossCatalogAssemblyError> {
        fragment.validate()?;
        if let Some(existing) = self.fragments.get(&fragment.provider_id) {
            if existing == &fragment {
                return Ok(());
            }
            return Err(BossCatalogAssemblyError::DuplicateProvider {
                provider_id: fragment.provider_id,
            });
        }
        self.fragments
            .insert(fragment.provider_id.clone(), fragment);
        Ok(())
    }

    pub fn assemble(&self) -> Result<BossCatalog, BossCatalogAssemblyError> {
        let mut behaviors = BTreeMap::new();
        let mut encounters = BTreeMap::new();
        let mut sheets = BTreeMap::new();
        let mut sprite_filenames = BTreeMap::new();
        let mut special_anim_keys = BTreeMap::new();
        let mut behavior_owners = BTreeMap::<String, String>::new();
        let mut sheet_owners = BTreeMap::<String, String>::new();
        let mut sprite_owners = BTreeMap::<String, String>::new();
        let mut special_owners = BTreeMap::<String, String>::new();
        let mut fallback_boss_ids = BTreeMap::new();
        let mut fallback_sheet_keys = BTreeMap::new();

        for (provider_id, fragment) in &self.fragments {
            for (id, behavior) in &fragment.behaviors {
                if let Some(first_provider) = behavior_owners.get(id) {
                    return Err(BossCatalogAssemblyError::DuplicateBoss {
                        boss_id: id.clone(),
                        first_provider: first_provider.clone(),
                        second_provider: provider_id.clone(),
                    });
                }
                behavior_owners.insert(id.clone(), provider_id.clone());
                behaviors.insert(id.clone(), behavior.clone());
                encounters.insert(
                    id.clone(),
                    fragment
                        .encounters
                        .get(id)
                        .expect("fragment validation pairs behavior and encounter")
                        .clone(),
                );
            }
            for (key, sheet) in &fragment.sheets {
                if let Some(first_provider) = sheet_owners.get(key) {
                    return Err(BossCatalogAssemblyError::DuplicateSheet {
                        sheet_key: key.clone(),
                        first_provider: first_provider.clone(),
                        second_provider: provider_id.clone(),
                    });
                }
                sheet_owners.insert(key.clone(), provider_id.clone());
                sheets.insert(key.clone(), sheet.clone());
            }
            for (key, filename) in &fragment.sprite_filenames {
                if let Some(first_provider) = sprite_owners.get(key) {
                    return Err(BossCatalogAssemblyError::DuplicateSpriteFilename {
                        sheet_key: key.clone(),
                        first_provider: first_provider.clone(),
                        second_provider: provider_id.clone(),
                    });
                }
                sprite_owners.insert(key.clone(), provider_id.clone());
                sprite_filenames.insert(key.clone(), filename.clone());
            }
            for (key, rows) in &fragment.special_anim_keys {
                if let Some(first_provider) = special_owners.get(key) {
                    return Err(BossCatalogAssemblyError::DuplicateSpecialAnimation {
                        special: key.clone(),
                        first_provider: first_provider.clone(),
                        second_provider: provider_id.clone(),
                    });
                }
                special_owners.insert(key.clone(), provider_id.clone());
                special_anim_keys.insert(key.clone(), rows.clone());
            }
            if let Some(boss_id) = fragment.fallback_boss_id.as_ref() {
                fallback_boss_ids.insert(provider_id.clone(), boss_id.clone());
            }
            if let Some(sheet_key) = fragment.fallback_sheet_key.as_ref() {
                fallback_sheet_keys.insert(provider_id.clone(), sheet_key.clone());
            }
        }

        Ok(BossCatalog {
            behaviors,
            encounters,
            sheets,
            sprite_filenames,
            special_anim_keys,
            fallback_boss_ids,
            fallback_sheet_keys,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BossCatalogAssemblyError {
    EmptyProviderId,
    EmptyBossId {
        provider_id: String,
    },
    DuplicateProvider {
        provider_id: String,
    },
    MalformedBehaviorProfiles {
        provider_id: String,
        message: String,
    },
    MalformedEncounter {
        provider_id: String,
        message: String,
    },
    MalformedSheets {
        provider_id: String,
        message: String,
    },
    DuplicateEncounterInFragment {
        provider_id: String,
    },
    BehaviorIdMismatch {
        provider_id: String,
        map_id: String,
        profile_id: String,
    },
    MissingBehavior {
        provider_id: String,
        boss_id: String,
    },
    MissingEncounter {
        provider_id: String,
        boss_id: String,
    },
    MissingFallbackBoss {
        provider_id: String,
        boss_id: String,
    },
    MissingFallbackSheet {
        provider_id: String,
        sheet_key: String,
    },
    EmptySheetKey {
        provider_id: String,
        sheet_key: String,
    },
    InvalidSpriteFilename {
        provider_id: String,
        sheet_key: String,
        filename: String,
    },
    MissingSpriteFilename {
        provider_id: String,
        sheet_key: String,
    },
    InvalidSpecialAnimation {
        provider_id: String,
        special: String,
    },
    DuplicateBoss {
        boss_id: String,
        first_provider: String,
        second_provider: String,
    },
    DuplicateSheet {
        sheet_key: String,
        first_provider: String,
        second_provider: String,
    },
    DuplicateSpriteFilename {
        sheet_key: String,
        first_provider: String,
        second_provider: String,
    },
    DuplicateSpecialAnimation {
        special: String,
        first_provider: String,
        second_provider: String,
    },
}

impl fmt::Display for BossCatalogAssemblyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyProviderId => write!(f, "boss catalog provider id must not be empty"),
            Self::EmptyBossId { provider_id } => {
                write!(f, "boss catalog fragment '{provider_id}' contains an empty boss id")
            }
            Self::DuplicateProvider { provider_id } => {
                write!(f, "boss catalog provider '{provider_id}' registered twice")
            }
            Self::MalformedBehaviorProfiles { provider_id, message } => write!(
                f,
                "boss behavior fragment '{provider_id}' is malformed RON: {message}"
            ),
            Self::MalformedEncounter { provider_id, message } => write!(
                f,
                "boss encounter fragment '{provider_id}' is malformed RON: {message}"
            ),
            Self::MalformedSheets { provider_id, message } => {
                write!(f, "boss sheet fragment '{provider_id}' is malformed RON: {message}")
            }
            Self::DuplicateEncounterInFragment { provider_id } => write!(
                f,
                "boss catalog fragment '{provider_id}' contains duplicate encounter ids"
            ),
            Self::BehaviorIdMismatch { provider_id, map_id, profile_id } => write!(
                f,
                "boss catalog fragment '{provider_id}' maps boss '{map_id}' to behavior id '{profile_id}'"
            ),
            Self::MissingBehavior { provider_id, boss_id } => write!(
                f,
                "boss catalog fragment '{provider_id}' has encounter '{boss_id}' without behavior"
            ),
            Self::MissingEncounter { provider_id, boss_id } => write!(
                f,
                "boss catalog fragment '{provider_id}' has behavior '{boss_id}' without encounter"
            ),
            Self::MissingFallbackBoss { provider_id, boss_id } => write!(
                f,
                "boss catalog fragment '{provider_id}' names missing fallback boss '{boss_id}'"
            ),
            Self::MissingFallbackSheet { provider_id, sheet_key } => write!(
                f,
                "boss catalog fragment '{provider_id}' names missing fallback sheet '{sheet_key}'"
            ),
            Self::EmptySheetKey { provider_id, sheet_key } => write!(
                f,
                "boss catalog fragment '{provider_id}' contains empty sheet key '{sheet_key}'"
            ),
            Self::InvalidSpriteFilename { provider_id, sheet_key, filename } => write!(
                f,
                "boss catalog fragment '{provider_id}' has invalid sprite filename '{filename}' for '{sheet_key}'"
            ),
            Self::MissingSpriteFilename { provider_id, sheet_key } => write!(
                f,
                "boss catalog fragment '{provider_id}' has sheet '{sheet_key}' without a sprite filename"
            ),
            Self::InvalidSpecialAnimation { provider_id, special } => write!(
                f,
                "boss catalog fragment '{provider_id}' has invalid special-animation row '{special}'"
            ),
            Self::DuplicateBoss { boss_id, first_provider, second_provider } => write!(
                f,
                "boss id '{boss_id}' is authored by both '{first_provider}' and '{second_provider}'"
            ),
            Self::DuplicateSheet { sheet_key, first_provider, second_provider } => write!(
                f,
                "boss sheet key '{sheet_key}' is authored by both '{first_provider}' and '{second_provider}'"
            ),
            Self::DuplicateSpriteFilename { sheet_key, first_provider, second_provider } => write!(
                f,
                "boss sprite asset key '{sheet_key}' is authored by both '{first_provider}' and '{second_provider}'"
            ),
            Self::DuplicateSpecialAnimation { special, first_provider, second_provider } => write!(
                f,
                "boss special animation '{special}' is authored by both '{first_provider}' and '{second_provider}'"
            ),
        }
    }
}

impl std::error::Error for BossCatalogAssemblyError {}

pub trait BossCatalogAppExt {
    fn try_register_boss_catalog_fragment(
        &mut self,
        fragment: BossCatalogFragment,
    ) -> Result<&mut Self, BossCatalogAssemblyError>;

    fn register_boss_catalog_fragment(&mut self, fragment: BossCatalogFragment) -> &mut Self {
        self.try_register_boss_catalog_fragment(fragment)
            .unwrap_or_else(|error| panic!("{error}"))
    }
}

impl BossCatalogAppExt for App {
    fn try_register_boss_catalog_fragment(
        &mut self,
        fragment: BossCatalogFragment,
    ) -> Result<&mut Self, BossCatalogAssemblyError> {
        let (registry, catalog) = {
            let mut candidate = self
                .world()
                .get_resource::<BossCatalogRegistry>()
                .cloned()
                .unwrap_or_default();
            candidate.register(fragment)?;
            let catalog = candidate.assemble()?;
            (candidate, catalog)
        };
        self.insert_resource(registry).insert_resource(catalog);
        Ok(self)
    }
}

#[cfg(test)]
fn test_boss_sprite_filenames() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("gradient_sentinel".into(), "boss_spritesheet.png".into()),
        (
            "mockingbird".into(),
            "mockingbird_boss/mockingbird_boss_spritesheet.png".into(),
        ),
        (
            "smirking_behemoth_boss".into(),
            "smirking_behemoth_boss_spritesheet.png".into(),
        ),
        (
            "giant_gnu".into(),
            "gnu_ton_boss/giant_gnu_spritesheet.png".into(),
        ),
        (
            "gnu_ton_rider".into(),
            "gnu_ton_boss/gnu_ton_rider_spritesheet.png".into(),
        ),
        (
            "flying_spaghetti_monster_boss".into(),
            "flying_spaghetti_monster_boss_spritesheet.png".into(),
        ),
        ("trex_boss".into(), "trex_enemy_spritesheet.png".into()),
    ])
}

#[cfg(test)]
pub(crate) fn test_boss_catalog() -> &'static BossCatalog {
    static CATALOG: std::sync::LazyLock<BossCatalog> = std::sync::LazyLock::new(|| {
        let encounters: &[&str] = &[
            include_str!("../../../../game/ambition_content/assets/data/boss_encounters/clockwork_warden.ron"),
            include_str!("../../../../game/ambition_content/assets/data/boss_encounters/mockingbird.ron"),
            include_str!("../../../../game/ambition_content/assets/data/boss_encounters/gnu_ton_rider.ron"),
            include_str!("../../../../game/ambition_content/assets/data/boss_encounters/smirking_behemoth_boss.ron"),
            include_str!("../../../../game/ambition_content/assets/data/boss_encounters/flying_spaghetti_monster_boss.ron"),
            include_str!("../../../../game/ambition_content/assets/data/boss_encounters/trex_boss.ron"),
            include_str!("../../../../game/ambition_content/assets/data/boss_encounters/mode_collapse_boss.ron"),
            include_str!("../../../../game/ambition_content/assets/data/boss_encounters/exploding_gradient_boss.ron"),
            include_str!("../../../../game/ambition_content/assets/data/boss_encounters/overflow_boss.ron"),
        ];
        let special_anim_keys = BTreeMap::from([
            (
                "overfit_volley".into(),
                vec!["spike_halo".into(), "eye_beam".into()],
            ),
            (
                "eye_beam".into(),
                vec!["eye_beam".into(), "spike_halo".into()],
            ),
            ("minima_trap".into(), vec!["spike_halo".into()]),
            ("saddle_point".into(), vec!["spike_halo".into()]),
            ("gradient_cascade".into(), vec!["spike_halo".into()]),
            ("mode_collapse_converge".into(), vec!["spike_halo".into()]),
            ("gradient_nova".into(), vec!["spike_halo".into()]),
            ("overflow_flood".into(), vec!["spike_halo".into()]),
            (
                "seismic_stomp".into(),
                vec!["floor_slam".into(), "spike_halo".into()],
            ),
            (
                "echo_fan".into(),
                vec!["spike_halo".into(), "eye_beam".into()],
            ),
        ]);
        let fragment = BossCatalogFragment::from_ron(
            "ambition-test",
            Some("clockwork_warden"),
            Some("gradient_sentinel"),
            include_str!("../../../../game/ambition_content/assets/data/boss_profiles.ron"),
            encounters,
            include_str!("../../../../game/ambition_content/assets/data/boss_sheets.ron"),
            test_boss_sprite_filenames(),
            special_anim_keys,
        )
        .expect("Ambition boss fixture should be valid");
        let mut registry = BossCatalogRegistry::default();
        registry.register(fragment).unwrap();
        registry.assemble().unwrap()
    });
    &CATALOG
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fragment(provider: &str) -> BossCatalogFragment {
        let encounters: &[&str] = &[
            include_str!("../../../../game/ambition_content/assets/data/boss_encounters/clockwork_warden.ron"),
            include_str!("../../../../game/ambition_content/assets/data/boss_encounters/mockingbird.ron"),
            include_str!("../../../../game/ambition_content/assets/data/boss_encounters/gnu_ton_rider.ron"),
            include_str!("../../../../game/ambition_content/assets/data/boss_encounters/smirking_behemoth_boss.ron"),
            include_str!("../../../../game/ambition_content/assets/data/boss_encounters/flying_spaghetti_monster_boss.ron"),
            include_str!("../../../../game/ambition_content/assets/data/boss_encounters/trex_boss.ron"),
            include_str!("../../../../game/ambition_content/assets/data/boss_encounters/mode_collapse_boss.ron"),
            include_str!("../../../../game/ambition_content/assets/data/boss_encounters/exploding_gradient_boss.ron"),
            include_str!("../../../../game/ambition_content/assets/data/boss_encounters/overflow_boss.ron"),
        ];
        BossCatalogFragment::from_ron(
            provider,
            Some("clockwork_warden"),
            Some("gradient_sentinel"),
            include_str!("../../../../game/ambition_content/assets/data/boss_profiles.ron"),
            encounters,
            include_str!("../../../../game/ambition_content/assets/data/boss_sheets.ron"),
            test_boss_sprite_filenames(),
            BTreeMap::new(),
        )
        .unwrap()
    }

    fn renamed_single_boss_fragment(provider: &str, boss_id: &str) -> BossCatalogFragment {
        let source = fragment("source");
        let mut behavior = source
            .behaviors
            .get("clockwork_warden")
            .expect("fixture behavior")
            .clone();
        behavior.id = boss_id.to_string();
        let mut encounter = source
            .encounters
            .get("clockwork_warden")
            .expect("fixture encounter")
            .clone();
        encounter.id = boss_id.to_string();
        encounter.name = format!("{provider} boss");
        BossCatalogFragment {
            provider_id: provider.to_string(),
            fallback_boss_id: Some(boss_id.to_string()),
            fallback_sheet_key: None,
            behaviors: BTreeMap::from([(boss_id.to_string(), behavior)]),
            encounters: BTreeMap::from([(boss_id.to_string(), encounter)]),
            sheets: BTreeMap::new(),
            sprite_filenames: BTreeMap::new(),
            special_anim_keys: BTreeMap::new(),
        }
    }

    #[test]
    fn separate_apps_are_isolated_and_failed_registration_is_transactional() {
        let mut first = App::new();
        first.register_boss_catalog_fragment(fragment("a"));
        let second = App::new();
        assert!(first
            .world()
            .resource::<BossCatalog>()
            .behavior("clockwork_warden")
            .is_some());
        assert!(!second.world().contains_resource::<BossCatalog>());

        let error = first
            .try_register_boss_catalog_fragment(fragment("b"))
            .err()
            .expect("duplicate boss ids must fail");
        assert!(matches!(
            error,
            BossCatalogAssemblyError::DuplicateBoss { .. }
        ));
        assert_eq!(
            first
                .world()
                .resource::<BossCatalogRegistry>()
                .providers()
                .collect::<Vec<_>>(),
            vec!["a"]
        );
    }

    #[test]
    fn provider_defaults_coexist_without_one_process_global_winner() {
        let mut registry = BossCatalogRegistry::default();
        registry
            .register(renamed_single_boss_fragment("alpha", "alpha_boss"))
            .unwrap();
        registry
            .register(renamed_single_boss_fragment("beta", "beta_boss"))
            .unwrap();
        let catalog = registry.assemble().unwrap();
        assert_eq!(
            catalog.fallback_boss_id_for_provider("alpha"),
            Some("alpha_boss")
        );
        assert_eq!(
            catalog.fallback_boss_id_for_provider("beta"),
            Some("beta_boss")
        );
        assert_eq!(
            catalog.fallback_boss_id(),
            None,
            "multiple provider defaults require active-session selection"
        );
    }

    #[test]
    fn behavior_sheet_resolution_uses_provider_fallback_and_explicit_targets() {
        let catalog = test_boss_catalog();
        let clockwork = catalog.behavior("clockwork_warden").unwrap();
        assert_eq!(
            catalog.sheet_for_behavior(clockwork),
            catalog.sheet_for_key("gradient_sentinel"),
            "a provider's fallback visual owns bosses without a dedicated sheet"
        );

        let mut rider = catalog.behavior("gnu_ton_rider").unwrap().clone();
        rider.sprite_target = Some("giant_gnu".into());
        assert_eq!(
            catalog.sheet_for_behavior(&rider),
            catalog.sheet_for_key("giant_gnu"),
            "an explicit authored sheet target overrides the provider fallback"
        );
    }

    #[test]
    fn registration_order_is_deterministic() {
        let alpha = renamed_single_boss_fragment("alpha", "alpha_boss");
        let beta = renamed_single_boss_fragment("beta", "beta_boss");

        let mut direct = BossCatalogRegistry::default();
        direct.register(alpha.clone()).unwrap();
        direct.register(beta.clone()).unwrap();

        let mut reverse = BossCatalogRegistry::default();
        reverse.register(beta).unwrap();
        reverse.register(alpha).unwrap();

        assert_eq!(
            direct.providers().collect::<Vec<_>>(),
            vec!["alpha", "beta"]
        );
        assert_eq!(
            reverse.providers().collect::<Vec<_>>(),
            vec!["alpha", "beta"]
        );

        let direct = direct.assemble().unwrap();
        let reverse = reverse.assemble().unwrap();
        for boss_id in ["alpha_boss", "beta_boss"] {
            assert_eq!(direct.behavior(boss_id), reverse.behavior(boss_id));
            assert_eq!(direct.encounter(boss_id), reverse.encounter(boss_id));
        }
        for provider in ["alpha", "beta"] {
            assert_eq!(
                direct.fallback_boss_id_for_provider(provider),
                reverse.fallback_boss_id_for_provider(provider)
            );
        }
    }
}
