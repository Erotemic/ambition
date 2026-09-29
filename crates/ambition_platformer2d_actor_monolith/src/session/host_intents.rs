//! Host intents enter the simulation on a stamped tick.
//!
//! A menu press is a host fact: it happens in `Update`, outside the rewinding
//! schedule. A write from there into simulation state is lost on a rewind. A
//! rollback-registered flag is restored to its snapshot, and a message is
//! spent on a speculative frame and not raised again when that frame is
//! simulated again.
//!
//! This ledger keeps the intent outside rollback state, stamped with the
//! first simulation tick that acts on it and with the gameplay session it
//! belongs to. The simulation releases the intent into the ordinary message
//! channel each time it simulates that tick, so the first pass and every
//! resimulation see the same input. The ledger forgets a record when no
//! rewind can reach its tick again.
//!
//! The same shape carries narrative facts from dialogue
//! (`ambition_conversation::NarrativeInputLedger`), gated by the live
//! conversation. Here the gate is the live gameplay session: an intent from a
//! session that has ended does not reach the next one.
//!
//! This is a LOCAL ingress. A peer does not see the ledger, so an intent that
//! must agree between peers rides the seat's `ControlFrame` instead.

use bevy::ecs::message::{Message, Messages};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use ambition_platformer2d_core::ConfirmedFrameBoundary;
use ambition_platformer2d_shared_tangle::lifecycle::{LiveSessionScope, SessionScopeId};

#[derive(Clone, Debug, PartialEq)]
struct StampedHostIntent<M> {
    /// The gameplay session the intent was made in.
    scope: SessionScopeId,
    /// The one `SimTick` on which the simulation acts on it.
    from_tick: u64,
    payload: M,
}

/// Host intents of one type, waiting for the tick they apply on.
///
/// ⛔ Never register this for rollback. It is the record of what arrived from
/// outside the timeline, and a rewind that erased it would erase the input a
/// resimulated tick must see again.
#[derive(Resource)]
pub struct HostIntentLedger<M: Message> {
    records: Vec<StampedHostIntent<M>>,
}

impl<M: Message> Default for HostIntentLedger<M> {
    fn default() -> Self {
        Self {
            records: Vec::new(),
        }
    }
}

impl<M: Message + Clone> HostIntentLedger<M> {
    /// How many intents are waiting. Bounded by the replay horizon.
    pub fn depth(&self) -> usize {
        self.records.len()
    }

    fn record(&mut self, scope: SessionScopeId, from_tick: u64, payload: M) {
        self.records.push(StampedHostIntent {
            scope,
            from_tick,
            payload,
        });
    }

    /// The intents the simulation acts on at `now` in session `live`.
    fn release(&self, now: u64, live: SessionScopeId) -> Vec<M> {
        self.records
            .iter()
            .filter(|record| record.from_tick == now && record.scope == live)
            .map(|record| record.payload.clone())
            .collect()
    }

    /// Forget records that no rewind can reach, and records of other sessions.
    fn prune(&mut self, now: u64, prediction_distance: u64, live: Option<SessionScopeId>) {
        let horizon = now.saturating_sub(prediction_distance);
        self.records
            .retain(|record| Some(record.scope) == live && record.from_tick >= horizon);
    }
}

/// The first tick the simulation may act on an intent made now.
///
/// The NEXT tick: the host observes input in `Update`, after this frame's
/// simulation has run, so the tick it can still act on is the one after.
fn next_tick(tick: Option<&ambition_time::SimTick>) -> u64 {
    tick.map_or(0, |tick| tick.0.saturating_add(1))
}

/// Write a host intent into the timeline. (host)
///
/// The one road a menu uses to ask the simulation for something, in place of
/// a `MessageWriter` or a `ResMut` on simulation state.
#[derive(SystemParam)]
pub struct HostIntentWriter<'w, 's, M: Message + Clone> {
    scope: LiveSessionScope<'w, 's>,
    tick: Option<Res<'w, ambition_time::SimTick>>,
    ledger: ResMut<'w, HostIntentLedger<M>>,
}

impl<M: Message + Clone> HostIntentWriter<'_, '_, M> {
    /// Record `payload` for the next simulation tick of the live session.
    ///
    /// With no live gameplay session there is no simulation to act on it, so
    /// the intent is logged and dropped rather than applied to a later one.
    pub fn write(&mut self, payload: M) {
        let Some(scope) = self.scope.get() else {
            warn!(
                target: "ambition_platformer2d::host_intents",
                "a host intent arrived with no live gameplay session; dropping {}",
                std::any::type_name::<M>(),
            );
            return;
        };
        let from_tick = next_tick(self.tick.as_deref());
        self.ledger.record(scope, from_tick, payload);
    }
}

/// [`HostIntentWriter::write`] from exclusive world access, for a host step
/// that holds the `World` (a harness, a test driver).
pub fn write_host_intent<M: Message + Clone>(world: &mut World, payload: M) {
    let Some(scope) = ambition_platformer2d_shared_tangle::lifecycle::live_session_scope(world)
    else {
        warn!(
            target: "ambition_platformer2d::host_intents",
            "a host intent arrived with no live gameplay session; dropping {}",
            std::any::type_name::<M>(),
        );
        return;
    };
    let from_tick = next_tick(world.get_resource::<ambition_time::SimTick>());
    world
        .get_resource_or_init::<HostIntentLedger<M>>()
        .record(scope, from_tick, payload);
}

/// Hand the simulation the intents for this tick. (sim)
///
/// At the head of the simulation, before the clock names the next tick, so the
/// tick compared is the one the writer stamped.
pub fn release_host_intents<M: Message + Clone>(
    scope: LiveSessionScope,
    tick: Option<Res<ambition_time::SimTick>>,
    ledger: Res<HostIntentLedger<M>>,
    mut messages: ResMut<Messages<M>>,
) {
    let Some(live) = scope.get() else {
        return;
    };
    let now = tick.map_or(0, |tick| tick.0);
    let released = ledger.release(now, live);
    if !released.is_empty() {
        messages.write_batch(released);
    }
}

/// Forget what can never be simulated again. (host)
///
/// Not in the simulation schedule: a resimulated tick that erased its own
/// input would reach a different history than the pass it reproduces.
pub fn prune_host_intents<M: Message + Clone>(
    scope: LiveSessionScope,
    tick: Option<Res<ambition_time::SimTick>>,
    // Absent on a host that does not speculate: a passed tick is settled.
    boundary: Option<Res<ConfirmedFrameBoundary>>,
    mut ledger: ResMut<HostIntentLedger<M>>,
) {
    let now = tick.map_or(0, |tick| tick.0);
    let prediction_distance = boundary.map_or(0, |boundary| {
        u64::try_from(boundary.current.saturating_sub(boundary.confirmed)).unwrap_or(0)
    });
    ledger.prune(now, prediction_distance, scope.get());
}

/// Registers one host-intent type: its channel, its ledger, the release and
/// the prune.
pub struct HostIntentPlugin<M: Message + Clone> {
    marker: std::marker::PhantomData<fn() -> M>,
}

impl<M: Message + Clone> Default for HostIntentPlugin<M> {
    fn default() -> Self {
        Self {
            marker: std::marker::PhantomData,
        }
    }
}

impl<M: Message + Clone> Plugin for HostIntentPlugin<M> {
    fn build(&self, app: &mut App) {
        use ambition_platformer2d_shared_tangle::schedule::{
            GameplaySimulationRoot, Platformer2dSimulationPhaseMonolith, SimClockHead,
            SimScheduleExt as _,
        };
        let sim = app.sim_schedule();
        app.add_message::<M>()
            .init_resource::<HostIntentLedger<M>>()
            .add_systems(
                sim,
                // After the clock names this step's tick, so an intent stamped
                // for tick N+1 is released in the step that IS tick N+1, and
                // before the phase whose systems read it.
                release_host_intents::<M>
                    .in_set(GameplaySimulationRoot)
                    .after(SimClockHead)
                    .before(Platformer2dSimulationPhaseMonolith::CoreSimulation),
            )
            .add_systems(Update, prune_host_intents::<M>);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Message, Clone, Debug, PartialEq, Eq)]
    struct Asked(u32);

    fn scope(raw: u64) -> SessionScopeId {
        SessionScopeId(raw)
    }

    /// Every simulation of the stamped tick sees the intent, so a rewind
    /// across it cannot lose it, and no other tick sees it.
    #[test]
    fn an_intent_is_released_on_its_tick_each_time_that_tick_is_simulated() {
        let mut ledger = HostIntentLedger::<Asked>::default();
        ledger.record(scope(1), 10, Asked(7));
        assert_eq!(ledger.release(9, scope(1)), vec![]);
        assert_eq!(ledger.release(10, scope(1)), vec![Asked(7)]);
        assert_eq!(ledger.release(10, scope(1)), vec![Asked(7)], "the resimulation");
        assert_eq!(ledger.release(11, scope(1)), vec![]);
    }

    /// An intent belongs to the session it was made in.
    #[test]
    fn an_intent_from_another_session_is_not_released_and_is_pruned() {
        let mut ledger = HostIntentLedger::<Asked>::default();
        ledger.record(scope(1), 10, Asked(7));
        assert_eq!(ledger.release(10, scope(2)), vec![]);
        ledger.prune(10, 8, Some(scope(2)));
        assert_eq!(ledger.depth(), 0);
    }

    /// A record stays while a rewind can still reach its tick.
    #[test]
    fn a_record_is_kept_inside_the_prediction_window() {
        let mut ledger = HostIntentLedger::<Asked>::default();
        ledger.record(scope(1), 10, Asked(7));
        ledger.prune(14, 4, Some(scope(1)));
        assert_eq!(ledger.depth(), 1, "tick 10 is still inside a window of 4 from 14");
        ledger.prune(15, 4, Some(scope(1)));
        assert_eq!(ledger.depth(), 0);
    }
}
