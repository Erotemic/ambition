//! Generic encounter acceptance (docs/systems/boss-encounter-architecture.md): the first non-boss, non-wave
//! encounter customer — the Noether attunement, a signal-driven NO-ACTOR
//! puzzle — completes through the GENERIC path in the real headless sim.
//!
//! What this proves: content added rules (enter the chamber → `Start`; each
//! kernel-face gravity flip → `Signal`; all four signals → the generic `All`
//! objective completes) without adding another lifecycle, objective
//! evaluator, cleanup path, or presentation authority. The switch messages
//! are written at the interaction seam (`SwitchActivated`, the same message
//! the interact dispatcher emits) — the plumbing from a body pressing E is
//! covered by the switch interaction tests; this gate owns everything
//! downstream of the fact.

#![cfg(feature = "rl_sim")]

use ambition_app::rl_sim::TimestepMode;
use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, Platformer2dSimHarnessOptions};

use ambition_content::encounters::{SYMMETRY_ATTUNEMENT_FLAG, SYMMETRY_ATTUNEMENT_ID};
use ambition_platformer2d::encounter::{
    Encounter, EncounterLifecycle, EncounterPhase, SwitchActivation,
};

fn attunement_phase(sim: &mut Platformer2dSimHarness) -> EncounterPhase {
    let mut q = sim
        .world_mut()
        .try_query::<(&Encounter, &EncounterLifecycle)>()
        .expect("query builds");
    q.iter(sim.world())
        .find(|(enc, _)| enc.id == SYMMETRY_ATTUNEMENT_ID)
        .map(|(_, lifecycle)| lifecycle.phase())
        .expect("the attunement authority exists")
}

fn flip_kernel_face(sim: &mut Platformer2dSimHarness, switch_id: &str, action: &str) {
    sim.world_mut().write_message(
        ambition_platformer2d::encounter::switches::SwitchActivated {
            activation: SwitchActivation {
                id: switch_id.to_string(),
                action: action.to_string(),
                target_encounter: String::new(),
            },
            pos: ambition_platformer2d::engine_core::Vec2::ZERO,
        },
    );
}

#[test]
fn the_noether_attunement_completes_through_the_generic_path() {
    let opts = Platformer2dSimHarnessOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        .with_required_start_room("symmetry_room");
    let mut sim = Platformer2dSimHarness::new_with_options(opts).expect("symmetry_room boots");

    // Entering the chamber starts the puzzle (content emits Start; the
    // generic reducer flips it Active — no content code touches the phase).
    for _ in 0..3 {
        sim.step(AgentAction::default());
    }
    assert_eq!(
        attunement_phase(&mut sim),
        EncounterPhase::Active,
        "room entry starts the attunement through the command ingress"
    );

    // Three of the four kernel faces: signals collect, objective unmet.
    for (id, action) in [
        ("kernel_switch_down", "SetGravityDown"),
        ("kernel_switch_left", "SetGravityLeft"),
        ("kernel_switch_up", "SetGravityUp"),
    ] {
        flip_kernel_face(&mut sim, id, action);
        sim.step(AgentAction::default());
    }
    assert_eq!(
        attunement_phase(&mut sim),
        EncounterPhase::Active,
        "three symmetries visited of four — the All objective must hold out"
    );

    // The fourth face completes the encounter through the generic objective,
    // and the content celebration records the persistent flag.
    flip_kernel_face(&mut sim, "kernel_switch_right", "SetGravityRight");
    for _ in 0..2 {
        sim.step(AgentAction::default());
    }
    assert_eq!(attunement_phase(&mut sim), EncounterPhase::Completed);
    let save = sim
        .world()
        .resource::<ambition_platformer2d::persistence::save::AmbitionGameSave>();
    assert!(
        save.data().flag(SYMMETRY_ATTUNEMENT_FLAG),
        "completion pays out through the generic Completed event"
    );

    // A completed attunement stays completed: the reducer refuses a Start
    // from a terminal phase, so lingering in the chamber cannot restart it.
    for _ in 0..3 {
        sim.step(AgentAction::default());
    }
    assert_eq!(attunement_phase(&mut sim), EncounterPhase::Completed);
}

/// ⛔⛤ **THE SWITCH ACTIVATION QUEUE IS A CROSS-TICK CHANNEL BY CONSTRUCTION,
/// AND NOTHING SAID SO — MEASURED 2026-09-18.**
///
/// `apply_switch_effects` (`features/ecs/effect_bus.rs`) PUSHES into
/// `SwitchActivationQueue` inside
/// `Platformer2dSimulationPhaseMonolith::GameplayEffects`.
/// `drain_switch_activations` (`ambition_encounter/src/switches.rs`) takes the
/// whole queue with `std::mem::take`, registered `.in_set(SwitchActivationDrained)`
/// — and that set is never `configure_sets`'d into any simulation phase.
///
/// ⚠ **"NOTHING ORDERS THEM" IS TRUE AND THE CONSEQUENCE IS NOT ARBITRARY, which
/// is the correction this arm exists to record.** The drain's position is pinned
/// by its CONSUMERS: `drive_wave_encounters` is
/// `.in_set(EncounterSimulation).after(SwitchActivationDrained)`, and the phase
/// chain is `... EncounterSimulation → Cutscene → GameplayEffects → Progression`.
/// So the drain must precede a system in `EncounterSimulation`, while the
/// producer sits two phases later in `GameplayEffects`. ⇒ **The producer is
/// structurally downstream of its consumer's consumer, so an activation can only
/// ever be drained on the FOLLOWING tick.** Measured: `(queued, resolved) ==
/// (1, 0)` one step after the push.
///
/// ⛔ AND THE OBVIOUS REPAIR IS A SCHEDULE CYCLE, not an improvement. Adding
/// `.after(apply_switch_effects)` to the drain would require the drain to run
/// after `GameplayEffects` and before `drive_wave_encounters` in
/// `EncounterSimulation`, which is earlier in the same frame. The one-tick delay
/// is the price of the phase order, not a missing edge.
///
/// ⇒ What this arm buys is that the delay is now STATED and pinned. The queue is
/// `rollback_resource_canonical`-adjacent rollback state, so the pending
/// activation is snapshotted and a rewind replays it — the latency is
/// deterministic and peer-stable. What would be a real defect is the delay
/// changing silently, and that is what goes red here.
///
/// ⚠ It lives in this file because this is the only fixture that drives a real
/// `SwitchActivated` through the shipped composition; the question is about the
/// engine's schedule, not about the attunement.
#[test]
fn a_switch_activation_is_drained_on_the_tick_after_it_was_pushed() {
    let opts = Platformer2dSimHarnessOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        .with_required_start_room("symmetry_room");
    let mut sim = Platformer2dSimHarness::new_with_options(opts).expect("symmetry_room boots");
    for _ in 0..3 {
        sim.step(AgentAction::default());
    }

    // ⚠ CLEARED FIRST, because three boot steps of the attunement room may have
    // resolved activations of their own and a non-empty reading would then say
    // nothing about THIS push.
    sim.world_mut()
        .resource_mut::<ambition_platformer2d::encounter::switches::ResolvedSwitchActivations>()
        .0
        .clear();
    sim.world_mut()
        .resource_mut::<ambition_platformer2d::encounter::switches::SwitchActivationQueue>()
        .0
        .clear();

    flip_kernel_face(&mut sim, "kernel_switch_down", "SetGravityDown");
    sim.step(AgentAction::default());

    let after_one = queue_and_resolved(&mut sim);
    // ⛔ THE PREMISE, CHECKED RATHER THAN ASSUMED: the producer ran at all. If
    // both are zero the message was never consumed and the reading below is
    // about a frame in which nothing happened.
    assert!(
        after_one.0 + after_one.1 > 0,
        "neither the queue nor the resolved list saw the activation, so \
         `apply_switch_effects` did not consume the `SwitchActivated` message \
         and this arm measured an empty frame"
    );
    assert_eq!(
        after_one,
        (1, 0),
        "the push and the drain changed which tick they meet on. Read \
         (queued, resolved) = {after_one:?}. (1, 0) is what the phase chain \
         forces — the producer is in `GameplayEffects` and the drain must \
         precede `drive_wave_encounters` in `EncounterSimulation`, two phases \
         earlier — so (0, 1) means somebody moved a phase, a set membership or \
         a consumer edge. Read this arm's doc before changing the number: the \
         fix is not `.after(apply_switch_effects)`, which is a schedule cycle."
    );

    // ⭐ AND THE NEXT TICK RESOLVES IT, which is what makes the reading above a
    // DELAY rather than a loss. Without this the arm is satisfied by a queue
    // that is never drained at all.
    sim.step(AgentAction::default());
    assert_eq!(
        queue_and_resolved(&mut sim),
        (0, 1),
        "the activation was still queued a tick later: it is not delayed, it is \
         STUCK, and every reader of `ResolvedSwitchActivations` never sees it"
    );
}

/// `(queued, resolved)` for the switch activation channel, in one place so the
/// two readings in the arm above cannot drift apart.
fn queue_and_resolved(sim: &mut Platformer2dSimHarness) -> (usize, usize) {
    let queued = sim
        .world_mut()
        .resource::<ambition_platformer2d::encounter::switches::SwitchActivationQueue>()
        .0
        .len();
    let resolved = sim
        .world_mut()
        .resource::<ambition_platformer2d::encounter::switches::ResolvedSwitchActivations>()
        .0
        .len();
    (queued, resolved)
}
