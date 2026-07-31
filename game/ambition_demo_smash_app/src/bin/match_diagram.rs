//! **Draw a RUNNING match.**
//!
//! `cargo run -p ambition_demo_smash_app --bin match_diagram -- [OUT.png]`
//!
//! `stage_diagram` draws the room. This drives the actual demo — select, lock
//! in, route to the stage, step the sim — and then draws whatever is standing
//! there, with each fighter's damage percent and remaining stocks.
//!
//! It exists because every claim about seating so far is a count. Tests assert
//! that two bodies exist and that they wear seats; none of them has asked WHERE
//! they are, and "two fighters exist" is true of a match with both of them
//! stacked at the origin, standing off the platform, or inside each other.

use ambition::engine_core::AabbExt;
use ambition_demo_smash_app::build_demo_app;

fn main() {
    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/smash_match.png".to_string());

    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    {
        use ambition_demo_smash::select::SmashSelect;
        let mut select = app.world_mut().resource_mut::<SmashSelect>();
        select.join(0);
        select.lock_in(0);
        select.join(1);
        select.browse(1, 1);
        select.lock_in(1);
    }
    // Long enough for the route to resolve, the session to prepare and seating
    // to run. If the fighters are missing below, this is the first suspect.
    for _ in 0..240 {
        app.update();
    }

    // **Hurt somebody**, so the picture shows the thing the stocks loop is
    // about. A match at 0% looks identical whether the meter works or not.
    let damage: i32 = std::env::args()
        .nth(2)
        .and_then(|arg| arg.parse().ok())
        .unwrap_or(140);
    if damage > 0 {
        use ambition::actor::MatchSeat;
        use ambition::characters::actor::BodyHealth;
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

    let png = stage_diagram::render_match_diagram(&fighters);
    std::fs::write(&out, png).expect("write the match diagram");
    println!("[match_diagram] wrote {out}");
}

#[path = "stage_diagram.rs"]
mod stage_diagram;

fn collect_fighters(app: &mut bevy::prelude::App) -> Vec<stage_diagram::DrawnFighter> {
    use ambition::actor::{FighterStocks, MatchSeat};
    use ambition::characters::actor::BodyHealth;
    use ambition::engine_core::CenteredAabb;

    let world = app.world_mut();
    let mut query =
        world.query::<(&MatchSeat, &CenteredAabb, &BodyHealth, Option<&FighterStocks>)>();
    let mut rows: Vec<(usize, stage_diagram::DrawnFighter)> = query
        .iter(world)
        .map(|(seat, aabb, health, stocks)| {
            (
                seat.0,
                stage_diagram::DrawnFighter {
                    aabb: ambition::engine_core::Aabb::new(aabb.center, aabb.half_size),
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
