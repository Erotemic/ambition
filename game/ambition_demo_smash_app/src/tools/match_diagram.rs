//! Draw a RUNNING match.
//!
//! `cargo run -p ambition_demo_smash_app --bin smash_tool -- match-diagram -- [OUT.png]`
//!
//! `stage_diagram` draws the room. This drives the actual demo — select, lock
//! in, route to the stage, step the sim — and then draws whatever is standing
//! there, with each fighter's damage percent and remaining stocks.
//!
//! It exists because every claim about seating so far is a count. Tests assert
//! that two bodies exist and that they wear seats; none of them has asked WHERE
//! they are, and "two fighters exist" is true of a match with both of them
//! stacked at the origin, standing off the platform, or inside each other.

use ambition_platformer2d::engine_core::AabbExt;
use clap::Args;

use crate::build_demo_app;

#[derive(Args, Debug)]
pub struct MatchDiagramArgs {
    /// Where to write the PNG.
    #[arg(default_value = "/tmp/smash_match.png")]
    pub out: String,
    /// Damage to deal before drawing, so the meter shows something. A match at
    /// 0% looks identical whether the meter works or not.
    #[arg(default_value_t = 140)]
    pub damage: i32,
}

pub fn run(args: MatchDiagramArgs) {
    let out = args.out;

    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    // CPU seats, not the select screen's. `SmashSelect::roster` makes every
    // locked seat a HUMAN — which is right for a couch game and is why a diagram
    // driven through it shows two fighters that never move: nobody is pressing
    // anything. To watch the fighter BRAIN, the roster has to ask for it.
    app.world_mut()
        .insert_resource(ambition_demo_smash::smash_roster([
            ambition_demo_smash::SMASH_CHARACTER_ID,
            ambition_demo_smash::SMASH_OPPONENT_ID,
        ]));
    app.world_mut()
        .write_message(ambition_platformer2d::game_shell::ShellCommand::GoTo(
            ambition_platformer2d::game_shell::ShellRouteId::new(ambition_demo_smash::SMASH_GAMEPLAY_ROUTE),
        ));
    // Long enough for the route to resolve, the session to prepare and seating
    // to run. If the fighters are missing below, this is the first suspect.
    for _ in 0..240 {
        app.update();
    }

    // Hurt somebody, so the picture shows the thing the stocks loop is
    // about. A match at 0% looks identical whether the meter works or not.
    let damage: i32 = args.damage;
    if damage > 0 {
        use ambition_platformer2d::actor::MatchSeat;
        use ambition_platformer2d::characters::actor::BodyHealth;
        let world = app.world_mut();
        let mut query = world.query::<(&MatchSeat, &mut BodyHealth)>();
        for (seat, mut health) in query.iter_mut(world) {
            // Seat 1 takes it, so the two sides read differently.
            if seat.0 == 1 {
                health.damage(damage);
            }
        }
        app.update();
    }

    // Did anybody MOVE? A fighter brain that thinks and emits nothing looks
    // exactly like a fighter brain that was never installed.
    let opening = collect_fighters(&mut app)
        .iter()
        .map(|f| f.aabb.center().x)
        .collect::<Vec<_>>();
    for _ in 0..180 {
        app.update();
    }
    let moved = collect_fighters(&mut app)
        .iter()
        .map(|f| f.aabb.center().x)
        .zip(opening.iter())
        .map(|(now, then)| (now - then).abs())
        .collect::<Vec<_>>();
    println!("[match_diagram] travel over 180 ticks: {moved:?}");
    // Does a real fight ever produce a KO? Run it long and report the peak
    // percent and any stock spent. If damage climbs and nothing is ever launched
    // off, the knockback curve does not reach this stage's blast line — which is
    // a tuning fact no unit test can hold an opinion about.
    {
        use ambition_platformer2d::actor::{FighterStocks, MatchSeat};
        use ambition_platformer2d::characters::actor::BodyHealth;
        let mut peak = 0.0f32;
        let mut spent = 0u32;
        for _ in 0..3_600 {
            app.update();
            let world = app.world_mut();
            let mut q = world.query::<(&MatchSeat, &BodyHealth, Option<&FighterStocks>)>();
            for (_, health, stocks) in q.iter(world) {
                peak = peak.max(health.damage_percent());
                if let Some(stocks) = stocks {
                    spent = spent.max(3u32.saturating_sub(stocks.remaining));
                }
            }
        }
        println!(
            "[match_diagram] 60s of fighting: peak {:.0}%, {spent} stock(s) spent",
            peak * 100.0
        );
    }
    {
        use ambition_platformer2d::actor::MatchSeat;
        use ambition_platformer2d::characters::brain::{Brain};
use ambition_platformer2d::characters::control::{ScriptedControl};
        let world = app.world_mut();
        let mut q = world.query::<(
            &MatchSeat,
            Option<&Brain>,
            bevy::prelude::Has<ScriptedControl>,
        )>();
        let mut rows: Vec<String> = q
            .iter(world)
            .map(|(seat, brain, held)| {
                format!(
                    "seat {} brain={} held={held}",
                    seat.0,
                    brain.map(|b| b.label()).unwrap_or("NONE")
                )
            })
            .collect();
        rows.sort();
        for row in rows {
            println!("[match_diagram] {row}");
        }
    }

    let fighters = collect_fighters(&mut app);
    println!("[match_diagram] {} fighter(s) on the stage", fighters.len());
    for (index, fighter) in fighters.iter().enumerate() {
        println!(
            "[match_diagram]   seat {index}: centre ({:.0}, {:.0})  {:.0}%  {} stock(s)",
            fighter.aabb.center().x,
            fighter.aabb.center().y,
            fighter.percent * 100.0,
            fighter.stocks
        );
    }

    let png = crate::stage_diagram::render_match_diagram(&fighters);
    std::fs::write(&out, png).expect("write the match diagram");
    println!("[match_diagram] wrote {out}");
}

fn collect_fighters(
    app: &mut bevy::prelude::App,
) -> Vec<crate::stage_diagram::DrawnFighter> {
    use ambition_platformer2d::actor::{FighterStocks, MatchSeat};
    use ambition_platformer2d::characters::actor::BodyHealth;
    use ambition_platformer2d::engine_core::CenteredAabb;

    let world = app.world_mut();
    let mut query = world.query::<(
        &MatchSeat,
        &CenteredAabb,
        &BodyHealth,
        Option<&FighterStocks>,
    )>();
    let mut rows: Vec<(usize, crate::stage_diagram::DrawnFighter)> = query
        .iter(world)
        .map(|(seat, aabb, health, stocks)| {
            (
                seat.0,
                crate::stage_diagram::DrawnFighter {
                    aabb: ambition_platformer2d::engine_core::Aabb::new(aabb.center, aabb.half_size),
                    percent: health.damage_percent(),
                    stocks: stocks.map(|s| s.remaining).unwrap_or(0),
                },
            )
        })
        .collect();
    // By SEAT, so the left fighter is always drawn in the left colour. Query
    // order is not an order.
    rows.sort_by_key(|(seat, _)| *seat);
    rows.into_iter().map(|(_, fighter)| fighter).collect()
}
