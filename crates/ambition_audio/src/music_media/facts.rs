use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const PLAYBACK_SCHEMA: &str = "ambition.music_playback.v1";

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MusicPlaybackDescription {
    pub schema: String,
    pub evidence: String,
    pub cue_id: String,
    pub score_one_shot: bool,
    pub compiled_score_fingerprint: String,
    #[serde(default)]
    pub render_fingerprint: Option<String>,
    pub score_clock: serde_json::Value,
    pub product_family: String,
    pub layer_groups: BTreeMap<String, Vec<String>>,
    pub sections: Vec<MusicMediaSection>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MusicMediaSection {
    pub id: String,
    pub form: Vec<String>,
    #[serde(rename = "loop")]
    pub looped: bool,
    pub lead_in: Option<String>,
    pub tail: Option<String>,
    pub lead_in_start_seconds: Option<f64>,
    pub tail_end_seconds: Option<f64>,
    pub tail_audio_end_seconds: Option<f64>,
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub start_beat: f64,
    pub end_beat: f64,
    #[serde(default)]
    pub duration_beats: Option<f64>,
    pub clock: MusicMediaClock,
    pub markers: Vec<MusicMediaMarker>,
    pub musical_end_seconds: f64,
    pub file_duration_seconds: f64,
    pub material: MusicRetainedMaterial,
    pub product_roles: BTreeMap<String, MusicGainBasis>,
    pub frame_plan: Option<MusicFramePlan>,
    #[serde(default)]
    pub products: Vec<MusicEncodedProduct>,
    #[serde(default)]
    pub available: bool,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MusicMediaClock {
    Constant {
        bpm: f64,
        beats_per_bar: f64,
        bar_origin_seconds: f64,
    },
    Boundaries {
        bar_seconds: Vec<f64>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MusicGainBasis {
    MasteredFull,
    GroupedPreMaster,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MusicMarkerRole {
    Entry,
    Exit,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MusicMediaMarker {
    pub id: String,
    pub role: MusicMarkerRole,
    pub beat: f64,
    pub seconds: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MusicTailConstruction {
    None,
    Folded,
    Appended,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MusicLeadConstruction {
    None,
    Folded,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MusicRetainedMaterial {
    pub tail: MusicTailConstruction,
    pub lead_in: MusicLeadConstruction,
    pub independent_tail: bool,
    pub scheduled_pickup: bool,
    pub tail_excludes_pickup: bool,
    pub unretained_pickup_attacks: usize,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MusicFramePlan {
    pub sample_rate: u32,
    pub score_origin_frame: u64,
    pub musical_end_frame: u64,
    pub file_frames: u64,
    pub loop_frames: Option<u64>,
    pub tail_source_frames: u64,
    pub tail_source_audio_frames: u64,
    pub lead_in_source_frames: u64,
    pub bar_frames: Vec<u64>,
    pub markers: Vec<MusicFrameMarker>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MusicFrameMarker {
    pub id: String,
    pub role: MusicMarkerRole,
    pub frame: u64,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MusicEncodedProduct {
    pub layer: String,
    pub path: String,
    pub sha256: String,
    pub frames: u64,
    pub sample_rate: u32,
    pub channels: u8,
    pub format: String,
    pub subtype: String,
    pub frame_observer: String,
    pub gain_basis: MusicGainBasis,
    pub runtime_stem_gain_mode: Option<String>,
    pub applied_runtime_gain_db: f64,
    pub mastering: Option<String>,
}
