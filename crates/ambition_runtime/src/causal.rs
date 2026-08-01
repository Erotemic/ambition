//! The ECS side of causal recording.
//!
//! `ambition_causal` is Bevy-free on purpose — every domain can depend on it
//! without depending on any other domain, which is what lets an explanation
//! survive a composition that has movement and no combat. This module is the
//! HOST adapter: the plugin, the tick stamp, and the one fact only a host can
//! publish.
//!
//! ⚠ **the resource itself lives in `ambition_causal`** (behind its `bevy`
//! feature, which is `bevy_ecs` alone), not here. A movement fact published from
//! `ambition_actors` must not require the runtime crate, and a `CausalRecording`
//! owned by a host would have forced exactly that.
//!
//! ## Why a resource and not the sink
//!
//! `ambition_causal`'s scoped sink is THREAD-LOCAL. That is sound for the pure
//! call tree it was built for (the fighter's decision, five hops below any
//! system, driven from one thread by `ladder_probe` or a test) and unsound
//! here: Bevy runs systems across worker threads, so a system publishing into a
//! sink opened on the main thread would publish into nothing.
//!
//! `ambition_causal::facts_lost_offthread()` counts exactly that, and
//! [`assert_no_offthread_loss`] is how a host turns the count into a failure
//! rather than a mystery. **A system publishes through
//! `ResMut<CausalRecording>`**, which is sound and — because Bevy's schedule
//! order is deterministic — also ordered.

use ambition_causal::{CausalFact, CausalRecording, Execution, FactDetail, RecordingPolicy, domains};
use bevy::prelude::*;


/// Install causal recording.
///
/// ⚠ **`RecordingPolicy::Off` is the default and the plugin does not change
/// it.** Installing the plugin makes recording POSSIBLE; a caller turns it on
/// for the domains it is investigating. An instrument that is on by default is
/// an instrument somebody turns off, and then it is not there when needed.
pub struct CausalPlugin;

impl Plugin for CausalPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CausalRecording>().add_systems(
            bevy::app::Last,
            (stamp_causal_tick, record_execution_identity).chain(),
        );
    }
}

/// Stamp the tick every subsequent fact belongs to.
///
/// The host is the only thing that knows the world's clock. A domain five hops
/// down does not, and a decision counter guessed there would be a second clock
/// nothing else could join against.
pub fn stamp_causal_tick(time: Option<Res<ambition_time::SimTick>>, mut log: ResMut<CausalRecording>) {
    if let Some(tick) = time {
        log.set_tick(tick.get());
    }
}

/// **Was this tick original execution or rollback resimulation?**
///
/// One of the inspector's required questions, and the one no domain below the
/// host can answer: `SimulationReplayState` and the session generation are
/// facts about the HOST's relationship to time, not about any body.
///
/// Published with no subject, so it explains every body on that tick —
/// a resimulated frame is resimulated for all of them.
///
/// ⚠ **the fact records the generation as well as the flag.** Frames restart at
/// zero on every session, so a tick number alone cannot tell a restart from a
/// rewind — the same reason `RollbackHealth` had to start carrying one.
pub fn record_execution_identity(
    replay: Option<Res<ambition_platformer_primitives::schedule::SimulationReplayState>>,
    boundary: Option<Res<ambition_engine_core::confirmed_frame::ConfirmedFrameBoundary>>,
    mut log: ResMut<CausalRecording>,
) {
    if !log.is_recording() {
        return;
    }
    let execution = if replay.is_some_and(|replay| replay.replaying_history) {
        Execution::Resimulated
    } else {
        Execution::Original
    };
    // The generation is stamped on the boundary a session install writes, so it
    // is present exactly while a session is. Absent = no rollback host, which
    // is generation zero and honestly so.
    let generation = boundary.map(|boundary| boundary.session).unwrap_or(0);
    log.record(
        CausalFact::new(
            domains::ROLLBACK,
            0,
            FactDetail::new(
                "tick_execution",
                match execution {
                    Execution::Original => "original execution",
                    Execution::Resimulated => "rollback resimulation",
                },
            ),
        )
        .executed(execution)
        .in_generation(generation as u32)
        .field("resimulated", execution == Execution::Resimulated)
        .field("generation", generation as i64),
    );
}

/// The movement-intent observer, re-exported so a host installs the plugin and
/// the publishers through ONE path and cannot get half of them.
pub use ambition_actors::causal::record_player_movement_intent;

/// Turn the off-thread loss counter into a failure.
///
/// Call it after driving an app with recording on. A non-zero count means some
/// domain published through the thread-local sink from a worker thread and the
/// fact is gone — which would otherwise read as "that domain did not act".
pub fn assert_no_offthread_loss() {
    let lost = ambition_causal::facts_lost_offthread();
    assert_eq!(
        lost, 0,
        "{lost} causal fact(s) were published on a thread with no sink. A domain publishing \
         from an ECS system must use `ResMut<CausalRecording>`, not the thread-local sink."
    );
}

/// Turn recording on for a set of domains.
pub fn record_domains(app: &mut App, policy: RecordingPolicy) {
    app.world_mut()
        .resource_mut::<CausalRecording>()
        .set_policy(policy);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_causal::SubjectKey;

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(CausalPlugin);
        app.insert_resource(ambition_time::SimTick(0));
        app
    }

    #[test]
    fn an_original_tick_and_a_resimulated_one_are_different_facts() {
        use ambition_platformer_primitives::schedule::SimulationReplayState;

        let mut app = app();
        record_domains(&mut app, RecordingPolicy::All);
        app.insert_resource(SimulationReplayState {
            replaying_history: false,
        });
        app.world_mut().resource_mut::<ambition_time::SimTick>().0 = 7;
        app.update();

        // Any subject: the fact is about the world, so it explains every body.
        let anybody = SubjectKey::Sim("fighter_1".into());
        let explanation = app.world().resource::<CausalRecording>().explain(7, &anybody);
        assert_eq!(
            explanation.execution(),
            Some(Execution::Original),
            "a tick nobody replayed is original"
        );

        app.world_mut()
            .resource_mut::<SimulationReplayState>()
            .replaying_history = true;
        app.world_mut().resource_mut::<ambition_time::SimTick>().0 = 8;
        app.update();
        let explanation = app.world().resource::<CausalRecording>().explain(8, &anybody);
        assert_eq!(
            explanation.execution(),
            Some(Execution::Resimulated),
            "a replayed tick says so — the thing the old text trace explicitly could not"
        );
        assert_no_offthread_loss();
    }

    #[test]
    fn recording_off_is_the_default_and_costs_nothing() {
        let mut app = app();
        app.update();
        assert!(
            app.world().resource::<CausalRecording>().is_empty(),
            "installing the plugin makes recording POSSIBLE, never automatic"
        );
    }

    #[test]
    fn the_tick_the_host_stamps_is_the_tick_the_facts_carry() {
        let mut app = app();
        record_domains(&mut app, RecordingPolicy::All);
        app.world_mut().resource_mut::<ambition_time::SimTick>().0 = 41;
        app.update();
        let log = app.world().resource::<CausalRecording>();
        assert!(
            log.facts().all(|fact| fact.tick == 41),
            "one clock — a domain that guessed its own would be unjoinable"
        );
    }
}
