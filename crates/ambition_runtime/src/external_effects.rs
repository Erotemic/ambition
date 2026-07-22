//! Holding external effects at the confirmed-frame boundary.
//!
//! A rollback host simulates the same frame more than once. Gameplay is
//! *supposed* to run again — that is what makes rollback correct — but a sound
//! that already reached the speakers cannot be unplayed, and a particle that
//! already spawned cannot be unspawned. Those effects leave the process, so
//! they must be keyed to the timeline the host has actually settled rather than
//! to whatever the simulation currently believes.
//!
//! # What this replaces
//!
//! The earlier [`ambition_sfx::SfxEmissionGate`] answered *"has this frame been
//! simulated before?"* and dropped the emission if so. That kills the duplicate
//! a local rollback produces, but under predicted remote input it is wrong in a
//! second way: the predicted pass emits sound A and A reaches the speakers; the
//! real input arrives and forces a rollback; the corrected pass emits sound B —
//! and the gate suppresses B, because the frame ran before. The duplicate is
//! gone, the phantom is kept, and the correction is lost.
//!
//! This module is the different mechanism that was owed. Nothing is suppressed
//! and nothing is decided at emit time. Effects are *deferred*.
//!
//! # The mechanism
//!
//! Each advance runs with the effect channel **swapped out for an empty one**:
//!
//! 1. **Open an outbox** at the start of each advance: the live channel is
//!    lifted out whole and replaced with a fresh empty one, so anything the sim
//!    writes lands in isolation.
//! 2. **Journal** at the end of each advance: the outbox is drained, stamped
//!    with the frame that produced it, and stored under that frame — *replacing*
//!    any intents an earlier pass recorded for it. Re-simulating a frame that
//!    now produces nothing therefore erases the phantom, which is the half a
//!    boolean gate structurally cannot do. The lifted channel is then put back
//!    exactly as it was.
//! 3. **Release** once the frame is confirmed: the intents are written into
//!    that same restored channel, where the ordinary presentation consumers read
//!    them, unchanged and unaware any of this happened.
//! 4. **Discard** on load: intents for frames after the one being restored came
//!    from a timeline that has been abandoned, so they are dropped rather than
//!    left to be released.
//!
//! Frames are released in ascending order, so effects reach presentation in
//! simulation order even when several frames confirm at once.
//!
//! # Why swap rather than clear
//!
//! The first version of this cleared the channel at the start of each advance
//! instead, which is simpler and wrong. **The sim is not the only writer.** Menu
//! and shell SFX, and the render-side explosion/fireworks fan-out, write the
//! same channels from `Update`; a clear at the next advance discarded whatever
//! they had queued, so a rollback host silently swallowed menu sounds. The
//! shipped test that caught it —
//! `app_it::shell_host_rendered::provider_relative_sfx_resolves_the_real_source_and_rejects_stale_work`
//! — writes a message directly and asserts the consumer counted it.
//!
//! Draining and restoring the contents would not work either: a message already
//! *consumed* by a reader is still physically present in Bevy's older buffer,
//! and re-writing it hands it to that reader a second time. Only the reader's
//! own cursor knows what it has seen. Lifting the whole [`Messages`] value —
//! counters and both buffers together — and putting it back untouched is what
//! keeps every reader's cursor meaningful, because from their side nothing
//! happened at all.
//!
//! # Cost, honestly
//!
//! An effect is delayed by however far confirmation lags simulation — bounded by
//! GGRS's prediction window, and zero when nothing is predicted. Every non-rollback
//! host (render-frame, fixed-tick, headless, every unit fixture) never installs
//! [`ConfirmedFrameBoundary`] at all, so none of these systems run and effects
//! fire the instant they are written, exactly as before this module existed.
//!
//! A released effect is an ordinary message in the ordinary channel, so it keeps
//! Bevy's usual two-frame lifetime and reaches a consumer that happened not to
//! run this frame. Deferral changes *when* an effect is handed over, and nothing
//! else about it.
//!
//! # What does NOT belong here
//!
//! Only effects whose consumer lives **outside** the simulation. Deferring a
//! message the sim itself reads would break the simulation: the consumer would
//! not see it on the pass that produced it, and would see it again on a later
//! frame it does not belong to. In particular [`ambition_vfx::EffectRequest`] is
//! *not* quarantined despite its name and its listing in the quarantine
//! work-list — all three of its readers (`apply_effects` spawning hitboxes,
//! `apply_summon_effects` spawning minions, `apply_enemy_projectile_effects`)
//! are sim-side. Same for `SpawnProjectile`. The test
//! `only_presentation_facing_effects_are_quarantined` pins the distinction.

use std::collections::BTreeMap;
use std::marker::PhantomData;

use bevy::ecs::message::{Message, Messages};
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;

use ambition_engine_core::ConfirmedFrameBoundary;

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
/// Deliberately **not** rollback state, and it must never be registered as
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
/// Paired with [`journal_sim_effects`], which puts it back. See the module docs
/// on why this is a swap and not a clear — the sim is not the only writer, and
/// discarding what the menu queued is how the first version of this broke.
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
        use ambition_platformer_primitives::schedule::{
            GameplaySimulationRoot, SimScheduleExt as _,
        };

        let sim = app.sim_schedule();
        let speculating = resource_exists::<ConfirmedFrameBoundary>;

        app.init_resource::<ExternalEffectJournal<M>>()
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

        // `LoadWorld` only exists under a rollback host. Registering the
        // discard against a schedule the host has not created would panic on a
        // fixed-tick app, so the bridge that owns that schedule installs it.
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
/// **This list is the classification.** A message belongs here when its reader
/// is presentation, persistence, or anything else the player observes directly;
/// it must stay out when the simulation itself reads it, because deferring such
/// a message would change what the simulation computes. The distinction is
/// pinned by `only_presentation_facing_effects_are_quarantined`.
///
/// | family | reader | why |
/// |---|---|---|
/// | `OwnedSfxMessage` | `audio_play_sfx_messages` (`Update`) | reaches the speakers |
/// | `VfxMessage` | `vfx_spawn_messages`, `spawn_slash_effects` (`Update`) | spawns visuals |
/// | `ExplosionRequest` | `process_explosion_requests` (`Update`) | fans out to the two above |
/// | `FireworksRequest` | `process_fireworks_requests` (`Update`) | spawns a visual sequence |
/// | `DebrisBurstMessage` | `physics_spawn_debris_messages` (`Update`) | spawns physics debris |
///
/// Deliberately absent: `EffectRequest` and `SpawnProjectile`, whose readers are
/// all sim-side despite the effect-shaped names.
///
/// The two presentation-side writers in the fan-out chain (`ExplosionRequest`
/// and `VfxMessage` are also written by `ambition_render`'s `Update` systems)
/// need no special handling: they run after the release, so what they produce is
/// already downstream of the confirmed boundary and flows straight through.
pub fn quarantine_presentation_effects(app: &mut App, load_schedule: impl ScheduleLabel + Clone) {
    use ambition_vfx::vfx::DebrisBurstMessage;
    use ambition_vfx::{ExplosionRequest, FireworksRequest, VfxMessage};

    app.add_plugins((
        ExternalEffectQuarantinePlugin::<ambition_sfx::OwnedSfxMessage>::default(),
        ExternalEffectQuarantinePlugin::<VfxMessage>::default(),
        ExternalEffectQuarantinePlugin::<ExplosionRequest>::default(),
        ExternalEffectQuarantinePlugin::<FireworksRequest>::default(),
        ExternalEffectQuarantinePlugin::<DebrisBurstMessage>::default(),
    ));

    quarantine_discard_on_load::<ambition_sfx::OwnedSfxMessage>(app, load_schedule.clone());
    quarantine_discard_on_load::<VfxMessage>(app, load_schedule.clone());
    quarantine_discard_on_load::<ExplosionRequest>(app, load_schedule.clone());
    quarantine_discard_on_load::<FireworksRequest>(app, load_schedule.clone());
    quarantine_discard_on_load::<DebrisBurstMessage>(app, load_schedule);
}

#[cfg(test)]
mod tests;
