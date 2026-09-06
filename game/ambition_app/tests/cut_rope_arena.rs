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
