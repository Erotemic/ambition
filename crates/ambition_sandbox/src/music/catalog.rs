use super::first_goblin::first_goblin_tune_v2_spec;
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

impl MusicCueCatalog {
    pub fn builtin() -> Self {
        let mut cues = HashMap::new();
        let goblin = first_goblin_tune_v2_spec();
        cues.insert(goblin.id.clone(), goblin);
        Self {
            cues,
            encounter_bindings: vec![EncounterMusicBinding {
                encounter_id: MOB_LAB_ENCOUNTER_ID.to_string(),
                cue_id: FIRST_GOBLIN_CUE_ID.to_string(),
                starting_state: "intro".to_string(),
                wave_states: vec![
                    "wave1".to_string(),
                    "wave2".to_string(),
                    "wave3".to_string(),
                ],
                wave2_reinforced_state: Some("wave2_brute".to_string()),
                cleared_state: "outro".to_string(),
            }],
        }
    }

    pub(super) fn cue(&self, id: &str) -> Option<&MusicCueSpec> {
        self.cues.get(id)
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

#[derive(Resource, Clone)]
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
