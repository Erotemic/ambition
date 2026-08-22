//! **Does a CPU match ever produce a grab?**
//!
//! `cargo run -p ambition_demo_smash_app --bin capture_probe [-- SECONDS]`
//!
//! Every claim about the capture mechanic so far is a unit test or a hand-built
//! chain. Those answer *"can this happen"*; the question a mechanic actually has
//! to pass is *"does it happen in a fight nobody arranged"* — and its opposite,
//! *"does it happen so much that the fight is only grabs"*. Neither is a
//! property any fixture can hold an opinion about.
//!
//! So this drives the real demo the way `match_diagram` does — select, seat two
//! CPU fighters, route to the stage, step the sim — and watches the
//! relationship table:
//!
//! ```text
//! holds        a capture that was established at all
//! pummels      the deepest a hold got
//! endings      thrown / escaped / timed out / interrupted
//! ```
//!
//! **an ENDING is classified from the hold's last observed state**, because by
//! the time a capture is gone the component that knew is gone with it. A hold
//! whose escape meter had filled escaped; one at its ceiling timed out; anything
//! else ended because somebody chose to end it — a throw, or a hit that broke
//! it. That is a coarse classification and it is the honest one available from
//! outside.
//!
//! **it is a PROBE, not a test.** It reports what a match did; it asserts
//! nothing, because the number a healthy match produces is not known yet and a
//! threshold invented here would be a fixture pretending to be a design.

use std::collections::HashMap;

use ambition_platformer2d::characters::smash_capture::SmashHoldState;
use ambition_platformer2d::combat::capture::{CaptureAttemptRequested, CapturedBy};

/// How many capture attempts the adapter actually asked for.
///
/// **the split that matters when a match produces grabs and no holds**: an
/// attempt that was never REQUESTED is an authoring or adapter problem, and one
/// that was requested and refused is an eligibility problem. From outside they
/// look identical.
#[derive(bevy::prelude::Resource, Default)]
struct AttemptsSeen(u32);

/// **Press Grab FOR them, on the tick a person would.**
///
/// **it has to be a SYSTEM in the sim schedule, and writing the frame from
/// outside `app.update()` measures nothing** — the fighter brain writes
/// `ActorControl` every tick, so a press stamped after the update is overwritten
/// before anything reads it. That mistake reported 567 presses and 3 attempts,
/// which reads exactly like the game refusing a grab.
#[derive(bevy::prelude::Resource, Default)]
struct Forced(u32);

fn force_a_grab_in_range(
    mut forced: bevy::prelude::ResMut<Forced>,
    mut bodies: bevy::prelude::Query<(
        bevy::prelude::Entity,
        &ambition_platformer2d::engine_core::BodyKinematics,
        &mut ambition_platformer2d::characters::control::ActorControl,
        Option<&ambition_platformer2d::combat::moveset::MovePlayback>,
    )>,
) {
    let mut rows: Vec<(bevy::prelude::Entity, f32, f32, bool)> = bodies
        .iter()
        .map(|(entity, kin, _, playback)| (entity, kin.pos.x, kin.facing, playback.is_some()))
        .collect();
    if rows.len() != 2 {
        return;
    }
    rows.sort_by_key(|(entity, ..)| *entity);
    let gap = (rows[0].1 - rows[1].1).abs();
    if gap > 30.0 {
        return;
    }
    for index in 0..2 {
        let (entity, x, facing, busy) = rows[index];
        let toward = (rows[1 - index].1 - x).signum();
        if busy || facing.signum() != toward {
            continue;
        }
        if let Ok((_, _, mut control, _)) = bodies.get_mut(entity) {
            control.0.grab_pressed = true;
            forced.0 += 1;
        }
        return;
    }
}

fn count_attempts(
    mut attempts: bevy::prelude::MessageReader<CaptureAttemptRequested>,
    mut seen: bevy::prelude::ResMut<AttemptsSeen>,
) {
    seen.0 += attempts.read().count() as u32;
}

fn main() {
    use bevy::prelude::IntoScheduleConfigs as _;
    let seconds: f32 = std::env::args()
        .nth(1)
        .and_then(|arg| arg.parse().ok())
        .unwrap_or(60.0);
    let ticks = (seconds * 60.0) as u32;
    // **`--force`: press Grab FOR them, when a person would.** The CPU's own
    // timing is a policy question; whether the live game can produce a hold at
    // all is not, and the two are only separable by taking the timing out of the
    // AI's hands. Presses on the tick the two are inside grab range and the
    // presser is not already committed to a move — which is exactly the moment a
    // player picks.
    let force = std::env::args().any(|arg| arg == "--force");

    let mut app = ambition_demo_smash_app::build_demo_app();
    app.init_resource::<AttemptsSeen>();
    app.add_systems(bevy::prelude::Update, count_attempts);
    app.init_resource::<Forced>();
    if force {
        let sim =
            ambition_platformer2d::platformer::schedule::SimScheduleExt::sim_schedule(&mut app);
        app.add_systems(
            sim,
            // **AFTER the brain, not merely before combat.** `.before(C)`
            // orders nothing against the systems that also run before C, so a
            // press stamped here raced the actor brain's own `*out = frame` and
            // lost — the second time the same clobber ate this experiment.
            force_a_grab_in_range
                .after(
                    ambition_platformer2d::platformer::schedule::Platformer2dSimulationPhaseMonolith::WorldPrep,
                )
                .before(ambition_platformer2d::platformer::schedule::CombatSet::Trigger),
        );
    }
    for _ in 0..30 {
        app.update();
    }
    // **CPU seats, not the select screen's** — `SmashSelect::roster` makes
    // every locked seat a HUMAN, and two humans with no controllers stand still
    // forever. The same note `match_diagram` carries, for the same reason.
    app.world_mut()
        .insert_resource(ambition_demo_smash::smash_roster([
            ambition_demo_smash::SMASH_CHARACTER_ID,
            ambition_demo_smash::SMASH_OPPONENT_ID,
        ]));
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(
                ambition_demo_smash::SMASH_GAMEPLAY_ROUTE,
            ),
        ));
    for _ in 0..240 {
        app.update();
    }

    #[derive(Default)]
    struct Tally {
        holds: u32,
        pummels: u32,
        deepest: u8,
        thrown_or_broken: u32,
        escaped: u32,
        timed_out: u32,
        held_ticks: u32,
    }
    // **Does the body even OWN a grab?** The first suspect behind a zero, and
    // the cheapest to eliminate: a fighter whose contract binds no grab verb can
    // never be offered one however good the scoring is.
    {
        let world = app.world_mut();
        let mut query = world.query::<(
            &ambition_platformer2d::actor::MatchSeat,
            &ambition_platformer2d::combat::moveset::ActorMoveset,
            Option<&ambition_platformer2d::engine_core::BodyAbilities>,
        )>();
        let mut rows: Vec<String> = query
            .iter(world)
            .map(|(seat, moveset, abilities)| {
                let grab = moveset
                    .0
                    .move_for_directional_verb(
                        ambition_platformer2d::entity_catalog::GRAB_VERB,
                        ambition_platformer2d::characters::actor::attack_gesture::AttackDir::Neutral,
                        true,
                    )
                    .map(|spec| spec.id.clone());
                let verbs: Vec<&str> =
                    moveset.0.verbs.keys().map(|verb| verb.as_str()).collect();
                format!(
                    "seat {} grab={:?} can_grab={:?} verbs={:?}",
                    seat.0,
                    grab,
                    abilities.map(|a| a.abilities.grab),
                    verbs
                )
            })
            .collect();
        rows.sort();
        for row in rows {
            println!("[capture_probe] {row}");
        }
    }

    let mut tally = Tally::default();
    // **WHERE A ZERO COMES FROM.** "No grabs happened" has five possible
    // causes and they are indistinguishable from the relationship table alone:
    // the kit offers none, the brain never chooses one, the press never
    // reaches the body, the move never plays, or acquisition declines. Counting
    // the presses and the moves that actually played localizes it in one run.
    let mut grab_presses = 0u32;
    // Of those, the ones made while the presser was already committed to a move,
    // so the press could not start anything. See the block that fills it.
    let mut grab_presses_while_committed = 0u32;
    let mut attempts_reported = 0u32;
    // **How close these two ever actually get.** A grab that reaches 42px cannot
    // land in a fight held at 100, and that is a fact about SPACING rather than
    // about capture.
    let mut closest = f32::MAX;
    let mut ticks_in_grab_range = 0u32;
    let mut moves_started: HashMap<String, u32> = HashMap::new();
    let mut last_move: HashMap<bevy::prelude::Entity, String> = HashMap::new();
    // `CapturedBy` says only who holds whom, so the probe tracks `SmashHoldState` across ticks.
    let mut live: HashMap<bevy::prelude::Entity, SmashHoldState> = HashMap::new();

    for _ in 0..ticks {
        app.update();
        let world = app.world_mut();
        let mut query = world.query::<(
            bevy::prelude::Entity,
            &CapturedBy,
            &ambition_platformer2d::characters::smash_capture::SmashHoldState,
        )>();
        let now: HashMap<bevy::prelude::Entity, SmashHoldState> =
            query.iter(world).map(|(e, _, state)| (e, *state)).collect();
        for (victim, held) in &now {
            if !live.contains_key(victim) {
                tally.holds += 1;
            }
            tally.held_ticks += 1;
            tally.deepest = tally.deepest.max(held.pummels_landed);
            if let Some(before) = live.get(victim) {
                tally.pummels += u32::from(held.pummels_landed - before.pummels_landed);
            }
        }
        for (victim, last) in &live {
            if now.contains_key(victim) {
                continue;
            }
            if last.escaped() {
                tally.escaped += 1;
            } else if last.held_for >= 3.9 {
                tally.timed_out += 1;
            } else {
                tally.thrown_or_broken += 1;
            }
        }
        live = now;

        // **WHY AN ATTEMPT WAS DECLINED, from outside the engine.** An
        // attempt reaches acquisition and a hold does not appear: the reasons
        // are the eligibility predicate's own terms, so print those terms on the
        // ticks it actually ran.
        {
            let world = app.world_mut();
            let mut seats = world.query::<(
                &ambition_platformer2d::actor::MatchSeat,
                &ambition_platformer2d::engine_core::BodyKinematics,
            )>();
            let mut xs: Vec<f32> = seats.iter(world).map(|(_, kin)| kin.pos.x).collect();
            if xs.len() == 2 {
                xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let gap = xs[1] - xs[0];
                closest = closest.min(gap);
                if gap <= 42.0 {
                    ticks_in_grab_range += 1;
                }
            }
        }

        let asked = app.world().resource::<AttemptsSeen>().0;
        if asked > attempts_reported {
            attempts_reported = asked;
            let world = app.world_mut();
            let mut query = world.query::<(
                &ambition_platformer2d::actor::MatchSeat,
                &ambition_platformer2d::engine_core::BodyKinematics,
                &ambition_platformer2d::engine_core::BodyGroundState,
                Option<&ambition_platformer2d::characters::actor::BodyHealth>,
                Option<&ambition_platformer2d::characters::actor::BodyCombat>,
                Option<&ambition_platformer2d::engine_core::BodyFlightState>,
                Option<&ambition_platformer2d::actors::features::ActorSurfaceState>,
                Option<&ambition_platformer2d::combat::targeting::MatchTeam>,
            )>();
            let mut rows: Vec<String> = query
                .iter(world)
                .map(|(seat, kin, ground, health, combat, flight, surface, team)| {
                    format!(
                        "seat {} at ({:.0},{:.0}) facing {:+.0} ground={}                          health={} combat={} flight={} surface={} team={:?}",
                        seat.0,
                        kin.pos.x,
                        kin.pos.y,
                        kin.facing,
                        ground.on_ground,
                        health.is_some(),
                        combat.is_some(),
                        flight.is_some(),
                        surface.is_some(),
                        team.map(|t| t.as_str().to_string()),
                    )
                })
                .collect();
            rows.sort();
            println!("[capture_probe] attempt #{asked}:");
            for row in rows {
                println!("[capture_probe]   {row}");
            }
        }

        let world = app.world_mut();
        let mut controls = world.query::<(
            bevy::prelude::Entity,
            &ambition_platformer2d::characters::control::ActorControl,
        )>();
        let pressing: Vec<bevy::prelude::Entity> = controls
            .iter(world)
            .filter(|(_, control)| control.0.grab_pressed)
            .map(|(entity, _)| entity)
            .collect();
        grab_presses += pressing.len() as u32;
        if !pressing.is_empty() {
            // **WAS THE PRESSER FREE TO ACT?** `trigger_moveset_moves` drops a
            // requested move outright when a `MovePlayback` is running and its
            // cancel window does not permit the new one — which for a smash into
            // a grab it never does. Counting presses without this cannot tell a
            // brain that grabs at the wrong DISTANCE from one that grabs at the
            // wrong TIME, and the first natural-behaviour run showed seven
            // presses producing exactly one grab.
            let mut committed = world.query::<(
                bevy::prelude::Entity,
                &ambition_platformer2d::combat::moveset::MovePlayback,
            )>();
            let busy: std::collections::HashMap<bevy::prelude::Entity, String> = committed
                .iter(world)
                .map(|(entity, pb)| (entity, pb.spec.id.clone()))
                .collect();
            let blocked: Vec<&String> = pressing.iter().filter_map(|e| busy.get(e)).collect();
            grab_presses_while_committed += blocked.len() as u32;
            let mut seats = world.query::<(
                &ambition_platformer2d::actor::MatchSeat,
                &ambition_platformer2d::engine_core::BodyKinematics,
            )>();
            let mut xs: Vec<f32> = seats.iter(world).map(|(_, kin)| kin.pos.x).collect();
            xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            if xs.len() == 2 {
                let gap = xs[1] - xs[0];
                match blocked.as_slice() {
                    [] => println!(
                        "[capture_probe] Grab pressed with the two {gap:.0}px apart — body FREE"
                    ),
                    busy_ids => println!(
                        "[capture_probe] Grab pressed with the two {gap:.0}px apart — \
                         SPENT mid-`{}`",
                        busy_ids
                            .iter()
                            .map(|id| id.as_str())
                            .collect::<Vec<_>>()
                            .join("`, mid-`")
                    ),
                }
            }
        }

        let mut playbacks = world.query::<(
            bevy::prelude::Entity,
            &ambition_platformer2d::combat::moveset::MovePlayback,
        )>();
        let playing: Vec<(bevy::prelude::Entity, String)> = playbacks
            .iter(world)
            .map(|(entity, playback)| (entity, playback.spec.id.clone()))
            .collect();
        for (body, id) in &playing {
            if last_move.get(body) != Some(id) {
                *moves_started.entry(id.clone()).or_default() += 1;
            }
        }
        last_move = playing.into_iter().collect();
    }

    println!("[capture_probe] {seconds:.0}s of CPU-versus-CPU");
    println!("[capture_probe]   holds established     {}", tally.holds);
    println!(
        "[capture_probe]   time spent held       {:.1}s ({:.1}% of the match)",
        tally.held_ticks as f32 / 60.0,
        100.0 * tally.held_ticks as f32 / ticks as f32
    );
    println!("[capture_probe]   pummels landed        {}", tally.pummels);
    println!(
        "[capture_probe]   deepest hold          {} pummel(s)",
        tally.deepest
    );
    println!(
        "[capture_probe]   ended by throw/hit    {}",
        tally.thrown_or_broken
    );
    println!("[capture_probe]   ended by escape       {}", tally.escaped);
    println!(
        "[capture_probe]   ended by the clock    {}",
        tally.timed_out
    );
    println!("[capture_probe]   Grab pressed          {grab_presses} tick(s)");
    println!(
        "[capture_probe]     …while COMMITTED    {grab_presses_while_committed} tick(s) (the press \
         cannot start a move and is spent)"
    );
    println!(
        "[capture_probe]   Grab FORCED           {} tick(s)",
        app.world().resource::<Forced>().0
    );
    println!(
        "[capture_probe]   closest approach      {closest:.0}px; inside a grab's \
         42px on {ticks_in_grab_range} tick(s) ({:.1}% of the match)",
        100.0 * ticks_in_grab_range as f32 / ticks as f32
    );
    println!(
        "[capture_probe]   attempts requested    {}",
        app.world().resource::<AttemptsSeen>().0
    );
    let mut started: Vec<(String, u32)> = moves_started.into_iter().collect();
    started.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    println!("[capture_probe]   moves started:");
    for (id, count) in started.iter().take(12) {
        println!("[capture_probe]     {count:>4}  {id}");
    }
    if started.is_empty() {
        println!("[capture_probe]     (none — nobody swung at all, so this match was not a fight)");
    }
}
