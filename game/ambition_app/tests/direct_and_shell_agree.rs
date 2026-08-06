//! **K2b's evidence, and then its epitaph.**
//!
//! This file was written while direct entry still spawned a `SessionRoot` at
//! PLUGIN-BUILD time and a shell activation built one asynchronously behind a
//! load barrier and eight preparation work items. `tracks.md` said the deletion
//! was staged rather than done in one commit because nothing proved the two
//! agreed: *"No test asserts on `publish_direct_prepared_session_root` or
//! `SessionScopeId(0)` directly — the coverage is all implicit, which is exactly
//! what makes risk 1 dangerous."* This was that test, it agreed, and the
//! build-time publisher was deleted on 2026-08-06.
//!
//! ⚠ **so its original assertion is now unwritable, and that is the point.**
//! There is no second path to compare against. What survives is the property the
//! comparison was standing in for: `compose_ambition_gameplay_host` produces a
//! live gameplay world, and it is the only thing that does.
//!
//! ⛔ **not deleted with the publisher, deliberately.** The composition it pins
//! has three ordered steps whose order is not obvious — the hosted marker before
//! the plugins, the simulation plugin before the shell — and getting either
//! backwards fails inside Bevy parameter validation, naming a system the caller
//! never heard of. A test that composes it the documented way and settles is the
//! cheapest possible statement that the recipe still works.

use ambition_app::app::shell_host::compose_ambition_gameplay_host;
use ambition_platformer2d::platformer::lifecycle::{
    session_world_component, settle_until_session_world, SESSION_SETTLE_FRAMES,
};
use ambition_platformer2d::runtime::demo_fixture::RoomSet;
use bevy::prelude::*;

#[test]
fn the_one_gameplay_composition_reaches_a_live_world() {
    let mut app = App::new();
    ambition_platformer2d::runtime::add_headless_foundation(&mut app);
    compose_ambition_gameplay_host(&mut app);

    let frames = match settle_until_session_world(&mut app, SESSION_SETTLE_FRAMES) {
        Ok(frames) => frames,
        Err(budget) => panic!(
            "the gameplay host produced no session world in {budget} frames. Since \
             K2b edit 2 this is the ONLY way to start a game, so this is not a slow \
             test — it is a host that does not boot."
        ),
    };
    let room = session_world_component::<RoomSet>(app.world())
        .map(|rooms| rooms.active_spec().id.clone())
        .expect("the activated session has a RoomSet");

    // ⚠ **> 0, and asserting it is the residue of what this file used to prove.**
    // The old direct path settled in ZERO frames because its root existed before
    // the first update. If this ever reaches zero again, something has
    // reintroduced a build-time root — which is the exact thing K2b deleted, and
    // it would otherwise reintroduce it silently.
    assert!(
        frames > 0,
        "the gameplay host settled in zero frames, which means a session world \
         existed before the first update. Activation is asynchronous; a root that \
         beats it is a second way to start a game"
    );
    assert!(
        !room.is_empty(),
        "the activated session names no room, so it is a world with nothing in it"
    );
    eprintln!("[k2b] the gameplay host settled in {frames} frames, in room {room}");
}
