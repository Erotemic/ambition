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
use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;
use bevy_ecs::message::{Message, MessageWriter};
use bevy_ecs::resource::Resource;
use bevy_ecs::system::{Query, Res, SystemParam};
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

/// **The presentation source a BODY emits under.**
///
/// Derived once per tick from the body's worn character and that character's
/// author, so every emitter can attribute a cue without repeating the lookup — and
/// so a cue attributed to the wrong provider becomes one bug in one place instead
/// of one per emitter.
///
/// Lives here, beside [`PresentationSourceId`], because the mechanics crates that
/// EMIT (`ambition_combat`'s hit feedback, movement fx) must be able to name it
/// without depending on the character catalog that DERIVES it.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct BodyPresentationSource(pub PresentationSourceId);

impl BodyPresentationSource {
    pub fn id(&self) -> &PresentationSourceId {
        &self.0
    }
}

/// Marks a [`BodyPresentationSource`] the per-tick DERIVATION granted, and may
/// therefore retract.
///
/// Without this the derivation cannot tell "a body that stopped wearing a character,
/// whose claim on that provider must be dropped" from "an entity whose source came
/// from somewhere else entirely" — and it would delete the second one. A projectile
/// inherits its firer's source at spawn and has no worn character of its own, so
/// under a single unmarked component every bolt lost its provenance on the tick
/// after it was fired, and impacted in the session's voice.
///
/// So: the derivation retracts only what the derivation granted. Anything else that
/// stamps a source owns its lifetime, which for a projectile is exactly the
/// projectile's.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DerivedPresentationSource;

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

    /// Emit a cue attributed to a BODY, when that body has a known source.
    ///
    /// The `Option` is the whole point, and it is deliberately not hidden behind a
    /// default: `None` means "this body has no presentation source", which falls
    /// back to the session context exactly as `write` does. Collapsing the two
    /// would silently attribute every character cue to the session provider — which
    /// is precisely the state §7.7 shipped in, where `write_from` existed and one
    /// caller used it.
    pub fn write_for_body(&mut self, source: Option<&PresentationSourceId>, request: SfxMessage) {
        match source {
            Some(source) => self.write_from(source.clone(), request),
            None => self.write(request),
        }
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

/// **A writer that can attribute a cue to the body that caused it.**
///
/// [`SfxWriter::write_for_body`] takes the source, which means every caller that
/// wanted to use it had to grow a `Query<&BodyPresentationSource>` and thread the
/// lookup down to the emit site. That cost is why §7.7 shipped with one
/// source-qualified caller and eighty-six plain ones: the mechanism existed and
/// using it was a refactor per call site.
///
/// This does the lookup, so a call site names the emitting entity — which it always
/// has, since it just resolved that entity to read its position — and the choice at
/// each site becomes the honest one: is this sound made BY something, or by the
/// world?
///
/// - [`write_for`](Self::write_for): a body's own cue. Its death, its block, its
///   ability, its footfall. Falls back to the session context when the entity has no
///   source, which is what an unworn body or a hazard should do.
/// - [`write_global`](Self::write_global): owned by the SESSION's world. A menu
///   blip, a room transition, a checkpoint chime. Identical to
///   [`SfxWriter::write`], named differently so that reading the call tells you
///   the site was CLASSIFIED rather than merely not converted.
/// - [`write_from`](Self::write_from): owned by a named content PROVIDER. A
///   course's own furniture — a monitor, a breakable brick, a distance marker —
///   makes sound no body caused, and it is the course's sound, not the host's.
///   Two cases were never enough: `write_global` reaches for the session context,
///   so under a shell host every course cue was attributed to the launcher.
///
/// Any emitting entity may carry the source, not just a worn body: a projectile
/// inherits its firer's at spawn, so a bolt that outlives its owner still impacts in
/// its own character's voice.
#[derive(SystemParam)]
pub struct BodySfxWriter<'w, 's> {
    sfx: SfxWriter<'w>,
    sources: Query<'w, 's, &'static BodyPresentationSource>,
}

impl BodySfxWriter<'_, '_> {
    /// Emit a cue caused by `body`, under `body`'s presentation source.
    pub fn write_for(&mut self, body: Entity, request: SfxMessage) {
        let source = self.sources.get(body).ok().map(BodyPresentationSource::id);
        self.sfx.write_for_body(source, request);
    }

    /// Emit a cue caused by `body` when the caller already holds its source —
    /// avoids a second lookup in the hit paths, which resolve attacker and victim
    /// sources up front so both are available before the writers are borrowed.
    pub fn write_for_body(&mut self, source: Option<&PresentationSourceId>, request: SfxMessage) {
        self.sfx.write_for_body(source, request);
    }

    /// Emit a cue that belongs to the WORLD, not to any body.
    ///
    /// "The world" means the SESSION's world — a menu blip, a pause chime, a
    /// checkpoint. It takes the session context's source, so it is the wrong
    /// operation for anything a course authored; see [`write_from`](Self::write_from).
    pub fn write_global(&mut self, request: SfxMessage) {
        self.sfx.write(request);
    }

    /// Emit a cue owned by a named CONTENT PROVIDER rather than by a body or by
    /// the session.
    ///
    /// The third case, and the one that was missing. A course's own furniture
    /// makes sound that no body caused: Sanic's distance markers and act-clear
    /// fanfare, a monitor popping, a Mary-O brick smashing. None of those belong
    /// to a character, so `write_for` is wrong; none of them belong to whoever is
    /// HOSTING, so `write_global` is wrong too — under a shell host that is the
    /// launcher's provider, and a crossover session would hear the wrong bank
    /// answer for the course's own cues.
    ///
    /// Ownership still comes from the active session context, exactly as
    /// [`SfxWriter::write_from`] does: naming a source says who authored the
    /// sound, never which session is allowed to play it.
    pub fn write_from(&mut self, source: impl Into<PresentationSourceId>, request: SfxMessage) {
        self.sfx.write_from(source, request);
    }

    /// This entity's presentation source, for a caller that must resolve it before
    /// it can borrow the writer.
    pub fn source_of(&self, body: Entity) -> Option<PresentationSourceId> {
        self.sources
            .get(body)
            .ok()
            .map(|source| source.id().clone())
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
