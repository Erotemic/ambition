//! The Smash fixture: the shipped composition with a match staged and live.
//!
//! In the SHIPPED composition, not the demo shell: the demo shell's catalog
//! carries George and two stand-ins, so most fighters cannot be seated there
//! and a rig that tried would measure an empty stage.

use ambition_platformer2d::actor::MatchSeat;
use ambition_platformer2d::characters::control::ControlHolds;
use ambition_platformer2d::game_shell::{ShellCommand, ShellRouteId};
use bevy::prelude::{App, Entity, With};

/// The shipped composition, windowless, 30 frames in: the frames are
/// load-bearing, since `PreparedCharacterRegistry` is filled by a `Startup`
/// system and a grid assembled before it would be empty.
pub fn shipped_host() -> App {
    let mut app = ambition_app::app::build_visible_app(ambition_app::app::VisibleRenderMode::NoWindow, true);
    for _ in 0..30 {
        app.update();
    }
    app
}

/// Step until the round is live: a cast exists and nothing in it is still
/// held by `ControlHolds`. Both halves, because a cast that does not exist
/// yet is not a cast whose hold has come off.
///
/// Waits for the condition, not a frame count: a fixed 240 frames encoded the
/// opening ceremony's length, and D248's dev mode, which runs the ceremony 10x
/// fast, turned every such settle into a different starting world.
pub fn wait_for_live_round(app: &mut App) {
    for _ in 0..900 {
        app.update();
        let (seated, held) = {
            let world = app.world_mut();
            let seated = world.query::<&MatchSeat>().iter(world).count();
            let held = world.query_filtered::<&MatchSeat, With<ControlHolds>>().iter(world).count();
            (seated, held)
        };
        if seated > 0 && held == 0 {
            return;
        }
    }
    panic!("the opening ceremony never released the cast, so nothing after this is about a live round");
}

/// A live round of `roster` in the shipped composition.
///
/// Stage it with `ambition_demo_smash::smash_roster`, which seats slot 0 as a
/// human, when the test drives seat 0: `smash_roster_at_levels` makes every
/// participant a CPU, and a control frame driven at slot 0 then reaches
/// nobody, which looks exactly like a broken move.
pub fn live_round(roster: ambition_platformer2d::actor::MatchParticipantRoster) -> App {
    let mut app = shipped_host();
    app.world_mut().insert_resource(roster);
    app.world_mut()
        .write_message(ShellCommand::GoTo(ShellRouteId::new(ambition_demo_smash::SMASH_GAMEPLAY_ROUTE)));
    wait_for_live_round(&mut app);
    app
}

/// The body in match seat `n`.
pub fn seat(app: &mut App, n: usize) -> Entity {
    let world = app.world_mut();
    world
        .query::<(Entity, &MatchSeat)>()
        .iter(world)
        .find(|(_, seat)| seat.0 == n)
        .map(|(entity, _)| entity)
        .unwrap_or_else(|| panic!("the match seats fighter {n}"))
}
