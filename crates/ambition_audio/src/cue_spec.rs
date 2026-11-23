//! Adaptive music cues as authored data: the cue graph (sections, layers,
//! states) and the encounter and room bindings that choose a cue.
//!
//! These types do not need the Kira backend, so a headless content compiler
//! can prepare them. The director (`crate::music`, with the `kira`
//! feature) plays the prepared data.

use serde::{Deserialize, Serialize};

/// How many layer slots one playback group can use. A layer's `slot`
/// must be below it.
pub const MAX_MUSIC_LAYERS: usize = 6;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MusicCueSpec {
    pub id: String,
    pub media: MusicCueMedia,
    #[serde(default)]
    pub playback: crate::music_policy::MusicPlaybackSpec,
    pub relative_volume: f32,
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

impl MusicCueSpec {
    pub(crate) fn playback_policy(&self) -> crate::music_policy::MusicPlaybackPolicy {
        let base = if matches!(self.media, MusicCueMedia::Assets { .. }) {
            crate::music_policy::MusicPlaybackPolicy::whole_assets()
        } else {
            Default::default()
        };
        self.playback.resolve(base)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum MusicCueMedia {
    Assets {
        sections: Vec<MusicAssetSectionSpec>,
    },
    Inline {
        asset_root: String,
        bpm: f32,
        beats_per_bar: f32,
        sections: Vec<MusicSectionSpec>,
    },
    Published {
        description: String,
        products: MusicProductSet,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MusicAssetSectionSpec {
    pub id: String,
    pub looped: bool,
    pub sources: Vec<MusicLayerSourceSpec>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MusicProductSet {
    MasteredFull,
    GroupedLayers,
}

impl MusicCueMedia {
    /// Resolve a source relative to the cue root. A qualified source keeps its owner.
    pub(crate) fn source_path(&self, source: &MusicLayerSourceSpec) -> String {
        let Self::Inline { asset_root, .. } = self else {
            unreachable!("inline source path")
        };
        if source.path.contains("://") || asset_root.is_empty() {
            source.path.clone()
        } else {
            format!("{}/{}", asset_root.trim_end_matches('/'), source.path)
        }
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
    /// Start this state when this one-shot ends, even if its request holds.
    #[serde(default)]
    pub next_state: Option<String>,
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
/// room. Each phase controls whether its one-shot section finishes before the
/// next phase's state starts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BossMusicBinding {
    pub boss_id: String,
    pub cue_id: String,
    pub phase_states: Vec<BossPhaseState>,
}

/// Control a one-shot section when its scene phase changes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MusicPhasePolicy {
    /// Let the current section reach its musical end.
    #[default]
    CompleteSection,
    /// Follow the current phase as soon as its target is ready.
    FollowPhase,
}

/// One phase of a boss fight and the state of its cue then. `phase` is a key
/// of [`BOSS_PHASE_KEYS`].
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BossPhaseState {
    pub phase: String,
    pub state: String,
    #[serde(default)]
    pub policy: MusicPhasePolicy,
}

impl BossMusicBinding {
    /// The state of the cue in the phase named `phase`, if the binding has one.
    pub fn state_in(&self, phase: &str) -> Option<&str> {
        self.phase_states
            .iter()
            .find(|row| row.phase == phase)
            .map(|row| row.state.as_str())
    }
}

/// The phases of a boss fight a binding can name: the names of
/// `BossEncounterPhase` (this crate does not depend on the boss domain; the
/// host maps its phase to these keys, and its test holds the lists together).
pub const BOSS_PHASE_KEYS: [&str; 7] = [
    "Intro",
    "Phase1",
    "Transition",
    "Phase2",
    "Stagger",
    "Enrage",
    "Death",
];

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
