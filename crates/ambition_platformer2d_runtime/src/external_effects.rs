//! Defers presentation-only simulation effects until their producing frame is confirmed.
//!
//! Each simulation advance swaps the live message channel for an empty outbox, journals the
//! resulting intents by frame, replaces that frame's journal entry on resimulation, releases
//! confirmed frames in order, and discards intents from abandoned future frames on load.
//! Swapping the entire `Messages` value preserves non-simulation writers and reader cursors.
//!
//! This mechanism is only for effects consumed outside the simulation. Messages read by the
//! simulation must remain on the simulation path and must not be quarantined.

use std::collections::BTreeMap;
use std::marker::PhantomData;

use bevy::ecs::message::{Message, Messages};
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;

use ambition_platformer2d_core::ConfirmedFrameBoundary;

/// Where the quarantine's four phases sit relative to everything else.
#[derive(SystemSet, Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum ExternalEffectSet {
    /// Sim schedule, before any gameplay: swap in an empty outbox.
    OpenOutbox,
    /// Sim schedule, after all gameplay: stamp the outbox with this frame.
    Journal,
    /// After the host finishes its advances: hand confirmed frames to presentation.
    Release,
    /// `LoadWorld`: drop intents from the abandoned branch.
    DiscardAbandoned,
}

/// Effect intents produced by the simulation, held by the frame that produced
/// them until that frame can never be simulated again.
///
/// Deliberately not rollback state, and it must never be registered as
/// such. This is host bookkeeping *about* the simulation, like
/// `RollbackExecutionStats`: rewinding it would restore a `released` count and
/// a pending set from before the effects were handed over, and every one of
/// them would be delivered a second time. The observable consequence is what
/// `app_it::effect_quarantine::rewinding_does_not_change_what_presentation_observes`
/// measures, so registering this for rollback fails that test rather than
/// passing quietly.
#[derive(Resource)]
pub struct ExternalEffectJournal<M: Message> {
    pending: BTreeMap<i32, Vec<M>>,
    /// Which session's timeline `pending` describes. A different generation
    /// means the intents belong to a world that no longer exists.
    session: u64,
    /// Total intents handed to presentation. Test-facing: the exactly-once
    /// claim is a count, not a vibe.
    released: u64,
    /// The real channel, held aside for the duration of one advance so the sim
    /// writes into an empty one. See the module docs on why this is a swap
    /// rather than a clear.
    lifted: Option<Messages<M>>,
}

impl<M: Message> Default for ExternalEffectJournal<M> {
    fn default() -> Self {
        Self {
            pending: BTreeMap::new(),
            session: 0,
            released: 0,
            lifted: None,
        }
    }
}

impl<M: Message> ExternalEffectJournal<M> {
    /// Record everything one simulation pass produced for `frame`.
    ///
    /// Always inserts, including an empty batch: a re-simulation that produces
    /// nothing must *erase* what the abandoned pass predicted, not leave it
    /// standing. Dropping the empty case is the subtle way to reintroduce the
    /// phantom this module exists to remove.
    pub fn record(&mut self, frame: i32, session: u64, intents: Vec<M>) {
        self.reset_if_new_session(session);
        self.pending.insert(frame, intents);
    }

    /// Take every intent whose frame is now settled, oldest frame first.
    pub fn take_confirmed(&mut self, boundary: &ConfirmedFrameBoundary) -> Vec<M> {
        self.reset_if_new_session(boundary.session);
        let confirmed: Vec<i32> = self
            .pending
            .range(..=boundary.confirmed)
            .map(|(frame, _)| *frame)
            .collect();
        let mut out = Vec::new();
        for frame in confirmed {
            if let Some(intents) = self.pending.remove(&frame) {
                out.extend(intents);
            }
        }
        self.released = self.released.saturating_add(out.len() as u64);
        out
    }

    /// Drop intents produced after `frame` — the host has restored `frame`, so
    /// everything that followed came from a branch it has abandoned.
    pub fn discard_after(&mut self, frame: i32) {
        self.pending.retain(|pending, _| *pending <= frame);
    }

    /// How many frames are waiting on confirmation. Bounded by the host's
    /// prediction window in practice; asserted by `the_journal_depth_stays_within_the_prediction_window`.
    pub fn depth(&self) -> usize {
        self.pending.len()
    }

    /// How many intents this journal has handed to presentation, ever.
    pub const fn released(&self) -> u64 {
        self.released
    }

    fn reset_if_new_session(&mut self, session: u64) {
        if self.session != session {
            self.pending.clear();
            self.session = session;
        }
    }
}

/// Lift the live channel aside and give the sim an empty one to write into.
///
/// Paired with [`journal_sim_effects`], which restores the lifted channel. This
/// swaps rather than clears because simulation is not the only writer.
pub fn open_sim_effect_outbox<M: Message>(world: &mut World) {
    let lifted = std::mem::take(&mut *world.resource_mut::<Messages<M>>());
    world.resource_mut::<ExternalEffectJournal<M>>().lifted = Some(lifted);
}

/// Stamp everything this pass produced with the frame that produced it, and
/// restore the channel [`open_sim_effect_outbox`] lifted aside.
pub fn journal_sim_effects<M: Message>(world: &mut World) {
    let boundary = world.resource::<ConfirmedFrameBoundary>();
    let (frame, session) = (boundary.current, boundary.session);
    let intents: Vec<M> = world.resource_mut::<Messages<M>>().drain().collect();
    let mut journal = world.resource_mut::<ExternalEffectJournal<M>>();
    let lifted = journal.lifted.take();
    journal.record(frame, session, intents);
    if let Some(lifted) = lifted {
        *world.resource_mut::<Messages<M>>() = lifted;
    }
}

/// Hand confirmed frames to the ordinary presentation consumers.
pub fn release_confirmed_effects<M: Message>(world: &mut World) {
    // Copied, not scoped: a resource is a component in this Bevy, and
    // `resource_scope` takes it out and puts it back, an archetype move each
    // call.
    let boundary = *world.resource::<ConfirmedFrameBoundary>();
    let released = world.resource_mut::<ExternalEffectJournal<M>>().take_confirmed(&boundary);
    if !released.is_empty() {
        world.resource_mut::<Messages<M>>().write_batch(released);
    }
}

/// Drop the abandoned branch's intents when the host restores an earlier frame.
///
/// Reads the restored frame from [`ConfirmedFrameBoundary::current`], which the
/// rollback bridge republishes at `LoadWorld` for exactly this reason — so this
/// module never needs to name a GGRS type.
pub fn discard_abandoned_predictions<M: Message>(world: &mut World) {
    let frame = world.resource::<ConfirmedFrameBoundary>().current;
    world.resource_mut::<ExternalEffectJournal<M>>().discard_after(frame);
}

/// Each quarantined family's step of each phase, in the order the families
/// were added. One system per phase runs every family's step: eight families were 25 systems a frame, each paying
/// the executor for a swap or an empty journal entry.
#[derive(Resource, Default)]
pub struct ExternalEffectFamilies {
    open: Vec<fn(&mut World)>,
    journal: Vec<fn(&mut World)>,
    release: Vec<fn(&mut World)>,
    discard: Vec<fn(&mut World)>,
}

/// Run one phase's step for every quarantined family. The list is taken out
/// of the resource and put back, not scoped: a resource is a component in this
/// Bevy, and `resource_scope` would move its entity between archetypes twice a
/// call.
fn run_effect_phase(world: &mut World, steps: fn(&mut ExternalEffectFamilies) -> &mut Vec<fn(&mut World)>) {
    let taken = std::mem::take(steps(&mut world.resource_mut::<ExternalEffectFamilies>()));
    for step in &taken {
        step(world);
    }
    *steps(&mut world.resource_mut::<ExternalEffectFamilies>()) = taken;
}

fn open_every_sim_effect_outbox(world: &mut World) {
    run_effect_phase(world, |families| &mut families.open);
}

fn journal_every_sim_effect(world: &mut World) {
    run_effect_phase(world, |families| &mut families.journal);
}

fn release_every_confirmed_effect(world: &mut World) {
    run_effect_phase(world, |families| &mut families.release);
}

fn discard_every_abandoned_prediction(world: &mut World) {
    run_effect_phase(world, |families| &mut families.discard);
}

/// Quarantines one effect family. Add one per presentation-facing message type.
///
/// Every family's steps run in the quarantine's one pass per phase, installed
/// with the first family. Each pass is gated on [`ConfirmedFrameBoundary`]
/// existing, so installing this on a host that never speculates is inert
/// rather than merely harmless.
pub struct ExternalEffectQuarantinePlugin<M: Message> {
    marker: PhantomData<fn() -> M>,
}

impl<M: Message> Default for ExternalEffectQuarantinePlugin<M> {
    fn default() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

impl<M: Message> Plugin for ExternalEffectQuarantinePlugin<M> {
    fn build(&self, app: &mut App) {
        use ambition_platformer2d_shared_tangle::schedule::{
            GameplaySimulationRoot, SimScheduleExt as _,
        };

        // A host that composes rollback WITHOUT presentation (a headless capability host,
        // `examples/capability_demo`'s GGRS round-trip) failed parameter validation on frame one,
        // naming a message type it had never heard of.
        //
        // `add_message` is guarded by `contains_resource`, so this is idempotent
        // and changes nothing for a composition that already registered it.
        app.add_message::<M>().init_resource::<ExternalEffectJournal<M>>();
        let first = !app.world().contains_resource::<ExternalEffectFamilies>();
        let mut families = app.world_mut().get_resource_or_init::<ExternalEffectFamilies>();
        families.open.push(open_sim_effect_outbox::<M>);
        families.journal.push(journal_sim_effects::<M>);
        families.release.push(release_confirmed_effects::<M>);
        if !first {
            return;
        }
        let sim = app.sim_schedule();
        let speculating = resource_exists::<ConfirmedFrameBoundary>;
        app.add_systems(
            sim,
            open_every_sim_effect_outbox
                .in_set(ExternalEffectSet::OpenOutbox)
                .before(GameplaySimulationRoot)
                .run_if(speculating),
        )
        .add_systems(
            sim,
            journal_every_sim_effect
                .in_set(ExternalEffectSet::Journal)
                .after(GameplaySimulationRoot)
                .run_if(speculating),
        )
        .add_systems(
            PreUpdate,
            release_every_confirmed_effect
                .in_set(ExternalEffectSet::Release)
                .run_if(speculating),
        );

        // `LoadWorld` only exists under a rollback host.
    }
}

/// Install the abandoned-branch discard into the host's restore schedule.
///
/// Separate from [`ExternalEffectQuarantinePlugin`] because only a rollback host
/// has a restore schedule to install it into. The discard pass is installed
/// with the first family, into that family's schedule: a host has one restore
/// schedule.
pub fn quarantine_discard_on_load<M: Message>(app: &mut App, load_schedule: impl ScheduleLabel) {
    let mut families = app.world_mut().get_resource_or_init::<ExternalEffectFamilies>();
    let first = families.discard.is_empty();
    families.discard.push(discard_abandoned_predictions::<M>);
    if first {
        app.add_systems(
            load_schedule,
            discard_every_abandoned_prediction
                .in_set(ExternalEffectSet::DiscardAbandoned)
                .run_if(resource_exists::<ConfirmedFrameBoundary>),
        );
    }
}

/// One use of the list of presentation-facing effect families.
pub trait PresentationEffectVisitor {
    fn visit<M: Message>(&mut self, app: &mut App);
}

/// Every effect family whose consumer lives outside the simulation.
///
/// This list is the classification. A message belongs here when its reader
/// is presentation, persistence, or anything else the player observes directly;
/// it must stay out when the simulation itself reads it, because deferring such
/// a message would change what the simulation computes. The distinction is
/// pinned by `only_presentation_facing_effects_are_quarantined`.
///
/// | family | reader | why |
/// |---|---|---|
/// | `OwnedSfxMessage` | `audio_play_sfx_messages` (`Update`) | reaches the speakers |
/// | `VfxInRoom` | `vfx_spawn_messages`, `spawn_slash_effects` (`Update`) | spawns visuals |
/// | `FxRequest` | `process_fx_requests` (`Update`) | fans out to the two above |
/// | `FireworksRequest` | `process_fireworks_requests` (`Update`) | spawns a visual sequence |
/// | `DebrisBurstMessage` | `physics_spawn_debris_messages` (`Update`) | spawns physics debris |
/// | `CameraShakeRequest` | `apply_camera_shake_requests` (`Update`) | moves the screen |
///
/// Deliberately absent: `EffectRequest` and `ProjectileSpawnRequest`, whose readers are
/// all sim-side despite the effect-shaped names.
pub fn for_each_presentation_effect(app: &mut App, visitor: &mut impl PresentationEffectVisitor) {
    use ambition_platformer2d_shared_tangle::camera_ease::{CameraShakeRequest, FinishZoomRequest};
    use ambition_vfx::vfx::{DebrisBurstMessage, KnockoutBeatRequested};
    use ambition_vfx::{FireworksRequest, FxRequest, VfxInRoom};

    visitor.visit::<ambition_sfx::OwnedSfxMessage>(app);
    visitor.visit::<VfxInRoom>(app);
    visitor.visit::<FxRequest>(app);
    visitor.visit::<FireworksRequest>(app);
    visitor.visit::<DebrisBurstMessage>(app);
    visitor.visit::<CameraShakeRequest>(app);
    visitor.visit::<FinishZoomRequest>(app);
    visitor.visit::<KnockoutBeatRequested>(app);
}

/// Quarantine every presentation-facing effect family
/// ([`for_each_presentation_effect`]).
///
/// The two presentation-side writers in the fan-out chain (`FxRequest`
/// and `VfxInRoom` are also written by `ambition_render`'s `Update` systems)
/// need no special handling: they run after the release, so what they produce is
/// already downstream of the confirmed boundary and flows straight through.
pub fn quarantine_presentation_effects(app: &mut App, load_schedule: impl ScheduleLabel + Clone) {
    struct Quarantine<L>(L);
    impl<L: ScheduleLabel + Clone> PresentationEffectVisitor for Quarantine<L> {
        fn visit<M: Message>(&mut self, app: &mut App) {
            app.add_plugins(ExternalEffectQuarantinePlugin::<M>::default());
            quarantine_discard_on_load::<M>(app, self.0.clone());
        }
    }
    for_each_presentation_effect(app, &mut Quarantine(load_schedule));
}

/// A presentation-facing effect ends with the session that asked for it: a
/// session activation empties each of these channels
/// (`lifecycle::session_messages`).
///
/// ⛔ THE LIFETIME OF A MESSAGE ACROSS A SESSION IS NOT ITS ROLLBACK CLASS.
/// [`for_each_presentation_effect`] says that the simulation reads none of
/// these, so a rewind defers them and does not clear them. That says nothing
/// about the next session. The effects of a session's last tick are released
/// in `PreUpdate` of the update that replaces it, and a reader that runs
/// after the activation presents them for the next session: `VfxInRoom` names
/// a `LiveRoomInstance`, and the first room of each session has the same one.
/// MEASURED 2026-10-04 on the shell host: two effects of the session that
/// ended were on the bus at the activation frame of the next one.
///
/// So each family ends at the activation, and a family that crosses is named
/// here with its reason. The default for a new family is that it ends.
///
/// - `OwnedSfxMessage` crosses. The host writes to it also (a menu row, a
///   start). Its reader plays a sound only for the live audio owner
///   (`ActiveAudioSelection::accepts_request_owner`), so a sound of the
///   session that ended is refused there.
pub fn end_presentation_effects_with_their_session(app: &mut App) {
    use ambition_platformer2d_shared_tangle::lifecycle::{
        clear_message_at_session_activation, keep_message_across_session_activation,
    };
    struct End;
    impl PresentationEffectVisitor for End {
        fn visit<M: Message>(&mut self, app: &mut App) {
            clear_message_at_session_activation::<M>(app);
        }
    }
    for_each_presentation_effect(app, &mut End);
    keep_message_across_session_activation::<ambition_sfx::OwnedSfxMessage>(app);
}

#[cfg(test)]
mod tests;
