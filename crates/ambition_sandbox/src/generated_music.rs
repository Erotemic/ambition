//! Runtime integration for generated/adaptive music assets.
//!
//! Bevy/Kira only loads already-rendered OGG stems produced by
//! `tools/audio/music_renderer` and schedules intro -> loop sections -> outro
//! while fading stems by battle state. The renderer's hash-suffixed output
//! files are renamed to stable section-prefixed paths by the installer
//! (`tools/audio/install_first_goblin_tune_v2_assets.py`), so re-rendering the
//! cue does not require Rust changes.
//!
//! Stems use two channel banks so a new section can fade in while the old one
//! fades out. Section switches are deferred to musical boundaries. Clearing
//! the encounter fades combat stems into a quiet bridge, plays the outro, then
//! resumes the default room music channel.
//!
//! Use `RUST_LOG=ambition_generated_music=debug,info` to trace section
//! transitions, stem starts, and bar/beat timing.

#![cfg(feature = "audio")]

use std::collections::HashMap;
use std::time::Duration;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::log::{debug, info, warn};
use bevy_kira_audio::prelude::{
    AudioChannel, AudioControl, AudioEasing, AudioSource as KiraAudioSource, AudioTween,
};

use crate::audio::{
    amplitude_to_decibels, switch_to_music_track, AudioLibrary, MusicChannel, MusicPlaybackState,
};
use crate::data::SandboxDataSpec;
use crate::encounter::{EncounterPhase, EncounterRegistry};
use crate::settings::UserSettings;

pub const GENERATED_GOBLIN_ENCOUNTER_ID: &str = "mob_lab";
pub const GENERATED_GOBLIN_CUE_ID: &str = "first_goblin_tune_v2";
pub const GENERATED_GOBLIN_ASSET_ROOT: &str =
    "audio/music/generated/first_goblin_tune_v2";

const BPM: f32 = 132.0;
const BEATS_PER_BAR: f32 = 4.0;
const INTRO_SECONDS: f32 = 7.272727;
const LOOP_SECONDS: f32 = 14.545455;
const OUTRO_SECONDS: f32 = 7.272727;
const LARGE_BRUTE_DELAY_SECONDS: f32 = 3.5;
/// Time constant for target stem gain changes.
///
/// This is intentionally applied as a proportional ramp rather than an
/// absolute per-second delta. The older absolute slew made quiet stems reach
/// their final level in a few frames, which sounded like a hard pop even
/// though the logged gain was small. With this ramp, every stem change has a
/// comparable perceptual fade shape.
const STEM_GAIN_BLEND_SECONDS: f32 = 1.05;
const LOOP_SECTION_CROSSFADE_SECONDS: f32 = 1.70;
const INTRO_TO_LOOP_CROSSFADE_SECONDS: f32 = 1.25;
const OUTRO_CROSSFADE_SECONDS: f32 = 1.65;
const DEFAULT_RETURN_OVERLAP_SECONDS: f32 = 1.35;
const MIN_TRANSITION_DELAY_SECONDS: f32 = 0.08;
/// Relative volume for the generated adaptive cue after user music volume.
///
/// The generated cue is assembled from several simultaneous OGG stems, so a
/// per-stem gain that looks reasonable can sum much louder than the legacy
/// single-channel procedural room music. Keep this conservative; raise it in
/// small steps once the mix is balanced.
const GENERATED_MUSIC_RELATIVE_VOLUME: f32 = 0.85;
const STEM_START_FADE_MS: u64 = 0;
const INTRO_FULL_START_FADE_MS: u64 = 0;
const OUTRO_FULL_START_FADE_MS: u64 = 0;
const DEBUG_LOG_PERIOD_SECONDS: f32 = 1.0;

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

// Second bank for real crossfades. The original six channels remain bank A so
// earlier overlay code / committed app wiring keeps working; these six are bank B.
#[derive(Resource)]
pub struct GeneratedMusicStringsAltChannel;
#[derive(Resource)]
pub struct GeneratedMusicBrassAltChannel;
#[derive(Resource)]
pub struct GeneratedMusicWindsAltChannel;
#[derive(Resource)]
pub struct GeneratedMusicChoirPadAltChannel;
#[derive(Resource)]
pub struct GeneratedMusicMalletsAltChannel;
#[derive(Resource)]
pub struct GeneratedMusicPercussionAltChannel;

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

    fn index(self) -> usize {
        match self {
            Self::Strings => 0,
            Self::Brass => 1,
            Self::Winds => 2,
            Self::ChoirPad => 3,
            Self::Mallets => 4,
            Self::Percussion => 5,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StemBank {
    A,
    B,
}

impl StemBank {
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
}

type StemGains = [f32; 6];

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
    active_bank: StemBank,
    fading_bank: Option<StemBank>,
    fade_stop_seconds: f32,
    current_gains: [StemGains; 2],
    target_gains: [StemGains; 2],
    pending_loop: Option<PendingLoopTransition>,
    default_resume_started: bool,
    debug_log_timer: f32,
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
            active_bank: StemBank::A,
            fading_bank: None,
            fade_stop_seconds: 0.0,
            current_gains: [[0.0; 6]; 2],
            target_gains: [[0.0; 6]; 2],
            pending_loop: None,
            default_resume_started: false,
            debug_log_timer: 0.0,
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

#[derive(Debug, Clone, Copy)]
struct PendingLoopTransition {
    section: GeneratedSection,
    wave_index: usize,
    delay_seconds: f32,
}

#[derive(SystemParam)]
pub struct GeneratedMusicChannels<'w> {
    strings_a: Res<'w, AudioChannel<GeneratedMusicStringsChannel>>,
    brass_a: Res<'w, AudioChannel<GeneratedMusicBrassChannel>>,
    winds_a: Res<'w, AudioChannel<GeneratedMusicWindsChannel>>,
    choir_pad_a: Res<'w, AudioChannel<GeneratedMusicChoirPadChannel>>,
    mallets_a: Res<'w, AudioChannel<GeneratedMusicMalletsChannel>>,
    percussion_a: Res<'w, AudioChannel<GeneratedMusicPercussionChannel>>,
    strings_b: Res<'w, AudioChannel<GeneratedMusicStringsAltChannel>>,
    brass_b: Res<'w, AudioChannel<GeneratedMusicBrassAltChannel>>,
    winds_b: Res<'w, AudioChannel<GeneratedMusicWindsAltChannel>>,
    choir_pad_b: Res<'w, AudioChannel<GeneratedMusicChoirPadAltChannel>>,
    mallets_b: Res<'w, AudioChannel<GeneratedMusicMalletsAltChannel>>,
    percussion_b: Res<'w, AudioChannel<GeneratedMusicPercussionAltChannel>>,
}

impl<'w> GeneratedMusicChannels<'w> {
    fn channel(&self, bank: StemBank, stem: GeneratedStem) -> &dyn GeneratedStemChannel {
        match (bank, stem) {
            (StemBank::A, GeneratedStem::Strings) => &*self.strings_a,
            (StemBank::A, GeneratedStem::Brass) => &*self.brass_a,
            (StemBank::A, GeneratedStem::Winds) => &*self.winds_a,
            (StemBank::A, GeneratedStem::ChoirPad) => &*self.choir_pad_a,
            (StemBank::A, GeneratedStem::Mallets) => &*self.mallets_a,
            (StemBank::A, GeneratedStem::Percussion) => &*self.percussion_a,
            (StemBank::B, GeneratedStem::Strings) => &*self.strings_b,
            (StemBank::B, GeneratedStem::Brass) => &*self.brass_b,
            (StemBank::B, GeneratedStem::Winds) => &*self.winds_b,
            (StemBank::B, GeneratedStem::ChoirPad) => &*self.choir_pad_b,
            (StemBank::B, GeneratedStem::Mallets) => &*self.mallets_b,
            (StemBank::B, GeneratedStem::Percussion) => &*self.percussion_b,
        }
    }

    pub fn stop_all(&self, fade_ms: u64) {
        self.stop_bank(StemBank::A, fade_ms);
        self.stop_bank(StemBank::B, fade_ms);
    }

    fn stop_bank(&self, bank: StemBank, fade_ms: u64) {
        for stem in GeneratedStem::ALL {
            self.channel(bank, stem).stop_with_fade(fade_ms);
        }
    }

    fn set_bank_silent(&self, bank: StemBank) {
        for stem in GeneratedStem::ALL {
            self.channel(bank, stem).set_linear_volume(0.0);
        }
    }

    fn set_stem_volume(&self, bank: StemBank, stem: GeneratedStem, linear: f32) {
        self.channel(bank, stem).set_linear_volume(linear);
    }

    fn play_stem(
        &self,
        bank: StemBank,
        stem: GeneratedStem,
        handle: Handle<KiraAudioSource>,
        looped: bool,
        fade_ms: u64,
    ) {
        self.channel(bank, stem).play_handle(handle, looped, fade_ms);
    }
}

/// Small object-safe adapter so the scheduler can address stem channels
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

impl_generated_stem_channel!(GeneratedMusicStringsChannel);
impl_generated_stem_channel!(GeneratedMusicBrassChannel);
impl_generated_stem_channel!(GeneratedMusicWindsChannel);
impl_generated_stem_channel!(GeneratedMusicChoirPadChannel);
impl_generated_stem_channel!(GeneratedMusicMalletsChannel);
impl_generated_stem_channel!(GeneratedMusicPercussionChannel);
impl_generated_stem_channel!(GeneratedMusicStringsAltChannel);
impl_generated_stem_channel!(GeneratedMusicBrassAltChannel);
impl_generated_stem_channel!(GeneratedMusicWindsAltChannel);
impl_generated_stem_channel!(GeneratedMusicChoirPadAltChannel);
impl_generated_stem_channel!(GeneratedMusicMalletsAltChannel);
impl_generated_stem_channel!(GeneratedMusicPercussionAltChannel);

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
                "{root}/adaptive/{section}/{section}.{stem}.ogg",
                root = GENERATED_GOBLIN_ASSET_ROOT,
                section = section.id(),
                stem = stem.id(),
            );
            stems.insert((section, stem), asset_server.load(rel));
        }
    }

    let intro_full = asset_server.load(format!(
        "{root}/adaptive/intro/intro.full.ogg",
        root = GENERATED_GOBLIN_ASSET_ROOT,
    ));
    let outro_full = asset_server.load(format!(
        "{root}/adaptive/outro/outro.full.ogg",
        root = GENERATED_GOBLIN_ASSET_ROOT,
    ));

    info!(
        target: "ambition_generated_music",
        "loaded generated goblin cue cue={} root={} stem_assets={}",
        GENERATED_GOBLIN_CUE_ID,
        GENERATED_GOBLIN_ASSET_ROOT,
        stems.len(),
    );

    commands.insert_resource(GeneratedGoblinMusicAssets {
        stems,
        intro_full,
        outro_full,
    });
    commands.insert_resource(GeneratedGoblinMusicState::default());
}

pub fn drive_goblin_generated_music(
    time: Res<Time>,
    encounters: Res<EncounterRegistry>,
    assets: Option<Res<GeneratedGoblinMusicAssets>>,
    state: Option<ResMut<GeneratedGoblinMusicState>>,
    generated_channels: GeneratedMusicChannels,
    base_music_channel: Res<AudioChannel<MusicChannel>>,
    library: Res<AudioLibrary>,
    mut music_state: ResMut<MusicPlaybackState>,
    sandbox_data: Res<SandboxDataSpec>,
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
            shutdown_generated_music(
                &mut state,
                &generated_channels,
                &library,
                &mut music_state,
                &base_music_channel,
                &sandbox_data,
            );
        }
        return;
    };

    match encounter.phase {
        EncounterPhase::Inactive => {
            // The encounter may reset from Cleared -> Inactive before the music
            // outro has had a chance to play. Treat Inactive as a post-clear
            // continuation when generated music is already active; Failed still
            // shuts down immediately below.
            if state.active {
                drive_clear_or_outro(
                    &mut state,
                    dt,
                    &assets,
                    &generated_channels,
                    &library,
                    &mut music_state,
                    &base_music_channel,
                    &sandbox_data,
                    &settings,
                );
            }
        }
        EncounterPhase::Failed => {
            if state.active {
                shutdown_generated_music(
                    &mut state,
                    &generated_channels,
                    &library,
                    &mut music_state,
                    &base_music_channel,
                    &sandbox_data,
                );
            }
        }
        EncounterPhase::Starting { .. } => {
            if !state.active || state.mode == GeneratedMusicMode::Idle {
                // The generated cue owns the encounter music. Quiet the legacy
                // generated-lofi channel once, then let the adaptive banks run.
                base_music_channel.stop().fade_out(AudioTween::new(
                    Duration::from_millis(650),
                    AudioEasing::OutPowi(2),
                ));
                start_intro(&mut state, &assets, &generated_channels, &settings);
            }
        }
        EncounterPhase::Active { wave_index, .. } => {
            if !state.active {
                base_music_channel.stop().fade_out(AudioTween::new(
                    Duration::from_millis(650),
                    AudioEasing::OutPowi(2),
                ));
                start_intro(&mut state, &assets, &generated_channels, &settings);
            }

            if state.mode == GeneratedMusicMode::Intro && state.seconds_in_mode >= INTRO_SECONDS {
                let section = section_for_wave(wave_index);
                begin_loop_crossfade(
                    &mut state,
                    section,
                    wave_index,
                    &assets,
                    &generated_channels,
                    gains_for_wave(
                        wave_index,
                        wave_index == 1 && encounter.run.wave_elapsed >= LARGE_BRUTE_DELAY_SECONDS,
                        settings.audio.effective_music(),
                    ),
                    INTRO_TO_LOOP_CROSSFADE_SECONDS,
                );
            } else if state.mode == GeneratedMusicMode::Loop {
                let desired = section_for_wave(wave_index);
                if state.current_section != Some(desired) && state.pending_loop.is_none() {
                    // Do not hard-cut at the instant a wave changes. Arm the
                    // next section for the next bar so the seam is musical.
                    let delay_seconds = seconds_until_next_bar(state.seconds_in_loop)
                        .max(MIN_TRANSITION_DELAY_SECONDS);
                    info!(
                        target: "ambition_generated_music",
                        "queue_loop_transition section={} wave={} current_section={:?} loop_t={:.3} bar_beat={} delay={:.3}s",
                        desired.id(),
                        wave_index + 1,
                        state.current_section,
                        state.seconds_in_loop,
                        format_bar_beat(state.seconds_in_loop),
                        delay_seconds,
                    );
                    state.pending_loop = Some(PendingLoopTransition {
                        section: desired,
                        wave_index,
                        delay_seconds,
                    });
                }

                if let Some(mut pending) = state.pending_loop {
                    // If the encounter advanced again before the queued seam,
                    // update the pending target rather than playing stale music.
                    pending.section = desired;
                    pending.wave_index = wave_index;
                    pending.delay_seconds -= dt;
                    if pending.delay_seconds <= 0.0 {
                        info!(
                            target: "ambition_generated_music",
                            "fire_loop_transition section={} wave={} loop_t={:.3} bar_beat={}",
                            pending.section.id(),
                            pending.wave_index + 1,
                            state.seconds_in_loop,
                            format_bar_beat(state.seconds_in_loop),
                        );
                        state.pending_loop = None;
                        begin_loop_crossfade(
                            &mut state,
                            pending.section,
                            pending.wave_index,
                            &assets,
                            &generated_channels,
                            gains_for_wave(
                                wave_index,
                                wave_index == 1
                                    && encounter.run.wave_elapsed >= LARGE_BRUTE_DELAY_SECONDS,
                                settings.audio.effective_music(),
                            ),
                            LOOP_SECTION_CROSSFADE_SECONDS,
                        );
                    } else {
                        state.pending_loop = Some(pending);
                    }
                } else {
                    state.current_wave_index = Some(wave_index);
                }

                let brute_reinforced =
                    wave_index == 1 && encounter.run.wave_elapsed >= LARGE_BRUTE_DELAY_SECONDS;
                let active_bank = state.active_bank;
                set_bank_targets(
                    &mut state,
                    active_bank,
                    gains_for_wave(wave_index, brute_reinforced, settings.audio.effective_music()),
                );
            }
        }
        EncounterPhase::Cleared => {
            drive_clear_or_outro(
                &mut state,
                dt,
                &assets,
                &generated_channels,
                &library,
                &mut music_state,
                &base_music_channel,
                &sandbox_data,
                &settings,
            );
        }
    }

    update_gain_smoothing(&mut state, &generated_channels, dt);
    log_periodic_state(&mut state, encounter.phase, encounter.run.wave_elapsed, dt);
}


fn drive_clear_or_outro(
    state: &mut GeneratedGoblinMusicState,
    dt: f32,
    assets: &GeneratedGoblinMusicAssets,
    channels: &GeneratedMusicChannels,
    library: &AudioLibrary,
    music_state: &mut MusicPlaybackState,
    base_music_channel: &AudioChannel<MusicChannel>,
    sandbox_data: &SandboxDataSpec,
    settings: &UserSettings,
) {
    if !state.active {
        return;
    }
    if state.mode != GeneratedMusicMode::Outro
        && state.mode != GeneratedMusicMode::Finished
        && !state.pending_outro
    {
        request_outro(state);
    }
    if state.pending_outro {
        state.outro_delay_seconds -= dt;
        let active_bank = state.active_bank;
        set_bank_targets(
            state,
            active_bank,
            gains_for_cleared_bridge(settings.audio.effective_music()),
        );
        if state.outro_delay_seconds <= 0.0 {
            start_outro(state, assets, channels, settings);
        }
    } else if state.mode == GeneratedMusicMode::Outro {
        if !state.default_resume_started
            && state.seconds_in_mode >= (OUTRO_SECONDS - DEFAULT_RETURN_OVERLAP_SECONDS).max(0.0)
        {
            resume_default_music(library, music_state, base_music_channel, sandbox_data);
            state.default_resume_started = true;
        }
        if state.seconds_in_mode >= OUTRO_SECONDS {
            info!(
                target: "ambition_generated_music",
                "finish_generated_outro t_outro={:.3}",
                state.seconds_in_mode,
            );
            state.mode = GeneratedMusicMode::Finished;
            state.active = false;
            channels.stop_all(900);
            zero_all_targets(state);
        }
    }
}

fn start_intro(
    state: &mut GeneratedGoblinMusicState,
    assets: &GeneratedGoblinMusicAssets,
    channels: &GeneratedMusicChannels,
    settings: &UserSettings,
) {
    let generated_master = effective_generated_master(settings);
    info!(
        target: "ambition_generated_music",
        "start_intro bank=A intro_seconds={:.3} user_music={:.3} generated_master={:.3} relative={:.2}",
        INTRO_SECONDS,
        settings.audio.effective_music(),
        generated_master,
        GENERATED_MUSIC_RELATIVE_VOLUME,
    );
    channels.stop_all(420);
    channels.set_bank_silent(StemBank::A);
    channels.set_bank_silent(StemBank::B);
    zero_all_current_and_targets(state);
    state.active_bank = StemBank::A;
    channels.play_stem(
        StemBank::A,
        GeneratedStem::Strings,
        assets.intro_full.clone(),
        false,
        INTRO_FULL_START_FADE_MS,
    );
    let mut gains = [0.0; 6];
    gains[GeneratedStem::Strings.index()] = generated_master;
    set_bank_targets(state, StemBank::A, gains);
    state.active = true;
    state.mode = GeneratedMusicMode::Intro;
    state.current_section = Some(GeneratedSection::Intro);
    state.current_wave_index = None;
    state.seconds_in_mode = 0.0;
    state.seconds_in_loop = 0.0;
    state.pending_outro = false;
    state.outro_delay_seconds = 0.0;
    state.pending_loop = None;
    state.fading_bank = None;
    state.fade_stop_seconds = 0.0;
    state.default_resume_started = false;
    state.debug_log_timer = 0.0;
}

fn begin_loop_crossfade(
    state: &mut GeneratedGoblinMusicState,
    section: GeneratedSection,
    wave_index: usize,
    assets: &GeneratedGoblinMusicAssets,
    channels: &GeneratedMusicChannels,
    target_gains: StemGains,
    crossfade_seconds: f32,
) {
    let old_bank = state.active_bank;
    let new_bank = old_bank.other();
    info!(
        target: "ambition_generated_music",
        "begin_loop_crossfade section={} wave={} old_bank={} new_bank={} loop_t={:.3} bar_beat={} crossfade={:.2}s gains={}",
        section.id(),
        wave_index + 1,
        old_bank.label(),
        new_bank.label(),
        state.seconds_in_loop,
        format_bar_beat(state.seconds_in_loop),
        crossfade_seconds,
        format_gains(target_gains),
    );
    channels.stop_bank(new_bank, 80);
    channels.set_bank_silent(new_bank);
    state.current_gains[new_bank.index()] = [0.0; 6];
    state.target_gains[new_bank.index()] = [0.0; 6];

    let mut started = 0usize;
    for stem in GeneratedStem::ALL {
        if let Some(handle) = assets.stem(section, stem) {
            channels.play_stem(new_bank, stem, handle, true, STEM_START_FADE_MS);
            started += 1;
        } else {
            warn!(
                target: "ambition_generated_music",
                "missing generated stem asset section={} stem={}",
                section.id(),
                stem.id(),
            );
        }
    }
    info!(
        target: "ambition_generated_music",
        "started_section_stems section={} bank={} stem_count={} fade_ms={} volume_blend={:.2}s note=all_stems_started_silent_then_volume_ramped",
        section.id(),
        new_bank.label(),
        started,
        STEM_START_FADE_MS,
        STEM_GAIN_BLEND_SECONDS,
    );

    set_bank_targets(state, old_bank, [0.0; 6]);
    set_bank_targets(state, new_bank, target_gains);
    state.active_bank = new_bank;
    state.fading_bank = Some(old_bank);
    state.fade_stop_seconds = crossfade_seconds + 0.35;
    state.mode = GeneratedMusicMode::Loop;
    state.current_section = Some(section);
    state.current_wave_index = Some(wave_index);
    state.seconds_in_mode = 0.0;
    state.seconds_in_loop = 0.0;
    state.pending_outro = false;
    state.outro_delay_seconds = 0.0;
    state.default_resume_started = false;
}

fn request_outro(state: &mut GeneratedGoblinMusicState) {
    state.pending_outro = true;
    // Use 4-bar exits for the first pass: less waiting after a clear, but
    // still lands on a phrase boundary. The stem bridge fades during the wait.
    state.outro_delay_seconds = seconds_until_next_phrase_marker(state.seconds_in_loop, 2.0)
        .max(MIN_TRANSITION_DELAY_SECONDS);
    state.pending_loop = None;
    info!(
        target: "ambition_generated_music",
        "request_outro loop_t={:.3} bar_beat={} delay={:.3}s active_bank={}",
        state.seconds_in_loop,
        format_bar_beat(state.seconds_in_loop),
        state.outro_delay_seconds,
        state.active_bank.label(),
    );
}

fn start_outro(
    state: &mut GeneratedGoblinMusicState,
    assets: &GeneratedGoblinMusicAssets,
    channels: &GeneratedMusicChannels,
    settings: &UserSettings,
) {
    let old_bank = state.active_bank;
    let new_bank = old_bank.other();
    let generated_master = effective_generated_master(settings);
    info!(
        target: "ambition_generated_music",
        "start_outro old_bank={} new_bank={} loop_t={:.3} bar_beat={} outro_seconds={:.3}",
        old_bank.label(),
        new_bank.label(),
        state.seconds_in_loop,
        format_bar_beat(state.seconds_in_loop),
        OUTRO_SECONDS,
    );
    channels.stop_bank(new_bank, 80);
    channels.set_bank_silent(new_bank);
    state.current_gains[new_bank.index()] = [0.0; 6];
    state.target_gains[new_bank.index()] = [0.0; 6];

    channels.play_stem(
        new_bank,
        GeneratedStem::Strings,
        assets.outro_full.clone(),
        false,
        OUTRO_FULL_START_FADE_MS,
    );
    set_bank_targets(state, old_bank, [0.0; 6]);
    let mut gains = [0.0; 6];
    gains[GeneratedStem::Strings.index()] = generated_master;
    set_bank_targets(state, new_bank, gains);

    state.active_bank = new_bank;
    state.fading_bank = Some(old_bank);
    state.fade_stop_seconds = OUTRO_CROSSFADE_SECONDS + 0.35;
    state.mode = GeneratedMusicMode::Outro;
    state.current_section = Some(GeneratedSection::Outro);
    state.seconds_in_mode = 0.0;
    state.pending_outro = false;
    state.outro_delay_seconds = 0.0;
    state.default_resume_started = false;
}

fn shutdown_generated_music(
    state: &mut GeneratedGoblinMusicState,
    channels: &GeneratedMusicChannels,
    library: &AudioLibrary,
    music_state: &mut MusicPlaybackState,
    base_music_channel: &AudioChannel<MusicChannel>,
    sandbox_data: &SandboxDataSpec,
) {
    info!(
        target: "ambition_generated_music",
        "shutdown_generated_music mode={:?} section={:?} active_bank={} loop_t={:.3}",
        state.mode,
        state.current_section,
        state.active_bank.label(),
        state.seconds_in_loop,
    );
    channels.stop_all(650);
    resume_default_music(library, music_state, base_music_channel, sandbox_data);
    *state = GeneratedGoblinMusicState::default();
}

fn resume_default_music(
    library: &AudioLibrary,
    music_state: &mut MusicPlaybackState,
    base_music_channel: &AudioChannel<MusicChannel>,
    sandbox_data: &SandboxDataSpec,
) {
    let target = sandbox_data.audio.default_music_track.as_str();
    if library.track(target).is_some() {
        info!(
            target: "ambition_generated_music",
            "resume_default_music target={}",
            target,
        );
        switch_to_music_track(library, music_state, base_music_channel, target);
    } else {
        warn!(
            target: "ambition_generated_music",
            "cannot resume missing default music target={}",
            target,
        );
    }
}


fn log_periodic_state(
    state: &mut GeneratedGoblinMusicState,
    phase: EncounterPhase,
    wave_elapsed: f32,
    dt: f32,
) {
    if !state.active {
        return;
    }
    state.debug_log_timer -= dt;
    if state.debug_log_timer > 0.0 {
        return;
    }
    state.debug_log_timer = DEBUG_LOG_PERIOD_SECONDS;
    debug!(
        target: "ambition_generated_music",
        "state mode={:?} phase={} section={:?} wave={:?} active_bank={} fading_bank={:?} t_mode={:.3} t_loop={:.3} bar_beat={} wave_elapsed={:.3} pending_loop={:?} pending_outro={} outro_delay={:.3} gains_a={} gains_b={}",
        state.mode,
        phase.label(),
        state.current_section,
        state.current_wave_index.map(|w| w + 1),
        state.active_bank.label(),
        state.fading_bank.map(|b| b.label()),
        state.seconds_in_mode,
        state.seconds_in_loop,
        format_bar_beat(state.seconds_in_loop),
        wave_elapsed,
        state.pending_loop.map(|p| (p.section.id(), p.wave_index + 1, p.delay_seconds)),
        state.pending_outro,
        state.outro_delay_seconds,
        format_gains(state.current_gains[StemBank::A.index()]),
        format_gains(state.current_gains[StemBank::B.index()]),
    );
}

impl StemBank {
    fn label(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
        }
    }
}

fn format_bar_beat(seconds_in_loop: f32) -> String {
    let seconds_per_beat = 60.0 / BPM;
    let beat_total = seconds_in_loop / seconds_per_beat;
    let bar = (beat_total / BEATS_PER_BAR).floor() as i32 + 1;
    let beat = beat_total.rem_euclid(BEATS_PER_BAR) + 1.0;
    format!("{}:{:.2}", bar, beat)
}

fn format_gains(gains: StemGains) -> String {
    let mut out = String::new();
    for stem in GeneratedStem::ALL {
        if !out.is_empty() {
            out.push(' ');
        }
        let gain = gains[stem.index()];
        out.push_str(stem.id());
        out.push('=');
        out.push_str(&format!("{:.2}", gain));
    }
    out
}

fn section_for_wave(wave_index: usize) -> GeneratedSection {
    match wave_index {
        0 => GeneratedSection::Wave1,
        1 => GeneratedSection::Wave2,
        2 => GeneratedSection::Wave3,
        _ => GeneratedSection::RecapLoop,
    }
}

fn seconds_until_next_bar(seconds_in_loop: f32) -> f32 {
    let seconds_per_bar = (60.0 / BPM) * BEATS_PER_BAR;
    let local = seconds_in_loop.rem_euclid(seconds_per_bar);
    let remaining = seconds_per_bar - local;
    remaining.clamp(MIN_TRANSITION_DELAY_SECONDS, seconds_per_bar)
}

fn seconds_until_next_phrase_marker(seconds_in_loop: f32, bars_per_phrase: f32) -> f32 {
    let seconds_per_bar = (60.0 / BPM) * BEATS_PER_BAR;
    let phrase_seconds = seconds_per_bar * bars_per_phrase.max(1.0);
    let local = seconds_in_loop.rem_euclid(phrase_seconds);
    let remaining = phrase_seconds - local;
    remaining.clamp(MIN_TRANSITION_DELAY_SECONDS, phrase_seconds)
}


fn effective_generated_master(settings: &UserSettings) -> f32 {
    (settings.audio.effective_music() * GENERATED_MUSIC_RELATIVE_VOLUME).clamp(0.0, 1.0)
}


fn gains_for_wave(wave_index: usize, brute_reinforced: bool, master: f32) -> StemGains {
    // first_goblin_tune_v2 clean demo mix:
    // wave 1 is lean and readable; wave 2 is the obvious musical lift;
    // the brute reinforcement strengthens brass/percussion without adding
    // noisy hats; wave 3 is heavier but still deliberately simple.
    let pairs = match wave_index {
        0 => [
            (GeneratedStem::Strings, 0.92),
            (GeneratedStem::Winds, 0.68),
            (GeneratedStem::Mallets, 0.14),
            (GeneratedStem::Percussion, 0.06),
            (GeneratedStem::Brass, 0.00),
            (GeneratedStem::ChoirPad, 0.00),
        ],
        1 if brute_reinforced => [
            (GeneratedStem::Strings, 1.00),
            (GeneratedStem::Winds, 0.82),
            (GeneratedStem::Mallets, 0.12),
            (GeneratedStem::Percussion, 0.42),
            (GeneratedStem::Brass, 0.54),
            (GeneratedStem::ChoirPad, 0.10),
        ],
        1 => [
            (GeneratedStem::Strings, 1.00),
            (GeneratedStem::Winds, 0.82),
            (GeneratedStem::Mallets, 0.12),
            (GeneratedStem::Percussion, 0.30),
            (GeneratedStem::Brass, 0.36),
            (GeneratedStem::ChoirPad, 0.04),
        ],
        _ => [
            (GeneratedStem::Strings, 1.00),
            (GeneratedStem::Winds, 0.56),
            (GeneratedStem::Mallets, 0.08),
            (GeneratedStem::Percussion, 0.44),
            (GeneratedStem::Brass, 0.60),
            (GeneratedStem::ChoirPad, 0.14),
        ],
    };
    pairs_to_gains(pairs, master)
}


fn gains_for_cleared_bridge(master: f32) -> StemGains {
    pairs_to_gains(
        [
            (GeneratedStem::Strings, 0.50),
            (GeneratedStem::Winds, 0.22),
            (GeneratedStem::Mallets, 0.00),
            (GeneratedStem::Percussion, 0.00),
            (GeneratedStem::Brass, 0.00),
            (GeneratedStem::ChoirPad, 0.05),
        ],
        master,
    )
}

fn pairs_to_gains(pairs: [(GeneratedStem, f32); 6], master: f32) -> StemGains {
    let scaled_master = (master * GENERATED_MUSIC_RELATIVE_VOLUME).clamp(0.0, 1.0);
    let mut gains = [0.0; 6];
    for (stem, gain) in pairs {
        gains[stem.index()] = (gain * scaled_master).clamp(0.0, 1.0);
    }
    gains
}

fn set_bank_targets(state: &mut GeneratedGoblinMusicState, bank: StemBank, gains: StemGains) {
    state.target_gains[bank.index()] = gains;
}

fn zero_all_targets(state: &mut GeneratedGoblinMusicState) {
    state.target_gains = [[0.0; 6]; 2];
}

fn zero_all_current_and_targets(state: &mut GeneratedGoblinMusicState) {
    state.current_gains = [[0.0; 6]; 2];
    state.target_gains = [[0.0; 6]; 2];
}

fn update_gain_smoothing(
    state: &mut GeneratedGoblinMusicState,
    channels: &GeneratedMusicChannels,
    dt: f32,
) {
    // Perceptual stem fade: move by a fraction of the remaining distance.
    // This avoids the old absolute slew where a quiet stem with target 0.08
    // reached full volume in roughly 0.1 seconds.  A one-second time constant
    // gives new layers an audible soft entrance while keeping combat state
    // changes responsive.
    let blend = if STEM_GAIN_BLEND_SECONDS <= 0.0 {
        1.0
    } else {
        (1.0 - (-dt / STEM_GAIN_BLEND_SECONDS).exp()).clamp(0.0, 1.0)
    };
    for bank in [StemBank::A, StemBank::B] {
        let bank_idx = bank.index();
        for stem in GeneratedStem::ALL {
            let idx = stem.index();
            let current = state.current_gains[bank_idx][idx];
            let target = state.target_gains[bank_idx][idx];
            let mut next = current + (target - current) * blend;
            if (target - next).abs() < 0.0005 {
                next = target;
            }
            state.current_gains[bank_idx][idx] = next;
            channels.set_stem_volume(bank, stem, next);
        }
    }

    if let Some(bank) = state.fading_bank {
        state.fade_stop_seconds -= dt;
        if state.fade_stop_seconds <= 0.0 {
            channels.stop_bank(bank, 80);
            state.current_gains[bank.index()] = [0.0; 6];
            state.target_gains[bank.index()] = [0.0; 6];
            state.fading_bank = None;
        }
    }
}
