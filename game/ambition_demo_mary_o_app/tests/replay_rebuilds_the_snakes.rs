//! Does a room replay put a shelled snake back on its feet, and WHO does it?
//!
//! ⭐ THE QUESTION IS ATTRIBUTION, not behaviour. `reset_snakes_on_room_reset`
//! writes `SnakeShell`, the recoil lock and the contact-damage toggle on every
//! snake when a replay is admitted. Since a replay became a canonical room
//! rebuild, snakes are room-scoped authored actors that the rebuild retires and
//! re-spawns — so the listener may be doing work the reconstruction already
//! does.
//!
//! ⛔⛔ EVERY MARY-O TEST STAYED GREEN WITH THAT LISTENER GUTTED, and that is
//! not evidence of redundancy — it is evidence that nothing covered it. A
//! deletion argued from silence is a deletion argued from nothing. This file is
//! the cover: poison the listener with this in place and the answer is a
//! measurement.

use ambition_demo_mary_o::snake::SnakeShell;
use ambition_demo_mary_o_app::build_demo_app;
use ambition_platformer2d::actors::session::reset::RoomReplayRequested;
use bevy::prelude::*;

fn boot() -> App {
    let mut app = build_demo_app();
    for _ in 0..120 {
        app.update();
    }
    app
}

fn shell_phases(app: &mut App) -> Vec<SnakeShell> {
    let world = app.world_mut();
    let mut q = world.query::<&SnakeShell>();
    q.iter(world).copied().collect()
}

#[test]
fn a_replay_puts_a_shelled_snake_back_on_its_feet() {
    let mut app = boot();

    // ⛔ THE PREMISE. A room with no snakes answers this question by having no
    // question in it.
    let before = shell_phases(&mut app);
    assert!(
        !before.is_empty(),
        "the demo's start room stages no snake, so this file measures nothing. \
         Point it at a room that does."
    );

    // Shell every snake. The PROVENANCE does not matter to the question — what
    // is being asked is whether a rebuild restores the state, not how it got
    // shelled — so this states it directly rather than driving a stomp.
    {
        let world = app.world_mut();
        let mut q = world.query::<&mut SnakeShell>();
        for mut shell in q.iter_mut(world) {
            *shell = SnakeShell::Boxed(0.0);
        }
    }
    app.update();
    assert!(
        shell_phases(&mut app)
            .iter()
            .all(|phase| !matches!(phase, SnakeShell::Walking)),
        "the fixture failed to shell the snakes it is about to ask a replay to \
         restore"
    );

    app.world_mut().write_message(RoomReplayRequested::manual());
    // A replay is a room rebuild now, so it lands at a confirmed lifecycle
    // boundary a couple of frames later — not on the frame it is asked for.
    for _ in 0..90 {
        app.update();
    }

    let after = shell_phases(&mut app);
    assert!(
        !after.is_empty(),
        "the replay left the room with no snakes at all"
    );
    assert!(
        after.iter().all(|phase| *phase == SnakeShell::Walking),
        "a replayed room still has a shelled snake in it: {after:?}"
    );
}
