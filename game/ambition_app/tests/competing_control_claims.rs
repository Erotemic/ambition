//! A4 — TWO BODIES CLAIMING ONE SEAT, AND WHAT THE PLAYER FEELS WHEN THEY DO.
//!
//! ⛔⛔ **THE BEHAVIOUR IS SILENCE, WHICH IS WHY IT NEEDS A TEST AND NOT AN
//! ASSERT.** `control::body_driving_seat` resolves the unique body carrying
//! `DrivingParticipant(slot)` and, when two entities hold it, logs an `error!`
//! and returns `None` — *"refusing ambiguous authority, so this seat drives
//! nothing until one of them vacates."* Nothing panics and nothing reddens: the
//! player's stick stops reaching a body, on every road that asks. Measured
//! 2026-09-10 by `git grep`, before this file existed: **no test anywhere named
//! `body_driving_seat`**, while four production readers depended on it —
//! `abilities::traversal::possession`, `control::input_systems`,
//! `control::queries` and `ambition_sim_view::local_view`.
//!
//! ⭐ **AND IT IS COMPOSED, NOT ASSEMBLED.** A fixture that hand-adds the systems
//! it wants names those functions directly and cannot notice one falling out of
//! the production schedule — which is exactly the move A4 contemplates for
//! `control/authority.rs`, the file that maintains this invariant. So this drives
//! the real headless sim through `Platformer2dSimHarness` and presses a real
//! stick.
//!
//! ⛔⛔ **AND WHAT IT FOUND WAS THE INVERSE OF THE DOCUMENTED BEHAVIOUR: THE SEAT
//! DROVE BOTH.** Measured on the first run, before any fix — two bodies holding
//! `DrivingParticipant(PRIMARY)`, one held right press, 40 frames: the player's
//! body travelled **180.00px** (175.17px unambiguously) and the rival travelled
//! **91.67px**, each at its own locomotion capability. Not "this seat drives
//! nothing" — *this seat drives everything that claims it*.
//!
//! The cause was two authorities for one fact. `body_driving_seat` resolves the
//! UNIQUE holder and refuses when there are two; `tick_controlled_brains`
//! iterated BODIES and read `slots.get(driver.0)` off each one, so it could not
//! tell one holder from two and the refusal never reached the road that moves
//! anything. `control/authority.rs` calls that state *"the exact two-writer state
//! this whole component exists to make impossible"*.
//!
//! ⚠ **AND THE FIRST FIX WAS WRONG IN A WAY ONLY THIS FIXTURE COULD SHOW.**
//! Skipping the write left the rival at 0.00px and the player still at 180.00px:
//! `ActorControl` is LATCHED, so a body that is not written keeps last tick's
//! frame, and under a held press that is indistinguishable from driving. The
//! refusal has to NEUTRALISE the frame. A fixture asserting only on the rival
//! would have called that fix done.
//!
//! ⚠ **REFUSAL WITHOUT RECOVERY IS HALF THE INVARIANT.** A test that only proved
//! "nothing happens while the claim is ambiguous" would pass just as well against
//! a composition where nothing was wired at all. Both other arms are the
//! anti-vacuity floor: the seat drives BEFORE the second claim, and drives again
//! AFTER it is withdrawn.

#![cfg(feature = "rl_sim")]

use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, TimestepMode};
use ambition_platformer2d::characters::control::{DrivingParticipant, PlayerSlot};
use ambition_platformer2d::combat::components::FeatureId;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::entity_catalog::placements::CharacterBrain;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use bevy::prelude::{Entity, World};

const RIVAL_ID: &str = "seat_rival";

/// How far the primary player travels over `steps` frames of a held right press.
fn drive_right(sim: &mut Platformer2dSimHarness, steps: usize) -> f32 {
    let before = player_pos(sim.world_mut()).x;
    for _ in 0..steps {
        sim.step(AgentAction::move_x(1.0));
    }
    player_pos(sim.world_mut()).x - before
}

/// The same press, measured on BOTH bodies. ⭐ THE RIVAL'S TRAVEL IS THE
/// QUESTION, NOT A DETAIL: "the seat drives nothing" and "the seat drives the
/// other one" are different defects and the player's position alone cannot tell
/// them apart.
fn drive_right_both(sim: &mut Platformer2dSimHarness, rival: Entity, steps: usize) -> (f32, f32) {
    let p0 = player_pos(sim.world_mut()).x;
    let r0 = body_x(sim.world_mut(), rival);
    for _ in 0..steps {
        sim.step(AgentAction::move_x(1.0));
    }
    (
        player_pos(sim.world_mut()).x - p0,
        body_x(sim.world_mut(), rival) - r0,
    )
}

fn body_x(world: &mut World, entity: Entity) -> f32 {
    world
        .get::<BodyKinematics>(entity)
        .expect("the rival body carries kinematics")
        .pos
        .x
}

fn player_pos(world: &mut World) -> ae::Vec2 {
    let mut q = world.query_filtered::<&BodyKinematics, PrimaryPlayerOnly>();
    q.single(world).expect("primary player").pos
}

/// How many entities hold the primary seat right now.
///
/// ⭐ ASSERTED AT EVERY ARM, because the state this fixture is about is not
/// visible in the thing it measures. A test that inserts a second
/// `DrivingParticipant` and reads a position proves nothing if the home avatar
/// never held the seat to begin with: the count would go 0 -> 1, there would be
/// no ambiguity, and "the body did not move" would have some other cause. The
/// A2 witness fixture went green for exactly that kind of reason before its own
/// premise arm was added.
fn primary_seat_holders(world: &mut World) -> usize {
    let mut q = world.query::<&DrivingParticipant>();
    q.iter(world)
        .filter(|seat| seat.0 == PlayerSlot::PRIMARY)
        .count()
}

fn rival_entity(world: &mut World) -> Entity {
    let mut q = world.query::<(Entity, &FeatureId)>();
    q.iter(world)
        .find(|(_, f)| f.as_str() == RIVAL_ID)
        .map(|(e, _)| e)
        .expect("the rival body is present")
}

/// A body that is not the player, fully constructed by the real spawn road — a
/// bare entity carrying only `DrivingParticipant` would be a claim no production
/// road can make, and the reachable case `control/authority.rs` names is exactly
/// this: *"a versus match whose seat-0 fighter legitimately holds PRIMARY"*
/// alongside a home avatar.
///
/// ⚠ `Passive`, AND BEHIND THE PLAYER. A fighter brain would close the 700px
/// over the 120 frames this test drives and turn an ambiguity measurement into a
/// combat one; a rival placed to the RIGHT would be walked into. Neither changes
/// the invariant — both would make the fixture measure something else.
fn spawn_rival(sim: &mut Platformer2dSimHarness) -> Entity {
    let p = player_pos(sim.world_mut());
    sim.spawn_enemy_character_at(
        RIVAL_ID,
        "Perfect Cellular Automaton",
        (p.x - 700.0, p.y),
        (14.0, 23.0),
        CharacterBrain::Passive,
        "perfect_cellular_automaton",
    );
    for _ in 0..8 {
        sim.step(AgentAction::default());
    }
    rival_entity(sim.world_mut())
}

#[test]
fn a_seat_claimed_by_two_bodies_drives_neither_and_recovers_when_one_vacates() {
    let mut sim = Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz())
        .expect("sandbox sim builds");
    let rival = spawn_rival(&mut sim);

    assert_eq!(
        primary_seat_holders(sim.world_mut()),
        1,
        "premise: exactly one body holds DrivingParticipant(PRIMARY) before the \
         rival claims it. With none, this fixture would measure a sim that never \
         drove; with two already, arm 1 is asserting the defect"
    );

    // ⭐ ARM 1 — THE FLOOR. The seat drives before anything is ambiguous. Without
    // this a composition that wired no control at all would satisfy arm 2.
    let unambiguous = drive_right(&mut sim, 40);
    assert!(
        unambiguous > 5.0,
        "premise: one holder of DrivingParticipant(PRIMARY) drives the primary \
         body. It travelled {unambiguous:.2}px over 40 frames of a held right \
         press, so this fixture cannot tell a refusal from a sim that never \
         drove anything"
    );

    // THE SECOND CLAIM. `PlayerSlot::PRIMARY` is now held by the player's body
    // and by a fully-constructed rival — the state `body_driving_seat` refuses.
    sim.world_mut()
        .entity_mut(rival)
        .insert(DrivingParticipant(PlayerSlot::PRIMARY));

    // ⚠ ARM 2 — THE REFUSAL, and it is measured on the PLAYER'S POSITION rather
    // than on the log line. An `error!` is not a behaviour; "the stick reaches
    // nothing" is.
    assert_eq!(
        primary_seat_holders(sim.world_mut()),
        2,
        "premise: the rival's claim survived to the measurement. \
         `project_driving_participant` sweeps a stale second holder, but ONLY \
         inside a possession window — it returns early unless \
         `PossessionState.home` is set, and this fixture never possesses. If the \
         count is 1 here, something else retracted the claim and arm 2 below is \
         measuring an unambiguous seat"
    );

    // ⚠ 5px OVER 40 FRAMES IS THIS FILE'S NEIGHBOUR'S BAR, NOT A NUMBER I CHOSE.
    // `unified_body_movement` pins "driving the slot moves the home body" at
    // `> 5.0` and "neutral input manufactures no intent" at `< 5.0` over the
    // same 40 frames, deliberately cadence-proof: a tighter bar re-rolls with
    // every combat-cadence change.
    let (ambiguous, rival_moved) = drive_right_both(&mut sim, rival, 40);
    assert!(
        ambiguous.abs() < 5.0 && rival_moved.abs() < 5.0,
        "two bodies hold DrivingParticipant(PRIMARY); the RIVAL travelled \
         {rival_moved:.2}px and the primary body still \
         travelled {ambiguous:.2}px (it travels {unambiguous:.2}px unambiguously). \
         `body_driving_seat` is supposed to refuse ambiguous authority so the \
         seat drives NOTHING until one of them vacates — a seat that picks one \
         of two holders is picking by query order, which a rollback \
         resimulation does not reproduce"
    );

    // ⭐ ARM 3 — RECOVERY. The refusal is a pause, not a latch: withdrawing the
    // second claim must hand the seat back. Without this arm, code that
    // permanently dropped the seat on first ambiguity would pass.
    sim.world_mut()
        .entity_mut(rival)
        .remove::<DrivingParticipant>();
    let recovered = drive_right(&mut sim, 40);
    assert!(
        recovered > 5.0,
        "the rival vacated the seat and the primary body still only travelled \
         {recovered:.2}px. The refusal latched: `body_driving_seat` is a per-tick \
         resolution, so one ambiguous frame must not cost the player control of \
         their own body"
    );
}
