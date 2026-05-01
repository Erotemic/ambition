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

#[derive(Clone, Debug, Deserialize)]
pub struct RoomManifestSpec {
    pub start_room: String,
    pub rooms: Vec<RoomSpecData>,
    pub links: Vec<RoomLinkSpec>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RoomSpecData {
    pub id: String,
    pub name: String,
    pub size: [f32; 2],
    pub spawn: [f32; 2],
    pub shell: ShellSpec,
    pub blocks: Vec<BlockSpec>,
    pub zones: Vec<LoadingZoneSpec>,
    #[serde(default)]
    pub objects: Vec<RoomObjectSpec>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ShellSpec {
    pub enabled: bool,
    pub openings: Vec<WallOpeningSpec>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub enum WallSideSpec {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct WallOpeningSpec {
    pub side: WallSideSpec,
    pub y: f32,
    pub height: f32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct LoadingZoneSpec {
    pub id: String,
    pub name: String,
    pub activation: LoadingZoneActivationSpec,
    pub min: [f32; 2],
    pub size: [f32; 2],
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub enum LoadingZoneActivationSpec {
    EdgeExit,
    Door,
}

#[derive(Clone, Debug, Deserialize)]
pub struct RoomLinkSpec {
    pub from_room: String,
    pub from_zone: String,
    pub to_room: String,
    pub to_zone: String,
    pub bidirectional: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub enum BlockSpec {
    Solid { name: String, min: [f32; 2], size: [f32; 2] },
    BlinkWall { name: String, min: [f32; 2], size: [f32; 2], tier: BlinkWallTierSpec },
    OneWay { name: String, min: [f32; 2], size: [f32; 2] },
    Hazard { name: String, min: [f32; 2], size: [f32; 2] },
    PogoOrb { name: String, center: [f32; 2], radius: f32 },
    Rebound { name: String, min: [f32; 2], size: [f32; 2], impulse: [f32; 2] },
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub enum BlinkWallTierSpec {
    Soft,
    Hard,
}

#[derive(Clone, Debug, Deserialize)]
pub enum RoomObjectSpec {
    DamageVolume {
        id: String,
        name: String,
        min: [f32; 2],
        size: [f32; 2],
        damage: i32,
        #[serde(default)]
        path: Option<KinematicPathSpec>,
    },
    Interactable {
        id: String,
        name: String,
        prompt: String,
        min: [f32; 2],
        size: [f32; 2],
        kind: InteractionKindSpec,
    },
    Pickup {
        id: String,
        name: String,
        min: [f32; 2],
        size: [f32; 2],
        kind: PickupKindSpec,
    },
    Chest {
        id: String,
        name: String,
        min: [f32; 2],
        size: [f32; 2],
        reward: Option<PickupKindSpec>,
    },
    Breakable {
        id: String,
        name: String,
        min: [f32; 2],
        size: [f32; 2],
        max_hp: i32,
        respawn: Option<RespawnPolicySpec>,
        #[serde(default)]
        solid: bool,
    },
    EnemySpawn {
        id: String,
        name: String,
        min: [f32; 2],
        size: [f32; 2],
        brain: EnemyBrainSpec,
    },
    BossSpawn {
        id: String,
        name: String,
        min: [f32; 2],
        size: [f32; 2],
        brain: BossBrainSpec,
    },
    KinematicPath {
        id: String,
        name: String,
        min: [f32; 2],
        size: [f32; 2],
        points: Vec<[f32; 2]>,
        speed: f32,
        mode: KinematicPathModeSpec,
    },
    DebugLabel {
        id: String,
        name: String,
        position: [f32; 2],
        text: String,
        category: DebugLabelKindSpec,
    },
}

#[derive(Clone, Debug, Deserialize)]
pub enum InteractionKindSpec {
    Door { target: Option<String> },
    Npc { dialogue_id: Option<String> },
    Chest,
    Pickup,
    Breakable,
    Custom(String),
}
#[derive(Clone, Debug, Deserialize)]
pub struct KinematicPathSpec {
    pub points: Vec<[f32; 2]>,
    pub speed: f32,
    pub mode: KinematicPathModeSpec,
}


#[derive(Clone, Debug, Deserialize)]
pub enum PickupKindSpec {
    Health { amount: i32 },
    Currency { amount: i32 },
    Ability { ability_id: String },
    StoryFlag { flag: String },
    Custom(String),
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub enum RespawnPolicySpec {
    Never,
    AfterSeconds(f32),
    OnRoomReload,
    Persistent,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub enum KinematicPathModeSpec {
    Once,
    Loop,
    PingPong,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub enum DebugLabelKindSpec {
    Room,
    LoadingZone,
    Hazard,
    Enemy,
    Boss,
    Interactable,
    Pickup,
    Custom,
}

#[derive(Clone, Debug, Deserialize)]
pub enum EnemyBrainSpec {
    Passive,
    Patrol { path_id: Option<String> },
    Guard { leash_radius: f32 },
    Custom(String),
}

#[derive(Clone, Debug, Deserialize)]
pub enum BossBrainSpec {
    Dormant,
    PhaseScript { script_id: String },
    Custom(String),
}

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

pub fn vec2(value: [f32; 2]) -> ae::Vec2 {
    ae::Vec2::new(value[0], value[1])
}
