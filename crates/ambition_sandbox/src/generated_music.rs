//! Runtime integration for generated/adaptive music assets.
//!
//! This module is intentionally a thin runtime player for OGG files produced by
//! `tools/audio/goblin_orchestra_renderer-*`.  The Python renderer remains the
//! authoring/build-time compiler; Bevy/Kira only loads already-rendered pieces
//! and schedules intro -> loop sections -> outro while fading stems by battle
//! state.

#![cfg(feature = "audio")]

use std::collections::HashMap;
use std::time::Duration;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_kira_audio::prelude::{
    AudioChannel, AudioControl, AudioEasing, AudioSource as KiraAudioSource, AudioTween,
};

use crate::audio::{amplitude_to_decibels, MusicChannel};
use crate::encounter::{EncounterPhase, EncounterRegistry};
use crate::settings::UserSettings;

pub const GENERATED_GOBLIN_ENCOUNTER_ID: &str = "mob_lab";
pub const GENERATED_GOBLIN_CUE_ID: &str = "first_goblin_encounter_orchestra";
pub const GENERATED_GOBLIN_ASSET_ROOT: &str =
    "audio/music/generated/first_goblin_encounter";

const BPM: f32 = 132.0;
const BEATS_PER_BAR: f32 = 4.0;
const INTRO_SECONDS: f32 = 7.272_727;
const LOOP_SECONDS: f32 = 29.090_91;
const OUTRO_SECONDS: f32 = 14.545_455;
const LARGE_BRUTE_DELAY_SECONDS: f32 = 3.5;

#[derive(Resource)]
pub struct GeneratedMusicStringsChannel;
#[derive(Resource)]
pub struct GeneratedMusicBrassChannel;
#[derive(Resource)]
pub struct GeneratedMusicWindsChannel;
#[derive(Resource)]
pub struct GeneratedMusicChoirPadChannel;
#[derive(Resource)]
pub struct GeneratedMusicMalletsChannel;
#[derive(Resource)]
pub struct GeneratedMusicPercussionChannel;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GeneratedStem {
    Strings,
    Brass,
    Winds,
    ChoirPad,
    Mallets,
    Percussion,
}

impl GeneratedStem {
    pub const ALL: [Self; 6] = [
        Self::Strings,
        Self::Brass,
        Self::Winds,
        Self::ChoirPad,
        Self::Mallets,
        Self::Percussion,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::Strings => "strings",
            Self::Brass => "brass",
            Self::Winds => "winds",
            Self::ChoirPad => "choir_pad",
            Self::Mallets => "mallets",
            Self::Percussion => "percussion",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GeneratedSection {
    Intro,
    Wave1,
    Wave2,
    Wave3,
    RecapLoop,
    Outro,
}

impl GeneratedSection {
    pub fn id(self) -> &'static str {
        match self {
            Self::Intro => "intro",
            Self::Wave1 => "wave1",
            Self::Wave2 => "wave2",
            Self::Wave3 => "wave3",
            Self::RecapLoop => "recap_loop",
            Self::Outro => "outro",
        }
    }

    pub fn duration_seconds(self) -> f32 {
        match self {
            Self::Intro => INTRO_SECONDS,
            Self::Wave1 | Self::Wave2 | Self::Wave3 | Self::RecapLoop => LOOP_SECONDS,
            Self::Outro => OUTRO_SECONDS,
        }
    }

    pub fn loopable(self) -> bool {
        matches!(self, Self::Wave1 | Self::Wave2 | Self::Wave3 | Self::RecapLoop)
    }
}

#[derive(Resource, Clone)]
pub struct GeneratedGoblinMusicAssets {
    pub stems: HashMap<(GeneratedSection, GeneratedStem), Handle<KiraAudioSource>>,
    pub intro_full: Handle<KiraAudioSource>,
    pub outro_full: Handle<KiraAudioSource>,
}

impl GeneratedGoblinMusicAssets {
    fn stem(&self, section: GeneratedSection, stem: GeneratedStem) -> Option<Handle<KiraAudioSource>> {
        self.stems.get(&(section, stem)).cloned()
    }
}

#[derive(Resource, Debug, Clone)]
pub struct GeneratedGoblinMusicState {
    pub active: bool,
    pub mode: GeneratedMusicMode,
    pub current_section: Option<GeneratedSection>,
    pub current_wave_index: Option<usize>,
    pub seconds_in_mode: f32,
    pub seconds_in_loop: f32,
    pub pending_outro: bool,
    pub outro_delay_seconds: f32,
}

impl Default for GeneratedGoblinMusicState {
    fn default() -> Self {
        Self {
            active: false,
            mode: GeneratedMusicMode::Idle,
            current_section: None,
            current_wave_index: None,
            seconds_in_mode: 0.0,
            seconds_in_loop: 0.0,
            pending_outro: false,
            outro_delay_seconds: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeneratedMusicMode {
    Idle,
    Intro,
    Loop,
    Outro,
    Finished,
}

#[derive(SystemParam)]
pub struct GeneratedMusicChannels<'w> {
    strings: Res<'w, AudioChannel<GeneratedMusicStringsChannel>>,
    brass: Res<'w, AudioChannel<GeneratedMusicBrassChannel>>,
    winds: Res<'w, AudioChannel<GeneratedMusicWindsChannel>>,
    choir_pad: Res<'w, AudioChannel<GeneratedMusicChoirPadChannel>>,
    mallets: Res<'w, AudioChannel<GeneratedMusicMalletsChannel>>,
    percussion: Res<'w, AudioChannel<GeneratedMusicPercussionChannel>>,
}

impl<'w> GeneratedMusicChannels<'w> {
    fn channel(&self, stem: GeneratedStem) -> &dyn GeneratedStemChannel {
        match stem {
            GeneratedStem::Strings => &*self.strings,
            GeneratedStem::Brass => &*self.brass,
            GeneratedStem::Winds => &*self.winds,
            GeneratedStem::ChoirPad => &*self.choir_pad,
            GeneratedStem::Mallets => &*self.mallets,
            GeneratedStem::Percussion => &*self.percussion,
        }
    }

    pub fn stop_all(&self, fade_ms: u64) {
        for stem in GeneratedStem::ALL {
            self.channel(stem).stop_with_fade(fade_ms);
        }
    }

    pub fn set_all_silent(&self) {
        for stem in GeneratedStem::ALL {
            self.channel(stem).set_linear_volume(0.0);
        }
    }

    pub fn set_stem_volume(&self, stem: GeneratedStem, linear: f32) {
        self.channel(stem).set_linear_volume(linear);
    }

    pub fn play_stem(
        &self,
        stem: GeneratedStem,
        handle: Handle<KiraAudioSource>,
        looped: bool,
        fade_ms: u64,
    ) {
        self.channel(stem).play_handle(handle, looped, fade_ms);
    }
}

/// Small object-safe adapter so the scheduler can address the six stem channels
/// uniformly without storing Kira's generic channel type in a collection.
trait GeneratedStemChannel {
    fn stop_with_fade(&self, fade_ms: u64);
    fn set_linear_volume(&self, linear: f32);
    fn play_handle(&self, handle: Handle<KiraAudioSource>, looped: bool, fade_ms: u64);
}

macro_rules! impl_generated_stem_channel {
    ($marker:ty) => {
        impl GeneratedStemChannel for AudioChannel<$marker> {
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
                    self.play(handle).looped().fade_in(AudioTween::new(
                        Duration::from_millis(fade_ms),
                        AudioEasing::InPowi(2),
                    ));
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

impl_generated_stem_channel!(GeneratedMusicStringsChannel);
impl_generated_stem_channel!(GeneratedMusicBrassChannel);
impl_generated_stem_channel!(GeneratedMusicWindsChannel);
impl_generated_stem_channel!(GeneratedMusicChoirPadChannel);
impl_generated_stem_channel!(GeneratedMusicMalletsChannel);
impl_generated_stem_channel!(GeneratedMusicPercussionChannel);

pub fn load_generated_goblin_music(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mut stems = HashMap::new();
    for section in [
        GeneratedSection::Intro,
        GeneratedSection::Wave1,
        GeneratedSection::Wave2,
        GeneratedSection::Wave3,
        GeneratedSection::RecapLoop,
        GeneratedSection::Outro,
    ] {
        for stem in GeneratedStem::ALL {
            let rel = format!(
                "{root}/adaptive/{section}/{file_base}.{section}.{stem}.ogg",
                root = GENERATED_GOBLIN_ASSET_ROOT,
                file_base = GENERATED_GOBLIN_FILE_BASE,
                section = section.id(),
                stem = stem.id(),
            );
            stems.insert((section, stem), asset_server.load(rel));
        }
    }

    let intro_full = asset_server.load(format!(
        "{root}/adaptive/intro/{file_base}.intro.full.ogg",
        root = GENERATED_GOBLIN_ASSET_ROOT,
        file_base = GENERATED_GOBLIN_FILE_BASE,
    ));
    let outro_full = asset_server.load(format!(
        "{root}/adaptive/outro/{file_base}.outro.full.ogg",
        root = GENERATED_GOBLIN_ASSET_ROOT,
        file_base = GENERATED_GOBLIN_FILE_BASE,
    ));

    commands.insert_resource(GeneratedGoblinMusicAssets {
        stems,
        intro_full,
        outro_full,
    });
    commands.insert_resource(GeneratedGoblinMusicState::default());
}

const GENERATED_GOBLIN_FILE_BASE: &str = "first_goblin_encounter_orchestra_e59dfc1a4347e366";

pub fn drive_goblin_generated_music(
    time: Res<Time>,
    encounters: Res<EncounterRegistry>,
    assets: Option<Res<GeneratedGoblinMusicAssets>>,
    mut state: Option<ResMut<GeneratedGoblinMusicState>>,
    generated_channels: GeneratedMusicChannels,
    base_music_channel: Res<AudioChannel<MusicChannel>>,
    settings: Res<UserSettings>,
) {
    let Some(assets) = assets else {
        return;
    };
    let Some(mut state) = state else {
        return;
    };
    let dt = time.delta_secs();
    state.seconds_in_mode += dt;
    if state.mode == GeneratedMusicMode::Loop {
        state.seconds_in_loop += dt;
    }

    let Some(encounter) = encounters.get(GENERATED_GOBLIN_ENCOUNTER_ID) else {
        if state.active {
            shutdown_generated_music(&mut state, &generated_channels);
        }
        return;
    };

    match encounter.phase {
        EncounterPhase::Inactive | EncounterPhase::Failed => {
            if state.active {
                shutdown_generated_music(&mut state, &generated_channels);
            }
        }
        EncounterPhase::Starting { .. } => {
            if !state.active || state.mode == GeneratedMusicMode::Idle {
                // The generated cue owns the encounter music. Quiet the legacy
                // generated-lofi channel once, then let the adaptive channels run.
                base_music_channel.stop().fade_out(AudioTween::new(
                    Duration::from_millis(250),
                    AudioEasing::OutPowi(2),
                ));
                start_intro(&mut state, &assets, &generated_channels, &settings);
            }
        }
        EncounterPhase::Active { wave_index, .. } => {
            if !state.active {
                base_music_channel.stop().fade_out(AudioTween::new(
                    Duration::from_millis(250),
                    AudioEasing::OutPowi(2),
                ));
                start_intro(&mut state, &assets, &generated_channels, &settings);
            }
            if state.mode == GeneratedMusicMode::Intro && state.seconds_in_mode >= INTRO_SECONDS {
                let section = section_for_wave(wave_index);
                start_loop_section(&mut state, section, wave_index, &assets, &generated_channels);
            } else if state.mode != GeneratedMusicMode::Intro {
                let desired = section_for_wave(wave_index);
                if state.current_section != Some(desired) {
                    start_loop_section(&mut state, desired, wave_index, &assets, &generated_channels);
                } else {
                    state.current_wave_index = Some(wave_index);
                }
                let brute_reinforced = wave_index == 1 && encounter.run.wave_elapsed >= LARGE_BRUTE_DELAY_SECONDS;
                let target = gains_for_wave(wave_index, brute_reinforced, settings.audio.effective_music());
                apply_stem_gains(&generated_channels, target);
            }
        }
        EncounterPhase::Cleared => {
            if state.active && state.mode != GeneratedMusicMode::Outro && state.mode != GeneratedMusicMode::Finished {
                request_outro(&mut state);
            }
            if state.pending_outro {
                state.outro_delay_seconds -= dt;
                let target = gains_for_cleared_bridge(settings.audio.effective_music());
                apply_stem_gains(&generated_channels, target);
                if state.outro_delay_seconds <= 0.0 {
                    start_outro(&mut state, &assets, &generated_channels, &settings);
                }
            } else if state.mode == GeneratedMusicMode::Outro && state.seconds_in_mode >= OUTRO_SECONDS {
                state.mode = GeneratedMusicMode::Finished;
                state.active = false;
                generated_channels.stop_all(400);
            }
        }
    }
}

fn start_intro(
    state: &mut GeneratedGoblinMusicState,
    assets: &GeneratedGoblinMusicAssets,
    channels: &GeneratedMusicChannels,
    settings: &UserSettings,
) {
    channels.stop_all(160);
    channels.set_all_silent();
    channels.set_stem_volume(GeneratedStem::Strings, settings.audio.effective_music());
    channels.play_stem(GeneratedStem::Strings, assets.intro_full.clone(), false, 220);
    state.active = true;
    state.mode = GeneratedMusicMode::Intro;
    state.current_section = Some(GeneratedSection::Intro);
    state.current_wave_index = None;
    state.seconds_in_mode = 0.0;
    state.seconds_in_loop = 0.0;
    state.pending_outro = false;
    state.outro_delay_seconds = 0.0;
}

fn start_loop_section(
    state: &mut GeneratedGoblinMusicState,
    section: GeneratedSection,
    wave_index: usize,
    assets: &GeneratedGoblinMusicAssets,
    channels: &GeneratedMusicChannels,
) {
    channels.stop_all(420);
    channels.set_all_silent();
    for stem in GeneratedStem::ALL {
        if let Some(handle) = assets.stem(section, stem) {
            channels.play_stem(stem, handle, true, 520);
        }
    }
    state.mode = GeneratedMusicMode::Loop;
    state.current_section = Some(section);
    state.current_wave_index = Some(wave_index);
    state.seconds_in_mode = 0.0;
    state.seconds_in_loop = 0.0;
}

fn request_outro(state: &mut GeneratedGoblinMusicState) {
    state.pending_outro = true;
    state.outro_delay_seconds = seconds_until_next_exit_marker(state.seconds_in_loop);
}

fn start_outro(
    state: &mut GeneratedGoblinMusicState,
    assets: &GeneratedGoblinMusicAssets,
    channels: &GeneratedMusicChannels,
    settings: &UserSettings,
) {
    channels.stop_all(650);
    channels.set_all_silent();
    channels.set_stem_volume(GeneratedStem::Strings, settings.audio.effective_music());
    channels.play_stem(GeneratedStem::Strings, assets.outro_full.clone(), false, 400);
    state.mode = GeneratedMusicMode::Outro;
    state.current_section = Some(GeneratedSection::Outro);
    state.seconds_in_mode = 0.0;
    state.pending_outro = false;
    state.outro_delay_seconds = 0.0;
}

fn shutdown_generated_music(state: &mut GeneratedGoblinMusicState, channels: &GeneratedMusicChannels) {
    channels.stop_all(400);
    *state = GeneratedGoblinMusicState::default();
}

fn section_for_wave(wave_index: usize) -> GeneratedSection {
    match wave_index {
        0 => GeneratedSection::Wave1,
        1 => GeneratedSection::Wave2,
        2 => GeneratedSection::Wave3,
        _ => GeneratedSection::RecapLoop,
    }
}

fn seconds_until_next_exit_marker(seconds_in_loop: f32) -> f32 {
    // v3 manifest's main loop sections are 16 bars and allow exits at half or
    // full phrase boundaries. Use the next 8-bar boundary so a clear never cuts
    // the loop in the middle of a phrase.
    let seconds_per_bar = (60.0 / BPM) * BEATS_PER_BAR;
    let phrase_seconds = seconds_per_bar * 8.0;
    let local = seconds_in_loop.rem_euclid(phrase_seconds);
    let remaining = phrase_seconds - local;
    remaining.clamp(0.15, phrase_seconds)
}

fn gains_for_wave(wave_index: usize, brute_reinforced: bool, master: f32) -> [(GeneratedStem, f32); 6] {
    let gains = match wave_index {
        0 => [
            (GeneratedStem::Strings, 1.00),
            (GeneratedStem::Winds, 0.65),
            (GeneratedStem::Mallets, 0.55),
            (GeneratedStem::Percussion, 0.35),
            (GeneratedStem::Brass, 0.15),
            (GeneratedStem::ChoirPad, 0.00),
        ],
        1 if brute_reinforced => [
            (GeneratedStem::Strings, 1.00),
            (GeneratedStem::Winds, 0.85),
            (GeneratedStem::Mallets, 0.75),
            (GeneratedStem::Percussion, 0.85),
            (GeneratedStem::Brass, 0.95),
            (GeneratedStem::ChoirPad, 0.55),
        ],
        1 => [
            (GeneratedStem::Strings, 1.00),
            (GeneratedStem::Winds, 0.85),
            (GeneratedStem::Mallets, 0.75),
            (GeneratedStem::Percussion, 0.70),
            (GeneratedStem::Brass, 0.45),
            (GeneratedStem::ChoirPad, 0.20),
        ],
        _ => [
            (GeneratedStem::Strings, 1.00),
            (GeneratedStem::Winds, 0.55),
            (GeneratedStem::Mallets, 0.45),
            (GeneratedStem::Percussion, 0.95),
            (GeneratedStem::Brass, 1.00),
            (GeneratedStem::ChoirPad, 0.75),
        ],
    };
    gains.map(|(stem, gain)| (stem, gain * master))
}

fn gains_for_cleared_bridge(master: f32) -> [(GeneratedStem, f32); 6] {
    [
        (GeneratedStem::Strings, 0.80 * master),
        (GeneratedStem::Winds, 0.50 * master),
        (GeneratedStem::Mallets, 0.10 * master),
        (GeneratedStem::Percussion, 0.00),
        (GeneratedStem::Brass, 0.00),
        (GeneratedStem::ChoirPad, 0.20 * master),
    ]
}

fn apply_stem_gains(channels: &GeneratedMusicChannels, gains: [(GeneratedStem, f32); 6]) {
    for (stem, gain) in gains {
        channels.set_stem_volume(stem, gain.clamp(0.0, 1.0));
    }
}
