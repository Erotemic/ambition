//! **Who owns the timeline decides whether a developer edit may touch it.**
//!
//! ⛔⛤ **THIS FACT HAD NO HOLDER UNTIL 2026-09-18, AND THAT IS WHY IT IS ITS OWN
//! FILE.** `decide_mechanical_edit_admission` is the one place that answers *"may
//! this host mutate mechanics around the live timeline"*, and its answer comes
//! from `mechanical_mutation_boundary`, which folds OWNERSHIP in. Two arms of
//! that fold are load-bearing and opposite:
//!
//!     SyncTestOwner::Caller           -> ForeignTimeline   -> Refuse
//!     SyncTestOwner::LocalMaintainer  -> LocallyRebasable  -> stop, then Publish
//!
//! Every rollback fixture in this suite installs its own session through
//! `start_sync_test_session`, which stamps `Caller`. The shipped game arms
//! `LocalSessionPolicy` and lets `maintain_local_session` install it, which
//! stamps `LocalMaintainer`. ⇒ Measured the day this was written:
//! `LocalSessionPolicy` appeared NOWHERE under `game/ambition_app/tests/` or
//! `crates/ambition_sim_harness/src/`, so the whole suite sat on the refusing
//! side and no arm said so. A test that spent a run reporting zero clones and
//! reading it as the defect it was written for is what found this.
//!
//! ⚠ **NEITHER ARM IS THE INTERESTING ONE ALONE.** The refusal proves the
//! protection exists; the admission proves it is not permanent. A guard that only
//! held the first would be satisfied by a decider that refused everything
//! forever, which is exactly the state the first draft of the clone witness was
//! silently measuring.

#![cfg(feature = "rl_sim")]

use ambition_app::rl_sim::{
    AgentAction, AmbitionSim, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode,
};
use ambition_platformer2d::engine_core::{MechanicalEditAdmission, PendingMechanicalEdits};

/// The boundary and the admission, as the world reports them right now.
fn verdict(sim: &mut Platformer2dSimHarness) -> (String, String) {
    let world = sim.world();
    (
        format!(
            "{:?}",
            ambition_platformer2d::rollback::mechanical_mutation_boundary(world)
        ),
        format!(
            "{:?}",
            world
                .get_resource::<MechanicalEditAdmission>()
                .expect("the rollback host installs the admission resource")
        ),
    )
}

/// Raise a proposal in a domain nothing else publishes, so the decider has a
/// pending edit to answer for and nothing consumes it behind our back.
fn propose_a_bare_edit(sim: &mut Platformer2dSimHarness) {
    let domain = ambition_platformer2d::engine_core::MechanicalDomain::of::<AdmissionProbeDomain>(
        "admission_probe",
    );
    sim.world_mut()
        .resource_mut::<PendingMechanicalEdits>()
        .propose(domain);
}

struct AdmissionProbeDomain;

/// **A harness-owned timeline refuses a developer edit, and keeps it pending.**
///
/// ⚠ THE SECOND HALF IS THE POINT. A refusal that DROPPED the proposal would be a
/// swallowed edit — the same defect class as `Q136` — so the arm asserts the
/// proposal is still there afterwards.
#[test]
fn a_caller_owned_timeline_refuses_a_developer_edit_and_keeps_it_pending() {
    let mut sim = Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_sync_test_rollback_settings(4, 10),
    )
    .expect("the sandbox builds headlessly under a sync-test session");
    for _ in 0..40 {
        sim.step(AgentAction::default());
    }
    propose_a_bare_edit(&mut sim);
    sim.step(AgentAction::default());

    let (boundary, admission) = verdict(&mut sim);
    assert_eq!(
        (boundary.as_str(), admission.as_str()),
        ("ForeignTimeline", "Refuse"),
        "a session this host did not start must not be rebased by a developer edit"
    );
    assert!(
        sim.world().resource::<PendingMechanicalEdits>().any_pending(),
        "the refusal DROPPED the proposal instead of staging it — a refused edit \
         that is also a lost edit is the defect this protocol exists to avoid"
    );
}

/// **A timeline this host owns admits the edit, by standing the baseline down.**
///
/// ⚠ THE EDIT THAT MAKES THIS FALSE is any change that makes
/// `locally_rebasable_timeline` answer for fewer cases — then the shipped game
/// silently joins the harness on the refusing side and every developer edit is
/// staged forever.
#[test]
fn a_maintainer_owned_timeline_admits_a_developer_edit() {
    let mut sim = crate::common::maintainer_owned_rollback_sim(40);
    sim.rollback_health()
        .expect("the maintainer-owned baseline is healthy before the edit");
    propose_a_bare_edit(&mut sim);
    sim.step(AgentAction::default());

    let (boundary, admission) = verdict(&mut sim);
    assert_eq!(
        (boundary.as_str(), admission.as_str()),
        ("LocallyRebasable", "Publish"),
        "the shipped ownership mode must admit an edit it can rebase around; if \
         this reads ForeignTimeline the fixture is not owning its session, and if \
         it reads NoTimeline there is no session at all and the arm is vacuous"
    );
}
