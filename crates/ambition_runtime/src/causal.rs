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


/// Where publishers run, relative to the frame stamp.
///
/// A publisher outside this set may run before the stamp and carry the previous
/// frame's identity — which is not a hypothetical: it is what the
/// parallel-schedule proof found the first time this plugin was written.
#[derive(bevy::prelude::SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RecordingSet {
    Publish,
}

/// Install causal recording.
///
/// ⚠ **`RecordingPolicy::Off` is the default and the plugin does not change
/// it.** Installing the plugin makes recording POSSIBLE; a caller turns it on
/// for the domains it is investigating. An instrument that is on by default is
/// an instrument somebody turns off, and then it is not there when needed.
pub struct CausalPlugin;

impl Plugin for CausalPlugin {
    fn build(&self, app: &mut App) {
        // ⛔ **The resource only.** The stamp does NOT go in `Last`, which is
        // where it started and where the parallel-schedule proof caught it: it
        // ran AFTER every publisher, so a fact published during `Update` carried
        // the previous frame's tick. The stamp belongs at the HEAD of the
        // schedule its publishers run in, which is the sim schedule, and only
        // `player_schedule` knows what that is.
        app.init_resource::<CausalRecording>();
    }
}

/// Stamp the frame every subsequent fact belongs to: its tick, whether the host
/// is replaying it, and which lifecycle generation it is in.
///
/// **Runs at the HEAD of the sim schedule**, before any publisher. The host is
/// the only thing that knows any of these: a domain five hops down does not
/// know the world's clock, and it certainly does not know whether the host is
/// resimulating — a movement fact that guessed `Original` would make a replayed
/// tick indistinguishable from its original, which is the one distinction the
/// inspector must never lose.
pub fn stamp_causal_frame(
    time: Option<Res<ambition_time::SimTick>>,
    replay: Option<Res<ambition_platformer_primitives::schedule::SimulationReplayState>>,
    boundary: Option<Res<ambition_engine_core::confirmed_frame::ConfirmedFrameBoundary>>,
    mut log: ResMut<CausalRecording>,
) {
    if let Some(tick) = time {
        log.set_tick(tick.get());
    }
    let execution = if replay.is_some_and(|replay| replay.replaying_history) {
        Execution::Resimulated
    } else {
        Execution::Original
    };
    let generation = boundary.map(|boundary| boundary.session).unwrap_or(0) as u32;
    log.set_frame(execution, generation);
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
pub fn record_execution_identity(mut log: ResMut<CausalRecording>) {
    if !log.is_recording() {
        return;
    }
    // Read back what `stamp_causal_frame` just decided, so this fact and every
    // other fact this frame agree by construction rather than by two systems
    // reading the same resources and hoping.
    let (execution, generation) = log.frame().unwrap_or((Execution::Original, 0));
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
        .in_generation(generation)
        .field("resimulated", execution == Execution::Resimulated)
        .field("generation", i64::from(generation)),
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
        // The head-of-schedule stamp, exactly where `player_schedule` installs
        // it in a real host: BEFORE anything publishes. Putting it after was the
        // bug the parallel proof caught.
        app.add_systems(
            Update,
            stamp_causal_frame.before(RecordingSet::Publish),
        );
        app.configure_sets(Update, RecordingSet::Publish);
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
        app.add_systems(Update, record_execution_identity.in_set(RecordingSet::Publish));
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

    /// **THE PARALLEL-SCHEDULE PROOF.** (GPT 5.6 review, requested explicitly.)
    ///
    /// The concern is real and was worth proving rather than reasoning about:
    /// `ambition_causal`'s thread-local sink cannot collect from Bevy's worker
    /// threads, so if the ECS path shared that mechanism the whole inspector
    /// integration would be silently lossy.
    ///
    /// It does not share it. A system publishes through
    /// `ResMut<CausalRecording>`, and that is sound for a reason stronger than
    /// "it happens to work": Bevy will not run two systems with conflicting
    /// resource access concurrently, so the publishes are SERIALISED by the
    /// scheduler itself and ordered by it deterministically.
    ///
    /// This drives several publishers with disjoint component access — the case
    /// most likely to be parallelised — under a real `App::update()`, and
    /// asserts every fact arrived AND that nothing went to the sink.
    #[test]
    fn facts_survive_a_parallel_schedule() {
        #[derive(bevy::prelude::Component)]
        struct Alpha;
        #[derive(bevy::prelude::Component)]
        struct Beta;
        #[derive(bevy::prelude::Component)]
        struct Gamma;

        macro_rules! publisher {
            ($name:ident, $marker:ty, $kind:literal, $seat:expr) => {
                fn $name(
                    mut log: ResMut<CausalRecording>,
                    // Disjoint component access, so the scheduler is free to
                    // consider these for parallel execution.
                    q: Query<&$marker>,
                ) {
                    for _ in &q {
                        log.record(
                            CausalFact::new(
                                domains::MOVEMENT,
                                0,
                                FactDetail::new($kind, $kind),
                            )
                            .about(ambition_causal::SubjectKey::Seat($seat)),
                        );
                    }
                }
            };
        }
        publisher!(publish_alpha, Alpha, "alpha", 0);
        publisher!(publish_beta, Beta, "beta", 0);
        publisher!(publish_gamma, Gamma, "gamma", 0);

        ambition_causal::reset_lost_offthread();
        let mut app = app();
        record_domains(&mut app, RecordingPolicy::All);
        for _ in 0..8 {
            app.world_mut().spawn(Alpha);
            app.world_mut().spawn(Beta);
            app.world_mut().spawn(Gamma);
        }
        app.add_systems(
            Update,
            (publish_alpha, publish_beta, publish_gamma).in_set(RecordingSet::Publish),
        );
        app.world_mut().resource_mut::<ambition_time::SimTick>().0 = 5;
        app.update();

        let log = app.world().resource::<CausalRecording>();
        let explanation = log.explain(5, &ambition_causal::SubjectKey::Seat(0));
        for kind in ["alpha", "beta", "gamma"] {
            assert_eq!(
                explanation.all(kind).count(),
                8,
                "every `{kind}` fact reached ONE coherent explanation — a thread-local \
                 collector would have dropped whichever ran off the main thread"
            );
        }
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
