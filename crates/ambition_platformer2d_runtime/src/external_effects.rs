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

impl<M: Message> Resource for ExternalEffectJournal<M> {}

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
pub fn open_sim_effect_outbox<M: Message>(
    mut messages: ResMut<Messages<M>>,
    mut journal: ResMut<ExternalEffectJournal<M>>,
) {
    journal.lifted = Some(std::mem::take(&mut *messages));
}

/// Stamp everything this pass produced with the frame that produced it, and
/// restore the channel [`open_sim_effect_outbox`] lifted aside.
pub fn journal_sim_effects<M: Message>(
    boundary: Res<ConfirmedFrameBoundary>,
    mut messages: ResMut<Messages<M>>,
    mut journal: ResMut<ExternalEffectJournal<M>>,
) {
    let intents: Vec<M> = messages.drain().collect();
    if let Some(lifted) = journal.lifted.take() {
        *messages = lifted;
    }
    journal.record(boundary.current, boundary.session, intents);
}

/// Hand confirmed frames to the ordinary presentation consumers.
pub fn release_confirmed_effects<M: Message>(
    boundary: Res<ConfirmedFrameBoundary>,
    mut messages: ResMut<Messages<M>>,
    mut journal: ResMut<ExternalEffectJournal<M>>,
) {
    let released = journal.take_confirmed(&boundary);
    if !released.is_empty() {
        messages.write_batch(released);
    }
}

/// Drop the abandoned branch's intents when the host restores an earlier frame.
///
/// Reads the restored frame from [`ConfirmedFrameBoundary::current`], which the
/// rollback bridge republishes at `LoadWorld` for exactly this reason — so this
/// module never needs to name a GGRS type.
pub fn discard_abandoned_predictions<M: Message>(
    boundary: Res<ConfirmedFrameBoundary>,
    mut journal: ResMut<ExternalEffectJournal<M>>,
) {
    journal.discard_after(boundary.current);
}

/// Quarantines one effect family. Add one per presentation-facing message type.
///
/// Every system is gated on [`ConfirmedFrameBoundary`] existing, so installing
/// this on a host that never speculates is inert rather than merely harmless.
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

        let sim = app.sim_schedule();
        let speculating = resource_exists::<ConfirmedFrameBoundary>;

        // A host that composes rollback WITHOUT presentation (a headless capability host,
        // `examples/capability_demo`'s GGRS round-trip) failed parameter validation on frame one,
        // naming a message type it had never heard of.
        //
        // `add_message` is guarded by `contains_resource`, so this is idempotent
        // and changes nothing for a composition that already registered it.
        app.add_message::<M>()
            .init_resource::<ExternalEffectJournal<M>>()
            .add_systems(
                sim,
                open_sim_effect_outbox::<M>
                    .in_set(ExternalEffectSet::OpenOutbox)
                    .before(GameplaySimulationRoot)
                    .run_if(speculating),
            )
            .add_systems(
                sim,
                journal_sim_effects::<M>
                    .in_set(ExternalEffectSet::Journal)
                    .after(GameplaySimulationRoot)
                    .run_if(speculating),
            )
            .add_systems(
                PreUpdate,
                release_confirmed_effects::<M>
                    .in_set(ExternalEffectSet::Release)
                    .run_if(speculating),
            );

        // `LoadWorld` only exists under a rollback host.
    }
}

/// Install the abandoned-branch discard into the host's restore schedule.
///
/// Separate from [`ExternalEffectQuarantinePlugin`] because only a rollback host
/// has a restore schedule to install it into.
pub fn quarantine_discard_on_load<M: Message>(app: &mut App, load_schedule: impl ScheduleLabel) {
    app.add_systems(
        load_schedule,
        discard_abandoned_predictions::<M>
            .in_set(ExternalEffectSet::DiscardAbandoned)
            .run_if(resource_exists::<ConfirmedFrameBoundary>),
    );
}

/// Quarantine every effect family whose consumer lives outside the simulation.
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
/// | `VfxMessage` | `vfx_spawn_messages`, `spawn_slash_effects` (`Update`) | spawns visuals |
/// | `FxRequest` | `process_fx_requests` (`Update`) | fans out to the two above |
/// | `FireworksRequest` | `process_fireworks_requests` (`Update`) | spawns a visual sequence |
/// | `DebrisBurstMessage` | `physics_spawn_debris_messages` (`Update`) | spawns physics debris |
/// | `CameraShakeRequest` | `apply_camera_shake_requests` (`Update`) | moves the screen |
///
/// Deliberately absent: `EffectRequest` and `ProjectileSpawnRequest`, whose readers are
/// all sim-side despite the effect-shaped names.
///
/// The two presentation-side writers in the fan-out chain (`FxRequest`
/// and `VfxMessage` are also written by `ambition_render`'s `Update` systems)
/// need no special handling: they run after the release, so what they produce is
/// already downstream of the confirmed boundary and flows straight through.
pub fn quarantine_presentation_effects(app: &mut App, load_schedule: impl ScheduleLabel + Clone) {
    use ambition_platformer2d_shared_tangle::camera_ease::CameraShakeRequest;
    use ambition_vfx::vfx::DebrisBurstMessage;
    use ambition_vfx::vfx::KnockoutBeatRequested;
    use ambition_vfx::{FireworksRequest, FxRequest, VfxMessage};

    app.add_plugins((
        ExternalEffectQuarantinePlugin::<ambition_sfx::OwnedSfxMessage>::default(),
        ExternalEffectQuarantinePlugin::<VfxMessage>::default(),
        ExternalEffectQuarantinePlugin::<FxRequest>::default(),
        ExternalEffectQuarantinePlugin::<FireworksRequest>::default(),
        ExternalEffectQuarantinePlugin::<DebrisBurstMessage>::default(),
        ExternalEffectQuarantinePlugin::<CameraShakeRequest>::default(),
        ExternalEffectQuarantinePlugin::<KnockoutBeatRequested>::default(),
    ));

    quarantine_discard_on_load::<ambition_sfx::OwnedSfxMessage>(app, load_schedule.clone());
    quarantine_discard_on_load::<VfxMessage>(app, load_schedule.clone());
    quarantine_discard_on_load::<FxRequest>(app, load_schedule.clone());
    quarantine_discard_on_load::<FireworksRequest>(app, load_schedule.clone());
    quarantine_discard_on_load::<DebrisBurstMessage>(app, load_schedule.clone());
    quarantine_discard_on_load::<CameraShakeRequest>(app, load_schedule.clone());
    quarantine_discard_on_load::<KnockoutBeatRequested>(app, load_schedule);
}

#[cfg(test)]
mod tests;
