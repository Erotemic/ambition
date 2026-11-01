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

use ambition_sfx::{OwnedSfxLoopMessage, SfxId, SfxLoopKey, SfxLoopRequest};
use bevy::ecs::message::Messages;
use bevy::prelude::{MessageReader, Res, ResMut, Resource};
use bevy_kira_audio::prelude::{
    AudioControl, PlaybackState,
};

use crate::bank_asset::SfxSourceResolver;

/// The fade a loop gets when its context ends under it.
const CONTEXT_END_FADE_SECS: f32 = 0.25;

/// One held loop.
#[derive(Clone, Debug)]
pub struct HeldSfxLoop {
    pub id: SfxId,
    pub source: ambition_sfx::PresentationSourceId,
    pub voice: crate::spatial::SfxVoice,
}

/// The loops playing now, by key.
#[derive(Resource, Default, Debug)]
pub struct SfxLoopPlayback {
    pub(crate) held: BTreeMap<SfxLoopKey, HeldSfxLoop>,
    pub(crate) fading: Vec<crate::spatial::SfxVoice>,
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

    pub fn has_voices(&self) -> bool {
        !self.held.is_empty() || !self.fading.is_empty()
    }
}

/// Run condition of [`audio_play_sfx_loops`]: a request waits, or a loop is held.
pub fn a_loop_request_waits_or_a_loop_is_held(
    messages: Option<Res<Messages<OwnedSfxLoopMessage>>>,
    loops: Res<SfxLoopPlayback>,
) -> bool {
    messages.is_some_and(|messages| !messages.is_empty()) || !loops.is_empty()
}

impl SfxLoopPlayback {
    fn stop(&mut self, mut held: HeldSfxLoop, fade: f32, now: std::time::Duration) {
        held.voice.request_stop(fade, now);
        self.fading.push(held.voice);
    }
}

pub fn audio_play_sfx_loops(
    mut messages: MessageReader<OwnedSfxLoopMessage>,
    mut resolver: SfxSourceResolver,
    output: crate::spatial::SfxVoiceOutput,
    mut loops: ResMut<SfxLoopPlayback>,
) {
    // A loop of a context that is no longer live ends with it.
    let ended: Vec<SfxLoopKey> = loops
        .held
        .iter()
        .filter(|(_, held)| {
            !resolver
                .selection()
                .accepts_request_owner(Some(held.voice.owner))
        })
        .map(|(key, _)| *key)
        .collect();
    for key in ended {
        if let Some(held) = loops.held.remove(&key) {
            loops.stop(held, CONTEXT_END_FADE_SECS, output.now());
        }
    }

    for owned in messages.read() {
        match owned.request {
            SfxLoopRequest::Hold {
                key,
                id,
                emitter,
                fade_in_secs,
            } => {
                if let Some(held) = loops.held.get_mut(&key) {
                    // Stopped under us (the channel stops on a context change).
                    let silenced = held.voice.instance().is_some_and(|handle| {
                        output.channel.state(handle) == PlaybackState::Stopped
                    });
                    if held.id == id
                        && held.source == owned.source
                        && owned.owner == Some(held.voice.owner)
                        && !silenced
                    {
                        held.voice.emitter = emitter;
                        continue;
                    }
                }
                let Some(accepted) = resolver.accept(owned.owner, &owned.source, id) else {
                    continue;
                };
                if let Some(held) = loops.held.remove(&key) {
                    loops.stop(held, fade_in_secs, output.now());
                }
                let duration = resolver.duration(&accepted.resolved.handle);
                let voice = output.start(
                    accepted.owner,
                    accepted.resolved.handle,
                    emitter,
                    duration,
                    Some(fade_in_secs),
                    1.0,
                );
                loops.held.insert(
                    key,
                    HeldSfxLoop {
                        id,
                        source: owned.source.clone(),
                        voice,
                    },
                );
                loops.started = loops.started.saturating_add(1);
            }
            SfxLoopRequest::Stop { key, fade_out_secs } => {
                if !resolver.selection().accepts_request_owner(owned.owner) {
                    continue;
                }
                if !loops
                    .held
                    .get(&key)
                    .is_some_and(|held| held.source == owned.source)
                {
                    continue;
                }
                if let Some(held) = loops.held.remove(&key) {
                    loops.stop(held, fade_out_secs, output.now());
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
        send(
            app,
            owner,
            SfxLoopRequest::Hold {
                key: KEY,
                id: SfxId::new(HUM),
                fade_in_secs: 1.0,
                emitter: ambition_sfx::SfxEmitter::Global,
            },
        );
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
        assert_eq!(
            loops.started, 1,
            "restating a hold does not start a second loop"
        );

        send(
            &mut app,
            1,
            SfxLoopRequest::Stop {
                key: KEY,
                fade_out_secs: 0.5,
            },
        );
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
        assert!(
            app.world().resource::<SfxLoopPlayback>().is_empty(),
            "the ended session's loop is gone"
        );
    }
    #[test]
    fn a_quiet_landing_keeps_its_strength_as_the_listener_moves() {
        use crate::{audio_play_sfx_messages, spatial::{SfxOneShots, update_sfx_attenuation}};
        use ambition_sfx::{OwnedSfxMessage, SfxListeners, SfxMessage};
        use bevy::{prelude::{IntoScheduleConfigs, Vec2}, time::TimeUpdateStrategy};
        let mut app = app();
        let registry = SfxRegistry {
            sample_rate: 44_100,
            sfx: vec![SfxSpec {
                cue: Some(crate::spec::SoundCueKey::Land), id: None,
                waveform: WaveformSpec::Sine, frequency: 110.0, frequency_end: 90.0,
                duration: 0.5, volume: 0.5, attack: 0.0, release: 0.02, noise: 0.0,
            }],
        };
        app.world_mut().resource_mut::<ActiveAudioSelection>()
            .select_gameplay(1, "fixture", None, Some(registry), BTreeSet::new());
        app.insert_resource(TimeUpdateStrategy::ManualDuration(std::time::Duration::from_millis(16)));
        app.add_message::<OwnedSfxMessage>();
        app.add_systems(Update, audio_play_sfx_messages.before(audio_play_sfx_loops));
        app.add_systems(Update, update_sfx_attenuation.after(audio_play_sfx_loops));
        app.world_mut().resource_mut::<SfxListeners>().0 = vec![Vec2::ZERO];
        app.world_mut().write_message(OwnedSfxMessage {
            owner: Some(AudioContextOwner::Gameplay(1)), source: "fixture".into(),
            request: SfxMessage::Land { pos: Vec2::ZERO, strength: 0.1 },
        });
        app.update();
        let gain = |app: &App| app.world().resource::<SfxOneShots>().0[0].gain;
        assert!((gain(&app) - 0.1).abs() < 1e-6);
        app.world_mut().resource_mut::<SfxListeners>().0[0] = Vec2::new(880.0, 0.0);
        app.update();
        assert!((gain(&app) - 0.05).abs() < 1e-6);
        app.world_mut().resource_mut::<SfxListeners>().0[0] = Vec2::ZERO;
        app.update();
        assert!((gain(&app) - 0.1).abs() < 1e-6);
        assert_eq!(app.world().resource::<crate::SfxPlaybackState>().accepted_playbacks, 1);
    }

    #[test]
    fn one_shots_and_held_loops_follow_listener_motion_without_restarting() {
        use crate::{
            audio_play_sfx_messages,
            spatial::{update_sfx_attenuation, SfxOneShots},
        };
        use ambition_sfx::{
            OwnedSfxMessage, SfxEmitter, SfxFalloff, SfxListeners, SfxMessage, SfxShape,
        };
        use bevy::{
            prelude::{IntoScheduleConfigs, Vec2},
            time::TimeUpdateStrategy,
        };
        let mut app = app();
        app.insert_resource(TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_millis(16),
        ));
        app.add_message::<OwnedSfxMessage>();
        app.add_systems(Update, audio_play_sfx_messages.before(audio_play_sfx_loops));
        app.add_systems(Update, update_sfx_attenuation.after(audio_play_sfx_loops));
        app.world_mut().resource_mut::<SfxListeners>().0 = vec![Vec2::ZERO];
        let emitter = SfxEmitter::Spatial {
            shape: SfxShape::Point(Vec2::ZERO),
            falloff: SfxFalloff::new(10.0, 110.0),
        };
        app.world_mut().write_message(OwnedSfxMessage {
            owner: Some(AudioContextOwner::Gameplay(1)),
            source: "fixture".into(),
            request: SfxMessage::Play {
                id: SfxId::new(HUM),
                emitter,
            },
        });
        send(
            &mut app,
            1,
            SfxLoopRequest::Hold {
                key: KEY,
                id: SfxId::new(HUM),
                emitter,
                fade_in_secs: 1.0,
            },
        );
        app.update();
        for (position, gain) in [(0.0, 1.0), (60.0, 0.5), (210.0, 0.0), (0.0, 1.0)] {
            app.world_mut().resource_mut::<SfxListeners>().0[0] = Vec2::new(position, 0.0);
            app.update();
            let shots = app.world().resource::<SfxOneShots>();
            assert_eq!(shots.0.len(), 1);
            assert_eq!(shots.0[0].gain, gain);
            let loops = app.world().resource::<SfxLoopPlayback>();
            assert_eq!(loops.held(KEY).unwrap().voice.gain, gain);
            assert_eq!(loops.started, 1);
        }
        app.world_mut().resource_mut::<SfxListeners>().0[0] = Vec2::new(60.0, 0.0);
        app.world_mut()
            .resource_mut::<crate::SfxMix>()
            .effective_sfx = 0.25;
        app.update();
        assert_eq!(app.world().resource::<SfxOneShots>().0[0].gain, 0.125);
        assert_eq!(
            app.world()
                .resource::<SfxLoopPlayback>()
                .held(KEY)
                .unwrap()
                .voice
                .gain,
            0.125
        );
        app.world_mut()
            .resource_mut::<crate::SfxMix>()
            .effective_sfx = 0.0;
        app.update();
        assert_eq!(app.world().resource::<SfxOneShots>().0[0].gain, 0.0);
        assert_eq!(
            app.world()
                .resource::<SfxLoopPlayback>()
                .held(KEY)
                .unwrap()
                .voice
                .gain,
            0.0
        );
        app.world_mut()
            .resource_mut::<crate::SfxMix>()
            .effective_sfx = 1.0;
        app.world_mut().resource_mut::<SfxListeners>().0[0] = Vec2::ZERO;
        // A new hold changes the source position, not the loop's identity.
        send(
            &mut app,
            1,
            SfxLoopRequest::Hold {
                key: KEY,
                id: SfxId::new(HUM),
                emitter: Vec2::splat(5000.0).into(),
                fade_in_secs: 1.0,
            },
        );
        app.update();
        let loops = app.world().resource::<SfxLoopPlayback>();
        assert_eq!(loops.held(KEY).unwrap().voice.gain, 0.0);
        assert_eq!(loops.started, 1);

        // A menu cue has no world position and stays audible with no listener.
        app.world_mut().resource_mut::<SfxListeners>().0.clear();
        app.world_mut().write_message(OwnedSfxMessage {
            owner: Some(AudioContextOwner::Gameplay(1)),
            source: "fixture".into(),
            request: SfxMessage::Play {
                id: SfxId::new(HUM),
                emitter: SfxEmitter::Global,
            },
        });
        app.update();
        assert_eq!(
            app.world().resource::<SfxOneShots>().0.last().unwrap().gain,
            1.0
        );
        send(
            &mut app,
            1,
            SfxLoopRequest::Hold {
                key: KEY,
                id: SfxId::new(HUM),
                emitter: SfxEmitter::Global,
                fade_in_secs: 1.0,
            },
        );
        app.update();
        send(
            &mut app,
            1,
            SfxLoopRequest::Stop {
                key: KEY,
                fade_out_secs: 0.3,
            },
        );
        app.update();
        assert_eq!(app.world().resource::<SfxLoopPlayback>().fading.len(), 1);
        app.world_mut()
            .resource_mut::<crate::SfxMix>()
            .effective_sfx = 0.0;
        app.update();
        assert_eq!(
            app.world().resource::<SfxLoopPlayback>().fading[0].gain,
            0.0
        );
        for _ in 0..40 {
            app.update();
        }
        assert!(app.world().resource::<SfxLoopPlayback>().fading.is_empty());
        assert!(
            app.world().resource::<SfxOneShots>().0.is_empty(),
            "finished one-shots are released"
        );
    }
}
