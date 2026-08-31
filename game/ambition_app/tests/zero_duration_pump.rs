//! ⭐⭐ A ZERO-DURATION UPDATE RUNS THE SCHEDULES AND NOT THE SIMULATION.
//!
//! This is the clock half of the move-capture design. A GPU readback is
//! asynchronous: the driver has to keep calling `App::update()` until it lands.
//! Every existing multi-shot driver does that with the ORDINARY period, so the
//! simulation advances for the whole GPU wait — `capture_scene --frames` spaces
//! its shots by `stride + however long the GPU took`, which is not a spacing at
//! all. For a room burst that is invisible; for a move animation it means
//! startup or active frames pass while a PNG is in flight.
//!
//! ⭐ SO THE PUMP MUST COST NOTHING. `TimeUpdateStrategy::ManualDuration`
//! advances Bevy's clocks by exactly the duration given, so `ZERO` freezes them
//! while the rest of the frame still runs — which is what services a readback.
//!
//! ⛔ THIS TEST NEEDS NO GPU ON PURPOSE. It proves the CLOCK property, so a
//! machine with no renderer stays a supported environment; the real readback is
//! proved separately where a GPU exists.

use ambition_platformer2d::game_shell::{ShellCommand, ShellRouteId};

fn sim_tick(app: &bevy::prelude::App) -> u64 {
    app.world()
        .get_resource::<ambition_platformer2d::runtime::SimTick>()
        .map(|t| t.0)
        .unwrap_or_default()
}

#[test]
fn a_zero_duration_pump_costs_no_simulation_and_the_canonical_period_resumes() {
    use ambition_platformer2d::actor::MatchSeat;
    use bevy::prelude::*;

    let mut app =
        ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    let canonical = ambition_platformer2d::sim::enable_manual_stepping(&mut app);

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

    // Both conditions: staged is not the same as a running session.
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
        "no live rollback session, so nothing below is about a running sim"
    );

    // ── THE PUMP COSTS NOTHING ──
    //
    // ⛔⛔ AND "NOTHING" MEANS THE WORLD, NOT ONLY THE CLOCK. `capture.rs` says
    // it in its own words — *"Frozen time is not a frozen WORLD"* — and this
    // test used to check `SimTick` alone. A zero-duration `Update` system that
    // moved a body would leave the tick pinned and still make the PNG a picture
    // of a DIFFERENT world state than the semantic observation sampled before
    // the pumps. The 2026-08-31 review graded that gap P3 for want of a
    // reproduced image; this is the measurement that would reproduce one.
    //
    // ⛔ IT ASKS WHERE THINGS ARE, NOT WHICH ENTITIES THEY ARE. The first
    // version of this keyed each position by `Entity`, and it went red — but on
    // IDENTITY, not on appearance: four entities despawn and respawn during the
    // pumps at BYTE-IDENTICAL positions (the same four `[0, 11813, -18000] …`
    // rows arrive under new indices). Nothing moved; a per-entity compare simply
    // could not say so. A camera cannot see an entity id, so the multiset of
    // drawn positions is the question — and it is still the one that catches a
    // zero-time system nudging a transform.
    // ⛔ AND THE OBVIOUS POISON IS INERT, which is worth knowing before someone
    // trusts a clean run of it: a system added to `Update` that nudges SEAT
    // transforms changes nothing here, because presentation re-syncs a body's
    // transform from `SimView` every frame and the seat heals itself before
    // `GlobalTransform` propagates. Only a mutation that survives that sync can
    // be drawn — poisoned in `Last`, where it reddens as it should. That is also
    // the honest scope of this guard: a value the renderer overwrites was never
    // going to reach a PNG.
    let visual_state = |app: &mut App| -> Vec<[i64; 3]> {
        let world = app.world_mut();
        let mut q = world.query::<&GlobalTransform>();
        let mut rows: Vec<[i64; 3]> = q
            .iter(world)
            .map(|t| {
                let v = t.translation();
                // Quantised: an f32 that round-trips through the transform
                // propagation can wobble in its last bit without anything
                // MOVING, and a bitwise compare would call that a change.
                [
                    (v.x * 1000.0).round() as i64,
                    (v.y * 1000.0).round() as i64,
                    (v.z * 1000.0).round() as i64,
                ]
            })
            .collect();
        rows.sort();
        rows
    };
    let frozen_at = sim_tick(&app);
    let world_before_pumps = visual_state(&mut app);
    assert!(
        !world_before_pumps.is_empty(),
        "premise: nothing is drawable, so a comparison of drawable state would \
         pass no matter what the pumps did"
    );
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::ZERO,
    ));
    for pump in 0..30 {
        app.update();
        assert_eq!(
            sim_tick(&app),
            frozen_at,
            "a zero-duration update advanced the simulation on pump {pump} — the \
             whole point is that a GPU wait costs no simulation time, so a PNG \
             can name the exact tick it was taken on"
        );
    }

    // ⛔⛔ THE SHUTTER'S SUBJECT DID NOT MOVE. Everything the renderer would
    // extract is where the semantic observation left it, so the picture and the
    // facts beside it describe ONE execution rather than two that share a tick
    // number.
    assert_eq!(
        visual_state(&mut app),
        world_before_pumps,
        "a zero-duration pump moved drawable state while `SimTick` stayed put — \
         so the PNG is of a different world than the observation sampled before \
         the pumps, and the two are presented as the same moment"
    );

    // ── AND THE CANONICAL PERIOD RESUMES, EXACTLY ONE TICK ──
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(canonical));
    app.update();
    assert_eq!(
        sim_tick(&app),
        frozen_at + 1,
        "restoring the canonical period did not resume one tick per update — a \
         pump that cannot be un-paused is a driver that captures one frame and \
         then stops simulating"
    );
    // ⛔ AND IT KEEPS RESUMING. A single +1 could be an accumulator discharging
    // something the pumps banked rather than the clock actually running again.
    for step in 0..10 {
        let before = sim_tick(&app);
        app.update();
        assert_eq!(
            sim_tick(&app),
            before + 1,
            "step {step} after the pump advanced by something other than one tick"
        );
    }
}
