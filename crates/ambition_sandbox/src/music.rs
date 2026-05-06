//! Unified music director for room, encounter, and adaptive cue playback.
//!
//! This is the first pass at replacing the special-case generated goblin
//! music path with a generic cue model. Gameplay code should request music by
//! intent: room default, encounter override, and cue state. The director then
//! resolves that into either a simple `AudioLibrary` track or an adaptive cue
//! made of file-backed sections/layers.
//!
//! A simple room track is conceptually a one-section/one-layer cue. In this
//! overlay the legacy procedural room tracks still live in `AudioLibrary`, but
//! selection and priority are owned here rather than split between
//! `audio::apply_encounter_music` and `generated_music.rs`. The adaptive
//! goblin cue is now just data in `MusicCueCatalog`.

#![cfg(feature = "audio")]

use std::collections::HashMap;
use std::time::Duration;

use bevy::ecs::system::SystemParam;
use bevy::log::{debug, info, warn};
use bevy::prelude::*;
use bevy_kira_audio::prelude::{
    AudioChannel, AudioControl, AudioEasing, AudioSource as KiraAudioSource, AudioTween,
};

use crate::audio::{
    amplitude_to_decibels, switch_to_music_track, AudioLibrary, MusicChannel, MusicPlaybackState,
};
use crate::data::SandboxDataSpec;
use crate::encounter::{EncounterMusicRequest, EncounterPhase, EncounterRegistry};
use crate::rooms::RoomMusicRequest;
use crate::settings::UserSettings;

pub const MUSIC_LOG_TARGET: &str = "ambition_music";
const MAX_LAYERS: usize = 6;
const MOB_LAB_ENCOUNTER_ID: &str = "mob_lab";
const FIRST_GOBLIN_CUE_ID: &str = "first_goblin_tune_v2";
const LARGE_BRUTE_DELAY_SECONDS: f32 = 3.5;

/// Relative volume for adaptive cues after user music volume.
///
/// Stacked layers sum hotter than the legacy single-channel procedural room
/// tracks, so keep the per-cue default conservative and let cue states shape
/// individual layer gains.
const ADAPTIVE_MUSIC_RELATIVE_VOLUME: f32 = 0.85;
const STEM_GAIN_BLEND_SECONDS: f32 = 1.05;
const LOOP_SECTION_CROSSFADE_SECONDS: f32 = 1.70;
const INTRO_TO_LOOP_CROSSFADE_SECONDS: f32 = 1.25;
const OUTRO_CROSSFADE_SECONDS: f32 = 1.65;
const DEFAULT_RETURN_OVERLAP_SECONDS: f32 = 1.35;
const MIN_TRANSITION_DELAY_SECONDS: f32 = 0.08;
const LAYER_START_FADE_MS: u64 = 0;
const DEBUG_LOG_PERIOD_SECONDS: f32 = 1.0;

// Two banks of six layer channels. This keeps the current Kira backend simple
// while letting the director crossfade a new section over an old section.
#[derive(Resource)]
pub struct MusicLayer0AChannel;
#[derive(Resource)]
pub struct MusicLayer1AChannel;
#[derive(Resource)]
pub struct MusicLayer2AChannel;
#[derive(Resource)]
pub struct MusicLayer3AChannel;
#[derive(Resource)]
pub struct MusicLayer4AChannel;
#[derive(Resource)]
pub struct MusicLayer5AChannel;

#[derive(Resource)]
pub struct MusicLayer0BChannel;
#[derive(Resource)]
pub struct MusicLayer1BChannel;
#[derive(Resource)]
pub struct MusicLayer2BChannel;
#[derive(Resource)]
pub struct MusicLayer3BChannel;
#[derive(Resource)]
pub struct MusicLayer4BChannel;
#[derive(Resource)]
pub struct MusicLayer5BChannel;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MusicBank {
    A,
    B,
}

impl MusicBank {
    fn other(self) -> Self {
        match self {
            Self::A => Self::B,
            Self::B => Self::A,
        }
    }

    fn index(self) -> usize {
        match self {
            Self::A => 0,
            Self::B => 1,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
        }
    }
}

type LayerGains = [f32; MAX_LAYERS];

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
    fn section(&self, id: &str) -> Option<&MusicSectionSpec> {
        self.sections.iter().find(|section| section.id == id)
    }

    fn state(&self, id: &str) -> Option<&MusicStateSpec> {
        self.states.iter().find(|state| state.id == id)
    }

    fn layer(&self, id: &str) -> Option<&MusicLayerSpec> {
        self.layers.iter().find(|layer| layer.id == id)
    }

    fn seconds_per_beat(&self) -> f32 {
        60.0 / self.bpm.max(1.0)
    }

    fn seconds_per_bar(&self) -> f32 {
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
    fn duration_seconds(&self, cue: &MusicCueSpec) -> f32 {
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
    cues: HashMap<String, MusicCueSpec>,
    encounter_bindings: Vec<EncounterMusicBinding>,
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
                wave_states: vec!["wave1".to_string(), "wave2".to_string(), "wave3".to_string()],
                wave2_reinforced_state: Some("wave2_brute".to_string()),
                cleared_state: "outro".to_string(),
            }],
        }
    }

    fn cue(&self, id: &str) -> Option<&MusicCueSpec> {
        self.cues.get(id)
    }

    fn binding_for_encounter(&self, id: &str) -> Option<&EncounterMusicBinding> {
        self.encounter_bindings
            .iter()
            .find(|binding| binding.encounter_id == id)
    }
}

#[derive(Resource, Clone)]
pub struct LoadedMusicCueAssets {
    sources: HashMap<MusicSourceKey, Handle<KiraAudioSource>>,
}

impl LoadedMusicCueAssets {
    fn get(&self, cue_id: &str, section_id: &str, layer_id: &str) -> Option<Handle<KiraAudioSource>> {
        self.sources
            .get(&MusicSourceKey::new(cue_id, section_id, layer_id))
            .cloned()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct MusicSourceKey {
    cue_id: String,
    section_id: String,
    layer_id: String,
}

impl MusicSourceKey {
    fn new(cue_id: &str, section_id: &str, layer_id: &str) -> Self {
        Self {
            cue_id: cue_id.to_string(),
            section_id: section_id.to_string(),
            layer_id: layer_id.to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MusicDirectorMode {
    Idle,
    SimpleTrack,
    AdaptiveIntro,
    AdaptiveLoop,
    AdaptiveOutro,
    AdaptiveFinished,
}

#[derive(Resource, Debug, Clone)]
pub struct MusicDirectorState {
    pub mode: MusicDirectorMode,
    pub active_cue_id: Option<String>,
    pub current_state_id: Option<String>,
    pub current_section_id: Option<String>,
    pub seconds_in_mode: f32,
    pub seconds_in_loop: f32,
    active_bank: MusicBank,
    fading_bank: Option<MusicBank>,
    fade_stop_seconds: f32,
    current_gains: [LayerGains; 2],
    target_gains: [LayerGains; 2],
    pending_state: Option<PendingMusicStateTransition>,
    default_resume_started: bool,
    debug_log_timer: f32,
    last_simple_track: Option<String>,
}

impl Default for MusicDirectorState {
    fn default() -> Self {
        Self {
            mode: MusicDirectorMode::Idle,
            active_cue_id: None,
            current_state_id: None,
            current_section_id: None,
            seconds_in_mode: 0.0,
            seconds_in_loop: 0.0,
            active_bank: MusicBank::A,
            fading_bank: None,
            fade_stop_seconds: 0.0,
            current_gains: [[0.0; MAX_LAYERS]; 2],
            target_gains: [[0.0; MAX_LAYERS]; 2],
            pending_state: None,
            default_resume_started: false,
            debug_log_timer: 0.0,
            last_simple_track: None,
        }
    }
}

#[derive(Debug, Clone)]
struct PendingMusicStateTransition {
    state_id: String,
    delay_seconds: f32,
}

#[derive(Debug, Clone)]
enum AdaptiveCueDirective {
    Play { cue_id: String, state_id: String },
    StopNow,
}

#[derive(SystemParam)]
pub struct MusicLayerChannels<'w> {
    layer0_a: Res<'w, AudioChannel<MusicLayer0AChannel>>,
    layer1_a: Res<'w, AudioChannel<MusicLayer1AChannel>>,
    layer2_a: Res<'w, AudioChannel<MusicLayer2AChannel>>,
    layer3_a: Res<'w, AudioChannel<MusicLayer3AChannel>>,
    layer4_a: Res<'w, AudioChannel<MusicLayer4AChannel>>,
    layer5_a: Res<'w, AudioChannel<MusicLayer5AChannel>>,
    layer0_b: Res<'w, AudioChannel<MusicLayer0BChannel>>,
    layer1_b: Res<'w, AudioChannel<MusicLayer1BChannel>>,
    layer2_b: Res<'w, AudioChannel<MusicLayer2BChannel>>,
    layer3_b: Res<'w, AudioChannel<MusicLayer3BChannel>>,
    layer4_b: Res<'w, AudioChannel<MusicLayer4BChannel>>,
    layer5_b: Res<'w, AudioChannel<MusicLayer5BChannel>>,
}

impl<'w> MusicLayerChannels<'w> {
    fn channel(&self, bank: MusicBank, slot: usize) -> &dyn MusicLayerChannel {
        match (bank, slot.min(MAX_LAYERS - 1)) {
            (MusicBank::A, 0) => &*self.layer0_a,
            (MusicBank::A, 1) => &*self.layer1_a,
            (MusicBank::A, 2) => &*self.layer2_a,
            (MusicBank::A, 3) => &*self.layer3_a,
            (MusicBank::A, 4) => &*self.layer4_a,
            (MusicBank::A, _) => &*self.layer5_a,
            (MusicBank::B, 0) => &*self.layer0_b,
            (MusicBank::B, 1) => &*self.layer1_b,
            (MusicBank::B, 2) => &*self.layer2_b,
            (MusicBank::B, 3) => &*self.layer3_b,
            (MusicBank::B, 4) => &*self.layer4_b,
            (MusicBank::B, _) => &*self.layer5_b,
        }
    }

    fn stop_all(&self, fade_ms: u64) {
        self.stop_bank(MusicBank::A, fade_ms);
        self.stop_bank(MusicBank::B, fade_ms);
    }

    fn stop_bank(&self, bank: MusicBank, fade_ms: u64) {
        for slot in 0..MAX_LAYERS {
            self.channel(bank, slot).stop_with_fade(fade_ms);
        }
    }

    fn set_bank_silent(&self, bank: MusicBank) {
        for slot in 0..MAX_LAYERS {
            self.channel(bank, slot).set_linear_volume(0.0);
        }
    }

    fn set_layer_volume(&self, bank: MusicBank, slot: usize, linear: f32) {
        self.channel(bank, slot).set_linear_volume(linear);
    }

    fn play_layer(
        &self,
        bank: MusicBank,
        slot: usize,
        handle: Handle<KiraAudioSource>,
        looped: bool,
        fade_ms: u64,
    ) {
        self.channel(bank, slot).play_handle(handle, looped, fade_ms);
    }
}

trait MusicLayerChannel {
    fn stop_with_fade(&self, fade_ms: u64);
    fn set_linear_volume(&self, linear: f32);
    fn play_handle(&self, handle: Handle<KiraAudioSource>, looped: bool, fade_ms: u64);
}

macro_rules! impl_music_layer_channel {
    ($marker:ty) => {
        impl MusicLayerChannel for AudioChannel<$marker> {
            fn stop_with_fade(&self, fade_ms: u64) {
                self.stop().fade_out(AudioTween::new(
                    Duration::from_millis(fade_ms),
                    AudioEasing::OutPowi(2),
                ));
            }

            fn set_linear_volume(&self, linear: f32) {
                self.set_volume(amplitude_to_decibels(linear));
            }

            fn play_handle(&self, handle: Handle<KiraAudioSource>, looped: bool, fade_ms: u64) {
                if looped {
                    if fade_ms == 0 {
                        self.play(handle).looped();
                    } else {
                        self.play(handle).looped().fade_in(AudioTween::new(
                            Duration::from_millis(fade_ms),
                            AudioEasing::InPowi(2),
                        ));
                    }
                } else if fade_ms == 0 {
                    self.play(handle);
                } else {
                    self.play(handle).fade_in(AudioTween::new(
                        Duration::from_millis(fade_ms),
                        AudioEasing::InPowi(2),
                    ));
                }
            }
        }
    };
}

impl_music_layer_channel!(MusicLayer0AChannel);
impl_music_layer_channel!(MusicLayer1AChannel);
impl_music_layer_channel!(MusicLayer2AChannel);
impl_music_layer_channel!(MusicLayer3AChannel);
impl_music_layer_channel!(MusicLayer4AChannel);
impl_music_layer_channel!(MusicLayer5AChannel);
impl_music_layer_channel!(MusicLayer0BChannel);
impl_music_layer_channel!(MusicLayer1BChannel);
impl_music_layer_channel!(MusicLayer2BChannel);
impl_music_layer_channel!(MusicLayer3BChannel);
impl_music_layer_channel!(MusicLayer4BChannel);
impl_music_layer_channel!(MusicLayer5BChannel);

/// Load file-backed cue sources and install the generic cue catalog.
pub fn load_music_cues(mut commands: Commands, asset_server: Res<AssetServer>) {
    let catalog = MusicCueCatalog::builtin();
    let mut sources = HashMap::new();
    for cue in catalog.cues.values() {
        for section in &cue.sections {
            for source in &section.sources {
                let rel = format!("{}/{}", cue.asset_root.trim_end_matches('/'), source.path);
                sources.insert(
                    MusicSourceKey::new(&cue.id, &section.id, &source.layer_id),
                    asset_server.load(rel),
                );
            }
        }
        info!(
            target: MUSIC_LOG_TARGET,
            "loaded music cue id={} sections={} layers={}",
            cue.id,
            cue.sections.len(),
            cue.layers.len(),
        );
    }

    commands.insert_resource(catalog);
    commands.insert_resource(LoadedMusicCueAssets { sources });
    commands.insert_resource(MusicDirectorState::default());
}

/// Unified music director.
///
/// Handles both simple track selection and adaptive cue state transitions. The
/// simple track backend still reuses the existing `AudioLibrary` / `MusicChannel`
/// sources; adaptive cues use the generic layer-bank scheduler below.
pub fn drive_music_director(
    time: Res<Time>,
    catalog: Option<Res<MusicCueCatalog>>,
    assets: Option<Res<LoadedMusicCueAssets>>,
    director: Option<ResMut<MusicDirectorState>>,
    encounters: Res<EncounterRegistry>,
    mut encounter_music: ResMut<EncounterMusicRequest>,
    room_music: Res<RoomMusicRequest>,
    layer_channels: MusicLayerChannels,
    base_music_channel: Res<AudioChannel<MusicChannel>>,
    library: Res<AudioLibrary>,
    mut music_state: ResMut<MusicPlaybackState>,
    sandbox_data: Res<SandboxDataSpec>,
    settings: Res<UserSettings>,
) {
    let Some(catalog) = catalog else { return; };
    let Some(assets) = assets else { return; };
    let Some(mut director) = director else { return; };

    let dt = time.delta_secs();
    director.seconds_in_mode += dt;
    if director.mode == MusicDirectorMode::AdaptiveLoop {
        director.seconds_in_loop += dt;
    }

    let adaptive = resolve_adaptive_directive(&catalog, &encounters, &director);
    match adaptive {
        Some(AdaptiveCueDirective::Play { cue_id, state_id }) => {
            if let (Some(cue), Some(target_state)) = (
                catalog.cue(&cue_id),
                catalog.cue(&cue_id).and_then(|cue| cue.state(&state_id)),
            ) {
                drive_adaptive_cue_state(
                    &mut director,
                    cue,
                    target_state,
                    &assets,
                    &layer_channels,
                    &base_music_channel,
                    &settings,
                    dt,
                );
            } else {
                warn!(
                    target: MUSIC_LOG_TARGET,
                    "adaptive directive references missing cue/state cue={} state={}",
                    cue_id,
                    state_id,
                );
            }
        }
        Some(AdaptiveCueDirective::StopNow) => {
            if director.active_cue_id.is_some() {
                shutdown_adaptive_cue(
                    &mut director,
                    &layer_channels,
                    &library,
                    &mut music_state,
                    &base_music_channel,
                    &room_music,
                    &sandbox_data,
                    &mut encounter_music,
                );
            }
        }
        None => {
            if director.active_cue_id.is_some()
                && director.mode != MusicDirectorMode::AdaptiveFinished
                && director.mode != MusicDirectorMode::Idle
            {
                // Leaving the room or losing the cue owner without a clear should
                // not leave the adaptive channels running.
                shutdown_adaptive_cue(
                    &mut director,
                    &layer_channels,
                    &library,
                    &mut music_state,
                    &base_music_channel,
                    &room_music,
                    &sandbox_data,
                    &mut encounter_music,
                );
            } else {
                apply_simple_music_intent(
                    &mut director,
                    &library,
                    &mut music_state,
                    &base_music_channel,
                    &room_music,
                    &sandbox_data,
                    &mut encounter_music,
                );
            }
        }
    }

    if let Some(cue_id) = director.active_cue_id.clone() {
        if let Some(cue) = catalog.cue(&cue_id) {
            update_gain_smoothing(&mut director, &layer_channels, dt);
            drive_outro_tail(
                &mut director,
                cue,
                &layer_channels,
                &library,
                &mut music_state,
                &base_music_channel,
                &room_music,
                &sandbox_data,
                &mut encounter_music,
            );
            log_periodic_state(&mut director, cue, dt);
        }
    }
}

fn resolve_adaptive_directive(
    catalog: &MusicCueCatalog,
    encounters: &EncounterRegistry,
    director: &MusicDirectorState,
) -> Option<AdaptiveCueDirective> {
    let binding = catalog.binding_for_encounter(MOB_LAB_ENCOUNTER_ID)?;
    let Some(encounter) = encounters.get(&binding.encounter_id) else {
        if director.active_cue_id.as_deref() == Some(binding.cue_id.as_str()) {
            return Some(AdaptiveCueDirective::StopNow);
        }
        return None;
    };

    match encounter.phase {
        EncounterPhase::Starting { .. } => Some(AdaptiveCueDirective::Play {
            cue_id: binding.cue_id.clone(),
            state_id: binding.starting_state.clone(),
        }),
        EncounterPhase::Active { wave_index, .. } => {
            let mut state_id = binding
                .wave_states
                .get(wave_index)
                .or_else(|| binding.wave_states.last())
                .cloned()
                .unwrap_or_else(|| binding.starting_state.clone());
            if wave_index == 1 && encounter.run.wave_elapsed >= LARGE_BRUTE_DELAY_SECONDS {
                if let Some(reinforced) = &binding.wave2_reinforced_state {
                    state_id = reinforced.clone();
                }
            }
            Some(AdaptiveCueDirective::Play {
                cue_id: binding.cue_id.clone(),
                state_id,
            })
        }
        EncounterPhase::Cleared => Some(AdaptiveCueDirective::Play {
            cue_id: binding.cue_id.clone(),
            state_id: binding.cleared_state.clone(),
        }),
        EncounterPhase::Inactive => {
            // The encounter often resets to Inactive immediately after clear;
            // if this adaptive cue is already active, continue into its outro
            // instead of hard-cutting to room music.
            if director.active_cue_id.as_deref() == Some(binding.cue_id.as_str())
                && director.mode != MusicDirectorMode::AdaptiveFinished
                && director.mode != MusicDirectorMode::Idle
            {
                Some(AdaptiveCueDirective::Play {
                    cue_id: binding.cue_id.clone(),
                    state_id: binding.cleared_state.clone(),
                })
            } else {
                None
            }
        }
        EncounterPhase::Failed => Some(AdaptiveCueDirective::StopNow),
    }
}

fn apply_simple_music_intent(
    director: &mut MusicDirectorState,
    library: &AudioLibrary,
    music_state: &mut MusicPlaybackState,
    base_music_channel: &AudioChannel<MusicChannel>,
    room_music: &RoomMusicRequest,
    sandbox_data: &SandboxDataSpec,
    encounter_music: &mut EncounterMusicRequest,
) {
    let target = resolved_simple_track(library, room_music, sandbox_data, encounter_music);
    let needs_switch = director.last_simple_track.as_deref() != Some(target.as_str())
        || music_state.active_track != target;
    if needs_switch && library.track(&target).is_some() {
        info!(target: MUSIC_LOG_TARGET, "simple_music target={}", target);
        switch_to_music_track(library, music_state, base_music_channel, &target);
        director.last_simple_track = Some(target.clone());
        director.mode = MusicDirectorMode::SimpleTrack;
    }
    encounter_music.last_applied = Some(target);
}

fn resolved_simple_track(
    library: &AudioLibrary,
    room_music: &RoomMusicRequest,
    sandbox_data: &SandboxDataSpec,
    encounter_music: &EncounterMusicRequest,
) -> String {
    if let Some(track) = &encounter_music.desired_track {
        if library.track(track).is_some() {
            return track.clone();
        }
    }
    room_music
        .desired_track
        .as_ref()
        .filter(|track| library.track(track).is_some())
        .cloned()
        .unwrap_or_else(|| sandbox_data.audio.default_music_track.clone())
}

fn drive_adaptive_cue_state(
    director: &mut MusicDirectorState,
    cue: &MusicCueSpec,
    target_state: &MusicStateSpec,
    assets: &LoadedMusicCueAssets,
    channels: &MusicLayerChannels,
    base_music_channel: &AudioChannel<MusicChannel>,
    settings: &UserSettings,
    dt: f32,
) {
    if director.active_cue_id.as_deref() != Some(cue.id.as_str()) {
        base_music_channel.stop().fade_out(AudioTween::new(
            Duration::from_millis(650),
            AudioEasing::OutPowi(2),
        ));
        start_adaptive_state(
            director,
            cue,
            target_state,
            assets,
            channels,
            settings,
            INTRO_TO_LOOP_CROSSFADE_SECONDS,
        );
        return;
    }

    let current_state_matches = director.current_state_id.as_deref() == Some(target_state.id.as_str());
    if current_state_matches {
        let active_bank = director.active_bank;
        set_bank_targets(
            director,
            active_bank,
            gains_for_state(cue, target_state, settings),
        );
        return;
    }

    let target_section = match cue.section(&target_state.section_id) {
        Some(section) => section,
        None => {
            warn!(
                target: MUSIC_LOG_TARGET,
                "music state references missing section cue={} state={} section={}",
                cue.id,
                target_state.id,
                target_state.section_id,
            );
            return;
        }
    };

    if let Some(current_section_id) = director.current_section_id.as_deref() {
        if current_section_id == target_section.id {
            let active_bank = director.active_bank;
            set_bank_targets(
                director,
                active_bank,
                gains_for_state(cue, target_state, settings),
            );
            director.current_state_id = Some(target_state.id.clone());
            return;
        }
    }

    if is_outro_target(cue, target_state) && director.mode != MusicDirectorMode::AdaptiveOutro {
        queue_or_fire_outro(director, cue, target_state, assets, channels, settings, dt);
        return;
    }

    if director.mode == MusicDirectorMode::AdaptiveIntro {
        let current_section = director
            .current_section_id
            .as_deref()
            .and_then(|id| cue.section(id));
        let intro_done = current_section
            .map(|section| director.seconds_in_mode >= section.duration_seconds(cue))
            .unwrap_or(true);
        if !intro_done {
            return;
        }
    }

    if let Some(mut pending) = director.pending_state.clone() {
        pending.state_id = target_state.id.clone();
        pending.delay_seconds -= dt;
        if pending.delay_seconds <= 0.0 {
            director.pending_state = None;
            start_adaptive_state(
                director,
                cue,
                target_state,
                assets,
                channels,
                settings,
                LOOP_SECTION_CROSSFADE_SECONDS,
            );
        } else {
            director.pending_state = Some(pending);
        }
    } else {
        let delay = if director.mode == MusicDirectorMode::AdaptiveLoop {
            seconds_until_next_bar(cue, director.seconds_in_loop).max(MIN_TRANSITION_DELAY_SECONDS)
        } else {
            MIN_TRANSITION_DELAY_SECONDS
        };
        info!(
            target: MUSIC_LOG_TARGET,
            "queue_music_state cue={} state={} section={} delay={:.3}s current_section={:?}",
            cue.id,
            target_state.id,
            target_section.id,
            delay,
            director.current_section_id,
        );
        director.pending_state = Some(PendingMusicStateTransition {
            state_id: target_state.id.clone(),
            delay_seconds: delay,
        });
    }
}

fn queue_or_fire_outro(
    director: &mut MusicDirectorState,
    cue: &MusicCueSpec,
    target_state: &MusicStateSpec,
    assets: &LoadedMusicCueAssets,
    channels: &MusicLayerChannels,
    settings: &UserSettings,
    dt: f32,
) {
    if director.pending_state.is_none() {
        let delay = seconds_until_next_phrase_marker(cue, director.seconds_in_loop, 2.0)
            .max(MIN_TRANSITION_DELAY_SECONDS);
        info!(
            target: MUSIC_LOG_TARGET,
            "queue_outro cue={} state={} delay={:.3}s loop_t={:.3}",
            cue.id,
            target_state.id,
            delay,
            director.seconds_in_loop,
        );
        director.pending_state = Some(PendingMusicStateTransition {
            state_id: target_state.id.clone(),
            delay_seconds: delay,
        });
    }

    if let Some(bridge_state_id) = cue.post_clear_bridge_state.as_deref() {
        if let Some(bridge) = cue.state(bridge_state_id) {
            let active_bank = director.active_bank;
            set_bank_targets(director, active_bank, gains_for_state(cue, bridge, settings));
        }
    }

    if let Some(mut pending) = director.pending_state.clone() {
        pending.delay_seconds -= dt;
        if pending.delay_seconds <= 0.0 {
            director.pending_state = None;
            start_adaptive_state(
                director,
                cue,
                target_state,
                assets,
                channels,
                settings,
                OUTRO_CROSSFADE_SECONDS,
            );
        } else {
            director.pending_state = Some(pending);
        }
    }
}

fn start_adaptive_state(
    director: &mut MusicDirectorState,
    cue: &MusicCueSpec,
    target_state: &MusicStateSpec,
    assets: &LoadedMusicCueAssets,
    channels: &MusicLayerChannels,
    settings: &UserSettings,
    crossfade_seconds: f32,
) {
    let Some(section) = cue.section(&target_state.section_id) else {
        warn!(
            target: MUSIC_LOG_TARGET,
            "cannot start missing music section cue={} state={} section={}",
            cue.id,
            target_state.id,
            target_state.section_id,
        );
        return;
    };

    let old_bank = director.active_bank;
    let new_bank = if director.active_cue_id.is_some() {
        old_bank.other()
    } else {
        MusicBank::A
    };

    info!(
        target: MUSIC_LOG_TARGET,
        "start_adaptive_state cue={} state={} section={} old_bank={} new_bank={} looped={} crossfade={:.2}s gains={}",
        cue.id,
        target_state.id,
        section.id,
        old_bank.label(),
        new_bank.label(),
        section.looped,
        crossfade_seconds,
        format_gains(gains_for_state(cue, target_state, settings)),
    );

    channels.stop_bank(new_bank, 80);
    channels.set_bank_silent(new_bank);
    director.current_gains[new_bank.index()] = [0.0; MAX_LAYERS];
    director.target_gains[new_bank.index()] = [0.0; MAX_LAYERS];

    let mut started = 0usize;
    for source in &section.sources {
        let slot = cue
            .layer(&source.layer_id)
            .map(|layer| layer.slot.min(MAX_LAYERS - 1))
            .unwrap_or(0);
        if let Some(handle) = assets.get(&cue.id, &section.id, &source.layer_id) {
            channels.play_layer(new_bank, slot, handle, section.looped, LAYER_START_FADE_MS);
            started += 1;
        } else {
            warn!(
                target: MUSIC_LOG_TARGET,
                "missing music source cue={} section={} layer={}",
                cue.id,
                section.id,
                source.layer_id,
            );
        }
    }

    if director.active_cue_id.is_some() && new_bank != old_bank {
        set_bank_targets(director, old_bank, [0.0; MAX_LAYERS]);
        director.fading_bank = Some(old_bank);
        director.fade_stop_seconds = crossfade_seconds + 0.35;
    } else {
        channels.stop_bank(old_bank.other(), 80);
        director.fading_bank = None;
        director.fade_stop_seconds = 0.0;
    }

    set_bank_targets(director, new_bank, gains_for_state(cue, target_state, settings));
    director.active_cue_id = Some(cue.id.clone());
    director.current_state_id = Some(target_state.id.clone());
    director.current_section_id = Some(section.id.clone());
    director.active_bank = new_bank;
    director.seconds_in_mode = 0.0;
    director.seconds_in_loop = 0.0;
    director.pending_state = None;
    director.default_resume_started = false;
    director.mode = if is_outro_target(cue, target_state) {
        MusicDirectorMode::AdaptiveOutro
    } else if section.looped {
        MusicDirectorMode::AdaptiveLoop
    } else {
        MusicDirectorMode::AdaptiveIntro
    };

    info!(
        target: MUSIC_LOG_TARGET,
        "started_music_sources cue={} state={} section={} bank={} source_count={} volume_blend={:.2}s",
        cue.id,
        target_state.id,
        section.id,
        new_bank.label(),
        started,
        STEM_GAIN_BLEND_SECONDS,
    );
}

fn drive_outro_tail(
    director: &mut MusicDirectorState,
    cue: &MusicCueSpec,
    channels: &MusicLayerChannels,
    library: &AudioLibrary,
    music_state: &mut MusicPlaybackState,
    base_music_channel: &AudioChannel<MusicChannel>,
    room_music: &RoomMusicRequest,
    sandbox_data: &SandboxDataSpec,
    encounter_music: &mut EncounterMusicRequest,
) {
    if director.mode != MusicDirectorMode::AdaptiveOutro {
        return;
    }
    let duration = director
        .current_section_id
        .as_deref()
        .and_then(|id| cue.section(id))
        .map(|section| section.duration_seconds(cue))
        .unwrap_or(0.0);
    if !director.default_resume_started
        && director.seconds_in_mode >= (duration - DEFAULT_RETURN_OVERLAP_SECONDS).max(0.0)
    {
        resume_simple_music(
            director,
            library,
            music_state,
            base_music_channel,
            room_music,
            sandbox_data,
            encounter_music,
        );
        director.default_resume_started = true;
    }
    if director.seconds_in_mode >= duration {
        info!(
            target: MUSIC_LOG_TARGET,
            "finish_adaptive_outro cue={} t={:.3}",
            cue.id,
            director.seconds_in_mode,
        );
        director.mode = MusicDirectorMode::AdaptiveFinished;
        director.active_cue_id = None;
        director.current_state_id = None;
        director.current_section_id = None;
        channels.stop_all(900);
        zero_all_targets(director);
    }
}

fn shutdown_adaptive_cue(
    director: &mut MusicDirectorState,
    channels: &MusicLayerChannels,
    library: &AudioLibrary,
    music_state: &mut MusicPlaybackState,
    base_music_channel: &AudioChannel<MusicChannel>,
    room_music: &RoomMusicRequest,
    sandbox_data: &SandboxDataSpec,
    encounter_music: &mut EncounterMusicRequest,
) {
    info!(
        target: MUSIC_LOG_TARGET,
        "shutdown_adaptive_cue cue={:?} mode={:?} state={:?} section={:?}",
        director.active_cue_id,
        director.mode,
        director.current_state_id,
        director.current_section_id,
    );
    channels.stop_all(650);
    director.active_cue_id = None;
    director.current_state_id = None;
    director.current_section_id = None;
    director.mode = MusicDirectorMode::Idle;
    director.pending_state = None;
    zero_all_current_and_targets(director);
    resume_simple_music(
        director,
        library,
        music_state,
        base_music_channel,
        room_music,
        sandbox_data,
        encounter_music,
    );
}

fn resume_simple_music(
    director: &mut MusicDirectorState,
    library: &AudioLibrary,
    music_state: &mut MusicPlaybackState,
    base_music_channel: &AudioChannel<MusicChannel>,
    room_music: &RoomMusicRequest,
    sandbox_data: &SandboxDataSpec,
    encounter_music: &mut EncounterMusicRequest,
) {
    let target = resolved_simple_track(library, room_music, sandbox_data, encounter_music);
    if library.track(&target).is_some() {
        info!(target: MUSIC_LOG_TARGET, "resume_simple_music target={}", target);
        switch_to_music_track(library, music_state, base_music_channel, &target);
        director.last_simple_track = Some(target.clone());
        encounter_music.last_applied = Some(target);
        director.mode = MusicDirectorMode::SimpleTrack;
    }
}

fn is_outro_target(cue: &MusicCueSpec, state: &MusicStateSpec) -> bool {
    cue.outro_state.as_deref() == Some(state.id.as_str())
}

fn gains_for_state(cue: &MusicCueSpec, state: &MusicStateSpec, settings: &UserSettings) -> LayerGains {
    let mut gains = [0.0; MAX_LAYERS];
    let master = settings.audio.effective_music() * cue.relative_volume;
    for layer_gain in &state.gains {
        if let Some(layer) = cue.layer(&layer_gain.layer_id) {
            let slot = layer.slot.min(MAX_LAYERS - 1);
            gains[slot] = layer_gain.gain.max(0.0) * master;
        }
    }
    gains
}

fn set_bank_targets(director: &mut MusicDirectorState, bank: MusicBank, gains: LayerGains) {
    director.target_gains[bank.index()] = gains;
}

fn zero_all_targets(director: &mut MusicDirectorState) {
    director.target_gains = [[0.0; MAX_LAYERS]; 2];
}

fn zero_all_current_and_targets(director: &mut MusicDirectorState) {
    director.current_gains = [[0.0; MAX_LAYERS]; 2];
    director.target_gains = [[0.0; MAX_LAYERS]; 2];
}

fn update_gain_smoothing(
    director: &mut MusicDirectorState,
    channels: &MusicLayerChannels,
    dt: f32,
) {
    let alpha = if STEM_GAIN_BLEND_SECONDS <= 0.0 {
        1.0
    } else {
        1.0 - (-dt / STEM_GAIN_BLEND_SECONDS).exp()
    };
    for bank in [MusicBank::A, MusicBank::B] {
        for slot in 0..MAX_LAYERS {
            let current = director.current_gains[bank.index()][slot];
            let target = director.target_gains[bank.index()][slot];
            let next = current + (target - current) * alpha;
            director.current_gains[bank.index()][slot] = if next.abs() < 0.0005 { 0.0 } else { next };
            channels.set_layer_volume(bank, slot, director.current_gains[bank.index()][slot]);
        }
    }

    if let Some(fading_bank) = director.fading_bank {
        director.fade_stop_seconds -= dt;
        if director.fade_stop_seconds <= 0.0 {
            channels.stop_bank(fading_bank, 120);
            director.current_gains[fading_bank.index()] = [0.0; MAX_LAYERS];
            director.target_gains[fading_bank.index()] = [0.0; MAX_LAYERS];
            director.fading_bank = None;
        }
    }
}

fn seconds_until_next_bar(cue: &MusicCueSpec, seconds_in_loop: f32) -> f32 {
    let bar = cue.seconds_per_bar().max(0.001);
    let rem = seconds_in_loop.rem_euclid(bar);
    if rem <= 0.001 { 0.0 } else { bar - rem }
}

fn seconds_until_next_phrase_marker(
    cue: &MusicCueSpec,
    seconds_in_loop: f32,
    bars_per_phrase: f32,
) -> f32 {
    let phrase = (cue.seconds_per_bar() * bars_per_phrase.max(1.0)).max(0.001);
    let rem = seconds_in_loop.rem_euclid(phrase);
    if rem <= 0.001 { 0.0 } else { phrase - rem }
}

fn log_periodic_state(director: &mut MusicDirectorState, cue: &MusicCueSpec, dt: f32) {
    director.debug_log_timer -= dt;
    if director.debug_log_timer > 0.0 {
        return;
    }
    director.debug_log_timer = DEBUG_LOG_PERIOD_SECONDS;
    debug!(
        target: MUSIC_LOG_TARGET,
        "music_director mode={:?} cue={:?} state={:?} section={:?} t_mode={:.3} t_loop={:.3} bar_beat={} active_bank={} gains_a={} gains_b={}",
        director.mode,
        director.active_cue_id,
        director.current_state_id,
        director.current_section_id,
        director.seconds_in_mode,
        director.seconds_in_loop,
        format_bar_beat(cue, director.seconds_in_loop),
        director.active_bank.label(),
        format_gains(director.current_gains[MusicBank::A.index()]),
        format_gains(director.current_gains[MusicBank::B.index()]),
    );
}

fn format_bar_beat(cue: &MusicCueSpec, seconds: f32) -> String {
    let beat = seconds / cue.seconds_per_beat();
    let beats_per_bar = cue.beats_per_bar.max(1.0);
    let bar = (beat / beats_per_bar).floor() as i32 + 1;
    let beat_in_bar = beat.rem_euclid(beats_per_bar) + 1.0;
    format!("{}.{}", bar, beat_in_bar.floor() as i32)
}

fn format_gains(gains: LayerGains) -> String {
    gains
        .iter()
        .map(|g| format!("{g:.2}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn first_goblin_tune_v2_spec() -> MusicCueSpec {
    let asset_root = "audio/music/generated/first_goblin_tune_v2".to_string();
    let layers = vec![
        MusicLayerSpec { id: "strings".into(), slot: 0 },
        MusicLayerSpec { id: "brass".into(), slot: 1 },
        MusicLayerSpec { id: "winds".into(), slot: 2 },
        MusicLayerSpec { id: "choir_pad".into(), slot: 3 },
        MusicLayerSpec { id: "mallets".into(), slot: 4 },
        MusicLayerSpec { id: "percussion".into(), slot: 5 },
        // The full intro/outro renders are intentionally mapped to slot 0;
        // they are exclusive one-shot layers, not simultaneous stems.
        MusicLayerSpec { id: "full".into(), slot: 0 },
    ];

    fn stem_sources(section: &str) -> Vec<MusicLayerSourceSpec> {
        ["strings", "brass", "winds", "choir_pad", "mallets", "percussion"]
            .into_iter()
            .map(|layer| MusicLayerSourceSpec {
                layer_id: layer.to_string(),
                path: format!("adaptive/{section}/{section}.{layer}.ogg"),
            })
            .collect()
    }

    fn full_source(section: &str) -> Vec<MusicLayerSourceSpec> {
        vec![MusicLayerSourceSpec {
            layer_id: "full".into(),
            path: format!("adaptive/{section}/{section}.full.ogg"),
        }]
    }

    fn gains(items: &[(&str, f32)]) -> Vec<MusicLayerGainSpec> {
        items
            .iter()
            .map(|(layer, gain)| MusicLayerGainSpec {
                layer_id: (*layer).to_string(),
                gain: *gain,
            })
            .collect()
    }

    MusicCueSpec {
        id: FIRST_GOBLIN_CUE_ID.to_string(),
        asset_root,
        bpm: 132.0,
        beats_per_bar: 4.0,
        relative_volume: ADAPTIVE_MUSIC_RELATIVE_VOLUME,
        layers,
        sections: vec![
            MusicSectionSpec {
                id: "intro".into(),
                duration_beats: 16.0,
                looped: false,
                sources: full_source("intro"),
            },
            MusicSectionSpec {
                id: "wave1".into(),
                duration_beats: 32.0,
                looped: true,
                sources: stem_sources("wave1"),
            },
            MusicSectionSpec {
                id: "wave2".into(),
                duration_beats: 32.0,
                looped: true,
                sources: stem_sources("wave2"),
            },
            MusicSectionSpec {
                id: "wave3".into(),
                duration_beats: 32.0,
                looped: true,
                sources: stem_sources("wave3"),
            },
            MusicSectionSpec {
                id: "recap_loop".into(),
                duration_beats: 32.0,
                looped: true,
                sources: stem_sources("recap_loop"),
            },
            MusicSectionSpec {
                id: "outro".into(),
                duration_beats: 16.0,
                looped: false,
                sources: full_source("outro"),
            },
        ],
        states: vec![
            MusicStateSpec {
                id: "intro".into(),
                section_id: "intro".into(),
                gains: gains(&[("full", 1.0)]),
            },
            MusicStateSpec {
                id: "wave1".into(),
                section_id: "wave1".into(),
                gains: gains(&[
                    ("strings", 0.54),
                    ("brass", 0.05),
                    ("winds", 0.24),
                    ("choir_pad", 0.08),
                    ("mallets", 0.11),
                    ("percussion", 0.00),
                ]),
            },
            MusicStateSpec {
                id: "wave2".into(),
                section_id: "wave2".into(),
                gains: gains(&[
                    ("strings", 0.62),
                    ("brass", 0.24),
                    ("winds", 0.22),
                    ("choir_pad", 0.10),
                    ("mallets", 0.10),
                    ("percussion", 0.30),
                ]),
            },
            MusicStateSpec {
                id: "wave2_brute".into(),
                section_id: "wave2".into(),
                gains: gains(&[
                    ("strings", 0.68),
                    ("brass", 0.34),
                    ("winds", 0.24),
                    ("choir_pad", 0.12),
                    ("mallets", 0.10),
                    ("percussion", 0.39),
                ]),
            },
            MusicStateSpec {
                id: "wave3".into(),
                section_id: "wave3".into(),
                gains: gains(&[
                    ("strings", 0.72),
                    ("brass", 0.44),
                    ("winds", 0.28),
                    ("choir_pad", 0.16),
                    ("mallets", 0.12),
                    ("percussion", 0.42),
                ]),
            },
            MusicStateSpec {
                id: "cleared_bridge".into(),
                section_id: "recap_loop".into(),
                gains: gains(&[
                    ("strings", 0.26),
                    ("brass", 0.00),
                    ("winds", 0.10),
                    ("choir_pad", 0.08),
                    ("mallets", 0.03),
                    ("percussion", 0.00),
                ]),
            },
            MusicStateSpec {
                id: "outro".into(),
                section_id: "outro".into(),
                gains: gains(&[("full", 1.0)]),
            },
        ],
        outro_state: Some("outro".into()),
        post_clear_bridge_state: Some("cleared_bridge".into()),
    }
}
