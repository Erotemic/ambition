//! Session-owned SFX requests.
//!
//! Gameplay and shell presentation author [`SfxMessage`] values through
//! [`SfxWriter`]. The writer captures the current [`AudioContextOwner`] at the
//! moment the request is emitted and publishes an [`OwnedSfxMessage`]. Playback
//! accepts the request only while that exact audio context is still active.
//!
//! This keeps mechanics independent of the game shell while preventing both
//! cross-provider leakage and the subtler same-provider relaunch leak: a Dash
//! queued by Sanic session A cannot play during Sanic session B merely because
//! both sessions authorize the same cue.

use crate::SfxId;
use bevy_ecs::message::{Message, MessageWriter};
use bevy_ecs::resource::Resource;
use bevy_ecs::system::{Res, SystemParam};
use bevy_math::Vec2;

/// Exact owner of one active audio context.
///
/// Frontend shell experiences and gameplay sessions share this vocabulary, so
/// title/startup/loading SFX are first-class rather than exceptions. Direct
/// development entry has one stable owner and never participates in shell
/// retirement.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AudioContextOwner {
    Frontend(u64),
    Gameplay(u64),
    Direct,
}

/// The context captured by [`SfxWriter`] for newly-authored requests.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct SfxEmissionContext {
    owner: Option<AudioContextOwner>,
}

impl SfxEmissionContext {
    pub const fn owner(&self) -> Option<AudioContextOwner> {
        self.owner
    }

    pub fn set(&mut self, owner: AudioContextOwner) {
        self.owner = Some(owner);
    }

    pub fn clear_if(&mut self, owner: AudioContextOwner) {
        if self.owner == Some(owner) {
            self.owner = None;
        }
    }

    pub fn clear(&mut self) {
        self.owner = None;
    }
}

/// A request to play a sound effect.
///
/// This remains the mechanics-facing vocabulary. It is deliberately not the
/// playback queue item: [`SfxWriter`] wraps it in [`OwnedSfxMessage`] with the
/// current context identity.
#[derive(Clone, Copy, Debug)]
pub enum SfxMessage {
    Jump { pos: Vec2 },
    DoubleJump { pos: Vec2 },
    Dash { pos: Vec2 },
    Blink { pos: Vec2, precision: bool },
    Pogo { pos: Vec2 },
    Slash { pos: Vec2 },
    Hit { pos: Vec2 },
    Death { pos: Vec2 },
    Reset { pos: Vec2 },
    Play { id: SfxId, pos: Vec2 },
}

/// Playback queue item with ownership captured at emission time.
#[derive(Message, Clone, Copy, Debug)]
pub struct OwnedSfxMessage {
    pub owner: Option<AudioContextOwner>,
    pub request: SfxMessage,
}

/// Mechanics-facing writer that captures exact audio ownership without adding
/// another system parameter at every call site.
///
/// A missing context is retained as `None` for narrow unit fixtures. Real shell
/// and direct compositions install an explicit context; playback rejects an
/// unowned request whenever an owned context is active.
#[derive(SystemParam)]
pub struct SfxWriter<'w> {
    messages: MessageWriter<'w, OwnedSfxMessage>,
    context: Option<Res<'w, SfxEmissionContext>>,
}

impl SfxWriter<'_> {
    pub fn write(&mut self, request: SfxMessage) {
        let owner = self.context.as_deref().and_then(SfxEmissionContext::owner);
        self.messages.write(OwnedSfxMessage { owner, request });
    }
}
