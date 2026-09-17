//! A recharacterize request is still there on the frame that reads it, in every
//! pass of that frame.
//!
//! ⛔⛤ **`RecharacterizeBody` IS A ONE-SHOT REQUEST THAT DELIBERATELY CROSSES A
//! FRAME BOUNDARY, WHICH IS WHY ITS PRESENCE IS SIMULATION STATE.** Mary-O
//! inserts it from `FeatureInteraction`; `apply_worn_character_gameplay` consumes
//! it in `PlayerInputSet::Persona`, an EARLIER phase, so the request always waits
//! a frame before it is read and removed. Its registration
//! (`actor.recharacterize_request`, landed for `Q142`) says a rewind across that
//! frame which did not put the request back would apply the template zero times
//! or twice. This is the behavioural arm that reading says is owed.
//!
//! ⚠ **IT STAGES THE INSERT FROM A SIM-SCHEDULE SYSTEM RATHER THAN DRIVING A
//! MARY-O POWERUP PICKUP**, for the reason the release-marker arm in
//! `cut_rope_arena.rs` records: a state change written from outside the
//! rewinding schedule is not replayed, so only an in-sim producer can make a
//! deterministic mid-window event. What that costs is coverage of the PICKUP
//! road; what it buys is the registration's own property, which is what the
//! registration owes.

#![cfg(feature = "rl_sim")]

use ambition_app::TimestepMode;
use ambition_app::{AgentAction, Platformer2dSimHarness, Platformer2dSimHarnessOptions};

/// The tick the staged request is inserted on. Late enough that the session is
/// published and the player body exists, early enough that the window closes
/// over it several times before the arm reads the log.
const REQUEST_TICK: u64 = 40;

/// Insert the request AFTER this frame's consumer has already run, so it waits a
/// frame exactly as the production producer's does.
fn stage_a_recharacterize_request(
    mut commands: bevy::prelude::Commands,
    tick: bevy::prelude::Res<ambition_platformer2d::time::SimTick>,
    bodies: bevy::prelude::Query<
        bevy::prelude::Entity,
        ambition_platformer2d::platformer::markers::PrimaryPlayerOnly,
    >,
    mut log: bevy::prelude::ResMut<RequestByPass>,
) {
    if tick.0 != REQUEST_TICK {
        return;
    }
    let mut staged = 0usize;
    for entity in &bodies {
        commands
            .entity(entity)
            .insert(ambition_platformer2d::characters::actor::RecharacterizeBody);
        staged += 1;
    }
    log.1.push((tick.0, staged));
}

/// A second reading, at the very END of the tick, so "the insert never landed"
/// and "the insert landed and something took it away before the next tick" are
/// different answers.
fn record_the_request_at_the_tail(
    tick: bevy::prelude::Res<ambition_platformer2d::time::SimTick>,
    bodies: bevy::prelude::Query<
        bevy::prelude::Entity,
        bevy::prelude::With<ambition_platformer2d::characters::actor::RecharacterizeBody>,
    >,
    mut log: bevy::prelude::ResMut<RequestByPass>,
) {
    let seen = bodies.iter().count();
    log.2.entry(tick.0).or_default().push(seen);
}

/// Per `SimTick`, whether each PASS of that tick saw the request at the head of
/// the consumer.
///
/// ⭐ THE OBSERVABLE IS THE MARKER AND NOT THE APPLIED TEMPLATE, and the reason
/// is a lesson from the release-marker arm next door: a visible CONSEQUENCE that
/// is not itself rollback state cannot witness a rollback defect. Re-applying
/// the same template is idempotent, so the applied overlay looks identical
/// whether the request was read once or not at all.
#[derive(bevy::prelude::Resource, Default)]
struct RequestByPass(
    std::collections::BTreeMap<u64, Vec<usize>>,
    Vec<(u64, usize)>,
    std::collections::BTreeMap<u64, Vec<usize>>,
);

fn record_the_request_each_pass_sees(
    tick: bevy::prelude::Res<ambition_platformer2d::time::SimTick>,
    bodies: bevy::prelude::Query<
        bevy::prelude::Entity,
        bevy::prelude::With<ambition_platformer2d::characters::actor::RecharacterizeBody>,
    >,
    mut log: bevy::prelude::ResMut<RequestByPass>,
) {
    let seen = bodies.iter().count();
    log.0.entry(tick.0).or_default().push(seen);
}

fn arena(rollback: bool) -> Platformer2dSimHarness {
    use ambition_platformer2d::sim::SimScheduleExt;
    let mut options = Platformer2dSimHarnessOptions::default().with_timestep(TimestepMode::fixed_60hz());
    if rollback {
        options = options.with_sync_test_rollback_settings(4, 10);
    }
    Platformer2dSimHarness::build(options, |app, options| {
        use bevy::prelude::IntoScheduleConfigs as _;
        ambition_app::rl_sim::ambition_sim_composition(app, options)?;
        let label = app.sim_schedule();
        app.init_resource::<RequestByPass>();
        app.add_systems(
            label,
            record_the_request_each_pass_sees
                .before(ambition_platformer2d::actors::avatar::apply_worn_character_gameplay),
        );
        app.add_systems(
            label,
            (
                stage_a_recharacterize_request,
                record_the_request_at_the_tail,
            )
                .chain()
                .after(ambition_platformer2d::actors::avatar::apply_worn_character_gameplay),
        );
        Ok(())
    })
    .expect("the sim harness builds with a staged recharacterize request")
}

fn sim_ticks(sim: &mut Platformer2dSimHarness) -> u64 {
    sim.world()
        .get_resource::<ambition_platformer2d::time::SimTick>()
        .map(|tick| tick.0)
        .unwrap_or(0)
}

/// ⭐⭐ **THE REQUEST IS PRESENT IN EVERY PASS OF THE FRAME THAT READS IT — WHICH
/// IS WHAT `actor.recharacterize_request` BUYS.**
///
/// ⛔ Unregistered, the removal is permanent across a rewind: the first pass of
/// the reading frame consumes the request, the rewind does not put it back, and
/// a resimulated pass of that same frame applies the template zero times where
/// the first pass applied it once. The frame is then not the frame it was.
#[test]
fn a_staged_recharacterize_request_survives_every_pass_of_the_frame_that_reads_it() {
    let mut fixed = arena(false);
    let mut rewinding = arena(true);

    for _ in 0..(REQUEST_TICK as usize + 60) {
        fixed.step(AgentAction::default());
        rewinding.step(AgentAction::default());
    }

    // ⛔ THE LIVENESS FLOOR, FIRST. A frozen rollback world never reaches the
    // request tick, and "no log row" reads exactly like "the request was lost".
    let ticks = sim_ticks(&mut rewinding);
    assert!(
        ticks > REQUEST_TICK,
        "the rewinding arena reached tick {ticks} and the request is staged at \
         {REQUEST_TICK}, so nothing in this arm was exercised"
    );

    // ⛔⛤ **THE READING FRAME IS LABELLED `REQUEST_TICK`, NOT `REQUEST_TICK + 1`,
    // AND THE OFF-BY-ONE IS IN THE CLOCK RATHER THAN IN THE ARM.**
    // `advance_sim_tick` runs BETWEEN the head recorder and the consumer, so a
    // system early in the frame reads the tick number the frame BEFORE it,
    // while the staging system late in the same frame reads the new one. The
    // first version of this arm asserted `REQUEST_TICK + 1` and the control
    // failed with `[0]`; `probe_where_a_staged_recharacterize_request_goes`
    // below is what separated "the insert never landed" from "the label is one
    // behind" — the outside observer sees the marker at tick 40 and gone at 41,
    // while the in-schedule tail sees it at 40 too.
    let reading_tick = REQUEST_TICK;
    let passes = |sim: &Platformer2dSimHarness, tick: u64| -> Vec<usize> {
        sim.world()
            .resource::<RequestByPass>()
            .0
            .get(&tick)
            .cloned()
            .unwrap_or_default()
    };

    // THE CONTROL, and it is the absence of the subject. One pass, one request:
    // it says the staged insert reaches the consumer a frame later at all.
    assert_eq!(
        passes(&fixed, reading_tick),
        vec![1],
        "the fixed-tick control read {:?} at tick {reading_tick}. Without this \
         the rollback reading below says nothing — an insert that never reached \
         the consumer would report zeroes on both hosts. Staged: {:?}. Log: {:?}",
        passes(&fixed, reading_tick),
        fixed.world().resource::<RequestByPass>().1,
        {
            let log = fixed.world().resource::<RequestByPass>();
            (REQUEST_TICK..REQUEST_TICK + 4)
                .map(|tick| (tick, log.0.get(&tick).cloned(), log.2.get(&tick).cloned()))
                .collect::<Vec<_>>()
        }
    );
    // ⛔ AND THE REQUEST MUST NOT STILL BE THERE THE TICK AFTER, or the removal
    // is not happening and every pass trivially sees it. This is the assertion
    // that makes the arm about a CONSUMED request rather than a permanent one.
    assert_eq!(
        passes(&fixed, reading_tick + 1),
        vec![0],
        "the fixed-tick control still held the request at tick {}, so it is not \
         being consumed and this arm is watching a permanent component",
        reading_tick + 1
    );

    let rewinding_passes = passes(&rewinding, reading_tick);
    // ⛔ THE ANTI-VACUITY FLOOR, on the rollback side: a host that ran the
    // reading frame once has nothing to disagree with.
    assert!(
        rewinding_passes.len() > 1,
        "the rewinding arena ran tick {reading_tick} {} time(s), so no \
         resimulation of it was ever compared. Check distance 4 should give \
         several",
        rewinding_passes.len()
    );
    assert!(
        rewinding_passes.iter().all(|seen| *seen == 1),
        "a resimulated pass of the reading frame saw no request: \
         {rewinding_passes:?}. A 0 after a 1 is the pre-registration signature — \
         the consumption was not restored, so that pass applies the template \
         zero times where the first pass applied it once"
    );
}


/// Where does a staged request actually go? Read from OUTSIDE the schedule, so
/// no in-schedule ordering assumption is involved.
#[test]
#[ignore = "probe: prints the marker as an outside observer sees it, per step"]
fn probe_where_a_staged_recharacterize_request_goes() {
    let mut sim = arena(false);
    for step in 0..(REQUEST_TICK as usize + 8) {
        sim.step(AgentAction::default());
        let tick = sim_ticks(&mut sim);
        let markers = {
            let world = sim.world_mut();
            let mut q = world.query_filtered::<
                bevy::prelude::Entity,
                bevy::prelude::With<ambition_platformer2d::characters::actor::RecharacterizeBody>,
            >();
            q.iter(world).count()
        };
        let bodies = {
            let world = sim.world_mut();
            let mut q = world.query_filtered::<
                bevy::prelude::Entity,
                ambition_platformer2d::platformer::markers::PrimaryPlayerOnly,
            >();
            q.iter(world).count()
        };
        if step + 8 >= REQUEST_TICK as usize {
            eprintln!("PROBE step {step:>3} tick {tick:>3}: markers {markers}, primary bodies {bodies}");
        }
    }
    let log = sim.world().resource::<RequestByPass>();
    eprintln!("PROBE staged: {:?}", log.1);
    for tick in (REQUEST_TICK - 2)..(REQUEST_TICK + 4) {
        eprintln!(
            "PROBE tick {tick}: head {:?} tail {:?}",
            log.0.get(&tick),
            log.2.get(&tick)
        );
    }
}
