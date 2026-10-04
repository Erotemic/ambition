//! App-local composition of provider-authored boss data.
//!
//! A boss is assembled from five authored surfaces that must agree: behavior
//! profiles, encounter specs, sprite-sheet overrides, provider-owned sprite
//! filenames, and special-attack telegraph rows. Providers contribute
//! immutable fragments; a Bevy [`App`] assembles one deterministic
//! [`BossCatalog`] resource. Runtime systems and pure spawn helpers receive
//! that catalog explicitly, so two Apps in one process may host different
//! boss sets.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use bevy::prelude::{App, Resource};

use super::behavior::BossBehaviorProfile;
use super::BossEncounterSpec;
use ambition_sprite_sheet::boss::BossSheetSpec;

/// The state a provider's code keeps on one boss, put on the boss in the batch
/// that builds it: a content conductor's memory, for example.
///
/// It receives the boss's scope and its body as built. It is code, not
/// content, so it is not part of the catalog's canonical dump; the code that
/// reads the state it inserts is not in the dump either.
pub type BossBirthKit = fn(
    &mut ambition_platformer2d_shared_tangle::construction::EntityScope,
    &ambition_platformer2d_core::BodyKinematics,
);

/// One App's complete authored boss authority.
#[derive(Resource, Clone, Debug, Default, serde::Serialize)]
pub struct BossCatalog {
    behaviors: BTreeMap<String, BossBehaviorProfile>,
    encounters: BTreeMap<String, BossEncounterSpec>,
    sheets: BTreeMap<String, BossSheetSpec>,
    sprite_filenames: BTreeMap<String, String>,
    special_anim_keys: BTreeMap<String, Vec<String>>,
    strike_anim_keys: BTreeMap<String, Vec<String>>,
    strike_anims: BTreeMap<String, Option<ambition_sprite_sheet::boss::BossAnim>>,
    hurtbox_sample_rows: BTreeMap<String, String>,
    fallback_boss_ids: BTreeMap<String, String>,
    fallback_sheet_keys: BTreeMap<String, String>,
    #[serde(skip)]
    birth_kits: BTreeMap<String, BossBirthKit>,
}

impl BossCatalog {
    /// Canonical generation material for every boss this App holds.
    ///
    /// The boss catalog is mechanical content: `PlatformerSessionBuilder`
    /// consumes it, and it carries behavior profiles, encounter definitions,
    /// sheet specs and fallback identities. So it must reach the
    /// fingerprint; otherwise two compositions could differ in how a boss
    /// fights and share one `PreparedContentIdentity`.
    ///
    /// Every field is a `BTreeMap`, so iteration order follows the content,
    /// not registration order. The derived `Serialize` keeps this exhaustive
    /// when a field is added.
    ///
    /// It returns an error and does not hash an error message: two catalogs
    /// failing for the same reason would hash identically, and that hash is
    /// what the rollback timeline contract compares.
    pub fn deterministic_dump(&self) -> Result<String, String> {
        ron::to_string(self).map_err(|error| {
            format!("the boss catalog cannot be rendered as canonical generation material: {error}")
        })
    }

    pub fn is_empty(&self) -> bool {
        self.behaviors.is_empty()
            && self.encounters.is_empty()
            && self.sheets.is_empty()
            && self.sprite_filenames.is_empty()
            && self.special_anim_keys.is_empty()
            && self.strike_anim_keys.is_empty()
            && self.strike_anims.is_empty()
            && self.hurtbox_sample_rows.is_empty()
            && self.fallback_boss_ids.is_empty()
            && self.fallback_sheet_keys.is_empty()
    }

    pub fn behavior(&self, id: &str) -> Option<&BossBehaviorProfile> {
        self.behaviors.get(id)
    }

    pub fn encounter(&self, id: &str) -> Option<&BossEncounterSpec> {
        self.encounters.get(id)
    }

    /// The kit that finishes boss `id` at construction, if its provider gave
    /// one. See [`BossBirthKit`].
    pub fn birth_kit(&self, id: &str) -> Option<BossBirthKit> {
        self.birth_kits.get(id).copied()
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

    /// The content-authored sheet for `key`, else the provider's fallback
    /// sheet, else the unauthored layout
    /// ([`BossSheetSpec::unauthored`](ambition_sprite_sheet::boss::BossSheetSpec::unauthored))
    /// for a catalog that authors no sheets.
    pub fn sheet_for_key(&self, key: &str) -> BossSheetSpec {
        self.sheets
            .get(key)
            .or_else(|| self.sheets.get(self.fallback_sheet_key()?))
            .cloned()
            .unwrap_or_else(ambition_sprite_sheet::boss::BossSheetSpec::unauthored)
    }

    /// Resolve render geometry for a live behavior. Providers usually key a
    /// sheet by behavior id; composite actors may instead point at another
    /// authored sheet through `sprite_target` (for example a rider borrowing
    /// its mount's geometry). A target is used only when it is an actual sheet
    /// key, so generator record names do not accidentally replace behavior ids.
    pub fn sheet_for_behavior(&self, behavior: &BossBehaviorProfile) -> BossSheetSpec {
        self.worn_sheet_key(behavior).map_or_else(
            ambition_sprite_sheet::boss::BossSheetSpec::unauthored,
            |key| self.sheet_for_key(key),
        )
    }

    /// The sheet key a behavior wears: its `sprite_target` when that is a
    /// sheet key, else its own id when that is one, else the provider's
    /// fallback sheet.
    pub fn worn_sheet_key<'a>(&'a self, behavior: &'a BossBehaviorProfile) -> Option<&'a str> {
        behavior
            .sprite_target
            .as_deref()
            .filter(|target| self.sheets.contains_key(*target))
            .or_else(|| {
                self.sheets
                    .contains_key(&behavior.id)
                    .then_some(behavior.id.as_str())
            })
            .or_else(|| self.fallback_sheet_key())
    }

    /// The baked record the worn sheet's image publishes, e.g. `boss` for
    /// `boss_spritesheet.png`. `None` when the catalog names no image for it.
    pub fn worn_sheet_record_target<'a>(
        &'a self,
        behavior: &'a BossBehaviorProfile,
    ) -> Option<&'a str> {
        let key = self.worn_sheet_key(behavior)?;
        let filename = self.sprite_filenames.get(key)?;
        ambition_sprite_sheet::boss::boss_ron_target(filename)
    }

    pub fn special_animation_keys(&self, key: &str) -> &[String] {
        self.special_anim_keys
            .get(key)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// The sheet rows the geometry strike `key` claims, in the order they are
    /// tried. Empty for a strike no provider gives rows to: that strike keeps
    /// its static boxes.
    pub fn strike_animation_keys(&self, key: &str) -> &[String] {
        self.strike_anim_keys
            .get(key)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// The generic animation row an attack plays while it telegraphs and
    /// while it is active. A provider authors the row of each geometry
    /// strike, and `None` for a strike that plays none. A move with no
    /// authored row (each content special, and a strike no provider names)
    /// plays the spike-halo row, the closest generic cue: a ring of damage
    /// around the boss.
    pub fn attack_animation(
        &self,
        profile: &ambition_characters::brain::BossAttackProfile,
    ) -> Option<ambition_sprite_sheet::boss::BossAnim> {
        match self.strike_anims.get(profile.move_id().as_str()) {
            Some(authored) => *authored,
            None => Some(ambition_sprite_sheet::boss::BossAnim::SpikeHalo),
        }
    }

    /// The sheet row the damageable box of an attack is sampled from: the
    /// row a provider authors for the move, else the first row the move
    /// claims. A provider authors it when the box must follow the row that
    /// is drawn and the move's first row is a gameplay key of its own.
    pub fn hurtbox_sample_row(
        &self,
        profile: &ambition_characters::brain::BossAttackProfile,
    ) -> Option<String> {
        self.hurtbox_sample_rows
            .get(profile.move_id().as_str())
            .cloned()
            .or_else(|| {
                crate::behavior::boss_animation_keys_for_profile(self, profile)
                    .first()
                    .cloned()
            })
    }
}

/// A provider's boss art keys, authored as one RON file beside its sheets.
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BossArtKeys {
    /// The image file that draws each boss sheet, by sheet key.
    #[serde(default)]
    pub sprite_filenames: BTreeMap<String, String>,
    /// The sheet rows each special move claims, by special id.
    #[serde(default)]
    pub special_animation_rows: BTreeMap<String, Vec<String>>,
    /// The sheet rows each geometry strike claims, by strike key. The first
    /// row is the canonical runtime key; the others are row-name aliases.
    #[serde(default)]
    pub strike_animation_rows: BTreeMap<String, Vec<String>>,
    /// The generic animation row each geometry strike plays, by strike key.
    /// `None` is a strike that plays no row. A strike with no entry plays the
    /// spike-halo row, as each special does.
    #[serde(default)]
    pub strike_animations: BTreeMap<String, Option<ambition_sprite_sheet::boss::BossAnim>>,
    /// The sheet row the damageable box of a move is sampled from, by move
    /// id, when it is not the first row the move claims.
    #[serde(default)]
    pub hurtbox_sample_rows: BTreeMap<String, String>,
}

impl BossArtKeys {
    pub fn from_ron(ron: &str) -> Result<Self, ron::error::SpannedError> {
        ron::from_str(ron)
    }
}

/// One provider's immutable boss definitions.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct BossCatalogFragment {
    provider_id: String,
    fallback_boss_id: Option<String>,
    fallback_sheet_key: Option<String>,
    behaviors: BTreeMap<String, BossBehaviorProfile>,
    encounters: BTreeMap<String, BossEncounterSpec>,
    sheets: BTreeMap<String, BossSheetSpec>,
    sprite_filenames: BTreeMap<String, String>,
    special_anim_keys: BTreeMap<String, Vec<String>>,
    strike_anim_keys: BTreeMap<String, Vec<String>>,
    strike_anims: BTreeMap<String, Option<ambition_sprite_sheet::boss::BossAnim>>,
    hurtbox_sample_rows: BTreeMap<String, String>,
    #[serde(skip)]
    birth_kits: BTreeMap<String, BossBirthKit>,
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
        art: BossArtKeys,
    ) -> Result<Self, BossCatalogAssemblyError> {
        let provider_id = provider_id.into();
        let behaviors =
            ron::from_str::<BTreeMap<String, BossBehaviorProfile>>(behavior_profiles_ron).map_err(
                |error| BossCatalogAssemblyError::MalformedBehaviorProfiles {
                    provider_id: provider_id.clone(),
                    message: error.to_string(),
                },
            )?;
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
        Self::from_prepared(
            provider_id,
            fallback_boss_id,
            fallback_sheet_key,
            behaviors,
            encounters,
            boss_sheets_ron,
            art,
        )
    }

    /// The same assembly, with the roster and the encounters already parsed.
    ///
    /// This lets the content pack be the load path. [`Self::from_ron`] would
    /// re-parse bytes the compiler already read and judged. A provider whose
    /// content comes from a `PreparedContentPack` passes the lowered values
    /// here.
    #[allow(clippy::too_many_arguments)]
    pub fn from_prepared(
        provider_id: impl Into<String>,
        fallback_boss_id: Option<impl Into<String>>,
        fallback_sheet_key: Option<impl Into<String>>,
        behaviors: BTreeMap<String, BossBehaviorProfile>,
        encounters: BTreeMap<String, BossEncounterSpec>,
        boss_sheets_ron: &str,
        art: BossArtKeys,
    ) -> Result<Self, BossCatalogAssemblyError> {
        let provider_id = provider_id.into();
        let sheets =
            ron::from_str::<BTreeMap<String, BossSheetSpec>>(boss_sheets_ron).map_err(|error| {
                BossCatalogAssemblyError::MalformedSheets {
                    provider_id: provider_id.clone(),
                    message: error.to_string(),
                }
            })?;
        let fragment = Self {
            provider_id,
            fallback_boss_id: fallback_boss_id.map(Into::into),
            fallback_sheet_key: fallback_sheet_key.map(Into::into),
            behaviors,
            encounters,
            sheets,
            sprite_filenames: art.sprite_filenames,
            special_anim_keys: art.special_animation_rows,
            strike_anim_keys: art.strike_animation_rows,
            strike_anims: art.strike_animations,
            hurtbox_sample_rows: art.hurtbox_sample_rows,
            birth_kits: BTreeMap::new(),
        };
        fragment.validate()?;
        Ok(fragment)
    }

    /// Give boss `id` a [`BossBirthKit`]. Registration refuses a kit for a
    /// boss this fragment does not define.
    pub fn with_birth_kit(mut self, id: impl Into<String>, kit: BossBirthKit) -> Self {
        self.birth_kits.insert(id.into(), kit);
        self
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
        if let Some(id) = self.birth_kits.keys().find(|id| !self.behaviors.contains_key(*id)) {
            return Err(BossCatalogAssemblyError::BirthKitWithoutBoss {
                provider_id: self.provider_id.clone(),
                boss_id: id.clone(),
            });
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
        for (strike, rows) in &self.strike_anim_keys {
            if strike.trim().is_empty() || rows.iter().any(|row| row.trim().is_empty()) {
                return Err(BossCatalogAssemblyError::InvalidStrikeAnimation {
                    provider_id: self.provider_id.clone(),
                    strike: strike.clone(),
                });
            }
        }
        let unnamed_strike = self.strike_anims.keys().find(|strike| strike.trim().is_empty());
        let unnamed_sample = self
            .hurtbox_sample_rows
            .iter()
            .find(|(move_id, row)| move_id.trim().is_empty() || row.trim().is_empty())
            .map(|(move_id, _)| move_id);
        if let Some(strike) = unnamed_strike.or(unnamed_sample) {
            return Err(BossCatalogAssemblyError::InvalidStrikeAnimation {
                provider_id: self.provider_id.clone(),
                strike: strike.clone(),
            });
        }
        Ok(())
    }
}

/// Provider fragments linked into one App.
#[derive(Resource, Clone, Debug, Default, serde::Serialize)]
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

    /// The registry with `fragment` in place of its provider's current one
    /// (or added), and the catalog it assembles. For a content reload: the
    /// candidate is checked whole before anything is published.
    pub fn with_replaced(
        &self,
        fragment: BossCatalogFragment,
    ) -> Result<(Self, BossCatalog), BossCatalogAssemblyError> {
        fragment.validate()?;
        let mut next = self.clone();
        next.fragments.insert(fragment.provider_id.clone(), fragment);
        let catalog = next.assemble()?;
        Ok((next, catalog))
    }

    pub fn assemble(&self) -> Result<BossCatalog, BossCatalogAssemblyError> {
        let mut behaviors = BTreeMap::new();
        let mut encounters = BTreeMap::new();
        let mut sheets = BTreeMap::new();
        let mut sprite_filenames = BTreeMap::new();
        let mut special_anim_keys = BTreeMap::new();
        let mut strike_anim_keys = BTreeMap::new();
        let mut strike_anims = BTreeMap::new();
        let mut hurtbox_sample_rows = BTreeMap::new();
        // One owner for each move id, across the four art tables a move id
        // keys: its special rows, its strike rows, its animation row and its
        // hurtbox sample row. `attack_animation` and `hurtbox_sample_row`
        // read by move id for a strike and a special alike, so the four
        // tables are one namespace.
        let mut move_art_owners = BTreeMap::<String, String>::new();
        let mut behavior_owners = BTreeMap::<String, String>::new();
        let mut sheet_owners = BTreeMap::<String, String>::new();
        let mut sprite_owners = BTreeMap::<String, String>::new();
        let mut fallback_boss_ids = BTreeMap::new();
        let mut fallback_sheet_keys = BTreeMap::new();
        let mut birth_kits = BTreeMap::new();

        for (provider_id, fragment) in &self.fragments {
            // A kit names one of its own fragment's bosses (`validate`), and
            // two providers cannot define one boss, so kits cannot collide.
            birth_kits.extend(fragment.birth_kits.iter().map(|(id, kit)| (id.clone(), *kit)));
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
            // A provider may author several art keys of its own move; a second
            // provider may author none of them.
            let move_ids = fragment
                .special_anim_keys
                .keys()
                .chain(fragment.strike_anim_keys.keys())
                .chain(fragment.strike_anims.keys())
                .chain(fragment.hurtbox_sample_rows.keys());
            for move_id in move_ids {
                match move_art_owners.get(move_id) {
                    Some(first_provider) if first_provider != provider_id => {
                        return Err(BossCatalogAssemblyError::DuplicateMoveArt {
                            move_id: move_id.clone(),
                            first_provider: first_provider.clone(),
                            second_provider: provider_id.clone(),
                        });
                    }
                    Some(_) => {}
                    None => {
                        move_art_owners.insert(move_id.clone(), provider_id.clone());
                    }
                }
            }
            special_anim_keys.extend(fragment.special_anim_keys.iter().map(|(key, rows)| (key.clone(), rows.clone())));
            strike_anim_keys.extend(fragment.strike_anim_keys.iter().map(|(key, rows)| (key.clone(), rows.clone())));
            strike_anims.extend(fragment.strike_anims.iter().map(|(key, anim)| (key.clone(), *anim)));
            hurtbox_sample_rows.extend(
                fragment
                    .hurtbox_sample_rows
                    .iter()
                    .map(|(key, row)| (key.clone(), row.clone())),
            );
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
            strike_anim_keys,
            strike_anims,
            hurtbox_sample_rows,
            fallback_boss_ids,
            fallback_sheet_keys,
            birth_kits,
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
    BirthKitWithoutBoss {
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
    InvalidStrikeAnimation {
        provider_id: String,
        strike: String,
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
    /// Two providers author art keys of one move id.
    DuplicateMoveArt {
        move_id: String,
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
            Self::BirthKitWithoutBoss { provider_id, boss_id } => write!(
                f,
                "boss catalog fragment '{provider_id}' gives a birth kit to '{boss_id}', which it does not define"
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
            Self::InvalidStrikeAnimation { provider_id, strike } => write!(
                f,
                "boss catalog fragment '{provider_id}' has invalid strike-animation row '{strike}'"
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
            Self::DuplicateMoveArt { move_id, first_provider, second_provider } => write!(
                f,
                "boss move '{move_id}' has art keys from both '{first_provider}' and '{second_provider}'; one provider authors all the art keys of a move"
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

#[cfg(any(test, feature = "test-support"))]
pub fn test_boss_catalog() -> &'static BossCatalog {
    static CATALOG: std::sync::LazyLock<BossCatalog> = std::sync::LazyLock::new(|| {
        let encounters: &[&str] = &[
            include_str!("../../../game/ambition_content/assets/data/boss_encounters/clockwork_warden.ron"),
            include_str!("../../../game/ambition_content/assets/data/boss_encounters/mockingbird.ron"),
            include_str!("../../../game/ambition_content/assets/data/boss_encounters/gnu_ton_rider.ron"),
            include_str!("../../../game/ambition_content/assets/data/boss_encounters/smirking_behemoth_boss.ron"),
            include_str!("../../../game/ambition_content/assets/data/boss_encounters/flying_spaghetti_monster_boss.ron"),
            include_str!("../../../game/ambition_content/assets/data/boss_encounters/trex_boss.ron"),
            include_str!("../../../game/ambition_content/assets/data/boss_encounters/mode_collapse_boss.ron"),
            include_str!("../../../game/ambition_content/assets/data/boss_encounters/exploding_gradient_boss.ron"),
            include_str!("../../../game/ambition_content/assets/data/boss_encounters/overflow_boss.ron"),
        ];
        let fragment = BossCatalogFragment::from_ron(
            "ambition-test",
            Some("clockwork_warden"),
            Some("gradient_sentinel"),
            include_str!("../../../game/ambition_content/assets/data/boss_profiles.ron"),
            encounters,
            include_str!("../../../game/ambition_content/assets/data/boss_sheets.ron"),
            BossArtKeys::from_ron(include_str!(
                "../../../game/ambition_content/assets/data/boss_art_keys.ron"
            ))
            .expect("Ambition's boss art keys parse"),
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
            include_str!("../../../game/ambition_content/assets/data/boss_encounters/clockwork_warden.ron"),
            include_str!("../../../game/ambition_content/assets/data/boss_encounters/mockingbird.ron"),
            include_str!("../../../game/ambition_content/assets/data/boss_encounters/gnu_ton_rider.ron"),
            include_str!("../../../game/ambition_content/assets/data/boss_encounters/smirking_behemoth_boss.ron"),
            include_str!("../../../game/ambition_content/assets/data/boss_encounters/flying_spaghetti_monster_boss.ron"),
            include_str!("../../../game/ambition_content/assets/data/boss_encounters/trex_boss.ron"),
            include_str!("../../../game/ambition_content/assets/data/boss_encounters/mode_collapse_boss.ron"),
            include_str!("../../../game/ambition_content/assets/data/boss_encounters/exploding_gradient_boss.ron"),
            include_str!("../../../game/ambition_content/assets/data/boss_encounters/overflow_boss.ron"),
        ];
        BossCatalogFragment::from_ron(
            provider,
            Some("clockwork_warden"),
            Some("gradient_sentinel"),
            include_str!("../../../game/ambition_content/assets/data/boss_profiles.ron"),
            encounters,
            include_str!("../../../game/ambition_content/assets/data/boss_sheets.ron"),
            BossArtKeys::from_ron(include_str!(
                "../../../game/ambition_content/assets/data/boss_art_keys.ron"
            ))
            .unwrap(),
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
            strike_anim_keys: BTreeMap::new(),
            strike_anims: BTreeMap::new(),
            hurtbox_sample_rows: BTreeMap::new(),
            birth_kits: BTreeMap::new(),
        }
    }

    /// A kit reaches the assembled catalog under its boss, and a kit for a
    /// boss the fragment does not define is refused, not kept unused.
    #[test]
    fn a_birth_kit_is_assembled_under_its_boss_and_refused_for_a_stranger() {
        fn kit(
            _: &mut ambition_platformer2d_shared_tangle::construction::EntityScope,
            _: &ambition_platformer2d_core::BodyKinematics,
        ) {
        }
        let mut registry = BossCatalogRegistry::default();
        registry
            .register(renamed_single_boss_fragment("a", "alpha").with_birth_kit("alpha", kit))
            .expect("a kit for the fragment's own boss registers");
        let catalog = registry.assemble().expect("assembles");
        assert!(catalog.birth_kit("alpha").is_some(), "the kit reached the catalog");
        assert!(catalog.birth_kit("beta").is_none());
        let refused = BossCatalogRegistry::default()
            .register(renamed_single_boss_fragment("b", "beta").with_birth_kit("gamma", kit))
            .expect_err("a kit for an undefined boss would never run");
        assert_eq!(
            refused,
            BossCatalogAssemblyError::BirthKitWithoutBoss {
                provider_id: "b".to_string(),
                boss_id: "gamma".to_string(),
            }
        );
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

    /// A catalog that authors no sheet gives the unauthored layout, and that
    /// layout is not the layout of an authored sheet. The engine held a copy
    /// of the gradient sentinel's layout as its default, so that layout had
    /// two readers: the constant and `boss_sheets.ron`.
    #[test]
    fn a_boss_with_no_authored_sheet_wears_the_unauthored_layout() {
        use ambition_sprite_sheet::boss::BossSheetSpec;
        let empty = BossCatalog::default();
        let worn = empty.sheet_for_key("no_such_sheet");
        let size = worn.render_size(bevy::math::Vec2::new(50.0, 80.0));
        let catalog = test_boss_catalog();
        let copies: Vec<&String> = catalog
            .sheets
            .iter()
            .filter(|(_, sheet)| **sheet == BossSheetSpec::unauthored())
            .map(|(key, _)| key)
            .collect();
        assert_eq!(
            (worn == BossSheetSpec::unauthored(), (size.x, size.y), catalog.sheets.len(), copies),
            (true, (80.0, 80.0), 7, Vec::<&String>::new()),
            "(an empty catalog gives the unauthored layout, its size for a 50 x 80 body, the \
             authored sheets read, the authored sheets equal to the unauthored layout)"
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

    /// Two fragments, `a` and `b`, each with one boss and no art keys.
    fn two_providers() -> (BossCatalogFragment, BossCatalogFragment) {
        (renamed_single_boss_fragment("a", "boss_a"), renamed_single_boss_fragment("b", "boss_b"))
    }

    fn assemble_pair(
        a: BossCatalogFragment,
        b: BossCatalogFragment,
    ) -> Result<BossCatalog, BossCatalogAssemblyError> {
        let mut registry = BossCatalogRegistry::default();
        registry.register(a).expect("fragment a registers");
        registry.register(b).expect("fragment b registers");
        registry.assemble()
    }

    /// The providers that an assembly error names as the two authors of one move.
    fn named_authors(error: &BossCatalogAssemblyError) -> Option<(&str, &str)> {
        match error {
            BossCatalogAssemblyError::DuplicateMoveArt { first_provider, second_provider, .. } => {
                Some((first_provider.as_str(), second_provider.as_str()))
            }
            _ => None,
        }
    }

    /// One move has one author for all of its art keys. Provider `b` cannot
    /// set the animation row of a strike whose sheet rows `a` authors.
    #[test]
    fn a_second_provider_cannot_set_the_animation_of_a_move_it_does_not_own() {
        let (mut a, mut b) = two_providers();
        a.strike_anim_keys.insert("floor_slam".into(), vec!["floor_slam".into()]);
        b.strike_anims.insert("floor_slam".into(), None);
        let error = assemble_pair(a, b).expect_err("b's animation row for a's move is refused");
        assert_eq!(named_authors(&error), Some(("a", "b")), "the error names both authors: {error}");
    }

    /// Provider `b` cannot set the hurtbox sample row of a strike `a` authors.
    #[test]
    fn a_second_provider_cannot_set_the_hurtbox_row_of_a_move_it_does_not_own() {
        let (mut a, mut b) = two_providers();
        a.strike_anim_keys.insert("floor_slam".into(), vec!["floor_slam".into()]);
        b.hurtbox_sample_rows.insert("floor_slam".into(), "floor_slam".into());
        let error = assemble_pair(a, b).expect_err("b's hurtbox row for a's move is refused");
        assert_eq!(named_authors(&error), Some(("a", "b")), "the error names both authors: {error}");
    }

    /// The hurtbox sample row is read by move id, so it reaches a special
    /// too: `b` cannot set it for a special whose rows `a` authors.
    #[test]
    fn a_second_provider_cannot_set_the_hurtbox_row_of_a_special_it_does_not_own() {
        let (mut a, mut b) = two_providers();
        a.special_anim_keys.insert("apple_rain".into(), vec!["apple_rain".into()]);
        b.hurtbox_sample_rows.insert("apple_rain".into(), "head_down".into());
        let error = assemble_pair(a, b).expect_err("b's hurtbox row for a's special is refused");
        assert_eq!(named_authors(&error), Some(("a", "b")), "the error names both authors: {error}");
    }

    /// One provider may author every art key of its own move.
    #[test]
    fn one_provider_authors_all_the_art_keys_of_its_move() {
        let (mut a, b) = two_providers();
        a.strike_anim_keys.insert("floor_slam".into(), vec!["floor_slam".into()]);
        a.strike_anims.insert("floor_slam".into(), None);
        a.hurtbox_sample_rows.insert("floor_slam".into(), "floor_slam".into());
        let catalog = assemble_pair(a, b).expect("one author for every art key of a move assembles");
        let profile = ambition_characters::brain::BossAttackProfile::Strike("floor_slam".into());
        assert_eq!(catalog.strike_animation_keys("floor_slam"), ["floor_slam".to_string()]);
        assert_eq!(catalog.attack_animation(&profile), None);
        assert_eq!(catalog.hurtbox_sample_row(&profile).as_deref(), Some("floor_slam"));
    }
}
