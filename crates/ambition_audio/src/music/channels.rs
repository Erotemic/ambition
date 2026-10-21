use super::*;

/// The kira channel of each music layer, by bank (A, B) and slot.
///
/// Dynamic channels, not typed ones: `bevy_kira_audio` runs two systems every
/// frame for each typed channel (its play and state passes) and one for all
/// dynamic channels together. Twelve typed layers were twenty-four systems in
/// every game; nothing reads a layer's instance states, which is all a typed
/// channel adds.
const LAYER_CHANNELS: [[&str; MAX_LAYERS]; 2] = [
    [
        "ambition.music.layer0.a",
        "ambition.music.layer1.a",
        "ambition.music.layer2.a",
        "ambition.music.layer3.a",
        "ambition.music.layer4.a",
        "ambition.music.layer5.a",
    ],
    [
        "ambition.music.layer0.b",
        "ambition.music.layer1.b",
        "ambition.music.layer2.b",
        "ambition.music.layer3.b",
        "ambition.music.layer4.b",
        "ambition.music.layer5.b",
    ],
];

/// Create the music layer channels. With a device backend `AudioPlugin` owns
/// `DynamicAudioChannels`; a recording backend has no plugin, so the resource
/// is made here and its commands are never played.
pub fn add_music_layer_channels(app: &mut App) {
    let mut channels = app.world_mut().get_resource_or_init::<DynamicAudioChannels>();
    for key in LAYER_CHANNELS.iter().flatten() {
        if !channels.is_channel(key) {
            channels.create_channel(key);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum MusicBank {
    A,
    B,
}

impl MusicBank {
    pub(super) fn other(self) -> Self {
        match self {
            Self::A => Self::B,
            Self::B => Self::A,
        }
    }

    pub(super) fn index(self) -> usize {
        match self {
            Self::A => 0,
            Self::B => 1,
        }
    }

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
        }
    }
}

pub(super) type LayerGains = [f32; MAX_LAYERS];

#[derive(SystemParam)]
pub struct MusicLayerChannels<'w> {
    channels: Res<'w, DynamicAudioChannels>,
    output: Option<Res<'w, crate::output::AudioOutputMode>>,
}

impl<'w> MusicLayerChannels<'w> {
    /// Return the host-selected final playback side effect.
    pub fn output_mode(&self) -> crate::output::AudioOutputMode {
        self.output.as_deref().copied().unwrap_or_default()
    }

    fn channel(&self, bank: MusicBank, slot: usize) -> &DynamicAudioChannel {
        self.channels
            .channel(LAYER_CHANNELS[bank.index()][slot.min(MAX_LAYERS - 1)])
    }

    pub(super) fn stop_all(&self, fade_ms: u64) {
        self.stop_bank(MusicBank::A, fade_ms);
        self.stop_bank(MusicBank::B, fade_ms);
    }

    pub(super) fn stop_bank(&self, bank: MusicBank, fade_ms: u64) {
        for slot in 0..MAX_LAYERS {
            stop_with_fade(self.channel(bank, slot), fade_ms);
        }
    }

    pub(super) fn set_bank_silent(&self, bank: MusicBank) {
        for slot in 0..MAX_LAYERS {
            set_linear_volume(self.channel(bank, slot), 0.0);
        }
    }

    pub(super) fn set_layer_volume(&self, bank: MusicBank, slot: usize, linear: f32) {
        set_linear_volume(self.channel(bank, slot), linear);
    }

    pub(super) fn play_layer(
        &self,
        bank: MusicBank,
        slot: usize,
        handle: Handle<KiraAudioSource>,
        looped: bool,
        fade_ms: u64,
    ) {
        if self.output_mode().emits_to_device() {
            play_handle(self.channel(bank, slot), handle, looped, fade_ms);
        }
    }
}

fn stop_with_fade(channel: &DynamicAudioChannel, fade_ms: u64) {
    channel.stop().fade_out(AudioTween::new(
        Duration::from_millis(fade_ms),
        AudioEasing::OutPowi(2),
    ));
}

fn set_linear_volume(channel: &DynamicAudioChannel, linear: f32) {
    channel.set_volume(amplitude_to_decibels(linear));
}

fn play_handle(channel: &DynamicAudioChannel, handle: Handle<KiraAudioSource>, looped: bool, fade_ms: u64) {
    if looped {
        if fade_ms == 0 {
            channel.play(handle).looped();
        } else {
            channel.play(handle).looped().fade_in(AudioTween::new(
                Duration::from_millis(fade_ms),
                AudioEasing::InPowi(2),
            ));
        }
    } else if fade_ms == 0 {
        channel.play(handle);
    } else {
        channel.play(handle).fade_in(AudioTween::new(
            Duration::from_millis(fade_ms),
            AudioEasing::InPowi(2),
        ));
    }
}
