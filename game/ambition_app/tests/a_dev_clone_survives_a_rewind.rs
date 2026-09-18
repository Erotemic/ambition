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

fn arena(rollback: bool) -> Platformer2dSimHarness {
    let mut options =
        Platformer2dSimHarnessOptions::default().with_timestep(TimestepMode::fixed_60hz());
    if rollback {
        // The same settings every other rollback arm in this suite uses: a
        // prediction window of 4 and a check distance of 10, so GGRS rewinds and
        // resimulates on every update rather than occasionally.
        options = options.with_sync_test_rollback_settings(4, 10);
    }
    let mut sim =
        Platformer2dSimHarness::new_with_options(options).expect("the sandbox builds headlessly");
    if rollback {
        hand_the_timeline_to_the_local_maintainer(&mut sim);
    }
    sim
}

/// Re-own the harness's sync-test session the way the SHIPPED GAME owns it.
///
/// ⛔⛔ **WITHOUT THIS THE ROLLBACK ARM MEASURES THE FIXTURE, AND IT DID.** The
/// first run of this file reported 0 clones under rollback and 1 under fixed
/// tick, which reads exactly like the defect. It was not: the diagnostic printed
/// `boundary=ForeignTimeline admission=Refuse` on every tick, forever.
/// `with_sync_test_rollback_settings` installs the session through
/// `start_sync_test_session`, which stamps `SyncTestOwner::Caller`, and
/// `locally_rebasable_timeline` answers only for `SyncTestOwner::
/// LocalMaintainer` — so a mechanical edit is REFUSED, correctly, because a
/// harness that installed its own timeline did not ask for it to be rebased.
///
/// ⇒ The shipped game does not run that way: `LocalSessionPolicy` is armed (the
/// rollback observatory does it) and `maintain_local_session` installs the
/// session as `LocalMaintainer`. Measured 2026-09-18: NO test in this workspace
/// touched `LocalSessionPolicy`, so no arm anywhere exercised the ownership mode
/// the game actually uses — every rollback test was on the refusing side of the
/// admission road without saying so.
///
/// ⚠ It stops the caller-owned session first and then lets the maintainer build
/// its own, rather than editing the ownership resource: the owner stamp and the
/// installed session are one fact, and writing half of it is how a fixture comes
/// to describe a world that cannot exist.
fn hand_the_timeline_to_the_local_maintainer(sim: &mut Platformer2dSimHarness) {
    use ambition_platformer2d::rollback::local_session::LocalSessionPolicy;

    let world = sim.world_mut();
    ambition_platformer2d::rollback::stop_session(world);
    // ⚠ THE ORDER IS `(check_distance, max_prediction_window)` AND GGRS REQUIRES
    // THE FIRST TO BE SMALLER. Inverting them does not fail loudly here — the
    // maintainer catches `Invalid Request: Check distance too big`, records it in
    // `LocalSessionOwnership::last_error` and carries on with NO session, so the
    // arm silently becomes a no-rollback arm that passes.
    world.insert_resource(LocalSessionPolicy {
        check_distance: 4,
        max_prediction_window: 10,
        autostart: true,
    });
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
    // carry the `SimId` the clone descends from.
    sim.world_mut().resource_mut::<SpawnPlayerCloneRequest>().0 = true;
    assert!(
        sim.world().resource::<SpawnPlayerCloneRequest>().0,
        "the request did not even land in the resource, so this arm is not \
         measuring what happens to one"
    );
    for _ in 0..120 {
        sim.step(AgentAction::default());
    }
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
        sim.rollback_health().unwrap_or_else(|error| {
            panic!(
                "a maintainer-owned timeline desynced at step {step} with no clone press \
                 anywhere near it, so the arms above are measuring this rather than the \
                 press: {error}"
            )
        });
    }
}
