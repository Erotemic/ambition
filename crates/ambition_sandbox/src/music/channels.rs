use super::*;

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
    pub(super) fn channel(&self, bank: MusicBank, slot: usize) -> &dyn MusicLayerChannel {
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

    pub(super) fn stop_all(&self, fade_ms: u64) {
        self.stop_bank(MusicBank::A, fade_ms);
        self.stop_bank(MusicBank::B, fade_ms);
    }

    pub(super) fn stop_bank(&self, bank: MusicBank, fade_ms: u64) {
        for slot in 0..MAX_LAYERS {
            self.channel(bank, slot).stop_with_fade(fade_ms);
        }
    }

    pub(super) fn set_bank_silent(&self, bank: MusicBank) {
        for slot in 0..MAX_LAYERS {
            self.channel(bank, slot).set_linear_volume(0.0);
        }
    }

    pub(super) fn set_layer_volume(&self, bank: MusicBank, slot: usize, linear: f32) {
        self.channel(bank, slot).set_linear_volume(linear);
    }

    pub(super) fn play_layer(
        &self,
        bank: MusicBank,
        slot: usize,
        handle: Handle<KiraAudioSource>,
        looped: bool,
        fade_ms: u64,
    ) {
        self.channel(bank, slot)
            .play_handle(handle, looped, fade_ms);
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
