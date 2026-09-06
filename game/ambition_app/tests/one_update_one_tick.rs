//! ⭐⭐ THE MANUAL-STEP CONTRACT, ASSERTED PER UPDATE.
//!
//! `app::enable_manual_stepping` promises that one `App::update()` advances the
//! simulation by exactly one tick. A probe once "verified" this by reading the
//! frame counter before 120 calls and after 120 calls and checking the delta was
//! 120 — which a run containing one zero-tick update and one two-tick update
//! passes. The property is per update, so the assertion has to be per update.
//!
//! ⛔ AND IT ASSERTS ON `SimTick`, THE ENGINE'S OWN DECLARED TIMELINE. Its docs
//! call it *"the canonical timeline (netcode N0.1)... N0.2 input streams are
//! keyed by it, N0.4 hashes sim state per value of it, and rollback rewinds to
//! one"*, and `advance_sim_tick` sits unconditionally at the head of the sim
//! schedule. A driver that had to watch `RollbackFrameCount` instead would be
//! establishing a SECOND timeline to route around a defect in the first.

use ambition_platformer2d::game_shell::{ShellCommand, ShellRouteId};

fn sim_tick(app: &bevy::prelude::App) -> u64 {
    app.world()
        .get_resource::<ambition_platformer2d::runtime::SimTick>()
        .map(|t| t.0)
        .unwrap_or_default()
}

/// ⛔⛔ MANUAL TIME IS INSTALLED BEFORE THE SESSION EXISTS. Switching a running
/// rollback host from wall time to manual leaves whatever its accumulator had
/// already banked, so the first steps are not one-for-one and a test that did it
/// the other way would be measuring the changeover.
#[test]
fn one_update_is_one_simulation_tick_in_a_live_rollback_match() {
    use ambition_platformer2d::actor::MatchSeat;
    use bevy::prelude::*;

    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    let period = ambition_platformer2d::sim::enable_manual_stepping(&mut app);
    assert_eq!(
        period,
        std::time::Duration::from_nanos(1_000_000_000 / 60),
        "the smash composition runs the ROLLBACK host, whose period is the \
         truncated 16_666_666ns — the rounded fixed-step value costs a tick of \
         drift every few thousand frames"
    );

    for _ in 0..30 {
        app.update();
    }
    app.world_mut()
        .insert_resource(ambition_demo_smash::smash_roster([
            "npc_pirate_admiral",
            "npc_pirate_admiral",
        ]));
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(
            ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
        )));

    // ⛔⛔ TWO CONDITIONS, NOT ONE. A `MatchSeat` existing proves a fighter was
    // STAGED; it does not prove the GGRS SESSION is running. An update in the gap
    // between the two legitimately advances zero ticks, and a test that started
    // asserting there would accuse `SimTick` of a defect that is really its own
    // impatience — which matters enormously here, because a failure of this test
    // is meant to be read as an ENGINE INVARIANT defect.
    let mut live = false;
    for _ in 0..900 {
        app.update();
        let staged = {
            let world = app.world_mut();
            let mut all = world.query::<&MatchSeat>();
            all.iter(world).count() > 0
        };
        if staged && ambition_platformer2d::rollback::session_is_active(app.world()) {
            live = true;
            break;
        }
    }
    assert!(
        live,
        "no live rollback session: either the match never staged or GGRS never \
         activated, and nothing below is about a running simulation"
    );

    // ⭐ EVERY UPDATE, NOT THE SUM OF THEM.
    let mut previous = sim_tick(&app);
    let mut deltas: Vec<u64> = Vec::new();
    for _ in 0..120 {
        app.update();
        let now = sim_tick(&app);
        deltas.push(now.saturating_sub(previous));
        previous = now;
    }
    let wrong: Vec<(usize, u64)> = deltas
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, d)| *d != 1)
        .collect();
    assert!(
        wrong.is_empty(),
        "manual stepping promised one simulation tick per update and {} of 120 \
         updates disagreed (index, ticks): {:?} — an aggregate check over the \
         whole run would have hidden this",
        wrong.len(),
        &wrong[..wrong.len().min(8)],
    );
}
