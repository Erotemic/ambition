//! Bridges narrative inputs from non-rollback dialogue into deterministic simulation ticks.
//!
//! Each input is stamped with its conversation instance and the first simulation tick that may
//! consume it. The ledger releases records on that exact tick, only while their conversation is
//! still active, and prunes records once the replay horizon can no longer reach them.
//!
//! The ledger is host timeline bookkeeping, not rollback state: rewinding simulation must not
//! erase the external input that a resimulated tick needs to observe again.

use bevy::ecs::message::{Message, Messages};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use ambition_platformer2d_core::ConfirmedFrameBoundary;

use super::authority::ActiveConversation;
use super::instance::ConversationInstanceId;

/// One narrative fact, with everything the simulation needs to act on it at the
/// same moment in every replay.
#[derive(Clone, Debug, PartialEq, Eq)]
struct StampedNarrativeInput<M> {
    /// Which conversation said it.
    instance: ConversationInstanceId,
    /// The first — and, because the release is an edge, the only — `SimTick` on
    /// which the simulation acts on it.
    from_tick: u64,
    payload: M,
}

/// Narrative facts waiting for the tick they apply from.
///
/// One per payload type, so a game's own narrative vocabulary registers its own
/// ledger and this module names no content.
///
/// never register this for rollback. See the module docs: it is the record
/// of what arrived from outside, and rewinding an input erases it. The
/// `two_narrative_ends_in_one_window_both_replay` test fails rather than passing
/// quietly if it ever becomes rollback state.
#[derive(Resource)]
pub struct NarrativeInputLedger<M: Message> {
    records: Vec<StampedNarrativeInput<M>>,
}

impl<M: Message> Default for NarrativeInputLedger<M> {
    fn default() -> Self {
        Self {
            records: Vec::new(),
        }
    }
}

impl<M: Message + Clone> NarrativeInputLedger<M> {
    /// Write down what the narrative said, and when it applies from.
    pub fn record(&mut self, instance: ConversationInstanceId, from_tick: u64, payload: M) {
        self.records.push(StampedNarrativeInput {
            instance,
            from_tick,
            payload,
        });
    }

    /// Whether this instance already has a record here.
    ///
    /// a caller recording a genuine repeat (two `give_item` commands in one conversation) must
    /// NOT consult this.
    pub fn holds_for(&self, instance: &ConversationInstanceId) -> bool {
        self.records
            .iter()
            .any(|record| &record.instance == instance)
    }

    /// How many records are waiting. Bounded by the replay horizon; asserted by
    /// `the_ledger_depth_stays_within_the_replay_horizon`.
    pub fn depth(&self) -> usize {
        self.records.len()
    }

    /// What the simulation should act on right now.
    ///
    /// `now` is `None` in a composition with no timeline, and that is a
    /// different world rather than a missing value: with no `SimTick` there is no
    /// replay to reproduce anything for, so a record is delivered once and
    /// leaves. this fork is stated HERE, once, because the shape it replaces
    /// spelled it `tick.map_or(u64::MAX, ..)` at each reader — and a sentinel
    /// that means "act immediately" read as an ordinary comparison.
    fn release(&mut self, now: Option<u64>, live: Option<&ConversationInstanceId>) -> Vec<M> {
        let Some(live) = live else {
            return Vec::new();
        };
        let Some(now) = now else {
            let mut released = Vec::new();
            self.records.retain(|record| {
                if &record.instance == live {
                    released.push(record.payload.clone());
                    false
                } else {
                    true
                }
            });
            return released;
        };
        self.records
            .iter()
            .filter(|record| record.from_tick == now && &record.instance == live)
            .map(|record| record.payload.clone())
            .collect()
    }

    /// Drop records whose tick can never be simulated again.
    fn prune(&mut self, now: u64, prediction_distance: u64) {
        let horizon = now.saturating_sub(prediction_distance);
        self.records.retain(|record| record.from_tick >= horizon);
    }
}

/// Write down what the narrative just said. (presentation)
///
/// Bundles the three things every narrative writer needs — who is talking, what
/// tick it is, and where the record goes — so a Yarn command says
/// `narrative.write(..)` and nothing about stamping.
#[derive(SystemParam)]
pub struct NarrativeInputWriter<'w, M: Message + Clone> {
    conversation: Res<'w, ActiveConversation>,
    /// `Option` for the reason `prepare_the_match`'s own tick is: a composition
    /// with no timeline has no replay to disagree with, and a plain `Res` would
    /// panic in every one of them.
    tick: Option<Res<'w, ambition_time::SimTick>>,
    ledger: ResMut<'w, NarrativeInputLedger<M>>,
}

impl<M: Message + Clone> NarrativeInputWriter<'_, M> {
    /// Record `payload` against the live conversation, effective next tick.
    ///
    /// A command that fires with no conversation live is authored content
    /// reaching the runner outside one; it is logged and dropped rather than
    /// applied to whatever happens to be live later.
    pub fn write(&mut self, payload: M) {
        let Some(instance) = self.conversation.instance() else {
            warn!(
                target: "ambition_conversation",
                "a narrative command fired with no conversation live; dropping {}",
                std::any::type_name::<M>(),
            );
            return;
        };
        let from_tick = self.next_tick();
        self.ledger.record(instance.clone(), from_tick, payload);
    }

    /// Record `payload` only if this conversation has no record here yet — for a
    /// fact observed as a condition. See [`NarrativeInputLedger::holds_for`].
    pub fn write_once(&mut self, payload: M) {
        let Some(instance) = self.conversation.instance() else {
            return;
        };
        if self.ledger.holds_for(instance) {
            return;
        }
        let from_tick = self.next_tick();
        self.ledger.record(instance.clone(), from_tick, payload);
    }

    /// The first tick the simulation may act on something observed now.
    ///
    /// the NEXT tick, because presentation observes the runner in `Update`,
    /// after this frame's simulation has already run. Stamping the tick that has
    /// been simulated would make the original frame and its replay disagree
    /// about whether the fact was true during it — the same off-by-one
    /// `PreparedMatch::effective_from` exists to close.
    fn next_tick(&self) -> u64 {
        self.tick
            .as_ref()
            .map_or(0, |tick| tick.0.saturating_add(1))
    }
}

/// Hand the simulation what the narrative told it, on the tick it applies
/// from. (sim)
///
/// Runs at the head of the sim schedule and writes into the ordinary channel, so
/// consumers are unchanged and unaware any of this happened.
pub fn release_narrative_inputs<M: Message + Clone>(
    conversation: Res<ActiveConversation>,
    tick: Option<Res<ambition_time::SimTick>>,
    mut ledger: ResMut<NarrativeInputLedger<M>>,
    mut messages: ResMut<Messages<M>>,
) {
    let released = ledger.release(tick.map(|tick| tick.0), conversation.instance());
    if !released.is_empty() {
        messages.write_batch(released);
    }
}

/// Forget what can never be replayed again. (presentation/host)
///
/// not in the sim schedule, and that is not a placement preference. This
/// runs during resimulation if it is, and a replayed tick that erases its own
/// input reaches a different history than the run it is reproducing.
pub fn prune_narrative_inputs<M: Message + Clone>(
    tick: Option<Res<ambition_time::SimTick>>,
    // Absent on every non-speculating host, which means nothing is predicted and
    // a passed tick is settled the moment it passes.
    boundary: Option<Res<ConfirmedFrameBoundary>>,
    mut ledger: ResMut<NarrativeInputLedger<M>>,
) {
    let Some(now) = tick.map(|tick| tick.0) else {
        // No timeline: the release drains as it delivers, so there is nothing
        // here to age out.
        return;
    };
    let prediction_distance = boundary.map_or(0, |boundary| {
        u64::try_from(boundary.current.saturating_sub(boundary.confirmed)).unwrap_or(0)
    });
    ledger.prune(now, prediction_distance);
}

/// Registers one narrative-input family: the ledger, its channel, the release
/// and the prune.
///
/// Add one per payload type a game's Yarn vocabulary can produce.
pub struct NarrativeInputPlugin<M: Message + Clone> {
    marker: std::marker::PhantomData<fn() -> M>,
}

impl<M: Message + Clone> Default for NarrativeInputPlugin<M> {
    fn default() -> Self {
        Self {
            marker: std::marker::PhantomData,
        }
    }
}

impl<M: Message + Clone> Plugin for NarrativeInputPlugin<M> {
    fn build(&self, app: &mut App) {
        use ambition_platformer2d_shared_tangle::schedule::{
            GameplaySimulationRoot, Platformer2dSimulationPhaseMonolith, SimScheduleExt as _,
        };

        let sim = app.sim_schedule();
        // REGISTER THE CHANNEL THIS PLUGIN RELEASES INTO — the same lesson
        // `ExternalEffectQuarantinePlugin` records. Leaving it to whoever else
        // wants the message means it always works in a shipped app and nowhere
        // else. `add_message` is guarded by `contains_resource`, so this is
        // idempotent for a channel somebody already registered.
        app.add_message::<M>()
            .init_resource::<NarrativeInputLedger<M>>()
            // INSIDE the root set, at its head — the `ensure_sim_id`
            // placement, not the effect quarantine's `.before(root)` one. The
            // root carries the session gate, so a release outside it would hand
            // narrative facts to a frozen simulation at a title or loading route
            // and have them read later, at a tick that is not the one they were
            // stamped for.
            .add_systems(
                sim,
                release_narrative_inputs::<M>
                    .in_set(GameplaySimulationRoot)
                    .before(Platformer2dSimulationPhaseMonolith::CoreSimulation),
            )
            .add_systems(Update, prune_narrative_inputs::<M>);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Message, Clone, Debug, PartialEq, Eq)]
    struct Spoke(u32);

    fn instance(node: &str, tick: u64) -> ConversationInstanceId {
        ConversationInstanceId::mint(
            tick,
            node,
            None,
            None,
            &ambition_dialog::DialogueContext::scripted(),
        )
    }

    /// The release is an EDGE. A grant is not idempotent, so a level rule
    /// (`from_tick <= now`) would hand it over on every tick after the first.
    #[test]
    fn a_record_releases_on_its_tick_and_no_other() {
        let mut ledger = NarrativeInputLedger::<Spoke>::default();
        let chat = instance("chat", 5);
        ledger.record(chat.clone(), 10, Spoke(1));

        assert!(ledger.release(Some(9), Some(&chat)).is_empty(), "not yet");
        assert_eq!(ledger.release(Some(10), Some(&chat)), vec![Spoke(1)]);
        assert!(
            ledger.release(Some(11), Some(&chat)).is_empty(),
            "released twice: a level rule grants the item again every tick"
        );
        // and the tick REPLAYS: the record is still there, because nothing
        // marks it consumed.
        assert_eq!(
            ledger.release(Some(10), Some(&chat)),
            vec![Spoke(1)],
            "a resimulated tick must reach the same answer, so the record has to \
             survive its own delivery"
        );
    }

    /// A record from a branch the host abandoned does not reach the world.
    #[test]
    fn a_record_whose_conversation_is_not_live_never_releases() {
        let mut ledger = NarrativeInputLedger::<Spoke>::default();
        let abandoned = instance("chat", 5);
        let corrected = instance("chat", 5 + 1);
        ledger.record(abandoned, 10, Spoke(1));

        assert!(
            ledger.release(Some(10), Some(&corrected)).is_empty(),
            "the corrected branch opened a different conversation and inherited \
             the abandoned one's narrative effect"
        );
        assert!(
            ledger.release(Some(10), None).is_empty(),
            "nobody is talking, so nothing a conversation said applies"
        );
    }

    /// Two conversations' records coexist, which is the whole reason this is
    /// a ledger. The slot it replaces could hold one.
    #[test]
    fn records_from_two_conversations_both_survive() {
        let mut ledger = NarrativeInputLedger::<Spoke>::default();
        let first = instance("first", 100);
        let second = instance("second", 104);
        ledger.record(first.clone(), 103, Spoke(1));
        ledger.record(second.clone(), 106, Spoke(2));

        assert_eq!(ledger.release(Some(103), Some(&first)), vec![Spoke(1)]);
        assert_eq!(ledger.release(Some(106), Some(&second)), vec![Spoke(2)]);
    }

    /// The depth is bounded by the replay horizon, not by how long the
    /// session has been running.
    #[test]
    fn the_ledger_depth_stays_within_the_replay_horizon() {
        let mut ledger = NarrativeInputLedger::<Spoke>::default();
        // A conversation-heavy hour: one record per tick for 600 ticks.
        for tick in 0..600u64 {
            ledger.record(instance("chat", tick), tick, Spoke(tick as u32));
            // GGRS's maximum prediction window is eight frames.
            ledger.prune(tick, 8);
        }
        assert!(
            ledger.depth() <= 9,
            "the ledger grew with the session rather than with the window: {} \
             records held",
            ledger.depth()
        );
    }

    /// No timeline is no replay: the record is delivered once and leaves,
    /// which is exactly what the message-based seam did.
    #[test]
    fn a_composition_with_no_timeline_delivers_once() {
        let mut ledger = NarrativeInputLedger::<Spoke>::default();
        let chat = instance("chat", 0);
        ledger.record(chat.clone(), 0, Spoke(1));

        assert_eq!(ledger.release(None, Some(&chat)), vec![Spoke(1)]);
        assert!(
            ledger.release(None, Some(&chat)).is_empty(),
            "a fixture with a standing clock would otherwise re-deliver forever"
        );
    }

    /// `holds_for` is per conversation, not per ledger — the guard an
    /// observer of a CONDITION needs, and the one a repeated effect must not use.
    #[test]
    fn holds_for_answers_about_one_conversation() {
        let mut ledger = NarrativeInputLedger::<Spoke>::default();
        let first = instance("first", 100);
        let second = instance("second", 104);
        ledger.record(first.clone(), 103, Spoke(1));

        assert!(ledger.holds_for(&first));
        assert!(
            !ledger.holds_for(&second),
            "a record for one conversation reported another as already observed, \
             so the second one's ending would never be written down"
        );
    }
}
