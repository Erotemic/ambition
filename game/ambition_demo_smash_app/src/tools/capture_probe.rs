//! Probe capture behavior in an unarranged CPU match.
//!
//! `cargo run -p ambition_demo_smash_app --bin smash_tool -- capture-probe [-- SECONDS]`
//!
//! Reports capture attempts, established holds, pummels, and coarse ending
//! classifications. It is observational and intentionally has no pass/fail threshold.

use std::collections::HashMap;

use ambition_platformer2d::characters::smash_capture::SmashHoldState;


use ambition_platformer2d::combat::capture::{CaptureAttemptRequested, CapturedBy};

/// Capture attempts requested by the adapter, distinguished from accepted holds.
#[derive(bevy::prelude::Resource, Default)]
struct AttemptsSeen(u32);

/// Inject grab input inside the simulation schedule, before fighter control is consumed.
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

#[derive(clap::Args, Debug)]
pub struct CaptureProbeArgs {
    /// How many seconds of match to simulate (60 ticks each).
    ///
    /// ⛔ **THERE IS A CEILING AND IT IS THE MATCH, NOT THIS NUMBER.** Once both
    /// fighters are eliminated the stage is empty and every further tick
    /// contributes nothing — so asking for more seconds than a match lasts buys
    /// no data. ⚠ Measured 2026-09-04: a stand-in match on the shipped ladder
    /// resolves at about **134s**, and a 300-second probe returns a census
    /// byte-identical to a 120-second one.
    ///
    /// ⇒ Which cuts both ways, and the useful half is easy to miss: a census
    /// taken over a full match is a statement about a WHOLE match, not a
    /// truncated window — so a move absent from it is absent from the entire
    /// fight rather than from an arbitrary slice.
    #[arg(default_value_t = 60.0)]
    pub seconds: f32,
    /// Press Grab FOR them, when a person would. The CPU's own timing is a
    /// policy question; whether the live game can produce a hold at all is not,
    /// and the two are only separable by taking the timing out of the AI's
    /// hands.
    #[arg(long)]
    pub force: bool,
    /// Which fighter takes the first seat (default: the demo's stand-in).
    ///
    /// ⛔ **A MOVE CENSUS IS ONLY ABOUT THE FIGHTERS IT RAN.** The defaults are
    /// the two STAND-INS — both carry `fighter_moveset()`, 18 verbs against
    /// George's 26 — so probing them and concluding anything about the demo's
    /// authored fighter repeats the error `ladder_rig` made five separate ways
    /// on 2026-09-04: the instrument's subject differed from the shipped game's,
    /// and only the instrument was ever read.
    #[arg(long, value_name = "ID")]
    pub character: Option<String>,
    /// Which fighter takes the second seat (default: the demo's other stand-in).
    #[arg(long, value_name = "ID")]
    pub opponent: Option<String>,
    /// Load an authored difficulty ladder from a `.ron` and install it.
    ///
    /// ⛔ **WITHOUT THIS THE PROBE MEASURES THE ENGINE FLOOR**, not the shipped
    /// game: `build_demo_app` installs no `AuthoredFighterLadder`, so every rung
    /// carries `UtilityWeights::default()` — which IS the level-9 row. ⇒ A move
    /// census taken there describes a fighter no player meets, which is the
    /// error `ladder_rig` made five separate ways on 2026-09-04.
    #[arg(long, value_name = "PATH")]
    pub ladder: Option<String>,
}

pub fn run(args: CaptureProbeArgs) {
    use bevy::prelude::IntoScheduleConfigs as _;
    let seconds: f32 = args.seconds;
    // ⭐ Resolved once and REPORTED below, so a census names its own subject.
    let character = args
        .character
        .clone()
        .unwrap_or_else(|| ambition_demo_smash::SMASH_CHARACTER_ID.to_string());
    let opponent = args
        .opponent
        .clone()
        .unwrap_or_else(|| ambition_demo_smash::SMASH_OPPONENT_ID.to_string());
    println!("[capture_probe] fighters: `{character}` vs `{opponent}`");
    // ⛔ A parse failure EXITS rather than falling back to the floor: a run whose
    // header claims the authored rows while its fighters carry the floor's is the
    // exact failure this flag exists to remove.
    let authored_ladder = args.ladder.as_deref().map(|path| {
        let text = std::fs::read_to_string(path).unwrap_or_else(|err| {
            eprintln!("[capture_probe] --ladder {path}: {err}");
            std::process::exit(2);
        });
        let ladder =
            ambition_platformer2d::characters::brain::fighter::FighterBrainLadder::from_ron(&text)
                .unwrap_or_else(|err| {
                    eprintln!("[capture_probe] --ladder {path} did not parse: {err}");
                    std::process::exit(2);
                });
        ambition_platformer2d::characters::brain::fighter::AuthoredFighterLadder(ladder)
    });
    println!(
        "[capture_probe] ladder: {}",
        if authored_ladder.is_some() {
            "the AUTHORED rows"
        } else {
            "⛔ the ENGINE FLOOR — every rung carries UtilityWeights::default(), \
             which IS the level-9 row. NOT the shipped fighter."
        }
    );
    let ticks = (seconds * 60.0) as u32;
    // `--force`: press Grab FOR them, when a person would. The CPU's own
    // timing is a policy question; whether the live game can produce a hold at
    // all is not, and the two are only separable by taking the timing out of the
    // AI's hands. Presses on the tick the two are inside grab range and the
    // presser is not already committed to a move — which is exactly the moment a
    // player picks.
    let force = args.force;

    let mut app = crate::build_demo_app();
    // ⛔ BEFORE the warm-up, because `project_authored_fighter_ladder` applies the
    // rows on `Added<Brain>`: installed after the fighters exist it reaches
    // nobody, and the run would measure the floor under a header claiming the
    // authored rows.
    if let Some(ladder) = authored_ladder {
        app.world_mut().insert_resource(ladder);
    }
    app.init_resource::<AttemptsSeen>();
    app.add_systems(bevy::prelude::Update, count_attempts);
    app.init_resource::<Forced>();
    if force {
        let sim =
            ambition_platformer2d::platformer::schedule::SimScheduleExt::sim_schedule(&mut app);
        app.add_systems(
            sim,
            // AFTER the brain, not merely before combat. `.before(C)`
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
    // CPU seats, not the select screen's — `SmashSelect::roster` makes
    // every locked seat a HUMAN, and two humans with no controllers stand still
    // forever. The same note `match_diagram` carries, for the same reason.
    app.world_mut()
        // ⭐ WHICH FIGHTERS, because a move census is only about the fighters it
        // ran. The default pair are the STAND-INS — both carry
        // `fighter_moveset()` — so a probe of them says nothing about George's
        // 26 authored verbs. ⇒ `--character` / `--opponent` name them, the same
        // flags `ladder_rig` takes, and the header below says which ran.
        .insert_resource(ambition_demo_smash::smash_roster([
            character.clone(),
            opponent.clone(),
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
    // Does the body even OWN a grab? The first suspect behind a zero, and
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
    // WHERE A ZERO COMES FROM. "No grabs happened" has five possible
    // causes and they are indistinguishable from the relationship table alone:
    // the kit offers none, the brain never chooses one, the press never
    // reaches the body, the move never plays, or acquisition declines. Counting
    // the presses and the moves that actually played localizes it in one run.
    let mut grab_presses = 0u32;
    // Of those, the ones made while the presser was already committed to a move,
    // so the press could not start anything. See the block that fills it.
    let mut grab_presses_while_committed = 0u32;
    let mut attempts_reported = 0u32;
    // How close these two ever actually get. A grab that reaches 42px cannot
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

        // WHY AN ATTEMPT WAS DECLINED, from outside the engine. An
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
            // WAS THE PRESSER FREE TO ACT? `trigger_moveset_moves` drops a
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
    // ⛔⛔ THE COUNT GOES FIRST, AND THE TRUNCATION SAYS SO. This printed a bare
    // `take(12)` with no total and no notice — and George uses EXACTLY 12 distinct
    // moves, so the list was always full and a THIRTEENTH move starting would have
    // been dropped in silence. ⇒ That is precisely the signal `D-BRAIN-MENU`'s
    // acceptance test looks for ("did any tilt or smash start at all"), so the
    // instrument would have hidden the fix working.
    println!(
        "[capture_probe]   moves started: {} distinct",
        started.len()
    );
    for (id, count) in started.iter().take(12) {
        println!("[capture_probe]     {count:>4}  {id}");
    }
    if started.len() > 12 {
        println!(
            "[capture_probe]     … and {} more not shown — re-read the distinct count above",
            started.len() - 12
        );
    }
    if started.is_empty() {
        println!("[capture_probe]     (none — nobody swung at all, so this match was not a fight)");
    }
}
