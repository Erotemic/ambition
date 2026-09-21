//! Reconcile the authored-verdict ring with the host's rollback timeline.
//!
//! ⛔⛤ **THE RING KNEW ABOUT ROLLBACK AND THE HOST NEVER TOLD IT ANYTHING —
//! REVIEWED 2026-09-20.** `AuthoredVerdictLog` carried a `VerdictStamp` with a
//! `confirmed` flag and a `confirm_through` that re-stamps a settled frame,
//! and `confirm_through`'s only caller was its own unit test. In a real host
//! a verdict recorded on a speculative frame stayed marked speculative
//! forever: the frame settled, nothing told the ring, and the diagnostic went
//! on reporting a real historical event as a guess. A mechanism wired to
//! nothing is worse than an absent one, because the type documents a
//! behaviour the engine does not have.
//!
//! ⚠ **THE RING IS OPT-IN AND STAYS OPT-IN.** Nothing composes
//! `AuthoredVerdictLog`; a debugging session inserts the resource. This
//! system is registered unconditionally and asks the world whether it is
//! there, which is the same answer that module gives to *"is this world
//! recording"* — no env var, no feature flag.
//!
//! ⛔ It lives in the runtime rather than beside the ring because only this
//! crate can name both `ambition_platformer2d_core::ConfirmedFrameBoundary`
//! and the simulation schedule the reconciliation has to run in.

use ambition_platformer2d_shared_tangle::authored_logic::AuthoredVerdictLog;
use ambition_platformer2d_shared_tangle::schedule::{GameplaySimulationRoot, SimScheduleExt as _};
use bevy::prelude::{resource_exists, App, IntoScheduleConfigs, Plugin, Res};

/// Clear what a previous pass over this frame recorded, then settle whatever
/// the host has confirmed since.
///
/// Both jobs are the same job — reconciling the ring with the host's timeline
/// — and both must happen BEFORE this frame's questions are asked, which is
/// why they are one system ordered `.before(GameplaySimulationRoot)`.
///
/// 1. [`AuthoredVerdictLog::begin_pass`] clears the frame's previous batch, so
///    a re-simulated frame refills rather than doubling. On a frame simulated
///    once it finds nothing and costs one walk of a 256-entry ring.
/// 2. [`AuthoredVerdictLog::confirm_through`] re-stamps everything at or
///    below the confirmed line.
pub fn reconcile_authored_verdicts_with_the_timeline(
    boundary: Res<ambition_platformer2d_core::ConfirmedFrameBoundary>,
    log: Option<Res<AuthoredVerdictLog>>,
) {
    let Some(log) = log else {
        return;
    };
    log.begin_pass(boundary.session, boundary.current);
    log.confirm_through(boundary.session, boundary.confirmed);
}

/// Installs [`reconcile_authored_verdicts_with_the_timeline`].
pub struct AuthoredVerdictTimelinePlugin;

impl Plugin for AuthoredVerdictTimelinePlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        app.add_systems(
            sim,
            reconcile_authored_verdicts_with_the_timeline
                // ⛔⛤ **`.before(GameplaySimulationRoot)` IS NOT AFTER THE
                // PUBLISHER, AND THAT IS THE WHOLE ORDERING — REVIEW
                // 2026-09-21.** The GGRS bridge publishes this frame's
                // boundary `.before(CoreSimulation)`, which is NESTED inside
                // `GameplaySimulationRoot`, so nothing related the two and the
                // reconciliation could read the PREVIOUS pass's `current` —
                // clearing frame N's batch while the simulation was about to
                // record N+1. See `ConfirmedFrameBoundaryPublished`; in a
                // composition with no rollback host the set is empty and this
                // edge is a no-op.
                .after(ambition_platformer2d_core::ConfirmedFrameBoundaryPublished)
                .before(GameplaySimulationRoot)
                // ⚠ The BOUNDARY, not the log: a host with no rollback has no
                // timeline to reconcile against, and `Res<ConfirmedFrameBoundary>`
                // would fail parameter validation rather than skip.
                .run_if(resource_exists::<ambition_platformer2d_core::ConfirmedFrameBoundary>),
        );
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::schedule::{NodeId, Schedules, SystemKey, SystemSet as _};
    use bevy::prelude::App;

    /// THE RECONCILIATION RUNS AFTER THE FRAME IT RECONCILES AGAINST IS
    /// PUBLISHED.
    ///
    /// ⛔⛤ **THE EDGE IT USED TO HAVE DID NOT REACH THE PUBLISHER — REVIEW
    /// 2026-09-21.** `.before(GameplaySimulationRoot)` and the bridge's
    /// `.before(CoreSimulation)` do not relate two systems when the second set
    /// is NESTED in the first: both constraints are satisfied in either order.
    /// They touch one resource, so Bevy serialises them and picks; a pick is
    /// not a guarantee, and the wrong pick makes `begin_pass` clear the
    /// PREVIOUS frame's batch every pass.
    ///
    /// ⚠ **THIS IS THE READER'S HALF AND IT IS HALF.** The other half — that
    /// the publisher is actually IN the set — is asserted where the publisher
    /// lives (`ambition_platformer2d_rollback_ggrs::session`), because a set
    /// nothing joined would make this arm pass over an empty edge.
    #[test]
    fn the_reconciliation_is_ordered_after_the_boundary_is_published() {
        use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt as _;

        let mut app = App::new();
        app.add_plugins(super::AuthoredVerdictTimelinePlugin);
        let sim = app.sim_schedule();
        let schedules = app.world().resource::<Schedules>();
        let schedule = schedules.get(sim).expect("the plugin creates the schedule");
        let graph = schedule.graph();

        // BY SHAPE, NEVER BY NAME — `system.name()` is a placeholder unless the
        // build graph unifies `bevy_ecs/debug`, so a name-keyed lookup passes
        // under one `-p` and fails under another. This plugin schedules exactly
        // one system.
        let systems: Vec<SystemKey> = graph.systems.iter().map(|(key, _, _)| key).collect();
        assert_eq!(
            systems.len(),
            1,
            "AuthoredVerdictTimelinePlugin schedules the reconciliation and nothing else"
        );
        let published = graph
            .system_sets
            .get_key(ambition_platformer2d_core::ConfirmedFrameBoundaryPublished.intern())
            .expect("the `.after` registers the set even with no members");
        assert!(
            graph
                .dependency()
                .graph()
                .contains_edge(NodeId::Set(published), NodeId::System(systems[0])),
            "the reconciliation must run AFTER ConfirmedFrameBoundaryPublished — it reads \
             `boundary.current` to decide which frame's batch to clear, and reading the \
             previous pass's value deletes the verdicts that pass recorded"
        );
    }
}
