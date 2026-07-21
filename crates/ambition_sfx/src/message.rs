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
        self.messages.write(OwnedSfxMessage { owner, request });
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

    /// Drives the REAL `SfxWriter` through a real schedule, in the shape every
    /// gameplay emitter uses.
    fn emitted() -> usize {
        let mut world = World::new();
        world.init_resource::<Messages<OwnedSfxMessage>>();
        let mut schedule = Schedule::default();
        schedule.add_systems(emit);
        schedule.run(&mut world);

        let messages = world.resource::<Messages<OwnedSfxMessage>>();
        let mut cursor = messages.get_cursor();
        cursor.read(messages).count()
    }

    /// The writer is unconditional. If a future change reintroduces an
    /// emit-time suppression here, the confirmed-frame quarantine downstream
    /// silently loses the ability to correct a mispredicted sound — it can only
    /// replace intents it was given.
    #[test]
    fn the_writer_never_swallows_a_request() {
        assert_eq!(emitted(), 1);
    }
}
