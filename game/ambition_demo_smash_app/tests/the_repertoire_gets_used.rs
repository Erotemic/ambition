//! **Does the CPU actually PLAY the repertoire, or does it own one?**
//!
//! ⛔ the failure this exists to catch is the one that motivated the whole pass:
//! a fighter can carry sixteen authored moves and throw the same jab for a whole
//! match, and every other test in this crate passes while it does. Authoring is
//! not a behaviour claim. The claim has to be measured on the stage.
//!
//! So this is an INSTRUMENT first and an assertion second. It watches which
//! moves actually start on a body during a real CPU-versus-CPU match — the same
//! `MovePlayback` the runtime inserts when a press resolves, so nothing here can
//! observe a move the body did not perform — and reports the histogram in its
//! failure message. A regression tells you what the fighter did instead.
//!
//! ⚠ **the thresholds are floors, not targets.** ⛔ do not tune content against
//! them: "five distinct move ids" is the difference between a repertoire and a
//! single swing, and a fighter that hits exactly five is barely passing rather
//! than correct.

use std::collections::BTreeMap;

use ambition_demo_smash_app::build_demo_app;
use ambition_platformer2d::actor::MatchSeat;
use ambition_platformer2d::combat::moveset::MovePlayback;
use bevy::prelude::*;

/// Which body threw what, counted by move id.
#[derive(Default)]
struct MoveLedger {
    /// Every move START observed, per seat.
    started: BTreeMap<usize, BTreeMap<String, usize>>,
    /// The last `(move id, clock)` seen per entity, so a move that is still
    /// running is not counted again every tick.
    live: BTreeMap<Entity, (String, f32)>,
}

impl MoveLedger {
    /// Sample the world once. A move START is an entity whose playback names a
    /// different move than last tick, or the same move with a clock that went
    /// backwards (the press landed again).
    fn sample(&mut self, app: &mut App) {
        let world = app.world_mut();
        let mut q = world.query::<(Entity, &MatchSeat, &MovePlayback)>();
        let rows: Vec<(Entity, usize, String, f32)> = q
            .iter(world)
            .map(|(e, seat, pb)| (e, seat.0, pb.spec.id.clone(), pb.t))
            .collect();
        for (entity, seat, id, t) in rows {
            let fresh = match self.live.get(&entity) {
                Some((last_id, last_t)) => last_id != &id || t < *last_t,
                None => true,
            };
            if fresh {
                *self
                    .started
                    .entry(seat)
                    .or_default()
                    .entry(id.clone())
                    .or_default() += 1;
            }
            self.live.insert(entity, (id, t));
        }
        // A body whose move finished drops out of the query; forget it so the
        // next press counts as a start.
        let alive: std::collections::BTreeSet<Entity> = {
            let world = app.world_mut();
            let mut q = world.query_filtered::<Entity, With<MovePlayback>>();
            q.iter(world).collect()
        };
        self.live.retain(|e, _| alive.contains(e));
    }

    fn distinct_for_the_busiest_seat(&self) -> usize {
        self.started
            .values()
            .map(|by_move| by_move.len())
            .max()
            .unwrap_or(0)
    }

    fn every_move_seen(&self) -> BTreeMap<&str, usize> {
        let mut all: BTreeMap<&str, usize> = BTreeMap::new();
        for by_move in self.started.values() {
            for (id, n) in by_move {
                *all.entry(id.as_str()).or_default() += *n;
            }
        }
        all
    }
}

/// Seat two CPUs of the same character at the same rung, let the countdown pass,
/// and watch for `ticks` frames.
fn watch_a_match(character: &str, ticks: usize) -> MoveLedger {
    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    // ⚠ **both seats CPU, same rung.** `smash_roster_at_levels` is the helper
    // that seats every slot as a CPU; the sibling test in `the_stage_kills` has
    // the scar from using the one that makes seat 0 a human with no controller,
    // which measures one fighter pacing around a statue.
    app.world_mut()
        .insert_resource(ambition_demo_smash::smash_roster_at_levels(
            [character, character],
            &[5, 5],
        ));
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            ),
        ));
    // ⛔ **the warm-up has to outlast the countdown.** The stage opens suspended
    // and every fighter carries scripted control for the whole 3-2-1-GO, so a
    // window inside the hold measures fighters that are correctly forbidden to
    // act. Read from the ruleset rather than restating it.
    let countdown =
        ambition_demo_smash::smash_roster([character, character]).opening_countdown_ticks;
    for _ in 0..(countdown + 30) {
        app.update();
    }

    let mut ledger = MoveLedger::default();
    for _ in 0..ticks {
        app.update();
        ledger.sample(&mut app);
    }
    ledger
}

/// **A FIGHTER WITH SIXTEEN MOVES THROWS MORE THAN ONE OF THEM.**
///
/// ⭐ the acceptance number, measured through the ordinary press seam: the CPU
/// picks a binding, the body resolves it through `move_for_directional_verb` —
/// the same function a human's stick reaches — and the runtime inserts a
/// `MovePlayback`. Nothing in this path is CPU-specific, which is why the same
/// measurement is evidence for the human case too.
///
/// ⛔ **and the poison is the shared table.** `smash_duelist_a` carries eleven
/// moves with no specials at all; if THAT fighter also reached this floor, the
/// number would be measuring the brain's appetite for variety rather than
/// George's repertoire. Both are reported either way.
#[test]
fn the_cpu_reaches_for_more_than_one_move() {
    const WINDOW: usize = 900;
    const FLOOR: usize = 5;

    let george = watch_a_match(ambition_demo_smash::SMASH_GEORGE_BOOUL, WINDOW);
    let distinct = george.distinct_for_the_busiest_seat();
    assert!(
        distinct >= FLOOR,
        "over {WINDOW} ticks the busiest George seat started only {distinct} \
         distinct moves (floor {FLOOR}). What it actually threw: {:?}",
        george.every_move_seen()
    );
}

/// **THE RECOVERY IS A MOVE THE CPU THROWS, NOT A MOVE IT OWNS.**
///
/// ⛔⛔ this is the one that could pass vacuously in the most expensive way. L2
/// used to return an EMPTY attack list in `Situation::Recovery` — *"a body past
/// the blastzone has exactly one problem"* — so a fighter carrying a real Up-B
/// drifted and jumped at a stage it could not reach while holding the thing that
/// would have saved it. Authoring the move fixes nothing on its own; the brain
/// has to offer it.
///
/// ⚠ **the measurement is deliberately weak on WHEN and strict on WHETHER.**
/// Pinning the recovery to a particular offstage position would be pinning the
/// tuning of a demo. What must never be true is that the move is never thrown in
/// a match where fighters are being launched off a stage.
#[test]
fn the_cpu_throws_its_authored_recovery_during_a_match() {
    const WINDOW: usize = 1800;

    let ledger = watch_a_match(ambition_demo_smash::SMASH_GEORGE_BOOUL, WINDOW);
    let seen = ledger.every_move_seen();
    let ascents = seen.get("excluded_middle").copied().unwrap_or(0);
    assert!(
        ascents > 0,
        "over {WINDOW} ticks nobody threw the authored recovery. Everything the \
         two Georges did throw: {seen:?}"
    );
}

/// **AND IT IS THE REPERTOIRE DOING IT, not the brain's appetite for variety.**
///
/// The poison for the test above, run as its own case so a failure says which
/// half broke: the stand-in duelists author no special at all, so their busiest
/// seat cannot possibly produce `excluded_middle` — and if a future change made
/// them, this measurement would be reading something other than what a character
/// authored.
#[test]
fn a_fighter_that_authored_no_special_throws_none() {
    let ledger = watch_a_match(ambition_demo_smash::SMASH_CHARACTER_ID, 600);
    let seen = ledger.every_move_seen();
    assert!(
        !seen.contains_key("excluded_middle"),
        "the stand-in table has no specials, but a seat threw one: {seen:?}"
    );
    // It is not standing still either — the contrast is about the REPERTOIRE,
    // and an empty ledger would make the assertion above meaningless.
    assert!(
        !seen.is_empty(),
        "the stand-in fighters threw nothing at all, so this poison proves nothing"
    );
}
