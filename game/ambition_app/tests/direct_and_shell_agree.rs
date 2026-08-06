//! **K2b's evidence: the two entry paths produce the same world.**
//!
//! Direct entry spawns its `SessionRoot` at PLUGIN-BUILD time; a shell-routed
//! host activates asynchronously behind a load barrier and eight preparation
//! work items. The plan for K2b is to delete the first and make direct entry
//! *a shell host whose initial route is the gameplay route* — and the reason
//! that plan is staged rather than done in one commit is that nothing proved
//! the two agree. `tracks.md` says so outright: *"No test asserts on
//! `publish_direct_prepared_session_root` or `SessionScopeId(0)` directly — the
//! coverage is all implicit, which is exactly what makes risk 1 dangerous."*
//!
//! This is that test. Both apps settle through the same helper and are asked the
//! same question.
//!
//! ⚠ **it compares the ROOM, not the entity.** The scopes differ by
//! construction — direct entry mints `SessionScopeId(0)` as a placeholder and a
//! shell activation mints its own — and an assertion on ids would pin the
//! placeholder this work exists to delete.

use ambition_app::app::AmbitionGameSimulationPlugin;
use ambition_platformer2d::platformer::lifecycle::{
    session_world_component, settle_until_session_world, SESSION_SETTLE_FRAMES,
};
use ambition_platformer2d::runtime::demo_fixture::RoomSet;
use bevy::prelude::*;

fn active_room(app: &App) -> Option<String> {
    session_world_component::<RoomSet>(app.world()).map(|rooms| rooms.active_spec().id.clone())
}

#[test]
fn direct_entry_and_a_shell_booted_to_gameplay_land_in_the_same_room() {
    // ── the path that exists today ──────────────────────────────────────────
    let mut direct = App::new();
    ambition_platformer2d::runtime::add_headless_foundation(&mut direct);
    direct.add_plugins(AmbitionGameSimulationPlugin);
    let direct_frames = settle_until_session_world(&mut direct, SESSION_SETTLE_FRAMES)
        .expect("direct entry has a session world");
    assert_eq!(
        direct_frames, 0,
        "direct entry's root is built at PLUGIN-BUILD time, so it must be there \
         before the first update — if this ever takes a frame, the staging \
         assumption behind K2b.1 has changed"
    );
    let direct_room = active_room(&direct).expect("direct entry has a RoomSet");

    // ── the path K2b moves to ───────────────────────────────────────────────
    // ⚠ **the simulation plugin comes FIRST, and finding that out is half the
    // value of this test.** Composing the shell host alone panics on the first
    // frame: `settle_versus_round` requires `Res<WorldTime>`, which
    // `AmbitionGameSimulationPlugin` installs. The shell is an ADAPTER over a
    // composed game, not a composition of one — the CLI adds the sim plugins and
    // then calls the composer, in that order, and a caller who reverses it gets
    // a parameter-validation panic naming a system it never heard of.
    let mut shell = App::new();
    ambition_platformer2d::runtime::add_headless_foundation(&mut shell);
    // ⚠ **BEFORE the sim plugin, and that ordering is load-bearing.**
    // `publish_direct_prepared_session_root` runs at the end of the sim plugin's
    // build and skips when this resource is present. Composing the shell without
    // it produces TWO canonical roots — the build-time one and the activation's
    // — and `session_world_entity` panics with "more than one canonical
    // SessionRoot exists". That is the collision `tracks.md` predicts in prose;
    // it is real, and this is the line that avoids it.
    shell.insert_resource(ambition_app::app::shell_host::AmbitionShellHosted);
    shell.add_plugins(AmbitionGameSimulationPlugin);
    ambition_app::app::shell_host::compose_ambition_shell_host_booting_to(
        &mut shell,
        ambition_app::app::shell_host::AMBITION_GAMEPLAY_ROUTE,
    );
    let shell_frames = match settle_until_session_world(&mut shell, SESSION_SETTLE_FRAMES) {
        Ok(frames) => frames,
        Err(budget) => panic!(
            "a shell host booted straight to the gameplay route produced no session \
             world in {budget} frames. That is K2b's risk 1 in one sentence: the \
             root exists only once the load barrier reaches Ready and every \
             preparation work item has completed."
        ),
    };
    let shell_room = active_room(&shell).expect("the shell activation has a RoomSet");

    assert_eq!(
        direct_room, shell_room,
        "the two entry paths disagree about which room the player starts in, so \
         deleting the build-time root would change the game rather than move it"
    );
    assert!(
        shell_frames > 0,
        "the shell path settled in zero frames, which means it is not actually \
         activating asynchronously — and then this test is comparing one path \
         with itself"
    );
    eprintln!("[k2b] direct settled in {direct_frames} frames, shell in {shell_frames}");
}
