use super::*;

#[derive(Clone, Debug, PartialEq)]
pub struct MusicCueSpec {
    pub id: String,
    pub asset_root: String,
    pub bpm: f32,
    pub beats_per_bar: f32,
    pub relative_volume: f32,
    pub sections: Vec<MusicSectionSpec>,
    pub layers: Vec<MusicLayerSpec>,
    pub states: Vec<MusicStateSpec>,
    pub outro_state: Option<String>,
    pub post_clear_bridge_state: Option<String>,
    /// Optional per-state runtime layer-balance table, authored with
    /// the cue (legacy stem-balance data for multi-stem cues; cues
    /// that play one mastered `full` layer per section leave this
    /// empty and let the renderer own loudness).
    pub runtime_balance_overrides: Vec<MusicStateBalanceOverride>,
}

impl MusicCueSpec {
    pub(super) fn section(&self, id: &str) -> Option<&MusicSectionSpec> {
        self.sections.iter().find(|section| section.id == id)
    }

    pub(super) fn state(&self, id: &str) -> Option<&MusicStateSpec> {
        self.states.iter().find(|state| state.id == id)
    }

    pub(super) fn layer(&self, id: &str) -> Option<&MusicLayerSpec> {
        self.layers.iter().find(|layer| layer.id == id)
    }

    pub(super) fn seconds_per_beat(&self) -> f32 {
        60.0 / self.bpm.max(1.0)
    }

    pub(super) fn seconds_per_bar(&self) -> f32 {
        self.beats_per_bar.max(1.0) * self.seconds_per_beat()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MusicSectionSpec {
    pub id: String,
    pub duration_beats: f32,
    pub looped: bool,
    pub sources: Vec<MusicLayerSourceSpec>,
}

impl MusicSectionSpec {
    pub(super) fn duration_seconds(&self, cue: &MusicCueSpec) -> f32 {
        self.duration_beats.max(0.0) * cue.seconds_per_beat()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MusicLayerSpec {
    pub id: String,
    pub slot: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MusicLayerSourceSpec {
    pub layer_id: String,
    pub path: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MusicStateSpec {
    pub id: String,
    pub section_id: String,
    pub gains: Vec<MusicLayerGainSpec>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MusicLayerGainSpec {
    pub layer_id: String,
    pub gain: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EncounterMusicBinding {
    pub encounter_id: String,
    pub cue_id: String,
    pub starting_state: String,
    pub wave_states: Vec<String>,
    pub wave2_reinforced_state: Option<String>,
    pub cleared_state: String,
}

#[derive(Resource, Clone, Debug, PartialEq)]
pub struct MusicCueCatalog {
    pub(super) cues: HashMap<String, MusicCueSpec>,
    pub(super) encounter_bindings: Vec<EncounterMusicBinding>,
}

/// One state's authored layer-gain overrides (see
/// [`MusicCueSpec::runtime_balance_overrides`]).
#[derive(Clone, Debug, PartialEq)]
pub struct MusicStateBalanceOverride {
    pub state_id: String,
    pub layer_gains: Vec<(String, f32)>,
}

impl MusicCueCatalog {
    /// Build a catalog from host-authored parts. The HOST owns which
    /// cues exist and which encounters bind to them; this crate only
    /// plays them.
    pub fn from_parts(
        cues: Vec<MusicCueSpec>,
        encounter_bindings: Vec<EncounterMusicBinding>,
    ) -> Self {
        let cues = cues
            .into_iter()
            .map(|cue| (cue.id.clone(), cue))
            .collect::<HashMap<_, _>>();
        Self {
            cues,
            encounter_bindings,
        }
    }

    pub(super) fn cue(&self, id: &str) -> Option<&MusicCueSpec> {
        self.cues.get(id)
    }

    /// The ids of every adaptive cue this catalog defines. A provider registers
    /// these through [`AdaptiveMusicCatalogRegistry`] so the music authority
    /// can gate adaptive playback to the cues that provider actually authored.
    pub fn cue_ids(&self) -> impl Iterator<Item = &str> {
        self.cues.keys().map(String::as_str)
    }

    /// The host's encounter -> cue bindings (read by the host's
    /// intent-mapping adapter).
    pub fn encounter_bindings(&self) -> &[EncounterMusicBinding] {
        &self.encounter_bindings
    }

    /// Append an encounter -> cue binding (host-side catalog assembly /
    /// test fixtures).
    pub fn add_encounter_binding(&mut self, binding: EncounterMusicBinding) {
        self.encounter_bindings.push(binding);
    }

    /// Validate internal cue/state/layer/binding references.
    ///
    /// This is intentionally independent of the audio backend: it checks the
    /// authored adaptive-music graph before the director tries to resolve a
    /// state or play a layer source at runtime.
    pub fn validate_references(&self) -> Vec<String> {
        let mut errors = Vec::new();
        for cue in self.cues.values() {
            let sections = cue
                .sections
                .iter()
                .map(|section| section.id.as_str())
                .collect::<std::collections::BTreeSet<_>>();
            let layers = cue
                .layers
                .iter()
                .map(|layer| layer.id.as_str())
                .collect::<std::collections::BTreeSet<_>>();
            let states = cue
                .states
                .iter()
                .map(|state| state.id.as_str())
                .collect::<std::collections::BTreeSet<_>>();

            for state in &cue.states {
                if !sections.contains(state.section_id.as_str()) {
                    errors.push(format!(
                        "cue '{}' state '{}' references unknown section '{}'",
                        cue.id, state.id, state.section_id
                    ));
                }
                for gain in &state.gains {
                    if !layers.contains(gain.layer_id.as_str()) {
                        errors.push(format!(
                            "cue '{}' state '{}' references unknown layer '{}'",
                            cue.id, state.id, gain.layer_id
                        ));
                    }
                }
            }

            for section in &cue.sections {
                for source in &section.sources {
                    if !layers.contains(source.layer_id.as_str()) {
                        errors.push(format!(
                            "cue '{}' section '{}' references unknown layer '{}'",
                            cue.id, section.id, source.layer_id
                        ));
                    }
                    if source.path.trim().is_empty() {
                        errors.push(format!(
                            "cue '{}' section '{}' has an empty source path for layer '{}'",
                            cue.id, section.id, source.layer_id
                        ));
                    }
                }
            }

            for (field, value) in [
                ("outro_state", cue.outro_state.as_ref()),
                (
                    "post_clear_bridge_state",
                    cue.post_clear_bridge_state.as_ref(),
                ),
            ] {
                if let Some(state_id) = value {
                    if !states.contains(state_id.as_str()) {
                        errors.push(format!(
                            "cue '{}' {field} references unknown state '{}'",
                            cue.id, state_id
                        ));
                    }
                }
            }
        }

        for binding in &self.encounter_bindings {
            let Some(cue) = self.cues.get(&binding.cue_id) else {
                errors.push(format!(
                    "encounter binding '{}' references unknown cue '{}'",
                    binding.encounter_id, binding.cue_id
                ));
                continue;
            };
            let states = cue
                .states
                .iter()
                .map(|state| state.id.as_str())
                .collect::<std::collections::BTreeSet<_>>();
            for (field, state_id) in [
                ("starting_state", binding.starting_state.as_str()),
                ("cleared_state", binding.cleared_state.as_str()),
            ] {
                if !states.contains(state_id) {
                    errors.push(format!(
                        "encounter binding '{}' {field} references unknown state '{}' on cue '{}'",
                        binding.encounter_id, state_id, binding.cue_id
                    ));
                }
            }
            for state_id in &binding.wave_states {
                if !states.contains(state_id.as_str()) {
                    errors.push(format!(
                        "encounter binding '{}' wave_states references unknown state '{}' on cue '{}'",
                        binding.encounter_id, state_id, binding.cue_id
                    ));
                }
            }
            if let Some(state_id) = &binding.wave2_reinforced_state {
                if !states.contains(state_id.as_str()) {
                    errors.push(format!(
                        "encounter binding '{}' wave2_reinforced_state references unknown state '{}' on cue '{}'",
                        binding.encounter_id, state_id, binding.cue_id
                    ));
                }
            }
        }
        errors
    }

    /// Find the binding that maps an encounter id to its adaptive
    /// cue. Used by tests + tooling that want to inspect which cue
    /// will fire for a given encounter; the live `resolve_adaptive_directive`
    /// iterates `encounter_bindings` directly so future bindings drop
    /// in without touching the resolver.
    #[allow(dead_code)]
    pub(super) fn binding_for_encounter(&self, id: &str) -> Option<&EncounterMusicBinding> {
        self.encounter_bindings
            .iter()
            .find(|binding| binding.encounter_id == id)
    }
}

#[derive(Resource, Clone, Default)]
pub struct LoadedMusicCueAssets {
    pub(super) sources: HashMap<MusicSourceKey, Handle<KiraAudioSource>>,
}

impl LoadedMusicCueAssets {
    pub(super) fn get(
        &self,
        provider_id: &str,
        cue_id: &str,
        section_id: &str,
        layer_id: &str,
    ) -> Option<Handle<KiraAudioSource>> {
        self.sources
            .get(&MusicSourceKey::new(
                provider_id,
                cue_id,
                section_id,
                layer_id,
            ))
            .cloned()
    }

    /// Lazily request a cue's file-backed sources the first time it is about to
    /// play (load-on-play). Idempotent: already-requested sources are left as-is,
    /// so a cue loads exactly once and steady-state playback does no work.
    ///
    /// This replaces eager "load every catalog cue at startup": authored cues are
    /// only `asset_server.load()`ed when their `Play` directive actually fires.
    pub(super) fn ensure_cue_loaded(
        &mut self,
        provider_id: &str,
        cue: &MusicCueSpec,
        asset_server: &AssetServer,
    ) {
        for section in &cue.sections {
            for source in &section.sources {
                let key = MusicSourceKey::new(provider_id, &cue.id, &section.id, &source.layer_id);
                if !self.sources.contains_key(&key) {
                    let rel = format!("{}/{}", cue.asset_root.trim_end_matches('/'), source.path);
                    self.sources.insert(key, asset_server.load(rel));
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct MusicSourceKey {
    provider_id: String,
    cue_id: String,
    section_id: String,
    layer_id: String,
}

impl MusicSourceKey {
    pub(super) fn new(provider_id: &str, cue_id: &str, section_id: &str, layer_id: &str) -> Self {
        Self {
            provider_id: provider_id.to_string(),
            cue_id: cue_id.to_string(),
            section_id: section_id.to_string(),
            layer_id: layer_id.to_string(),
        }
    }
}

/// App-local adaptive music definitions contributed by linked providers.
///
/// Storage and authority remain distinct: this registry may cache definitions
/// for every linked provider, while `ActiveAudioSelection` chooses the one
/// provider whose catalog may drive the director for the current shell context.
#[derive(Resource, Clone, Debug, Default)]
pub struct AdaptiveMusicCatalogRegistry {
    providers: std::collections::BTreeMap<String, MusicCueCatalog>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AdaptiveMusicCatalogError {
    EmptyProviderId,
    InvalidCatalog {
        provider_id: String,
        errors: Vec<String>,
    },
    DuplicateProvider {
        provider_id: String,
    },
}

impl std::fmt::Display for AdaptiveMusicCatalogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyProviderId => write!(f, "adaptive music provider id must not be empty"),
            Self::InvalidCatalog {
                provider_id,
                errors,
            } => write!(
                f,
                "adaptive music catalog '{provider_id}' is invalid: {}",
                errors.join("; ")
            ),
            Self::DuplicateProvider { provider_id } => write!(
                f,
                "adaptive music provider '{provider_id}' registered different definitions twice"
            ),
        }
    }
}

impl std::error::Error for AdaptiveMusicCatalogError {}

impl AdaptiveMusicCatalogRegistry {
    pub fn register(
        &mut self,
        provider_id: impl Into<String>,
        catalog: MusicCueCatalog,
    ) -> Result<(), AdaptiveMusicCatalogError> {
        let provider_id = provider_id.into();
        if provider_id.trim().is_empty() {
            return Err(AdaptiveMusicCatalogError::EmptyProviderId);
        }
        let errors = catalog.validate_references();
        if !errors.is_empty() {
            return Err(AdaptiveMusicCatalogError::InvalidCatalog {
                provider_id,
                errors,
            });
        }
        if let Some(existing) = self.providers.get(&provider_id) {
            if existing == &catalog {
                return Ok(());
            }
            return Err(AdaptiveMusicCatalogError::DuplicateProvider { provider_id });
        }
        // Cue ids are provider-local. Two providers may deliberately use the
        // same neutral cue/state vocabulary while resolving different assets;
        // the active audio context selects one complete catalog.
        self.providers.insert(provider_id, catalog);
        Ok(())
    }

    pub fn catalog_for(&self, provider_id: &str) -> Option<&MusicCueCatalog> {
        self.providers.get(provider_id)
    }

    pub fn cue_ids_for(&self, provider_id: &str) -> impl Iterator<Item = &str> {
        self.catalog_for(provider_id)
            .into_iter()
            .flat_map(MusicCueCatalog::cue_ids)
    }

    pub fn providers(&self) -> impl Iterator<Item = &str> {
        self.providers.keys().map(String::as_str)
    }
}

pub trait AdaptiveMusicCatalogAppExt {
    fn try_register_adaptive_music_catalog(
        &mut self,
        provider_id: impl Into<String>,
        catalog: MusicCueCatalog,
    ) -> Result<&mut Self, AdaptiveMusicCatalogError>;

    fn register_adaptive_music_catalog(
        &mut self,
        provider_id: impl Into<String>,
        catalog: MusicCueCatalog,
    ) -> &mut Self {
        self.try_register_adaptive_music_catalog(provider_id, catalog)
            .unwrap_or_else(|error| panic!("{error}"))
    }
}

impl AdaptiveMusicCatalogAppExt for App {
    fn try_register_adaptive_music_catalog(
        &mut self,
        provider_id: impl Into<String>,
        catalog: MusicCueCatalog,
    ) -> Result<&mut Self, AdaptiveMusicCatalogError> {
        let registry = {
            let mut candidate = self
                .world()
                .get_resource::<AdaptiveMusicCatalogRegistry>()
                .cloned()
                .unwrap_or_default();
            candidate.register(provider_id, catalog)?;
            candidate
        };
        self.insert_resource(registry);
        Ok(self)
    }
}

#[cfg(test)]
mod provider_registry_tests {
    use super::*;
    use bevy::prelude::App;

    fn catalog(cue_id: &str, path: &str) -> MusicCueCatalog {
        MusicCueCatalog::from_parts(
            vec![MusicCueSpec {
                id: cue_id.to_owned(),
                asset_root: "audio/adaptive".to_owned(),
                bpm: 120.0,
                beats_per_bar: 4.0,
                relative_volume: 1.0,
                sections: vec![MusicSectionSpec {
                    id: "loop".to_owned(),
                    duration_beats: 4.0,
                    looped: true,
                    sources: vec![MusicLayerSourceSpec {
                        layer_id: "full".to_owned(),
                        path: path.to_owned(),
                    }],
                }],
                layers: vec![MusicLayerSpec {
                    id: "full".to_owned(),
                    slot: 0,
                }],
                states: vec![MusicStateSpec {
                    id: "main".to_owned(),
                    section_id: "loop".to_owned(),
                    gains: vec![MusicLayerGainSpec {
                        layer_id: "full".to_owned(),
                        gain: 1.0,
                    }],
                }],
                outro_state: None,
                post_clear_bridge_state: None,
                runtime_balance_overrides: Vec::new(),
            }],
            Vec::new(),
        )
    }

    #[test]
    fn two_apps_keep_different_provider_catalogs() {
        let mut a = App::new();
        a.register_adaptive_music_catalog("a", catalog("a_cue", "a.ogg"));
        let mut b = App::new();
        b.register_adaptive_music_catalog("b", catalog("b_cue", "b.ogg"));

        let a_registry = a.world().resource::<AdaptiveMusicCatalogRegistry>();
        assert!(a_registry.catalog_for("a").is_some());
        assert!(a_registry.catalog_for("b").is_none());
        let b_registry = b.world().resource::<AdaptiveMusicCatalogRegistry>();
        assert!(b_registry.catalog_for("b").is_some());
        assert!(b_registry.catalog_for("a").is_none());
    }

    #[test]
    fn the_same_cue_id_is_provider_local() {
        let mut registry = AdaptiveMusicCatalogRegistry::default();
        registry.register("a", catalog("shared", "a.ogg")).unwrap();
        registry.register("b", catalog("shared", "b.ogg")).unwrap();

        let a_path = &registry
            .catalog_for("a")
            .and_then(|catalog| catalog.cue("shared"))
            .expect("provider a owns shared")
            .sections[0]
            .sources[0]
            .path;
        let b_path = &registry
            .catalog_for("b")
            .and_then(|catalog| catalog.cue("shared"))
            .expect("provider b owns shared")
            .sections[0]
            .sources[0]
            .path;
        assert_eq!(a_path, "a.ogg");
        assert_eq!(b_path, "b.ogg");
    }

    #[test]
    fn failed_app_registration_preserves_the_prior_catalog() {
        let mut app = App::new();
        app.register_adaptive_music_catalog("a", catalog("shared", "a.ogg"));
        let error = app
            .try_register_adaptive_music_catalog("a", catalog("shared", "changed.ogg"))
            .unwrap_err();
        assert!(matches!(
            error,
            AdaptiveMusicCatalogError::DuplicateProvider { .. }
        ));
        let registry = app.world().resource::<AdaptiveMusicCatalogRegistry>();
        let path = &registry
            .catalog_for("a")
            .and_then(|catalog| catalog.cue("shared"))
            .expect("prior provider remains")
            .sections[0]
            .sources[0]
            .path;
        assert_eq!(path, "a.ogg");
    }
}
