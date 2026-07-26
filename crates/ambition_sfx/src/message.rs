//! Session-owned SFX requests.
//!
//! Gameplay and shell presentation author [`SfxMessage`] values through
//! [`SfxWriter`]. The writer captures the current [`AudioContextOwner`] and
//! [`PresentationSourceId`] when the request is emitted, then publishes an
//! [`OwnedSfxMessage`]. Playback accepts the request only while that exact audio
//! context is active and that source is authorized for the session.
//!
//! This keeps mechanics independent of the game shell while preventing both
//! cross-provider leakage and the subtler same-provider relaunch leak: a Dash
//! from Sanic's package resolves as Sanic even inside an Ambition-owned match,
//! while a Dash queued by Sanic session A cannot play during Sanic session B.

use crate::SfxId;
use bevy_ecs::message::{Message, MessageWriter};
use bevy_ecs::resource::Resource;
use bevy_ecs::system::{Res, SystemParam};
use bevy_math::Vec2;
use std::fmt;

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

/// Stable identity of the authored presentation package that supplies a cue.
///
/// This is intentionally distinct from [`AudioContextOwner`]. The owner says
/// which live session is allowed to reach the speakers; the source says which
/// provider/package owns the requested cue. A crossover match has one owner and
/// several sources.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PresentationSourceId(String);

impl PresentationSourceId {
    const UNSCOPED: &'static str = "__unscoped__";

    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        assert!(
            !value.trim().is_empty(),
            "presentation source id cannot be empty"
        );
        Self(value)
    }

    /// Sentinel used only when a narrow unit fixture omitted the real emission
    /// context. Playback never authorizes it.
    pub fn unscoped() -> Self {
        Self(Self::UNSCOPED.to_owned())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_unscoped(&self) -> bool {
        self.0 == Self::UNSCOPED
    }
}

impl Default for PresentationSourceId {
    fn default() -> Self {
        Self::unscoped()
    }
}

impl From<&str> for PresentationSourceId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for PresentationSourceId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl fmt::Display for PresentationSourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The context captured by [`SfxWriter`] for newly-authored requests.
#[derive(Resource, Clone, Debug, Default)]
pub struct SfxEmissionContext {
    owner: Option<AudioContextOwner>,
    source: PresentationSourceId,
}

impl SfxEmissionContext {
    pub const fn owner(&self) -> Option<AudioContextOwner> {
        self.owner
    }

    pub fn source(&self) -> &PresentationSourceId {
        &self.source
    }

    pub fn set(&mut self, owner: AudioContextOwner, source: impl Into<PresentationSourceId>) {
        self.owner = Some(owner);
        self.source = source.into();
    }

    pub fn clear_if(&mut self, owner: AudioContextOwner) {
        if self.owner == Some(owner) {
            self.clear();
        }
    }

    pub fn clear(&mut self) {
        self.owner = None;
        self.source = PresentationSourceId::unscoped();
    }
}

/// A request to play a sound effect.
///
/// This remains the mechanics-facing vocabulary. It is deliberately not the
/// playback queue item: [`SfxWriter`] wraps it in [`OwnedSfxMessage`] with the
/// current context identity.
#[derive(Clone, Copy, Debug)]
pub enum SfxMessage {
    Jump {
        pos: Vec2,
    },
    DoubleJump {
        pos: Vec2,
    },
    Dash {
        pos: Vec2,
    },
    Blink {
        pos: Vec2,
        precision: bool,
    },
    Pogo {
        pos: Vec2,
    },
    /// Touchdown after an airborne arc. Emitted once per landing edge by the
    /// shared movement-fx pass (beside the landing dust), so any provider that
    /// authors `player.land` voices a footfall without per-game wiring.
    Land {
        pos: Vec2,
    },
    Slash {
        pos: Vec2,
    },
    Hit {
        pos: Vec2,
    },
    Death {
        pos: Vec2,
    },
    Reset {
        pos: Vec2,
    },
    Play {
        id: SfxId,
        pos: Vec2,
    },
}

/// Playback queue item with ownership and presentation provenance captured at
/// emission time.
#[derive(Message, Clone, Debug)]
pub struct OwnedSfxMessage {
    pub owner: Option<AudioContextOwner>,
    pub source: PresentationSourceId,
    pub request: SfxMessage,
}

/// Mechanics-facing writer that captures exact audio ownership without adding
/// another system parameter at every call site.
///
/// A missing context is retained as `None` for narrow unit fixtures. Real shell
/// and direct compositions install an explicit context; playback rejects an
/// unowned request whenever an owned context is active.
///
/// # Rollback is not this crate's problem
///
/// This writer once carried an `SfxEmissionGate` that dropped the request
/// outright while a rollback host re-simulated a frame. That was removed with
/// the confirmed-frame quarantine (`ambition_runtime::external_effects`), and
/// the removal is load-bearing rather than tidying: suppressing at emit time
/// destroys the corrected sound before anything can decide whether the
/// prediction it replaces was ever heard. A speculating host now defers this
/// message instead, which it can only do if the message is actually written.
///
/// So: always write. Deciding when a sound is allowed to reach the speakers
/// belongs to the host that knows which frames are settled, not to the mechanic
/// that knows a sword swung.
#[derive(SystemParam)]
pub struct SfxWriter<'w> {
    messages: MessageWriter<'w, OwnedSfxMessage>,
    context: Option<Res<'w, SfxEmissionContext>>,
}

impl SfxWriter<'_> {
    pub fn write(&mut self, request: SfxMessage) {
        let owner = self.context.as_deref().and_then(SfxEmissionContext::owner);
        let source = self
            .context
            .as_deref()
            .map(|context| context.source().clone())
            .unwrap_or_default();
        self.messages.write(OwnedSfxMessage {
            owner,
            source,
            request,
        });
    }

    /// Emit a cue from a source other than the context's default package.
    ///
    /// Character, stage, announcer, and ruleset emitters use this in composed
    /// sessions. Ownership is still captured from the active session context;
    /// callers cannot forge a retired session merely by naming another source.
    pub fn write_from(&mut self, source: impl Into<PresentationSourceId>, request: SfxMessage) {
        let owner = self.context.as_deref().and_then(SfxEmissionContext::owner);
        self.messages.write(OwnedSfxMessage {
            owner,
            source: source.into(),
            request,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::message::Messages;
    use bevy_ecs::schedule::Schedule;
    use bevy_ecs::world::World;

    fn emit(mut sfx: SfxWriter) {
        sfx.write(SfxMessage::Jump { pos: Vec2::ZERO });
    }

    fn emit_from_secondary(mut sfx: SfxWriter) {
        sfx.write_from("secondary.provider", SfxMessage::Dash { pos: Vec2::ZERO });
    }

    /// Drives the REAL `SfxWriter` through a real schedule, in the shape every
    /// gameplay emitter uses.
    fn emitted() -> Vec<OwnedSfxMessage> {
        let mut world = World::new();
        world.init_resource::<Messages<OwnedSfxMessage>>();
        let mut context = SfxEmissionContext::default();
        context.set(AudioContextOwner::Direct, "test.provider");
        world.insert_resource(context);
        let mut schedule = Schedule::default();
        schedule.add_systems(emit);
        schedule.run(&mut world);

        let messages = world.resource::<Messages<OwnedSfxMessage>>();
        let mut cursor = messages.get_cursor();
        cursor.read(messages).cloned().collect()
    }

    /// The writer is unconditional. If a future change reintroduces an
    /// emit-time suppression here, the confirmed-frame quarantine downstream
    /// silently loses the ability to correct a mispredicted sound — it can only
    /// replace intents it was given.
    #[test]
    fn the_writer_never_swallows_a_request() {
        let emitted = emitted();
        assert_eq!(emitted.len(), 1);
        assert_eq!(emitted[0].owner, Some(AudioContextOwner::Direct));
        assert_eq!(emitted[0].source.as_str(), "test.provider");
    }

    #[test]
    fn explicit_source_overrides_only_provenance_not_session_ownership() {
        let mut world = World::new();
        world.init_resource::<Messages<OwnedSfxMessage>>();
        let mut context = SfxEmissionContext::default();
        context.set(AudioContextOwner::Gameplay(7), "primary.provider");
        world.insert_resource(context);
        let mut schedule = Schedule::default();
        schedule.add_systems(emit_from_secondary);
        schedule.run(&mut world);

        let messages = world.resource::<Messages<OwnedSfxMessage>>();
        let mut cursor = messages.get_cursor();
        let emitted = cursor
            .read(messages)
            .next()
            .expect("one sourced SFX request is emitted");
        assert_eq!(emitted.owner, Some(AudioContextOwner::Gameplay(7)));
        assert_eq!(emitted.source.as_str(), "secondary.provider");
    }
}
