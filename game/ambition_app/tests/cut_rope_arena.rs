//! The cut-rope fight's ONE content-specific trigger, driven headlessly.
//!
//! ⛔⛔ `boss_lifecycle`'s header records this fight as *"content-specific +
//! headless-hard (R5 rewrites cut-rope as an EncounterScript); they remain an
//! explicit in-game verification item"*. **R5 HAS LANDED** —
//! `setup_cut_rope_encounter` is registered in `ContentEncounterScriptSet` and
//! its own doc says the fight is now "the generic encounter pieces... no
//! cut-rope-specific physics or steering". ⇒ The deferral's stated condition has
//! been met, and nobody re-derived it. Measured: the room boots headlessly in
//! **0.87 s** with both authored props present.
//!
//! ⭐ WHAT THIS PINS, and why it is the right seam rather than a convenient one.
//! `detect_cut_rope_rope_cut`'s doc: *"The rope-cut is the ONLY cut-rope-specific
//! trigger; everything after it is the generic encounter script + falling-hazard
//! mechanic."* So `EncounterGate("rope_cut")` is the whole contract between this
//! content and the generic machinery. The arena's own state is entirely private
//! — a test cannot read `rope_cut` even from inside the workspace — and that is
//! correct: the published message is the seam, and asserting it is asserting what
//! other code depends on.

#![cfg(feature = "rl_sim")]

use ambition_app::AmbitionSim;
use ambition_app::{AgentAction, Platformer2dSimHarness, Platformer2dSimHarnessOptions};
use ambition_app::TimestepMode;
use ambition_platformer2d::boss_encounter::EncounterGate;
use ambition_platformer2d::world::rooms::RoomSet;

const CUT_ROPE_ROOM: &str = "you_have_to_cut_the_rope";
const ROPE_KIND: &str = "cut_rope_rope";

fn cut_rope_sim() -> Platformer2dSimHarness {
    let opts = Platformer2dSimHarnessOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        // ⚠ REQUIRED, not optional: the tolerant road falls back to the authored
        // start room and would run this whole test in the wrong room, green.
        .with_required_start_room(CUT_ROPE_ROOM);
    Platformer2dSimHarness::new_with_options(opts).expect("the cut-rope room builds headlessly")
}

/// The authored rope prop's centre, read from the live room rather than authored
/// into the test — a literal here would go stale the moment the map moves.
fn rope_pos(sim: &mut Platformer2dSimHarness) -> ambition_platformer2d::engine_core::Vec2 {
    let world = sim.world_mut();
    let mut q = world.query::<&RoomSet>();
    q.iter(world)
        .next()
        .and_then(|rooms| {
            let spec = rooms.active_spec();
            assert_eq!(spec.id, CUT_ROPE_ROOM, "the harness started in the wrong room");
            spec.props
                .iter()
                .find(|p| p.kind == ROPE_KIND)
                .map(|p| p.pos)
        })
        .expect("the cut-rope room authors a rope prop")
}

fn slash(sim: &mut Platformer2dSimHarness, at: ambition_platformer2d::engine_core::Vec2) {
    use ambition_platformer2d::combat::events::{HitEvent, HitMode, HitSource, HitTarget};
    sim.world_mut().write_message(HitEvent {
        volume: ambition_platformer2d::engine_core::Aabb::new(
            at,
            ambition_platformer2d::engine_core::Vec2::splat(24.0),
        )
        .into(),
        damage: 10,
        source: HitSource::Melee,
        attacker: None,
        target: HitTarget::UnresolvedFeatures,
        mode: HitMode::Knockback,
        knockback: None,
        ignored_targets: Vec::new(),
        strike_sfx: None,
            attacker_move_instance: None,
    });
}

fn rope_cut_gates(sim: &mut Platformer2dSimHarness) -> usize {
    sim.world_mut()
        .get_resource::<bevy::ecs::message::Messages<EncounterGate>>()
        .map(|gates| {
            gates
                .iter_current_update_messages()
                .filter(|gate| gate.gate == "rope_cut")
                .count()
        })
        .unwrap_or(0)
}

/// Slashing the authored rope hands the fight to the generic encounter script.
#[test]
fn slashing_the_rope_publishes_the_gate_the_encounter_script_waits_on() {
    let mut sim = cut_rope_sim();
    for _ in 0..10 {
        sim.step(AgentAction::default());
    }
    let rope = rope_pos(&mut sim);

    slash(&mut sim, rope);
    sim.step(AgentAction::default());

    assert!(
        rope_cut_gates(&mut sim) > 0,
        "a hit on the authored rope published no `rope_cut` gate, so the encounter \
         script never gets the lure-and-drop beat and the fight cannot be won"
    );
}

/// ⭐⭐ THE CONTROL ARM, and without it the test above is worth nothing: it would
/// pass identically if the gate fired every frame on its own, or on room entry.
/// Same room, same frames, no slash.
#[test]
fn the_rope_gate_does_not_fire_without_a_hit_on_the_rope() {
    let mut sim = cut_rope_sim();
    for _ in 0..10 {
        sim.step(AgentAction::default());
    }
    assert_eq!(
        rope_cut_gates(&mut sim),
        0,
        "the `rope_cut` gate fired without anything striking the rope"
    );

    // ⚠ AND A HIT SOMEWHERE ELSE IS NOT A ROPE CUT. This is the arm that says the
    // trigger is the ROPE and not merely "a HitEvent happened this frame" —
    // without it, a detector that ignored geometry entirely would pass both tests
    // above.
    let rope = rope_pos(&mut sim);
    slash(
        &mut sim,
        ambition_platformer2d::engine_core::Vec2::new(rope.x + 4_000.0, rope.y + 4_000.0),
    );
    sim.step(AgentAction::default());
    assert_eq!(
        rope_cut_gates(&mut sim),
        0,
        "a hit 4000px from the rope cut the rope, so the detector is not reading \
         the rope's geometry at all"
    );
}

/// A replay lets the rope be cut AGAIN — the reset path, which nothing pinned.
///
/// ⭐⭐ THIS IS THE ARM THE OTHER THREE DO NOT REACH. `detect_cut_rope_rope_cut`
/// short-circuits on `if state.rope_cut { continue; }`, so once the rope is cut
/// the trigger is dead until something clears the flag. If the reset never ran,
/// a player who died mid-fight would re-enter a room whose rope is already cut
/// and whose anvil will never drop again — the fight becomes unwinnable, and
/// every other test here still passes because they each cut the rope exactly
/// once in a fresh world.
///
/// ⇒ It also pins the seam a refactor wants to move.
/// `CutRopeBossArenaState.active_room` hand-rolls a room-change detector that
/// `FreshAttempt::began_in` already is, spelled three times, with a fourth site
/// checking the same condition and BAILING rather than resetting. Swapping that
/// for the engine's own mechanism carries a one-frame ordering hazard, and this
/// is the arm that would catch it.
#[test]
fn a_replay_lets_the_rope_be_cut_again() {
    use ambition_platformer2d::combat::events::{RoomReplayAdmitted, RoomResetReason};

    let mut sim = cut_rope_sim();
    for _ in 0..10 {
        sim.step(AgentAction::default());
    }
    let rope = rope_pos(&mut sim);

    slash(&mut sim, rope);
    sim.step(AgentAction::default());
    assert!(rope_cut_gates(&mut sim) > 0, "the first cut must land");

    // ⚠ The premise the rest of this test rests on: a SECOND slash with no replay
    // does nothing, because the detector short-circuits on `rope_cut`. Without
    // this, "the gate fired again after a replay" would prove nothing -- it would
    // pass on a detector that simply fires on every hit.
    slash(&mut sim, rope);
    sim.step(AgentAction::default());
    assert_eq!(
        rope_cut_gates(&mut sim),
        0,
        "a second slash fired the gate with no replay, so this test cannot tell a \
         working reset from a detector that never latched"
    );

    sim.world_mut().write_message(RoomReplayAdmitted {
        reason: RoomResetReason::PlayerDeath,
        subject: None,
    });
    sim.step(AgentAction::default());

    slash(&mut sim, rope);
    sim.step(AgentAction::default());
    assert!(
        rope_cut_gates(&mut sim) > 0,
        "after a replay the rope could not be cut again: the arena kept last \
         attempt's `rope_cut`, so the anvil never drops and the fight is unwinnable"
    );
}

/// ⭐⭐ **THE SCRIPT'S BEAT CLOCK, WITNESSED UNDER AN ACTUAL ROLLBACK WINDOW.**
///
/// `EncounterScript` became `component-clone-custom-checksum` on 2026-09-17
/// (schema v198) because `cursor` and `elapsed` are advanced every tick by
/// `tick_encounter_scripts`, in the sim schedule. ⛔ **THAT WAS A CLAIM ABOUT
/// REGISTRATION, NOT A READING OF THE NUMBER**, and it was made from the shape
/// of the code rather than from a rewind anybody had run.
///
/// ⚠ This room is the only place the claim can be tested through PRODUCTION:
/// `setup_cut_rope_encounter` is the workspace's one non-test inserter of an
/// `EncounterScript`, and it needs this room's authored anvil. A test-side
/// insert would not do — a registered component inserted outside the rewinding
/// schedule is taken back by the first rewind, which is a different defect
/// wearing this one's clothes.
///
/// ⛔⛤ **AND THE FIRST VERSION OF THIS ARM CUT THE ROPE, WHICH MADE ITS SUBJECT
/// DISAPPEAR.** `slash` writes a `HitEvent` from the test, outside the rewinding
/// schedule, and the rewind takes that write back: measured **1 rope-cut gate
/// without a rollback window and 0 under one**, so the script sat on beat 0 and
/// the arm read `Some(0)` vs `Some(2)` — a difference that says nothing about
/// the registration and everything about the injection. The premise guard that
/// caught it is why the number below is worth reading.
///
/// ⇒ So this drives NO input at all. The script is attached by production the
/// moment the anvil loads, and `EncounterScript::advance` adds `dt` to
/// `beat_elapsed` on every tick whether or not its trigger holds. The clock is
/// the half a resimulated tick inflates when nothing restores it, and it needs
/// no gate to observe.
const FRAMES_OF_WAITING: usize = 240;

fn cut_rope_rewinding_sim() -> Platformer2dSimHarness {
    let opts = Platformer2dSimHarnessOptions::default()
        .with_timestep(TimestepMode::fixed_60hz())
        .with_required_start_room(CUT_ROPE_ROOM)
        .with_sync_test_rollback_settings(4, 10);
    Platformer2dSimHarness::new_with_options(opts)
        .expect("the cut-rope room builds under a GGRS sync-test session too")
}

/// The beat the encounter's script is on and how long it has been on it.
///
/// `None` is a real answer and it is NOT "beat zero, zero seconds": it means no
/// script was ever attached, which is the vacuous case this arm has to refuse.
fn script_beat(sim: &mut Platformer2dSimHarness) -> Option<(usize, f32)> {
    use ambition_platformer2d::boss_encounter::EncounterScript;
    let world = sim.world_mut();
    let mut q = world.query::<&EncounterScript>();
    q.iter(world)
        .next()
        .map(|script| (script.cursor(), script.beat_elapsed()))
}

fn wait_out_the_first_beat(sim: &mut Platformer2dSimHarness) {
    for _ in 0..FRAMES_OF_WAITING {
        sim.step(AgentAction::default());
    }
}

#[test]
fn the_encounter_script_clock_reaches_the_same_value_with_and_without_a_rewind() {
    let mut fixed = cut_rope_sim();
    wait_out_the_first_beat(&mut fixed);
    let without_rollback = script_beat(&mut fixed);

    let mut rewinding = cut_rope_rewinding_sim();
    wait_out_the_first_beat(&mut rewinding);
    let under_rollback = script_beat(&mut rewinding);


    // ⛔⛔ THE FLOOR, BEFORE EITHER VALUE IS READ. `None == None` satisfies the
    // comparison below perfectly and means this arm examined a room with no
    // script in it.
    let (beat, elapsed) = without_rollback.expect(
        "no `EncounterScript` exists in the fixed-tick world, so this arm has no \
         subject and the comparison below would pass on two absent values. \
         `setup_cut_rope_encounter` attaches it once the authored anvil has \
         loaded — check the room still authors one.",
    );
    assert!(
        elapsed > 0.0,
        "the script's beat clock is still {elapsed} after {FRAMES_OF_WAITING} \
         frames on beat {beat}, so a clock that never advances would pass this \
         arm. `tick_encounter_scripts` is not reaching this encounter."
    );

    // ⛔⛤ **THE TWO WORLDS DO NOT RUN THE SAME NUMBER OF TICKS, AND COMPARING
    // THE CLOCKS DIRECTLY READ AS A DEFECT.** Measured: `4.0333304` under a
    // rollback window against `4.0166636` without — one frame's worth, which
    // looks exactly like a clock that was not restored until you ask how many
    // ticks each world actually ran. `SimTick` says **241 against 240**: the
    // sync-test harness steps once more, so the extra sixtieth of a second is
    // the tick, not the rewind.
    //
    // ⇒ The property is therefore a RATE, not a value: the beat clock must
    // advance exactly one `dt` per tick in each world. An unrestored clock
    // advances again on every resimulated tick — `check_distance` 4, so four
    // extra advances per step — and that is a difference of hundreds of frames,
    // not one.
    let (_, rollback_elapsed) = under_rollback.expect("the rewinding world has a script too");
    let fixed_ticks = sim_ticks(&mut fixed) as i64;
    let rollback_ticks = sim_ticks(&mut rewinding) as i64;

    // ⛔⛔ THE LIVENESS FLOOR, AND WITHOUT IT THE COMPARISON BELOW IS VACUOUS.
    // A sync-test session that invalidates keeps ACCEPTING `sim.step()` and
    // stops advancing `SimTick` — nothing panics and nothing prints. A frozen
    // rollback world would freeze the beat clock too, so both deltas would move
    // together and the assertion would agree with itself forever. This is the
    // mechanism `a_rollback_arm_must_refuse_a_frozen_world.py` routes every
    // sync-test arm through, and it is load-bearing rather than a health call
    // bolted on the end.
    assert!(
        rollback_ticks >= FRAMES_OF_WAITING as i64,
        "the rollback world ran {rollback_ticks} ticks for {FRAMES_OF_WAITING} \
         steps, so its sync-test session stopped advancing. Every comparison \
         below would hold over the frozen world."
    );
    let clock_frames = |seconds: f32| (f64::from(seconds) * 60.0).round() as i64;

    assert_eq!(
        clock_frames(rollback_elapsed) - clock_frames(elapsed),
        rollback_ticks - fixed_ticks,
        "the encounter script's beat clock and the sim tick disagree about how \
         much time passed: the clock moved {} frame(s) more under a rollback \
         window while the world ran {} tick(s) more ({rollback_elapsed} vs \
         {elapsed} seconds, {rollback_ticks} vs {fixed_ticks} ticks). `cursor` \
         and `elapsed` are advanced by `tick_encounter_scripts` inside the \
         rewinding schedule, so the component has to be restored with the world — \
         check its registration in `ambition_encounter`'s `register_rollback_state` \
         survived. A scripted fight whose clock ran ahead fires its timed beats \
         early, and `ForceKill` is one of them.",
        clock_frames(rollback_elapsed) - clock_frames(elapsed),
        rollback_ticks - fixed_ticks
    );
}

/// How many fixed steps this world has executed. The control the clock
/// comparison needs: two harnesses do not run the same number.
fn sim_ticks(sim: &mut Platformer2dSimHarness) -> u64 {
    sim.world_mut()
        .get_resource::<ambition_platformer2d::time::SimTick>()
        .expect("the sim tick is installed by the engine plugins")
        .0
}
