use super::*;

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
pub struct MusicLayerSpec {
    pub id: String,
    pub slot: usize,
}

#[derive(Clone, Debug)]
pub struct MusicLayerSourceSpec {
    pub layer_id: String,
    pub path: String,
}

#[derive(Clone, Debug)]
pub struct MusicStateSpec {
    pub id: String,
    pub section_id: String,
    pub gains: Vec<MusicLayerGainSpec>,
}

#[derive(Clone, Debug)]
pub struct MusicLayerGainSpec {
    pub layer_id: String,
    pub gain: f32,
}

#[derive(Clone, Debug)]
pub struct EncounterMusicBinding {
    pub encounter_id: String,
    pub cue_id: String,
    pub starting_state: String,
    pub wave_states: Vec<String>,
    pub wave2_reinforced_state: Option<String>,
    pub cleared_state: String,
}

#[derive(Resource, Clone, Debug)]
pub struct MusicCueCatalog {
    pub(super) cues: HashMap<String, MusicCueSpec>,
    pub(super) encounter_bindings: Vec<EncounterMusicBinding>,
}

/// One state's authored layer-gain overrides (see
/// [`MusicCueSpec::runtime_balance_overrides`]).
#[derive(Clone, Debug)]
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
        cue_id: &str,
        section_id: &str,
        layer_id: &str,
    ) -> Option<Handle<KiraAudioSource>> {
        self.sources
            .get(&MusicSourceKey::new(cue_id, section_id, layer_id))
            .cloned()
    }

    /// Lazily request a cue's file-backed sources the first time it is about to
    /// play (load-on-play). Idempotent: already-requested sources are left as-is,
    /// so a cue loads exactly once and steady-state playback does no work.
    ///
    /// This replaces eager "load every catalog cue at startup": authored cues are
    /// only `asset_server.load()`ed when their `Play` directive actually fires.
    pub(super) fn ensure_cue_loaded(&mut self, cue: &MusicCueSpec, asset_server: &AssetServer) {
        for section in &cue.sections {
            for source in &section.sources {
                let key = MusicSourceKey::new(&cue.id, &section.id, &source.layer_id);
                if !self.sources.contains_key(&key) {
                    let rel =
                        format!("{}/{}", cue.asset_root.trim_end_matches('/'), source.path);
                    self.sources.insert(key, asset_server.load(rel));
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct MusicSourceKey {
    cue_id: String,
    section_id: String,
    layer_id: String,
}

impl MusicSourceKey {
    pub(super) fn new(cue_id: &str, section_id: &str, layer_id: &str) -> Self {
        Self {
            cue_id: cue_id.to_string(),
            section_id: section_id.to_string(),
            layer_id: layer_id.to_string(),
        }
    }
}
