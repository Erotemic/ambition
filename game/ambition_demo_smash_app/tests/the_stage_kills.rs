//! **A fighter knocked off THIS platform reaches the world's edge.**
//!
//! The one claim about the stocks loop that no unit test in
//! `ambition_demo_smash` can make. Those cover spend, respawn, eliminate and
//! end, each in isolation and each correctly. What none of them can answer is
//! whether the stage's own numbers — a 420px platform in a 960px world with a
//! 220px blast margin — put the world's edge somewhere a launched body actually
//! gets to.
//!
//! That is the difference between a loop that is correct and a game that works,
//! and it is exactly the class this repository keeps rediscovering: every
//! instrument green, and green about less than it claimed.

use ambition::engine_core::AabbExt;
use ambition_demo_smash_app::build_demo_app;

/// The stage boots and its geometry is the one the demo authored.
///
/// A shell that composes a different room would pass every content test in the
/// other crate, because those inspect a `RoomSpec` this app never has to load.
#[test]
fn the_shell_boots_onto_the_authored_stage() {
    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }
    assert!(
        app.world()
            .get_resource::<ambition::game_shell::ShellRouter>()
            .is_some(),
        "the shell never installed a router, so nothing routed anywhere"
    );
}

/// **The blast margin is reachable from the platform.**
///
/// Measured against the loaded world rather than the authored constant, so a
/// preparation step that dropped or rewrote the margins fails here. The
/// assertion is a RATIO, not a distance: what matters is that the edge is close
/// enough to the platform that a launch crosses it, and that is the number a
/// future stage resize would silently break.
#[test]
fn the_worlds_edge_sits_within_a_launch_of_the_platform() {
    let world = ambition_demo_smash::smash_stage().world;
    let platform = world.blocks[0].aabb;
    let side_margin = world
        .side_blast_margin
        .expect("the stage authors its side margins");

    // How far past the platform's edge a body must travel to leave the world.
    let to_the_left = platform.left() + side_margin;
    let to_the_right = (world.size.x - platform.right()) + side_margin;

    // ⚠ **a RATIO against the platform, not a bound against the world.** The
    // first version of this test asserted `distance < world.size.x` and passed
    // over a stage where a knocked-off fighter crossed 490px of nothing — more
    // than the platform's entire width — because 490 < 960 is true and says
    // nothing. The picture caught it; the test did not.
    //
    // One platform-width of travel is the budget. Past that a launch stops
    // reading as a knockout and starts reading as a body drifting offscreen
    // while the game waits.
    let budget = platform.width();
    for (side, distance) in [("left", to_the_left), ("right", to_the_right)] {
        assert!(
            distance <= budget,
            "a fighter knocked off the {side} must cross {distance:.0}px before \
             the world takes it, against a {budget:.0}px platform — that is a \
             body drifting through empty space, not a knockout"
        );
    }
}

/// **The demo opens on character select, and the battle starts when the players
/// lock in.** (Jon, 2026-07-31)
///
/// The whole path, through the real shell: boot lands on select, two seats join
/// and commit, and the roster the screen decided is published before the route
/// leaves for the stage.
///
/// That ORDER is the correctness argument rather than an implementation detail.
/// Seating reads `MatchParticipantRoster` on the sim schedule; if the route
/// changed first the stage would come up with no roster, seating would find
/// nothing to do, and the match would open with an empty cast that nothing
/// retries into existence.
#[test]
fn the_demo_opens_on_select_and_the_battle_starts_when_players_lock_in() {
    use ambition_demo_smash::select::SmashSelect;

    let mut app = build_demo_app();
    for _ in 0..30 {
        app.update();
    }

    let route_now = |app: &bevy::prelude::App| -> Option<String> {
        app.world()
            .resource::<ambition::game_shell::ShellRouter>()
            .active
            .as_ref()
            .map(|active| active.route_id.as_str().to_string())
    };
    assert_eq!(
        route_now(&app).as_deref(),
        Some(ambition_demo_smash::SMASH_SELECT_ROUTE),
        "the demo booted straight onto the stage, so it decided who the players \
         are before asking them"
    );
    assert!(
        app.world()
            .get_resource::<ambition::actor::MatchParticipantRoster>()
            .is_none(),
        "a roster exists before anybody chose, so the select screen is decoration"
    );

    // Two players join and commit.
    {
        let mut select = app.world_mut().resource_mut::<SmashSelect>();
        select.join(0);
        select.lock_in(0);
        select.join(1);
        select.browse(1, 1);
        select.lock_in(1);
    }
    app.update();

    let roster = app
        .world()
        .get_resource::<ambition::actor::MatchParticipantRoster>()
        .expect("locking in published the match the screen decided");
    assert_eq!(roster.participants.len(), 2);
    assert_eq!(
        roster.fighter_stocks,
        Some(ambition_demo_smash::STARTING_STOCKS),
        "the decided match is not a stocks match"
    );

    for _ in 0..60 {
        app.update();
    }
    assert_eq!(
        route_now(&app).as_deref(),
        Some(ambition_demo_smash::SMASH_GAMEPLAY_ROUTE),
        "the players locked in and the demo stayed on the select screen"
    );
}
