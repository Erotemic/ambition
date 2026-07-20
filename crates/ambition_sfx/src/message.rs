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

/// Host gate on external audio effects.
///
/// A rollback host re-simulates frames it has already simulated once. Gameplay
/// is *supposed* to run again — that is what makes rollback correct — but the
/// sound already reached the speakers on the pass that first simulated the
/// frame. Emitting again is an audible duplicate, once per rollback.
///
/// So the host raises this gate while it re-simulates and lowers it for a frame
/// it is simulating for the first time. This crate stays independent of the
/// simulation (it cannot see a schedule, a frame counter, or a GGRS session):
/// the host publishes the fact, the same way it publishes audio ownership
/// through [`SfxEmissionContext`].
///
/// Absent resource = no rollback host = nothing is ever suppressed, which is
/// what every fixed-tick game, headless fixture, and unit test wants.
///
/// # This is duplicate suppression, NOT confirmed-frame release
///
/// The flag says *this frame ran before*, which is not the same as *this frame
/// is confirmed*. With predicted remote input the two diverge and this gate is
/// wrong in a second way:
///
/// 1. the predicted pass emits sound A, which reaches the speakers;
/// 2. the real input arrives and forces a rollback;
/// 3. the corrected re-simulation should emit sound B — and this gate
///    suppresses it, because the frame ran before.
///
/// The duplicate is gone, but the phantom is kept and the correction is lost.
/// Fixing that needs frame-stamped effect intents held until the host's
/// confirmed boundary, with abandoned predictions discarded — a different
/// mechanism, not a stricter boolean. Until then this is an honest interim fix
/// for the echo that local rollback produces today, and it must not be copied
/// as the final shape for VFX or any other external effect.
#[derive(Resource, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SfxEmissionGate {
    /// The host is re-simulating a frame it has already simulated once.
    pub frame_simulated_before: bool,
}

impl SfxEmissionGate {
    pub const fn suppresses_emission(self) -> bool {
        self.frame_simulated_before
    }
}

/// Mechanics-facing writer that captures exact audio ownership without adding
/// another system parameter at every call site.
///
/// A missing context is retained as `None` for narrow unit fixtures. Real shell
/// and direct compositions install an explicit context; playback rejects an
/// unowned request whenever an owned context is active.
///
/// This is also the ONE place external audio effects are suppressed on a
/// rollback re-simulation ([`SfxEmissionGate`] — read its caveat: duplicate
/// suppression, not confirmed-frame release). Every sim-side emitter already
/// writes through here, so the guard covers the ones written tomorrow too —
/// there is deliberately no second gate at the ~20 individual emit sites.
#[derive(SystemParam)]
pub struct SfxWriter<'w> {
    messages: MessageWriter<'w, OwnedSfxMessage>,
    context: Option<Res<'w, SfxEmissionContext>>,
    gate: Option<Res<'w, SfxEmissionGate>>,
}

impl SfxWriter<'_> {
    pub fn write(&mut self, request: SfxMessage) {
        if self
            .gate
            .as_deref()
            .copied()
            .is_some_and(SfxEmissionGate::suppresses_emission)
        {
            return;
        }
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

    /// Drives the REAL `SfxWriter` through a real schedule, so the guard is
    /// exercised in the shape every gameplay emitter uses.
    fn emitted_with_gate(gate: Option<SfxEmissionGate>) -> usize {
        let mut world = World::new();
        world.init_resource::<Messages<OwnedSfxMessage>>();
        if let Some(gate) = gate {
            world.insert_resource(gate);
        }
        let mut schedule = Schedule::default();
        schedule.add_systems(emit);
        schedule.run(&mut world);

        let messages = world.resource::<Messages<OwnedSfxMessage>>();
        let mut cursor = messages.get_cursor();
        cursor.read(messages).count()
    }

    #[test]
    fn a_replayed_frame_emits_no_sound() {
        assert_eq!(
            emitted_with_gate(Some(SfxEmissionGate {
                frame_simulated_before: true
            })),
            0,
            "audio for this frame already played on the pass that first simulated it"
        );
    }

    #[test]
    fn a_first_time_frame_emits_normally() {
        assert_eq!(
            emitted_with_gate(Some(SfxEmissionGate {
                frame_simulated_before: false
            })),
            1
        );
    }

    /// No rollback host installed the gate: a fixed-tick game, a headless
    /// fixture, and every existing unit test must be unaffected.
    #[test]
    fn an_absent_gate_never_suppresses() {
        assert_eq!(emitted_with_gate(None), 1);
    }
}
