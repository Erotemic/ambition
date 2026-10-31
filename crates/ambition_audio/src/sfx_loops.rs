//! Sustained (looping) SFX: a cue held under a key until it is stopped.
//!
//! A loop is accepted exactly as a one-shot is ([`SfxSourceResolver`]), then
//! played `looped()` on the SFX channel with a handle kept under its
//! [`SfxLoopKey`], so a later request can stop it with a fade.
//!
//! Holding is idempotent and restated by its emitter each frame it is wanted,
//! so the cases that silence a loop without a `Stop` heal themselves: the SFX
//! channel is stopped on an audio-context change, and the next hold starts the
//! loop again. A loop whose owner is no longer the live context is stopped
//! here, so the end of a session never leaves one running.

use std::collections::BTreeMap;
use std::time::Duration;

use ambition_sfx::{AudioContextOwner, OwnedSfxLoopMessage, SfxId, SfxLoopKey, SfxLoopRequest};
use bevy::asset::Handle;
use bevy::ecs::message::Messages;
use bevy::prelude::{MessageReader, Res, ResMut, Resource};
use bevy_kira_audio::prelude::{AudioChannel, AudioControl, AudioInstance, AudioTween, PlaybackState};

use crate::bank_asset::SfxSourceResolver;
use crate::library::SfxChannel;

/// The fade a loop gets when its context ends under it.
const CONTEXT_END_FADE_SECS: f32 = 0.25;

/// One held loop.
#[derive(Clone, Debug)]
pub struct HeldSfxLoop {
    pub owner: AudioContextOwner,
    pub id: SfxId,
    /// The playing instance; `None` where the output is not a device (headless),
    /// so the loop is accepted and held without a sound.
    pub instance: Option<Handle<AudioInstance>>,
}

/// The loops playing now, by key.
#[derive(Resource, Default, Debug)]
pub struct SfxLoopPlayback {
    held: BTreeMap<SfxLoopKey, HeldSfxLoop>,
    /// Stopped loops whose instance the backend had not created yet: stopped
    /// once it exists, so a loop stopped on the frame it started still ends.
    stopping: Vec<(Handle<AudioInstance>, f32)>,
    /// Loops started (or restarted) since the app began: a stable diagnostic.
    pub started: u64,
}

impl SfxLoopPlayback {
    pub fn held(&self, key: SfxLoopKey) -> Option<&HeldSfxLoop> {
        self.held.get(&key)
    }

    pub fn len(&self) -> usize {
        self.held.len()
    }

    pub fn is_empty(&self) -> bool {
        self.held.is_empty()
    }
}

/// Run condition of [`audio_play_sfx_loops`]: a request waits, or a loop is held.
pub fn a_loop_request_waits_or_a_loop_is_held(
    messages: Option<Res<Messages<OwnedSfxLoopMessage>>>,
    loops: Res<SfxLoopPlayback>,
) -> bool {
    messages.is_some_and(|messages| !messages.is_empty()) || !loops.is_empty() || !loops.stopping.is_empty()
}

fn tween(secs: f32) -> AudioTween {
    AudioTween::linear(Duration::from_secs_f32(secs.max(0.0)))
}

impl SfxLoopPlayback {
    /// Stop `held`'s instance with a fade, now or once the backend creates it.
    fn stop(&mut self, instances: &mut bevy::asset::Assets<AudioInstance>, held: HeldSfxLoop, fade: f32) {
        let Some(handle) = held.instance else { return };
        match instances.get_mut(&handle) {
            Some(mut instance) => instance.stop(tween(fade)),
            None => self.stopping.push((handle, fade)),
        }
    }

    /// Retry the stops that waited for their instance; drop the ones that ended.
    fn retry_stops(&mut self, channel: &AudioChannel<SfxChannel>, instances: &mut bevy::asset::Assets<AudioInstance>) {
        self.stopping.retain(|(handle, fade)| match instances.get_mut(handle) {
            Some(mut instance) => {
                instance.stop(tween(*fade));
                false
            }
            None => channel.state(handle) == PlaybackState::Queued,
        });
    }
}

pub fn audio_play_sfx_loops(
    mut messages: MessageReader<OwnedSfxLoopMessage>,
    mut resolver: SfxSourceResolver,
    sfx_channel: Res<AudioChannel<SfxChannel>>,
    output: Option<Res<crate::output::AudioOutputMode>>,
    mut instances: ResMut<bevy::asset::Assets<AudioInstance>>,
    mut loops: ResMut<SfxLoopPlayback>,
) {
    loops.retry_stops(&sfx_channel, &mut instances);
    // A loop of a context that is no longer live ends with it.
    let ended: Vec<SfxLoopKey> = loops
        .held
        .iter()
        .filter(|(_, held)| !resolver.selection().accepts_request_owner(Some(held.owner)))
        .map(|(key, _)| *key)
        .collect();
    for key in ended {
        if let Some(held) = loops.held.remove(&key) {
            loops.stop(&mut instances, held, CONTEXT_END_FADE_SECS);
        }
    }

    let device = crate::output::emits_to_device(output.as_deref());
    for owned in messages.read() {
        match owned.request {
            SfxLoopRequest::Hold { key, id, fade_in_secs } => {
                if let Some(held) = loops.held.get(&key) {
                    // Stopped under us (the channel stops on a context change).
                    let silenced = held
                        .instance
                        .as_ref()
                        .is_some_and(|handle| sfx_channel.state(handle) == PlaybackState::Stopped);
                    if held.id == id && !silenced {
                        continue;
                    }
                    let held = loops.held.remove(&key).expect("just read");
                    loops.stop(&mut instances, held, fade_in_secs);
                }
                let Some(accepted) = resolver.accept(owned.owner, &owned.source, id) else {
                    continue;
                };
                let instance = device.then(|| {
                    sfx_channel
                        .play(accepted.resolved.handle)
                        .looped()
                        .fade_in(tween(fade_in_secs))
                        .handle()
                });
                loops.held.insert(key, HeldSfxLoop { owner: accepted.owner, id, instance });
                loops.started = loops.started.saturating_add(1);
            }
            SfxLoopRequest::Stop { key, fade_out_secs } => {
                if let Some(held) = loops.held.remove(&key) {
                    loops.stop(&mut instances, held, fade_out_secs);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use ambition_sfx::{AudioContextOwner, OwnedSfxLoopMessage, SfxId, SfxLoopKey, SfxLoopRequest};
    use bevy::asset::{AssetApp, AssetPlugin};
    use bevy::prelude::{App, MinimalPlugins, Update};
    use bevy_kira_audio::prelude::{AudioChannel, AudioInstance, AudioSource as KiraAudioSource};

    use super::{audio_play_sfx_loops, SfxLoopPlayback};
    use crate::bank_asset::SfxBankAssetPlugin;
    use crate::library::SfxChannel;
    use crate::output::AudioOutputMode;
    use crate::selection::ActiveAudioSelection;
    use crate::spec::{SfxRegistry, SfxSpec, WaveformSpec};

    const HUM: &str = "fixture.hum";
    const KEY: SfxLoopKey = SfxLoopKey(7);

    fn select(app: &mut App, owner: u64) {
        let registry = SfxRegistry {
            sample_rate: 44_100,
            sfx: vec![SfxSpec {
                cue: None,
                id: Some(HUM.into()),
                waveform: WaveformSpec::Sine,
                frequency: 110.0,
                frequency_end: 110.0,
                duration: 0.5,
                volume: 0.5,
                attack: 0.0,
                release: 0.02,
                noise: 0.0,
            }],
        };
        let mut selection = ActiveAudioSelection::default();
        selection.select_gameplay(owner, "fixture", None, Some(registry), BTreeSet::new());
        app.insert_resource(selection);
    }

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(AssetPlugin::default())
            .add_plugins(SfxBankAssetPlugin)
            .init_asset::<KiraAudioSource>()
            .init_asset::<AudioInstance>()
            .insert_resource(AudioChannel::<SfxChannel>::default())
            .insert_resource(AudioOutputMode::Recording)
            .add_message::<OwnedSfxLoopMessage>()
            .init_resource::<SfxLoopPlayback>()
            .add_systems(Update, audio_play_sfx_loops);
        select(&mut app, 1);
        app
    }

    fn send(app: &mut App, owner: u64, request: SfxLoopRequest) {
        app.world_mut().write_message(OwnedSfxLoopMessage {
            owner: Some(AudioContextOwner::Gameplay(owner)),
            source: "fixture".into(),
            request,
        });
    }

    fn hold(app: &mut App, owner: u64) {
        send(app, owner, SfxLoopRequest::Hold { key: KEY, id: SfxId::new(HUM), fade_in_secs: 1.0 });
    }

    /// A hold restated every frame starts its loop once; a stop ends it.
    #[test]
    fn a_held_loop_starts_once_and_a_stop_ends_it() {
        let mut app = app();
        for _ in 0..3 {
            hold(&mut app, 1);
            app.update();
        }
        let loops = app.world().resource::<SfxLoopPlayback>();
        assert_eq!(loops.held(KEY).map(|held| held.id), Some(SfxId::new(HUM)));
        assert_eq!(loops.started, 1, "restating a hold does not start a second loop");

        send(&mut app, 1, SfxLoopRequest::Stop { key: KEY, fade_out_secs: 0.5 });
        app.update();
        assert!(app.world().resource::<SfxLoopPlayback>().is_empty());
    }

    /// A loop whose session ended is stopped, even though its emitter (which
    /// ran in that session) never sent a stop; and a hold from that ended
    /// session is refused.
    #[test]
    fn a_loop_ends_with_its_audio_context() {
        let mut app = app();
        hold(&mut app, 1);
        app.update();
        assert!(!app.world().resource::<SfxLoopPlayback>().is_empty());

        select(&mut app, 2);
        hold(&mut app, 1);
        app.update();
        assert!(app.world().resource::<SfxLoopPlayback>().is_empty(), "the ended session's loop is gone");
    }
}
