//! Adaptive music cues as authored data: the cue graph (sections, layers,
//! states) and the encounter and room bindings that choose a cue.
//!
//! These types do not need the Kira backend, so a headless content compiler
//! can read and check them. The director (`crate::music`, with the `kira`
//! feature) plays them.

use serde::{Deserialize, Serialize};

/// How many layer channels the director has in each bank. A layer's `slot`
/// must be below it.
pub const MAX_MUSIC_LAYERS: usize = 6;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MusicCueSpec {
    pub id: String,
    pub asset_root: String,
    pub bpm: f32,
    pub beats_per_bar: f32,
    pub relative_volume: f32,
    pub sections: Vec<MusicSectionSpec>,
    pub layers: Vec<MusicLayerSpec>,
    pub states: Vec<MusicStateSpec>,
    #[serde(default)]
    pub outro_state: Option<String>,
    #[serde(default)]
    pub post_clear_bridge_state: Option<String>,
    /// Optional per-state runtime layer-balance table, authored with
    /// the cue (legacy stem-balance data for multi-stem cues; cues
    /// that play one mastered `full` layer per section leave this
    /// empty and let the renderer own loudness).
    #[serde(default)]
    pub runtime_balance_overrides: Vec<MusicStateBalanceOverride>,
}

// The director's lookups; it is the only reader.
#[cfg(feature = "kira")]
impl MusicCueSpec {
    pub(crate) fn section(&self, id: &str) -> Option<&MusicSectionSpec> {
        self.sections.iter().find(|section| section.id == id)
    }

    pub(crate) fn state(&self, id: &str) -> Option<&MusicStateSpec> {
        self.states.iter().find(|state| state.id == id)
    }

    pub(crate) fn layer(&self, id: &str) -> Option<&MusicLayerSpec> {
        self.layers.iter().find(|layer| layer.id == id)
    }

    pub(crate) fn seconds_per_beat(&self) -> f32 {
        60.0 / self.bpm.max(1.0)
    }

    pub(crate) fn seconds_per_bar(&self) -> f32 {
        self.beats_per_bar.max(1.0) * self.seconds_per_beat()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MusicSectionSpec {
    pub id: String,
    pub duration_beats: f32,
    pub looped: bool,
    pub sources: Vec<MusicLayerSourceSpec>,
}

// The director's lookups; it is the only reader.
#[cfg(feature = "kira")]
impl MusicSectionSpec {
    pub(crate) fn duration_seconds(&self, cue: &MusicCueSpec) -> f32 {
        self.duration_beats.max(0.0) * cue.seconds_per_beat()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MusicLayerSpec {
    pub id: String,
    pub slot: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MusicLayerSourceSpec {
    pub layer_id: String,
    pub path: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MusicStateSpec {
    pub id: String,
    pub section_id: String,
    pub gains: Vec<MusicLayerGainSpec>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MusicLayerGainSpec {
    pub layer_id: String,
    pub gain: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EncounterMusicBinding {
    pub encounter_id: String,
    pub cue_id: String,
    pub starting_state: String,
    pub wave_states: Vec<String>,
    #[serde(default)]
    pub wave2_reinforced_state: Option<String>,
    pub cleared_state: String,
}

/// The adaptive music of one room: the cue state the room holds while it is
/// the room the music plays for. It takes the place of the room's own
/// `music_track`.
///
/// Rooms that bind one cue share its playback. A change of room changes the
/// state only: states on one section change layer gains while the section
/// plays on, and a state on another section starts at the next bar.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoomMusicBinding {
    pub room_id: String,
    pub cue_id: String,
    pub state: String,
    /// The state the cue starts in when it is not playing: a section that
    /// does not loop. The director plays that section to its end, then goes
    /// to `state`. `None` starts in `state`.
    #[serde(default)]
    pub entry_state: Option<String>,
    /// The state while a fight is on in the room. `None` gives the room to
    /// the fight's own track for that time.
    #[serde(default)]
    pub fight_state: Option<String>,
}

/// The adaptive music of a boss: the cue state for each phase of its fight,
/// for every placement of the archetype `boss_id` (the story's fight and a
/// practice copy alike). A phase with no state gives the music back to the
/// room. A state on a section that does not loop plays to its end before the
/// next phase's state starts (a wake, a transition).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BossMusicBinding {
    pub boss_id: String,
    pub cue_id: String,
    pub phase_states: Vec<BossPhaseState>,
}

/// One phase of a boss fight and the state of its cue then. `phase` is a key
/// of [`BOSS_PHASE_KEYS`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BossPhaseState {
    pub phase: String,
    pub state: String,
}

impl BossMusicBinding {
    /// The state of the cue in the phase named `phase`, if the binding has one.
    pub fn state_in(&self, phase: &str) -> Option<&str> {
        self.phase_states.iter().find(|row| row.phase == phase).map(|row| row.state.as_str())
    }
}

/// The phases of a boss fight a binding can name: the names of
/// `BossEncounterPhase` (this crate does not depend on the boss domain; the
/// host maps its phase to these keys, and its test holds the lists together).
pub const BOSS_PHASE_KEYS: [&str; 7] = ["Intro", "Phase1", "Transition", "Phase2", "Stagger", "Enrage", "Death"];

/// The track a room plays in one phase of the day, in place of its own
/// `music_track`. A phase is a key of the host's day cycle (`"morning"`,
/// `"day"`, `"evening"`, `"night"` in `ambition_time::DayPhase`). A room with
/// rows for some phases plays its `music_track` in the others.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoomPhaseTrack {
    pub room_id: String,
    pub phase: String,
    pub track: String,
}

/// One state's authored layer-gain overrides (see
/// [`MusicCueSpec::runtime_balance_overrides`]).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MusicStateBalanceOverride {
    pub state_id: String,
    pub layer_gains: Vec<(String, f32)>,
}

/// One authored file of adaptive music: the cues, and the encounters and
/// rooms that bind to them.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoredMusicCues {
    #[serde(default)]
    pub cues: Vec<MusicCueSpec>,
    #[serde(default)]
    pub encounter_bindings: Vec<EncounterMusicBinding>,
    #[serde(default)]
    pub room_bindings: Vec<RoomMusicBinding>,
    #[serde(default)]
    pub boss_bindings: Vec<BossMusicBinding>,
    /// The tracks of rooms by the time of day.
    #[serde(default)]
    pub room_phase_tracks: Vec<RoomPhaseTrack>,
}

/// Every reference in `cues` and `bindings` that names nothing: a state's
/// section, a gain's or a source's layer, a cue's outro and bridge states,
/// and a binding's cue and states. Also an empty source path, and a room
/// binding's entry state on a section that loops (the director would never
/// leave it). In the order given, so one input gives one report.
pub fn cue_reference_errors<'a>(
    cues: impl IntoIterator<Item = &'a MusicCueSpec>,
    bindings: &[EncounterMusicBinding],
    room_bindings: &[RoomMusicBinding],
    boss_bindings: &[BossMusicBinding],
) -> Vec<String> {
    let cues: Vec<&MusicCueSpec> = cues.into_iter().collect();
    let mut errors = Vec::new();
    for cue in &cues {
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
            ("post_clear_bridge_state", cue.post_clear_bridge_state.as_ref()),
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

    for binding in bindings {
        let Some(cue) = cues.iter().find(|cue| cue.id == binding.cue_id) else {
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

    for binding in room_bindings {
        let Some(cue) = cues.iter().find(|cue| cue.id == binding.cue_id) else {
            errors.push(format!(
                "room binding '{}' references unknown cue '{}'",
                binding.room_id, binding.cue_id
            ));
            continue;
        };
        for (field, state_id) in [
            ("state", Some(&binding.state)),
            ("entry_state", binding.entry_state.as_ref()),
            ("fight_state", binding.fight_state.as_ref()),
        ] {
            let Some(state_id) = state_id else { continue };
            let Some(state) = cue.states.iter().find(|state| &state.id == state_id) else {
                errors.push(format!(
                    "room binding '{}' {field} references unknown state '{}' on cue '{}'",
                    binding.room_id, state_id, binding.cue_id
                ));
                continue;
            };
            let loops = cue
                .sections
                .iter()
                .any(|section| section.id == state.section_id && section.looped);
            if field == "entry_state" && loops {
                errors.push(format!(
                    "room binding '{}' entry_state '{}' is on a section of cue '{}' that loops",
                    binding.room_id, state_id, binding.cue_id
                ));
            }
        }
    }

    for binding in boss_bindings {
        let Some(cue) = cues.iter().find(|cue| cue.id == binding.cue_id) else {
            errors.push(format!("boss binding '{}' references unknown cue '{}'", binding.boss_id, binding.cue_id));
            continue;
        };
        for row in &binding.phase_states {
            if !BOSS_PHASE_KEYS.contains(&row.phase.as_str()) {
                errors.push(format!(
                    "boss binding '{}' names unknown phase '{}' (one of {:?})",
                    binding.boss_id, row.phase, BOSS_PHASE_KEYS
                ));
            }
            if !cue.states.iter().any(|state| state.id == row.state) {
                errors.push(format!(
                    "boss binding '{}' phase '{}' references unknown state '{}' on cue '{}'",
                    binding.boss_id, row.phase, row.state, binding.cue_id
                ));
            }
        }
    }
    errors
}
