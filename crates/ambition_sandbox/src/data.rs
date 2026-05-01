//! Data manifests for the sandbox.
//!
//! The goal of this module is to keep tuning/audio iteration data in RON while
//! still letting the current code synthesize assets at startup. `bevy_common_assets` registers
//! `SandboxDataSpec` as a real Bevy asset type; `load_embedded` gives us a
//! synchronous bootstrap path until the sandbox grows a loading state.
//!
//! Bevy resolves `ambition/sandbox.ron` relative to the sandbox crate asset
//! root (`crates/ambition_sandbox/assets`) when this package is run through
//! Cargo, so the embedded copy intentionally lives there too. World/room
//! authoring has moved to LDtk; this RON asset intentionally owns only
//! non-spatial sandbox tuning and generated-audio configuration.

use ambition_engine as ae;
use bevy::asset::{Asset, AssetServer};
use bevy::prelude::{Commands, Handle, Res, Resource};
use bevy::reflect::TypePath;
use serde::{Deserialize, Serialize};

pub const SANDBOX_DATA_ASSET: &str = "ambition/sandbox.ron";

#[derive(Clone, Debug, Deserialize, Asset, TypePath, Resource)]
pub struct SandboxDataSpec {
    pub abilities: ae::AbilitySet,
    pub tuning: ae::MovementTuning,
    pub audio: AudioSpec,
}

impl SandboxDataSpec {
    pub fn load_embedded() -> Self {
        ron::from_str(include_str!("../assets/ambition/sandbox.ron"))
            .expect("embedded assets/ambition/sandbox.ron should parse")
    }
}

#[derive(Resource, Clone, Debug)]
pub struct SandboxDataAsset(pub Handle<SandboxDataSpec>);

pub fn load_data_asset_handle(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(SandboxDataAsset(asset_server.load(SANDBOX_DATA_ASSET)));
}

// Spatial/world authoring moved to LDtk. This module intentionally contains
// only non-spatial sandbox tuning and generated-audio data.

#[derive(Clone, Debug, Deserialize)]
pub struct AudioSpec {
    pub sample_rate: u32,
    pub sfx: Vec<SfxSpec>,
    pub music: MusicSpec,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Hash)]
pub enum SoundCueKey {
    Jump,
    DoubleJump,
    Dash,
    Blink,
    PrecisionBlink,
    Slash,
    Hit,
    Pogo,
    Reset,
    Death,
    Respawn,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum WaveformSpec {
    Sine,
    Square,
    Triangle,
    Saw,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct SfxSpec {
    pub cue: SoundCueKey,
    pub waveform: WaveformSpec,
    pub frequency: f32,
    pub frequency_end: f32,
    pub duration: f32,
    pub volume: f32,
    pub attack: f32,
    pub release: f32,
    pub noise: f32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MusicSpec {
    pub bpm: f32,
    pub total_beats: f32,
    pub root_hz: f32,
    pub bass_root_hz: f32,
    pub key_root_hz: f32,
    pub master_gain: f32,
    pub lowpass_alpha: f32,
    pub tape_hiss: f32,
    pub lead: Vec<NoteSpec>,
    pub chords: Vec<[i32; 4]>,
    pub bass_roots: Vec<i32>,
    pub gains: MusicGainsSpec,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct NoteSpec {
    pub start: f32,
    pub duration: f32,
    pub semitone: i32,
    pub volume: f32,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct MusicGainsSpec {
    pub chord_pad: f32,
    pub lead: f32,
    pub soft_keys: f32,
    pub bass: f32,
    pub drums: f32,
}

