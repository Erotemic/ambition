//! Contact survives a rewind — Task 2's fourth scenario family.
//!
//! The roadmap asks that "representative damage, transition, action, and contact scenarios can be
//! stepped, rewound, checksum-compared, and asserted through the real schedule". It walks contact
//! invariants hard and never rewinds one.
//!
//! That gap is not academic. Contact state is where path-dependence lives — ground
//! and wall flags come out of the swept kernel, and they feed coyote windows, wall
//! cling, one-way pass-through and hazard arming. A restore that puts a body back
//! at the right POSITION with the wrong contact flags produces a body that walks
//! on air for one tick, and a position-only assertion cannot see it.
//!
//! So this drives a contact-dense traversal under a GGRS sync-test session, where
//! GGRS resimulates every frame from a restored snapshot and compares checksums
//! itself. The assertions here add what a checksum cannot: that contact actually
//! HAPPENED during the rewound window, in more than one mode.
//!
//! The checksum genuinely covers the subject, which is worth naming rather than
//! assuming: `BodyGroundState` and `BodyWallState` are both registered
//! `rollback_component_canonical`, so they are inside the GGRS aggregate and a
//! restore that got either wrong is a mismatch and not a silent difference.

#![cfg(feature = "rl_sim")]

use ambition_app::rl_sim::{AgentAction, AmbitionSim, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode};

/// Frames to walk. Long enough for the body to land, run into a wall, leave the
/// ground again and come back, at 60Hz.
const FRAMES: usize = 240;

fn contact_sim() -> Platformer2dSimHarness {
    Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            // The lab floor, its blocks and its hazard band — a room whose
            // authored geometry the body is guaranteed to touch, rather than an
            // open field where "no contact" would pass vacuously.
            .with_required_start_room("combat_calibration_lab")
            .with_sync_test_rollback_settings(4, 10),
    )
    .expect("the Ambition GGRS sync-test harness builds in the calibration lab")
}

/// Run into things. Deliberately not a random walk: this test's subject is
/// whether contact survives a rewind, and a seeded policy that happens to spend
/// its run airborne would make the result depend on the seed. Holding a
/// direction and jumping periodically guarantees ground contact between jumps
/// and wall contact at the arena edge.
fn scripted_action(frame: usize) -> AgentAction {
    AgentAction {
        // Long runs in one direction, so the body reaches a wall and STAYS on it
        // rather than oscillating around its spawn. Reversing at 90 also means
        // both walls get sampled.
        move_x: if (frame / 90) % 2 == 0 { 1.0 } else { -1.0 },
        // Sparse jumps. A frequent jump keeps the body airborne, which is the
        // opposite of what a contact test wants: the first tuning here jumped
        // every 37 frames and spent 213 of 240 in the air, leaving the wall
        // guard hanging on four frames.
        jump: frame % 80 == 0,
        jump_held: frame % 80 < 6,
        ..AgentAction::default()
    }
}

#[test]
fn contact_state_survives_real_rewind_and_resimulation() {
    let mut sim = contact_sim();

    let mut grounded_frames = 0usize;
    let mut wall_frames = 0usize;
    let mut airborne_frames = 0usize;
    for frame in 0..FRAMES {
        let observation = sim.step(scripted_action(frame));
        // GGRS compares the checksum of every resimulated frame against the
        // original. This surfaces a divergence at the frame it happened rather
        // than wherever the consequences became visible.
        sim.rollback_health()
            .unwrap_or_else(|error| panic!("frame {frame}: {error}"));
        if observation.on_ground {
            grounded_frames += 1;
        } else {
            airborne_frames += 1;
        }
        if observation.on_wall {
            wall_frames += 1;
        }
    }

    let stats = sim
        .rollback_execution_stats()
        .expect("GGRS instrumentation is installed under a sync-test session");
    assert!(
        stats.load_runs > 0,
        "no LoadWorld request was ever issued, so nothing was rewound and this \
         test is an expensive fixed-tick walk: {stats:?}"
    );
    assert!(
        stats.advance_runs > FRAMES as u64,
        "resimulation must execute more GGRS frames than the harness stepped, or \
         the frames were simulated once each and never replayed: {stats:?}"
    );

    // The vacuity guards, and the reason this is not just another desync canary.
    // A body that spent the whole run in the air would prove nothing about
    // contact no matter how many times it was rewound.
    assert!(
        grounded_frames > 0,
        "the body never touched the ground in {FRAMES} frames, so no ground \
         contact was ever inside the rewound window"
    );
    assert!(
        airborne_frames > 0,
        "the body never left the ground, so the contact flag never CHANGED — a \
         constant is preserved by any restore, including a broken one"
    );
    assert!(
        wall_frames > 0,
        "the body never touched a wall, so only one contact mode was exercised; \
         wall contact is the one that drives cling and it has its own restore path"
    );
    eprintln!(
        "[rollback-contact] {FRAMES} frames: {grounded_frames} grounded, \
         {airborne_frames} airborne, {wall_frames} on a wall; {stats:?}"
    );
}
