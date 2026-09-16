//! A12 — the move-occurrence identity, witnessed under an actual rollback window.
//!
//! `MoveOccurrence` is the identity behind `MovePlayback::instance`: a shot
//! stamped by move A must not be credited to move B. Two structural facts are
//! supposed to make that survive a rewind, and each has its own guard:
//!
//! * it is `component-canonical`, so a rewind restores the exact value and two
//!   peers compare it (`rollback_schema_baseline.txt`);
//! * its only writer is `start_move`, reached from `trigger_moveset_moves`, which
//!   is registered through `app.add_systems(sim, ..)` — the REWINDING schedule
//!   (`check_rollback_mutators_run_in_sim.py`).
//!
//! ⛔⛤ **BOTH ARE CLAIMS ABOUT REGISTRATION. NEITHER IS A READING OF THE NUMBER.**
//! `ambition_combat`'s own arms cover the component-lifetime invariant — a body
//! carrying `MovePlayback` carries `MoveOccurrence` — on a hand-built App with no
//! rollback session at all. So the whole suite is green in a world where nothing
//! rewinds, which is the shape that hid
//! `a_bag_changed_from_update_is_silently_taken_back_by_the_rewind`.
//!
//! ⇒ This is a different KIND of evidence, not a third measurement: drive the same
//! world with and without a GGRS session and compare the number the body reaches.
//! A counter advanced per RESIMULATED TICK rather than per move started inflates
//! under sync-test and matches under a fixed-tick host, and nothing else here
//! would notice.
//!
//! ⭐ **THE GOOD VALUE, SO A FUTURE READER CAN TELL A PASS FROM A COINCIDENCE:**
//! measured 2026-09-16, `Some(9)` under rollback and `Some(9)` without — nine
//! moves started across 180 frames at one press every twelfth frame.
//!
//! ⛔ **POISONED, AND THE POISON IS THE PROPERTY.** Removing
//! `rollback_component_canonical::<MoveOccurrence>` from
//! `ambition_combat::rollback_registration` makes this arm fail with
//! `Some(1)` vs `Some(9)`: unregistered, every rewind drops the counter, so the
//! body never gets past its first move while the fixed-tick host reaches nine.
//! That is the defect the struct's doc describes, and until this arm existed
//! nothing in the repository failed when the registration went away except the
//! schema baseline — which would only have said the dump changed.

#![cfg(feature = "rl_sim")]

use ambition_app::rl_sim::{
    AgentAction, AmbitionSim, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode,
};
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;

/// The room the other rollback arms use, so a failure here is about the counter
/// rather than about an unusual world.
const ROOM: &str = "combat_calibration_lab";

/// How many frames to hold attack. Long enough for several moves to start and
/// finish, so the number reached is a count of MOVES and not of one.
const FRAMES: usize = 180;

fn rewinding_sim() -> Platformer2dSimHarness {
    Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room(ROOM)
            .with_sync_test_rollback_settings(4, 10),
    )
    .expect("Ambition GGRS sync-test harness builds in the calibration lab")
}

/// ⛔ THE CONTROL IS THE LOAD-BEARING HALF. "The counter reached 7" says nothing
/// on its own — 7 could be right, or the rewind could have inflated it from 3, or
/// a gate could have stopped moves entirely. The same world with NO rollback
/// session is the only thing that separates those.
fn fixed_tick_sim() -> Platformer2dSimHarness {
    Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_required_start_room(ROOM),
    )
    .expect("the same world builds without a rollback session")
}

/// ⚠ `None` is a real answer and it is NOT zero: a body that has started no move
/// carries no component. Collapsing the two would let "no move ever ran" read as
/// "the first move took number 0", which is the reassuring direction.
fn occurrence(sim: &mut Platformer2dSimHarness) -> Option<u32> {
    let world = sim.world_mut();
    let mut q = world
        .query_filtered::<&ambition_platformer2d::combat::moveset::MoveOccurrence, PrimaryPlayerOnly>();
    q.iter(world).next().map(|seen| seen.0)
}

fn press_attack() -> AgentAction {
    AgentAction {
        attack: true,
        ..AgentAction::default()
    }
}

/// Hold attack for `FRAMES`, releasing on alternate frames so the press is an
/// EDGE each time — a held button starts one move, not several.
fn drive(sim: &mut Platformer2dSimHarness) {
    for frame in 0..FRAMES {
        if frame % 12 == 0 {
            sim.step(press_attack());
        } else {
            sim.step(AgentAction::default());
        }
    }
}

#[test]
fn a_move_occurrence_reaches_the_same_number_with_and_without_a_rewind() {
    let mut rewinding = rewinding_sim();
    drive(&mut rewinding);
    let under_rollback = occurrence(&mut rewinding);

    let mut fixed = fixed_tick_sim();
    drive(&mut fixed);
    let without_rollback = occurrence(&mut fixed);

    // ⛔⛔ THE FLOOR, BEFORE EITHER NUMBER IS READ. `None == None` and `0 == 0`
    // both satisfy the comparison below perfectly, and both mean this arm
    // examined a body that never started a move.
    let reached = without_rollback.expect(
        "the primary player started NO move in the fixed-tick world across \
         180 frames, so this arm has no subject — the attack press is not \
         reaching the trigger in this room, and the comparison below would \
         pass on two absent values",
    );
    assert!(
        reached > 0,
        "the body started exactly one move (occurrence 0) across {FRAMES} \
         frames, so a counter that never advances would pass this arm. The \
         press cadence needs to start several moves."
    );

    assert_eq!(
        under_rollback,
        without_rollback,
        "the move-occurrence counter reaches a DIFFERENT number under a \
         rollback window than under a fixed-tick host ({under_rollback:?} vs \
         {without_rollback:?}). Either the rewind is not restoring it — it is \
         registered `component-canonical`, so check the registration survived — \
         or something outside the rewinding schedule advances it, which is the \
         `OwnedItems` defect class: a write that is restored away, or one that \
         is replayed and counted twice. Two moves that share a number credit \
         each other's hits, which is the A12 defect."
    );
}
