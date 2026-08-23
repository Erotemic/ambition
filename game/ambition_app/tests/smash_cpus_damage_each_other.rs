//! Prove two CPUs in the shipped Smash composition actually fight each other.
//!
//! Each fighter must accumulate substantial damage and enter hitstun, while both
//! seats remain present. Damage is expressed as a ratio (`1.0 == 100%`); hitstun
//! distinguishes opponent hits from environmental/self damage.

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

    let mut taken = [0.0f32; 2];
    let mut last = [0.0f32; 2];
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
                // ACCUMULATED, not peaked. A KO resets a body's percent to
                // zero, so the highest reading a seat ever shows is capped by
                // how long it survives — and the faster the fight, the LOWER
                // that number goes. Measured 2026-08-23: a scorer fix that
                // raised damage, tumbling and teching across every stream pushed
                // this seat's peak from 1.69 to 0.49, because it was dying
                // before it could accumulate. Summing the rises is immune to
                // that; it is also what "each fighter must accumulate
                // substantial damage" says.
                let now = health.damage_percent();
                if now > last[seat.0] {
                    taken[seat.0] += now - last[seat.0];
                }
                last[seat.0] = now;
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
            taken[seat] >= A_REAL_FIGHT,
            "seat {seat} took {:.0}% of its pool in total over {TICKS} ticks — the \
             CPUs are not fighting. ⚠ read the UNITS before believing this: the \
             value is a RATIO, so {:.2} means {:.0}%, and a rig that printed it \
             under a literal `%` is what turned a 169% duel into a documented \
             finding that they never hit each other.",
            taken[seat] * 100.0,
            taken[seat],
            taken[seat] * 100.0,
        );
        assert!(
            hitstun_ticks[seat] > 0,
            "seat {seat} never entered hitstun, so whatever moved its damage meter \
             by {:.0}% was not the other fighter",
            taken[seat] * 100.0,
        );
    }
}

/// SWEEP THE WHOLE GRID, because every CPU number this project has is one
/// matchup's.
///
/// The demo shell's catalog carries three fighters; this composition carries the
/// whole select grid (D189). So every measurement taken through
/// `ambition_demo_smash_app`'s rigs — the decision histogram, the move census,
/// the weight override, the launch distributions the trail is fitted against —
/// describes George against George or the two stand-in duelists, and nothing
/// establishes that any of it generalises.
///
/// This is the instrument that can say. It is `#[ignore]`d because it is a
/// MEASUREMENT rather than a guard: it asserts only the thing that would make
/// its own numbers meaningless, and printing is the deliverable.
///
/// ```text
/// cargo test -p ambition_app --test app_it -- --ignored --nocapture every_fighter_on_the_grid
/// ```
#[test]
#[ignore = "a measurement, not a guard: minutes per fighter, run it when a scoring change needs validating"]
fn every_fighter_on_the_grid_can_fight_its_mirror() {
    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    app.update();
    let ids: Vec<String> = {
        let registry = app
            .world()
            .get_resource::<PreparedCharacterRegistry>()
            .expect("the composed host has a prepared-character registry");
        SmashRoster::assemble(registry)
            .ids()
            .map(str::to_string)
            .collect()
    };
    assert!(
        ids.len() > 3,
        "this composition assembled {} fighters, which is the demo shell's count — \
         so this sweep would measure exactly what the cheaper rigs already do",
        ids.len()
    );
    drop(app);

    println!(
        "[grid-sweep] {} fighters, mirror matches, {TICKS} ticks each",
        ids.len()
    );
    println!(
        "[grid-sweep] {:<34} {:>9} {:>9} {:>9}",
        "fighter", "took0%", "took1%", "hitstun"
    );
    let mut silent: Vec<String> = Vec::new();
    for id in &ids {
        let (taken, hitstun) = mirror_bout(id);
        println!(
            "[grid-sweep] {id:<34} {:>8.0}% {:>8.0}% {:>9}",
            taken[0] * 100.0,
            taken[1] * 100.0,
            hitstun[0] + hitstun[1]
        );
        if taken[0] + taken[1] < 0.1 {
            silent.push(id.clone());
        }
    }
    // The ONE assertion, and it guards the reading rather than the game: a
    // fighter that cannot be seated at all prints zeros indistinguishable from a
    // fighter that stands still, and a sweep where that goes unsaid is a table
    // of numbers with holes in it nobody can see.
    assert!(
        silent.len() * 2 < ids.len(),
        "{} of {} fighters took no damage at all in their own mirror — that is a \
         seating failure wearing a balance number: {silent:?}",
        silent.len(),
        ids.len()
    );
}

/// One mirror match in the shipped composition: damage each seat ACCUMULATED
/// (a KO resets the meter, so a peak measures survival rather than violence) and
/// ticks each spent in hitstun.
fn mirror_bout(fighter: &str) -> ([f32; 2], [usize; 2]) {
    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    app.update();
    let roster = ambition_demo_smash::smash_roster_at_levels([fighter, fighter], &[RUNG, RUNG]);
    let countdown = roster.opening_countdown_ticks as usize;
    app.world_mut().insert_resource(roster);
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(
            ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
        )));

    let mut taken = [0.0f32; 2];
    let mut last = [0.0f32; 2];
    let mut hitstun = [0usize; 2];
    for _ in 0..(countdown + TICKS) {
        app.update();
        let world = app.world_mut();
        for (seat, health, combat) in world
            .query::<(&MatchSeat, &BodyHealth, Option<&BodyCombat>)>()
            .iter(world)
        {
            if seat.0 < 2 {
                let now = health.damage_percent();
                if now > last[seat.0] {
                    taken[seat.0] += now - last[seat.0];
                }
                last[seat.0] = now;
                if combat.is_some_and(|c| c.hitstun_timer > 0.0) {
                    hitstun[seat.0] += 1;
                }
            }
        }
    }
    (taken, hitstun)
}
