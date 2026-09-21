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
                .before(GameplaySimulationRoot)
                // ⚠ The BOUNDARY, not the log: a host with no rollback has no
                // timeline to reconcile against, and `Res<ConfirmedFrameBoundary>`
                // would fail parameter validation rather than skip.
                .run_if(resource_exists::<ambition_platformer2d_core::ConfirmedFrameBoundary>),
        );
    }
}
