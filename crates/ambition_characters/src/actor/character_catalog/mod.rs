//! Character catalog schema, validation, resolution, and App-local assembly.
//!
//! Experience providers own immutable RON fragments. A host composes those
//! fragments through [`CharacterCatalogAppExt`], which transactionally rebuilds
//! one deterministic [`CharacterCatalog`] resource for that Bevy `App`. Runtime
//! systems consume the resource or an explicit shared reference; provider-local
//! preset names are namespaced during assembly, while character IDs remain the
//! cross-provider identity.
//!
//! Architectural posture: **Rust = behavior, RON = content, LDtk = space.**
//! Brain and action variants remain typed in Rust for exhaustive behavior, while
//! provider-authored configurations and character definitions live in RON.

use bevy::prelude::*;

pub mod binding;
pub mod entry;
pub mod loader;
pub mod registry;
pub mod resolver;
pub mod validator;

pub use binding::{
    qualify_preset_like, resolve_initial_brain, BrainBinding, BrainBuildContext, BrainBuildError,
    BrainPresetId, BrainSelection, InitialBrainSelection, PresetSource,
};
#[allow(
    unused_imports,
    reason = "public surface; downstream phases consume these as they land"
)]
pub use entry::{
    ActionSetPreset, AxisTuningSpec, BarkSituation, BrainPreset, CharacterBarks, CharacterBodyKind,
    CharacterCatalogData, CharacterCatalogEntry, CharacterTier, CompositionLayer, MeleePreset,
    MomentumParamsSpec, MoveStylePreset, PlayableKitSource, RangedPreset, SpecialPreset,
    SpriteTuningSpec,
};
#[allow(
    unused_imports,
    reason = "CHARACTER_CATALOG_ASSET used by tooling that loads off disk"
)]
pub use loader::{parse_catalog, try_parse_catalog};
pub use registry::{
    AssembledCharacterCatalog, CharacterCatalogAppExt, CharacterCatalogAssemblyError,
    CharacterCatalogDefaults, CharacterCatalogFragment, CharacterCatalogOwners,
    CharacterCatalogRegistry,
};
pub use resolver::{action_set_from_preset, brain_from_preset, brain_from_preset_with_context};

/// Bevy resource holding the parsed catalog. Inserted at Startup by
/// [`CharacterCatalogPlugin`].
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct CharacterCatalog(CharacterCatalogData);

#[allow(
    dead_code,
    reason = "Public catalog API for future spawn-site consumers (EnemySpawn / BossSpawn migrations, custom spawn paths). Tested but not yet wired into a runtime call site."
)]
impl CharacterCatalog {
    /// Construct an assembled or fixture catalog from validated data. Provider
    /// registration should normally use [`CharacterCatalogFragment`]; this
    /// constructor exists for focused tests and tools that already own the data.
    pub fn from_data(data: CharacterCatalogData) -> Self {
        Self(data)
    }

    /// Read-only access to the complete catalog data. The field is private so
    /// callers cannot invalidate an assembled App resource after registration.
    pub fn data(&self) -> &CharacterCatalogData {
        &self.0
    }

    /// An empty catalog (no characters/presets) for explicitly content-free
    /// fixtures. Production systems that consume authored characters should
    /// require the App-local resource instead of silently substituting this.
    /// Every lookup returns `None`.
    pub fn empty() -> Self {
        Self(CharacterCatalogData {
            brain_presets: Default::default(),
            action_set_presets: Default::default(),
            characters: Default::default(),
        })
    }

    /// Look up a character by id. Returns `None` if the id is not
    /// in the catalog — callers fall back to a placeholder spawn.
    pub fn get(&self, id: &str) -> Option<&CharacterCatalogEntry> {
        self.0.characters.get(id)
    }

    /// Iterate every (id, entry) pair. Stable order — `BTreeMap`.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &CharacterCatalogEntry)> {
        self.0.characters.iter()
    }

    /// Count of registered characters.
    pub fn len(&self) -> usize {
        self.0.characters.len()
    }

    /// True iff the catalog has no characters. Pretty much never
    /// the case in a real build; included so `clippy::len_without_is_empty`
    /// stays quiet without `allow(missing_is_empty)`.
    pub fn is_empty(&self) -> bool {
        self.0.characters.is_empty()
    }

    /// Resolve a character_id into a runtime [`crate::brain::Brain`]
    /// using its catalog default. `spawn_world_x` becomes the patrol
    /// center for `Patrol`-brain characters.
    pub fn build_default_brain(
        &self,
        character_id: &str,
        spawn_world_x: f32,
    ) -> Option<crate::brain::Brain> {
        let entry = self.get(character_id)?;
        let preset = self.0.brain_presets.get(&entry.default_brain)?;
        Some(brain_from_preset(preset, spawn_world_x))
    }

    /// Build a runtime [`crate::brain::Brain`] from a NAMED brain preset,
    /// threading the per-spawn [`BrainBuildContext`] (patrol lane center / radius
    /// / path). `None` if the preset name is not in `brain_presets`.
    ///
    /// This is the single seam `preset name → Brain`. Both
    /// [`resolve_initial_brain`] (spawn) and the runtime `BrainCommand` reducer
    /// call it, so a preset resolves identically at spawn and at a later runtime
    /// switch. It never inspects the built brain.
    pub fn build_brain_from_preset(
        &self,
        preset_name: &str,
        ctx: &BrainBuildContext,
    ) -> Option<crate::brain::Brain> {
        let preset = self.0.brain_presets.get(preset_name)?;
        Some(brain_from_preset_with_context(preset, ctx))
    }

    /// Whether a named brain preset exists in this catalog.
    pub fn has_brain_preset(&self, preset_name: &str) -> bool {
        self.0.brain_presets.contains_key(preset_name)
    }

    /// Whether a named brain preset is a `Patrol` preset — the presets that
    /// consume placement `patrol_radius` / `patrol_path_id` parameters. Used by
    /// content validation to flag patrol parameters attached to an incompatible
    /// (non-patrol) preset.
    pub fn brain_preset_is_patrol(&self, preset_name: &str) -> bool {
        matches!(
            self.0.brain_presets.get(preset_name),
            Some(entry::BrainPreset::Patrol { .. })
        )
    }

    /// Resolve a character_id into a runtime [`crate::brain::action_set::ActionSet`]
    /// using its catalog default.
    pub fn build_default_action_set(
        &self,
        character_id: &str,
    ) -> Option<crate::brain::action_set::ActionSet> {
        let entry = self.get(character_id)?;
        let preset = self.0.action_set_presets.get(&entry.default_action_set)?;
        Some(action_set_from_preset(preset))
    }

    pub fn display_name(&self, character_id: &str) -> Option<&str> {
        self.get(character_id)
            .map(|entry| entry.display_name.as_str())
    }

    /// Resolve an authored display name to its stable character id.
    ///
    /// Catalog validation rejects duplicate display names across the assembled
    /// App, so this authoring-boundary conversion is deterministic. Runtime
    /// components carry the returned id; presentation never remains keyed by the
    /// display label.
    pub fn id_for_display_name(&self, display_name: &str) -> Option<&str> {
        self.iter()
            .find(|(_, entry)| entry.display_name == display_name)
            .map(|(id, _)| id.as_str())
    }

    pub fn bark_line(
        &self,
        character_id: &str,
        situation: BarkSituation,
        rotation: u32,
    ) -> Option<&str> {
        self.get(character_id)?.barks.pick(situation, rotation)
    }

    pub fn playable_kit_source(&self, character_id: &str) -> Option<PlayableKitSource> {
        self.get(character_id).map(|entry| entry.playable_kit)
    }

    pub fn momentum_params(
        &self,
        character_id: &str,
    ) -> Option<ambition_engine_core::MomentumParams> {
        self.get(character_id)?
            .momentum
            .as_ref()
            .map(|spec| spec.to_kernel())
    }

    /// The authored axis-swept movement tuning for `character_id`'s playable
    /// body. `Some` means the row authors its own feel (its live axis parameters
    /// come from here, not the global F3 dev tuning); `None` keeps the body on
    /// the shared editable tuning. The axis analogue of [`momentum_params`].
    ///
    /// [`momentum_params`]: Self::momentum_params
    pub fn axis_tuning(&self, character_id: &str) -> Option<ambition_engine_core::MovementTuning> {
        self.get(character_id)?
            .axis_tuning
            .as_ref()
            .map(|spec| spec.to_kernel())
    }

    /// The authored capability set for `character_id`'s playable body: the
    /// `union` of the grants the row lists. `None` means the row declared no
    /// grants, so "use the session's shared `EditableAbilitySet`".
    pub fn ability_set(&self, character_id: &str) -> Option<ambition_engine_core::AbilitySet> {
        let grants = self.get(character_id)?.abilities.as_deref()?;
        Some(ambition_engine_core::AbilitySet::compose(grants))
    }

    pub fn body_kind(&self, character_id: &str) -> Option<CharacterBodyKind> {
        self.get(character_id).map(|entry| entry.body_kind)
    }

    pub fn hall_dialogue_id(&self, character_id: &str) -> Option<&str> {
        self.get(character_id)?.hall_dialogue_id.as_deref()
    }
}

/// Plugin: load the catalog at app build, install it as a resource,
/// and run the validator on Startup. Pre-release stance is fail-loud:
/// the validator panics on internal inconsistency rather than
/// degrading silently.
pub struct CharacterCatalogPlugin {
    /// The catalog RON text (the game embeds its roster and passes it
    /// in — this crate owns schema + parsing, never the data).
    pub catalog_ron: &'static str,
}

impl Plugin for CharacterCatalogPlugin {
    fn build(&self, app: &mut App) {
        app.register_character_catalog_fragment(
            CharacterCatalogFragment::from_ron("default", None::<String>, self.catalog_ron)
                .expect("single character catalog should be valid"),
        );
        app.add_systems(Startup, validate_catalog_on_startup);
    }
}

/// Startup system: validate the catalog and panic with the joined
/// errors if any. Runs once.
pub fn validate_catalog_on_startup(catalog: Res<CharacterCatalog>) {
    let errors = validator::validate(catalog.data());
    if !errors.is_empty() {
        panic!(
            "character_catalog.ron has {} reference error(s):\n  - {}",
            errors.len(),
            errors.join("\n  - "),
        );
    }
}

// The roster-data tests (catalog parses, validator passes over the real
// roster, every entry's presets resolve, renderer coverage, display
// names, plugin install) live in `ambition_actors::character_roster` —
// they pin the GAME's data, which lives there. This crate keeps only
// the synthetic resolver-math tests below.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::brain::{Brain, StateMachineCfg};

    #[test]
    fn character_barks_pick_rotates_and_empty_is_none() {
        let barks = CharacterBarks {
            on_hit: vec!["a".into(), "b".into()],
            ..Default::default()
        };
        // Rotation cycles the pool.
        assert_eq!(barks.pick(BarkSituation::OnHit, 0), Some("a"));
        assert_eq!(barks.pick(BarkSituation::OnHit, 1), Some("b"));
        assert_eq!(barks.pick(BarkSituation::OnHit, 2), Some("a"));
        // An empty pool yields no line (caller falls back).
        assert_eq!(barks.pick(BarkSituation::Hall, 0), None);
        assert_eq!(barks.pick(BarkSituation::Idle, 7), None);
    }

    #[test]
    fn brain_preset_patrol_offsets_spawn_world_x() {
        // The patrol lane center is `spawn_world_x + spawn_local_x`.
        // Pin the offset arithmetic so a refactor that drops the add
        // breaks here rather than at first-spawn.
        let preset = BrainPreset::Patrol {
            spawn_local_x: 5.0,
            radius: 64.0,
            speed: 32.0,
            aggressiveness: 0.0,
            aggro_radius: 80.0,
            attack_range: 0.0,
        };
        let brain = brain_from_preset(&preset, 100.0);
        match brain {
            Brain::StateMachine(StateMachineCfg::Patrol { cfg, .. }) => {
                assert_eq!(cfg.lane.center_x, 105.0);
                assert_eq!(cfg.lane.radius_px, 64.0);
            }
            other => panic!("expected Patrol, got {other:?}"),
        }
    }

    #[test]
    fn brain_preset_smash_threads_cfg_and_difficulty() {
        // The data-exposed Smash fighter preset must resolve to a
        // StateMachineCfg::Smash with its tuning + difficulty floats
        // threaded through — this is the seam the PCA encounter swaps in.
        let preset = BrainPreset::Smash {
            aggro_radius: 460.0,
            engage_distance: 76.0,
            attack_range: 52.0,
            too_close_distance: 34.0,
            chase_speed: 150.0,
            retreat_speed: 120.0,
            crowding_threshold: 0.65,
            dash_to_close: true,
            reaction_delay_s: 0.12,
            commit_probability: 0.85,
            accuracy: 0.9,
            mash_speed_hz: 6.0,
        };
        let brain = brain_from_preset(&preset, 0.0);
        match brain {
            Brain::StateMachine(StateMachineCfg::Smash { cfg, .. }) => {
                assert_eq!(cfg.attack_range, 52.0);
                assert!(cfg.dash_to_close);
                assert_eq!(cfg.difficulty.reaction_delay_s, 0.12);
                assert_eq!(cfg.difficulty.commit_probability, 0.85);
                assert_eq!(cfg.difficulty.accuracy, 0.9);
            }
            other => panic!("expected Smash, got {other:?}"),
        }
        // Smash brains are hostile by construction (the encounter only
        // arms them on the explicit challenge choice).
        assert!(brain_from_preset(&preset, 0.0).is_hostile());
    }
}
