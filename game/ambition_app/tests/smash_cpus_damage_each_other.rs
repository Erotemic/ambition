//! **TWO CPUs IN THE SHIPPED COMPOSITION FIGHT EACH OTHER, AND THIS SAYS SO IN
//! UNITS NOBODY CAN MISREAD.**
//!
//! > *"In a real duel neither fighter exceeds 0.84% peak damage — they never hit
//! > each other; the 'outlast' numbers the ladder rig reports are measuring who
//! > walked off later."*
//!
//! `0.84` was **84%**. [`BodyHealth::damage_percent`] returns a RATIO — its own doc says *"`1.88`
//! is a legal answer and is how a HUD prints `188%`"* — and `ladder_rig` printed it under a literal
//! `%`.
//!
//! ## What this asserts, and why each term is needed
//!
//! ```text
//! peak damage    each fighter's meter passes half its pool  ← "a fight happened"
//! hitstun        each fighter spent time in hitstun         ← "and the damage
//!                                                              came from the OTHER
//!                                                              one, not a hazard"
//! bodies seated  both seats existed for most of the match   ← the zero floor
//! ```
//!
//! **the peak alone is not enough.** A stage hazard, a self-destruct or a
//! scripted spike would move the meter with nobody landing anything; hitstun is
//! what a body enters when something HITS it. and the seat count is asserted
//! because a match that never seated two bodies reports two peaks of zero and
//! would fail loudly for the wrong reason — or, with the assertions inverted,
//! pass silently.
//!
//! **UNITS, stated once**: every number here is a ratio of the fighter's own
//! damage pool. `1.0` is 100%. The `×100` belongs at a print site and appears
//! nowhere in the assertions.

use ambition_demo_smash::select::SmashRoster;
use ambition_platformer2d::actor::MatchSeat;
use ambition_platformer2d::actors::character_runtime::PreparedCharacterRegistry;
use ambition_platformer2d::characters::actor::{BodyCombat, BodyHealth};
use ambition_platformer2d::game_shell::{ShellCommand, ShellRouteId};
use bevy::prelude::*;

/// An ordinary fighter a player can pick — asserted to be on the assembled grid
/// below, because a name this composition cannot seat proves nothing.
const FIGHTER: &str = "npc_pirate_admiral";

/// The top authored rung. If any rung fights, this one does.
const RUNG: u8 = 9;

/// One minute at 60Hz — the same budget `ladder_rig` uses, so the two are
/// readable against each other.
const TICKS: usize = 3_600;

/// Half a pool. deliberately far below the 1.69 measured: this
/// guards *"a fight happened"*, not the tuning, and a test that pinned the
/// measured value would go red on every balance change.
const A_REAL_FIGHT: f32 = 0.5;

#[test]
fn two_cpus_in_the_shipped_composition_damage_each_other() {
    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    // the frame is load-bearing: `PreparedCharacterRegistry` is filled by a
    // `Startup` system, so a build that has never updated has a catalog and no
    // registry.
    app.update();

    {
        let registry = app
            .world()
            .get_resource::<PreparedCharacterRegistry>()
            .expect("the composed host has a prepared-character registry");
        let grid = SmashRoster::assemble(registry);
        let ids: Vec<&str> = grid.ids().collect();
        assert!(
            ids.contains(&FIGHTER),
            "`{FIGHTER}` is not on the assembled smash grid in this composition, so \
             seating it proves nothing about what a player can pick. Grid: {ids:?}"
        );
    }

    let roster = ambition_demo_smash::smash_roster_at_levels([FIGHTER, FIGHTER], &[RUNG, RUNG]);
    let countdown = roster.opening_countdown_ticks as usize;
    app.world_mut().insert_resource(roster);
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(
            ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
        )));

    let mut peak = [0.0f32; 2];
    let mut hitstun_ticks = [0usize; 2];
    let mut both_seated_ticks = 0usize;
    for _ in 0..(countdown + TICKS) {
        app.update();
        let world = app.world_mut();
        let mut seated = 0usize;
        for (seat, health, combat) in world
            .query::<(&MatchSeat, &BodyHealth, Option<&BodyCombat>)>()
            .iter(world)
        {
            if seat.0 < 2 {
                seated += 1;
                peak[seat.0] = peak[seat.0].max(health.damage_percent());
                if combat.is_some_and(|c| c.hitstun_timer > 0.0) {
                    hitstun_ticks[seat.0] += 1;
                }
            }
        }
        if seated == 2 {
            both_seated_ticks += 1;
        }
    }

    assert!(
        both_seated_ticks > TICKS / 2,
        "the match seated two fighters on only {both_seated_ticks} of {TICKS} ticks, \
         so nothing below is a measurement of a duel"
    );

    for seat in 0..2 {
        assert!(
            peak[seat] >= A_REAL_FIGHT,
            "seat {seat} peaked at {:.0}% of its pool over {TICKS} ticks — the CPUs \
             are not fighting. ⚠ read the UNITS before believing this: the value is \
             a RATIO, so {:.2} means {:.0}%, and a rig that printed it under a \
             literal `%` is what turned a 169% duel into a documented finding that \
             they never hit each other.",
            peak[seat] * 100.0,
            peak[seat],
            peak[seat] * 100.0,
        );
        assert!(
            hitstun_ticks[seat] > 0,
            "seat {seat} never entered hitstun, so whatever moved its damage meter \
             to {:.0}% was not the other fighter",
            peak[seat] * 100.0,
        );
    }
}
