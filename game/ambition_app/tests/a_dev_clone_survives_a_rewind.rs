//! **A developer clone asked for outside the simulation survives a rewind.**
//!
//! `Q136`'s first landed road, and the specimen was chosen for its stakes rather
//! than its difficulty: a dev hotkey that spawns a brain-driven body. No save
//! data, no peer checksum, no player progression — so the ingress road could be
//! built and witnessed here without a mis-step costing a timeline.
//!
//! ⛔⛤ **WHAT WAS WRONG, AND WHY THE SPLIT THAT CAUSED IT WAS CORRECT.**
//! `request_player_clone_on_key` reads `ButtonInput<KeyCode>` in
//! `bevy::app::Update` and sets `SpawnPlayerCloneRequest`; the spawn used to
//! consume that flag inside the simulation schedule. The split itself is right
//! and `plugins.rs` says why: `ButtonInput` is winit frame state, it does not
//! rewind, so reading `just_pressed` on the deterministic tick sees one physical
//! press once per SIM RUN and a frame that steps the sim twice spawns TWO
//! clones. What the split created was the other half — the sim spent the flag on
//! a speculative frame, the rewind despawned the clone (every `BodyKinematics`
//! body is a rollback anchor through `require_rollback::<BodyKinematics>`), the
//! flag is not rollback-registered so nothing restored it, and the `just_pressed`
//! edge was several host frames gone. The press vanished.
//!
//! ⇒ The two failures are one problem seen from its two sides, which is why the
//! fix is a ROAD and not a patch: the spawn is now a mechanical edit, proposed
//! in `MechanicalEditSet::Propose` and published in `Publish`, with
//! `decide_mechanical_edit_admission` between them.
//!
//! ⚠ **THE FIXED-TICK ARM IS NOT DECORATION.** Without a rollback session there
//! is nothing to rewind, so every hour of local play exercises the working path
//! and a broken road still spawns clones. It is here so a zero in the rollback
//! arm can be read: a fixture that cannot spawn a clone at all and a road that
//! loses the press print the same zero.

#![cfg(feature = "rl_sim")]

use ambition_app::rl_sim::{
    AgentAction, AmbitionSim, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode,
};
use ambition_app::app::{PlayerClone, SpawnPlayerCloneRequest};
use bevy::prelude::With;

/// ⛔⛤ **THE ROLLBACK ARM'S SETTINGS HAVE ONE OWNER, AND THEY DID NOT.** This
/// function used to build its own options with its own `(4, 10)` beside the
/// shared fixture's copy of the same pair. Poisoning the shared pair therefore
/// left this file running the old one: all three arms stayed GREEN, which reads
/// as an insensitive witness and was really a poison that never applied.
///
/// ⚠ The settle is ZERO frames here, unlike most callers: two arms below need
/// frame zero, before the primary carries a `SimId`.
fn arena(rollback: bool) -> Platformer2dSimHarness {
    if rollback {
        return crate::common::maintainer_owned_rollback_sim(0);
    }
    Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default().with_timestep(TimestepMode::fixed_60hz()),
    )
    .expect("the sandbox builds headlessly")
}

/// ⛔⛤ **ANTI-VACUITY, AND A POISON FOUND THAT IT WAS MISSING.** Inverting the
/// shared fixture's `(check_distance, max_prediction_window)` leaves NO session
/// installed — `maintain_local_session` records `Invalid Request: Check distance
/// too big` and carries on — and every arm in this file stayed GREEN: with no
/// timeline the admission is `NoTimeline`, the clone spawns, one clone is counted
/// and `rollback_health()` on a world with no session reports fine. A rollback
/// arm that passes when there is no rollback is measuring nothing.
///
/// ⇒ So the arms ASK, rather than assuming the fixture worked. `LocallyRebasable`
/// is the specific answer they need: `NoTimeline` means no session, and
/// `ForeignTimeline` means the session is there but this host may not rebase it,
/// which is the caller-owned mode the first draft of this file measured by
/// accident.
fn assert_the_timeline_is_ours(sim: &mut Platformer2dSimHarness, when: &str) {
    let boundary = format!(
        "{:?}",
        ambition_platformer2d::rollback::mechanical_mutation_boundary(sim.world())
    );
    assert_eq!(
        boundary, "LocallyRebasable",
        "{when}: this arm needs a live timeline THIS host owns and the \
         boundary reports `{boundary}`. `NoTimeline` means no session is \
         installed — either the settings order was inverted, or this is being \
         asked before the first step installs one; `ForeignTimeline` means the \
         session is caller-owned and every mechanical edit is refused"
    );
}

fn clones(sim: &mut Platformer2dSimHarness) -> usize {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<bevy::prelude::Entity, With<PlayerClone>>();
    q.iter(world).count()
}

/// How many clones exist after asking for one from OUTSIDE the simulation.
///
/// The request is written the way every caller writes it — the resource seam the
/// hotkey, the menu and the live tests all use — and then the sim is stepped far
/// enough that a rollback host has rewound across the frame that consumed it.
fn clones_after_asking_from_outside(rollback: bool) -> usize {
    let mut sim = arena(rollback);
    // Let the primary settle first: the clone descends from it, and a body with
    // no `SimId` yet is a refusal the spawn is now required to SURVIVE rather
    // than consume. Settling separates the two so this arm measures the road.
    for _ in 0..40 {
        sim.step(AgentAction::default());
    }
    // ⛔⛤ **HEALTH BEFORE THE PRESS, BECAUSE AN UNHEALTHY TIMELINE ANSWERS THIS
    // ARM'S QUESTION WITH THE WRONG WORD.** `mechanical_mutation_boundary` maps a
    // recorded divergence to `Unhealthy`, which the decider turns into `Refuse` —
    // so a diverged baseline produces zero clones for a reason that has nothing
    // to do with the rewind this arm is about, and the failure message would send
    // the next reader after the wrong defect.
    if rollback {
        assert_the_timeline_is_ours(&mut sim, "before the press");
        sim.rollback_health()
            .unwrap_or_else(|error| panic!("the baseline was not healthy when the press landed: {error}"));
    }
    sim.world_mut().resource_mut::<SpawnPlayerCloneRequest>().0 = true;
    for _ in 0..60 {
        sim.step(AgentAction::default());
    }
    // ⭐ AND AGAIN AFTER, which is a claim about the FIX and not about the
    // fixture: an admitted mechanical edit stands the local baseline down and
    // `maintain_local_session` rebases it, so a healthy session here says the
    // rebase produced a timeline that agrees with itself rather than one carrying
    // the edit as a divergence.
    if rollback {
        sim.rollback_health().unwrap_or_else(|error| {
            panic!("the rebased timeline is not healthy after the edit published: {error}")
        });
    }
    clones(&mut sim)
}

/// ⭐ **THE ROAD: one press, one clone, on both hosts.**
///
/// ⚠ THE EDIT THAT MAKES THIS FALSE is the one this test was written against:
/// register `spawn_requested_player_clone` into `app.sim_schedule()` in
/// `Platformer2dSimulationPhaseMonolith::WorldPrep` again, as it was before
/// 2026-09-18. The fixed-tick arm stays GREEN — the two schedules are the same
/// thing there — and the rollback arm reports 0.
#[test]
fn a_clone_asked_for_outside_the_simulation_is_not_lost_to_a_rewind() {
    let fixed = clones_after_asking_from_outside(false);
    assert_eq!(
        fixed, 1,
        "the FIXED-TICK control did not spawn exactly one clone ({fixed}). 0 means \
         this fixture cannot spawn a clone at all, and the rollback arm below \
         would then be measuring the fixture rather than the road; more than 1 \
         means the request is being consumed more than once, which is the \
         double-spawn the Update/sim split exists to prevent"
    );

    let rewound = clones_after_asking_from_outside(true);
    assert_eq!(
        rewound, 1,
        "a clone asked for from outside the simulation did not survive under a \
         rollback host: {rewound} exist where the fixed-tick control has {fixed}. \
         0 is Q136's first mechanism — the sim spent an unregistered flag on a \
         speculative frame and the rewind took the body without restoring the \
         request"
    );
}

/// **A press the spawn could not honour yet is kept, not spent.**
///
/// ⛔⛤ `request.0 = false` used to stand ABOVE every refusal in
/// `spawn_requested_player_clone`, so a press that arrived before the primary
/// carried a `SimId` was consumed and the clone never appeared. Refusing a
/// sub-step is not refusing the operation.
///
/// ⚠ THE ANTI-VACUITY PROBLEM IS REAL HERE: a request asked for on frame zero
/// might simply be honoured on frame zero, in which case this arm proves
/// nothing. So it asserts the request LANDED first, and then that a clone
/// eventually exists — the press surviving whatever early frames refused it.
#[test]
fn a_press_the_spawn_cannot_honour_yet_is_kept_rather_than_consumed() {
    let mut sim = arena(true);
    // Deliberately BEFORE settling: frame zero, when the primary may not yet
    // carry the `SimId` the clone descends from. ⚠ There is no timeline yet
    // either — `maintain_local_session` installs GGRS on the first step, once
    // gameplay is active — so this press is raised into a `NoTimeline` world and
    // the anti-vacuity question is asked after the run instead, about the 120
    // steps that actually did the measuring.
    sim.world_mut().resource_mut::<SpawnPlayerCloneRequest>().0 = true;
    assert!(
        sim.world().resource::<SpawnPlayerCloneRequest>().0,
        "the request did not even land in the resource, so this arm is not \
         measuring what happens to one"
    );
    for _ in 0..120 {
        sim.step(AgentAction::default());
    }
    assert_the_timeline_is_ours(&mut sim, "after the run this arm measures");
    sim.rollback_health()
        .unwrap_or_else(|error| panic!("the timeline this arm measures is not healthy: {error}"));
    let count = clones(&mut sim);
    assert_eq!(
        count, 1,
        "a clone requested before the primary was identified produced {count} \
         bodies. 0 means the press was consumed by a refusal it should have \
         outlived"
    );
}


/// **CONTROL: a maintainer-owned timeline is healthy with no press at all.**
///
/// ⛔⛤ THE TWO ARMS ABOVE READ THE SESSION'S HEALTH, AND WITHOUT THIS ARM A
/// FAILURE THERE IS UNATTRIBUTABLE. This fixture is the first in the workspace
/// to own its session as `SyncTestOwner::LocalMaintainer`, so "does that mode
/// desync on its own?" had no recorded answer. It does not — measured 2026-09-18
/// over 120 steps — which is what makes a mismatch in the arms above a fact
/// about the press.
///
/// ⚠ IT IS NOT A GENERAL CLAIM ABOUT THE MODE, only about this room and this
/// input stream. A different room could desync for reasons of its own and this
/// arm would not know.
#[test]
fn the_maintainer_owned_timeline_is_healthy_with_no_press() {
    let mut sim = arena(true);
    for step in 0..120 {
        sim.step(AgentAction::default());
        if step == 0 {
            // The first step is what installs the session, so this is the
            // earliest moment the question has an answer.
            assert_the_timeline_is_ours(&mut sim, "once the first step had run");
        }
        sim.rollback_health().unwrap_or_else(|error| {
            panic!(
                "a maintainer-owned timeline desynced at step {step} with no clone press \
                 anywhere near it, so the arms above are measuring this rather than the \
                 press: {error}"
            )
        });
    }
}
