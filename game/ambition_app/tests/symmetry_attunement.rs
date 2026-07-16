//! **E13 exit (encounter-orchestration.md):** the first non-boss, non-wave
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
use ambition_app::{AgentAction, SandboxSim, SandboxSimOptions};

use ambition::encounter::{Encounter, EncounterLifecycle, EncounterPhase, SwitchActivation};
use ambition_content::encounters::{SYMMETRY_ATTUNEMENT_FLAG, SYMMETRY_ATTUNEMENT_ID};

fn attunement_phase(sim: &mut SandboxSim) -> EncounterPhase {
    let mut q = sim
        .world_mut()
        .try_query::<(&Encounter, &EncounterLifecycle)>()
        .expect("query builds");
    q.iter(sim.world())
        .find(|(enc, _)| enc.id == SYMMETRY_ATTUNEMENT_ID)
        .map(|(_, lifecycle)| lifecycle.phase)
        .expect("the attunement authority exists")
}

fn flip_kernel_face(sim: &mut SandboxSim, switch_id: &str, action: &str) {
    sim.world_mut()
        .write_message(ambition::actors::features::SwitchActivated {
            activation: SwitchActivation {
                id: switch_id.to_string(),
                action: action.to_string(),
                target_encounter: String::new(),
            },
            pos: ambition::engine_core::Vec2::ZERO,
        });
}

#[test]
fn the_noether_attunement_completes_through_the_generic_path() {
    let opts = SandboxSimOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        .with_start_room("symmetry_room");
    let mut sim = SandboxSim::new_with_options(opts).expect("symmetry_room boots");

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
        .resource::<ambition::persistence::save::SandboxSave>();
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
