//! **Stocks are a real loop: spend, respawn, eliminate, end the match.** (S4)
//!
//! `FighterStocks` was vocabulary with no consumer. This drives the whole loop
//! through the SHIPPED composition — `AmbitionGameSimulationPlugin`, the real
//! `CombatSet::Settle` ordering, the real messages — rather than a hand-built app
//! with the two systems in `Update`. The distinction matters here more than
//! usual: every step of this loop is a system somebody has to have installed, and
//! this repository's most-repeated defect is a reader wired correctly against a
//! resource no plugin publishes.
//!
//! ## Why the KO is injected rather than earned
//!
//! Knocking a fighter off a stage for real needs a stage, a launch, and a blast
//! margin, and a test that arranges all three is testing the arena. The seam
//! under test starts at `BodyKnockedOut`, which is written from the two
//! `RulesetOwnsDeath` death arms — those arms are covered where they live. What
//! is unproven until here is everything AFTER the announcement, and injecting the
//! message is how you get to it without a stage.

use ambition_platformer2d::combat::components::FighterStocks;
use ambition_platformer2d::combat::stocks::{
    BodyKnockedOut, FighterEliminated, FighterStockSpent, StocksMatchDecided,
};
use ambition_platformer2d::combat::targeting::MatchTeam;
use ambition_app::app::{AmbitionGameSimulationPlugin, StartRoomOverride};
use bevy::app::ScheduleRunnerPlugin;
use bevy::asset::AssetPlugin;
use bevy::image::ImagePlugin;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::time::TimeUpdateStrategy;
use bevy::transform::TransformPlugin;

use ambition_platformer2d::actors::character_runtime::{ActiveMatch, MatchSeat};
use ambition_platformer2d::characters::actor::{BodyHealth, DeathPolicy, Health};

const POOL: i32 = 50;

fn composed_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(std::time::Duration::ZERO)));
    app.add_plugins(AssetPlugin::default());
    app.add_plugins(ImagePlugin::default());
    app.add_plugins(TransformPlugin);
    app.add_plugins(StatesPlugin);
    app.init_state::<ambition_platformer2d::sim::GameMode>();
    app.insert_resource(TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f32(1.0 / 60.0),
    ));
    app.insert_resource(StartRoomOverride("portal_lab".to_string()));
    app.add_plugins(AmbitionGameSimulationPlugin);
    app.finish();
    app.update();
    // A LIVE match. `decide_stocks_match` refuses to end a match that is not
    // running, which is what stops it deciding against a half-seated cast.
    app.world_mut()
        .insert_resource(ActiveMatch::for_test(2, None));
    app
}

/// A seated stocks fighter: the pair the roster hands out together.
fn fighter(app: &mut App, seat: usize, team: &str, stocks: u32) -> Entity {
    app.world_mut()
        .spawn((
            MatchSeat(seat),
            MatchTeam::new(team.to_string()),
            FighterStocks::new(stocks),
            BodyHealth::new(Health::new(POOL)).with_policy(DeathPolicy::Unbounded),
        ))
        .id()
}

fn knock_out(app: &mut App, body: Entity) {
    app.world_mut()
        .resource_mut::<Messages<BodyKnockedOut>>()
        .write(BodyKnockedOut {
            body,
            cause: ambition_platformer2d::combat::HitSource::LeftTheWorld,
        });
    app.update();
}

fn spends(app: &App) -> Vec<FighterStockSpent> {
    let messages = app.world().resource::<Messages<FighterStockSpent>>();
    let mut cursor = messages.get_cursor();
    cursor.read(messages).copied().collect()
}

fn decided(app: &App) -> Vec<StocksMatchDecided> {
    let messages = app.world().resource::<Messages<StocksMatchDecided>>();
    let mut cursor = messages.get_cursor();
    cursor.read(messages).cloned().collect()
}

/// **The whole loop, in the order a match runs it.**
#[test]
fn a_stocks_match_spends_respawns_eliminates_and_ends() {
    let mut app = composed_app();
    let blue = fighter(&mut app, 0, "blue", 2);
    let red = fighter(&mut app, 1, "red", 2);

    // SPEND. One KO, one stock.
    app.world_mut()
        .get_mut::<BodyHealth>(blue)
        .unwrap()
        .damage(137);
    knock_out(&mut app, blue);
    let spent = spends(&app);
    assert_eq!(
        spent.len(),
        1,
        "one knockout did not spend exactly one stock: {spent:?}"
    );
    assert_eq!(spent[0].remaining, 1);
    assert!(!spent[0].eliminated);

    // RESPAWN. The meter that caused the KO is cleared, or every stock after the
    // first is shorter than the one before it.
    assert_eq!(
        app.world().get::<BodyHealth>(blue).unwrap().damage_taken(),
        0,
        "the fighter came back still carrying the damage that killed it"
    );
    assert!(
        app.world().get::<FighterEliminated>(blue).is_none(),
        "a fighter with a stock left was marked out of the match"
    );
    assert!(
        decided(&app).is_empty(),
        "the match ended while both sides still had fighters in play"
    );

    // ELIMINATE, and END. Blue's last stock goes, and red is the only side left.
    knock_out(&mut app, blue);
    assert!(
        app.world().get::<FighterEliminated>(blue).is_some(),
        "spending the last stock did not eliminate the fighter"
    );
    let outcome = decided(&app);
    assert_eq!(
        outcome.len(),
        1,
        "the match did not end exactly once when the last side was left \
         standing: {outcome:?}"
    );
    assert_eq!(outcome[0].winner.as_deref(), Some("red"));

    // ONCE. The decision is announced on the frame it becomes true, not on every
    // frame after it — a ruleset acting on this would run its match-over
    // sequence every tick.
    app.world_mut()
        .resource_mut::<Messages<StocksMatchDecided>>()
        .clear();
    for _ in 0..5 {
        app.update();
    }
    assert!(
        decided(&app).is_empty(),
        "the match kept re-announcing its own ending"
    );
    // …and red never lost a stock for winning.
    assert_eq!(app.world().get::<FighterStocks>(red).unwrap().remaining, 2);
}

/// **Both sides going out together is a DRAW, not a win for whoever the query
/// happened to reach last.**
#[test]
fn a_stocks_match_that_empties_both_sides_is_a_draw() {
    let mut app = composed_app();
    let blue = fighter(&mut app, 0, "blue", 1);
    let red = fighter(&mut app, 1, "red", 1);

    app.world_mut()
        .resource_mut::<Messages<BodyKnockedOut>>()
        .write(BodyKnockedOut {
            body: blue,
            cause: ambition_platformer2d::combat::HitSource::LeftTheWorld,
        });
    app.world_mut()
        .resource_mut::<Messages<BodyKnockedOut>>()
        .write(BodyKnockedOut {
            body: red,
            cause: ambition_platformer2d::combat::HitSource::LeftTheWorld,
        });
    app.update();

    let outcome = decided(&app);
    assert_eq!(outcome.len(), 1, "a simultaneous KO did not end the match");
    assert_eq!(
        outcome[0].winner, None,
        "a draw was awarded to a side, which is the shape a `winner: String` \
         would have had to invent a sentinel for"
    );
}

/// **A match that is not running cannot end**, which is what stops the sweep
/// deciding against a cast that is still being seated.
#[test]
fn a_stage_with_no_live_match_is_never_decided() {
    let mut app = composed_app();
    app.world_mut().remove_resource::<ActiveMatch>();
    let blue = fighter(&mut app, 0, "blue", 1);
    let _red = fighter(&mut app, 1, "red", 1);

    knock_out(&mut app, blue);
    assert!(
        decided(&app).is_empty(),
        "a stage with no ActiveMatch announced a winner, so a half-seated cast \
         can be judged before it exists"
    );
}
