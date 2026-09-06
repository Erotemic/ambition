//! End-to-end door-entry test through real player interaction input.
//! The controlled body enters an authored door zone, presses interact, and must
//! reach a different active room without synthesizing the transition request.

use ambition_app::{AgentAction, Platformer2dSimHarness};
use ambition_platformer2d::engine_core::AabbExt;
use bevy::prelude::With;

use crate::common::{base, fixed_60hz_sim};

/// The active room's id, as the sim reports it.
fn active_room(sim: &mut Platformer2dSimHarness) -> String {
    sim.observation().active_room.clone()
}

/// Stand the controlled body in the centre of an authored `Door` zone of the
/// active room, and report the zone's name. `None` when the room authors none.
fn stand_in_a_door(sim: &mut Platformer2dSimHarness) -> Option<String> {
    let door = {
        let world = sim.world_mut();
        let mut query = world.query::<&ambition_platformer2d::world::rooms::RoomSet>();
        let room_set = query.iter(world).next()?;
        room_set
            .active_loading_zones()
            .iter()
            .find(|zone| {
                zone.activation == ambition_platformer2d::world::rooms::LoadingZoneActivation::Door
            })
            .cloned()?
    };
    let world = sim.world_mut();
    let mut player = world.query_filtered::<&mut ambition_platformer2d::platformer::body::BodyKinematics, With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>();
    let mut kin = player.single_mut(world).ok()?;
    kin.pos = door.aabb.center();
    kin.vel = ambition_platformer2d::engine_core::Vec2::ZERO;
    Some(door.name.clone())
}

fn interact() -> AgentAction {
    AgentAction {
        interact: true,
        interact_held: true,
        ..base()
    }
}

/// Pressing interact in a door goes through it.
#[test]
fn standing_in_a_door_and_pressing_interact_changes_the_room() {
    let mut sim = fixed_60hz_sim();
    // Let the room finish constructing before anything is placed in it.
    for _ in 0..10 {
        sim.step(base());
    }
    let before = active_room(&mut sim);
    let Some(door) = stand_in_a_door(&mut sim) else {
        panic!(
            "the start room '{before}' authors no `Door` loading zone, so this \
             test is measuring nothing — point it at a room that has one"
        );
    };

    // Press and hold. A door is buffered-interact, and the transition commits a
    // frame or two later (a rollback host defers it to a confirmed boundary), so
    // the press is held across the window rather than tapped once.
    for _ in 0..30 {
        sim.step(interact());
        if active_room(&mut sim) != before {
            return;
        }
    }

    panic!(
        "held interact inside the `{door}` door of '{before}' for 30 frames and \
         the room never changed. The transition itself is covered elsewhere by \
         writing `RoomTransitionRequested` directly — so what this failure names \
         is the INPUT half: the press reaching \
         `SlotInteractionState::primary().buffered()` and \
         `detect_room_transition_system` consuming it"
    );
}

/// And the same door, in the SHIPPED HOST, pressed as a key.
///
/// The key is READ from the live input map rather than hardcoded: the interact
/// key differs per preset (`F` on the arrow presets, `E` on the WASD ones), and
/// a test that pins one of them would go green on a build where the other is
/// active and the player's key does nothing.
#[cfg(feature = "input")]
#[test]
fn a_door_in_the_shipped_host_opens_for_the_interact_key() {
    use ambition_app::app::shell_host;
    use ambition_platformer2d::game_shell::ShellCommand;
    use ambition_platformer2d::input::{InputParticipant, Platformer2dInputActionMonolith};
    use bevy::asset::AssetPlugin;
    use bevy::image::ImagePlugin;
    use bevy::prelude::*;
    use bevy::state::app::StatesPlugin;
    use bevy::transform::TransformPlugin;
    use bevy::MinimalPlugins;
    use leafwing_input_manager::prelude::InputMap;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.add_plugins(ImagePlugin::default());
    app.add_plugins(TransformPlugin);
    app.add_plugins(StatesPlugin);
    app.init_state::<ambition_platformer2d::platformer::schedule::GameMode>();
    app.insert_resource(shell_host::AmbitionShellHosted);
    ambition_app::app::init_sandbox_resources(&mut app);
    ambition_app::app::add_simulation_plugins(&mut app);
    app.add_plugins(ambition_platformer2d::host::PlatformerHostPlugins);
    shell_host::compose_ambition_shell_host(&mut app);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs_f64(1.0 / 60.0),
    ));

    for _ in 0..8 {
        app.update();
    }
    app.world_mut().write_message(ShellCommand::GoTo(
        shell_host::AMBITION_GAMEPLAY_ROUTE.into(),
    ));
    for _ in 0..40 {
        app.update();
    }

    // The interact key THIS build binds, from the participant's own map.
    let interact_key = {
        let world = app.world_mut();
        let mut q = world
            .query_filtered::<&InputMap<Platformer2dInputActionMonolith>, With<InputParticipant>>();
        let map = q
            .iter(world)
            .next()
            .expect("the host spawns a primary input participant at boot");
        map.get_buttonlike(&Platformer2dInputActionMonolith::Interact)
            .and_then(|bindings| bindings.first().cloned())
            .expect("Interact has a binding, or no key opens a door at all")
    };

    // Stand in an authored Door zone of the live session's room.
    let door = {
        let world = app.world_mut();
        let mut rooms = world.query::<&ambition_platformer2d::world::rooms::RoomSet>();
        let zone = rooms
            .iter(world)
            .next()
            .expect("a live session room set")
            .active_loading_zones()
            .iter()
            .find(|zone| {
                zone.activation == ambition_platformer2d::world::rooms::LoadingZoneActivation::Door
            })
            .cloned();
        //  LOUD, not a quiet `return`. A test that skips itself when it cannot
        // find its subject is a test that reports green for the one reason it
        // exists to catch.
        let zone = zone.unwrap_or_else(|| {
            let mut rooms = world.query::<&ambition_platformer2d::world::rooms::RoomSet>();
            let room = rooms
                .iter(world)
                .next()
                .map(|set| set.active_spec().id.clone())
                .unwrap_or_default();
            panic!(
                "the host's gameplay start room '{room}' authors no `Door` \
                 loading zone, so this test pressed a key at nothing. Point it \
                 at a room that has one."
            )
        });
        let mut player = world.query_filtered::<&mut ambition_platformer2d::platformer::body::BodyKinematics, With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>();
        if let Ok(mut kin) = player.single_mut(world) {
            kin.pos = zone.aabb.center();
            kin.vel = ambition_platformer2d::engine_core::Vec2::ZERO;
        }
        zone
    };

    let room_before = {
        let world = app.world_mut();
        let mut rooms = world.query::<&ambition_platformer2d::world::rooms::RoomSet>();
        rooms
            .iter(world)
            .next()
            .expect("a live session room set")
            .active_spec()
            .id
            .clone()
    };

    interact_key.press(app.world_mut());
    for _ in 0..40 {
        app.update();
        let world = app.world_mut();
        let mut rooms = world.query::<&ambition_platformer2d::world::rooms::RoomSet>();
        let now = rooms
            .iter(world)
            .next()
            .map(|set| set.active_spec().id.clone());
        if now.as_deref() != Some(room_before.as_str()) {
            return;
        }
    }

    panic!(
        "the shipped host held its own Interact binding inside the `{}` door of \
         '{room_before}' for 40 frames and the room never changed. The sim-harness \
         case in this file passes, so the break is in what the HOST adds: the \
         input context that has to grant the participant's actions to gameplay, \
         or the binding above reaching `ControlFrame.interact_pressed`",
        door.name
    );
}

/// The other way in: double-tap Up.
///
/// The binding rule is that a SINGLE press of Up must not open anything — Up is
/// too useful as a direction — and that a deliberate double-tap stays as the
/// fallback. It is the gesture a player who has been in this game a while
/// reaches for, so "I can't enter doors anymore" can mean this half broke while
/// the explicit Interact key still works.
#[test]
fn a_deliberate_double_tap_up_opens_a_door_and_one_press_does_not() {
    let mut sim = fixed_60hz_sim();
    for _ in 0..10 {
        sim.step(base());
    }
    let before = active_room(&mut sim);
    let door = stand_in_a_door(&mut sim)
        .unwrap_or_else(|| panic!("the start room '{before}' authors no `Door` loading zone"));

    let up = AgentAction {
        up_pressed: true,
        move_y: -1.0,
        ..base()
    };

    // ONE tap, held for a beat, then released. Nothing may open.
    sim.step(up);
    for _ in 0..6 {
        sim.step(base());
    }
    assert_eq!(
        active_room(&mut sim),
        before,
        "a single Up opened the `{door}` door. Up is a direction — a door that \
         opens on one press of it opens while somebody is jumping past it"
    );

    // The second tap, inside the window.
    sim.step(up);
    for _ in 0..30 {
        sim.step(base());
        if active_room(&mut sim) != before {
            return;
        }
    }

    panic!(
        "double-tapping Up inside the `{door}` door of '{before}' did not open \
         it. The explicit Interact key is covered above, so what this names is \
         the GESTURE half: `register_up_tap` seeing the second edge inside \
         `up_double_tap_window`, and `double_tap_up_pending` reaching the \
         interact buffer"
    );
}

/// Holding Up opens the door only after the hold threshold and emits one
/// interaction edge. `AgentAction::up_pressed` is the rising edge; the held level is `move_y`.
#[test]
fn holding_up_opens_a_door_and_a_short_hold_does_not() {
    let mut sim = fixed_60hz_sim();
    for _ in 0..10 {
        sim.step(base());
    }
    let before = active_room(&mut sim);
    let door = stand_in_a_door(&mut sim)
        .unwrap_or_else(|| panic!("the start room '{before}' authors no `Door` loading zone"));

    let hold = AgentAction {
        move_y: -1.0,
        ..base()
    };

    for _ in 0..60 {
        sim.step(hold);
        assert_eq!(
            active_room(&mut sim),
            before,
            "one second of Up opened the `{door}` door. The hold is two seconds              so that holding a direction while climbing or aiming cannot enter              a room"
        );
    }

    for _ in 0..180 {
        sim.step(hold);
        if active_room(&mut sim) != before {
            return;
        }
    }

    panic!(
        "holding Up in the `{door}` door of '{before}' did not open it. The          press and the double-tap are covered above, so what this names is          `held_up_interact` crossing `interaction_hold_time` and reaching the          interact buffer"
    );
}

/// A DOOR under a ROLLBACK host — the combination nothing covered.
///
/// Covers the rollback-host door path: hold interact on a real door and require the room to change.
#[cfg(feature = "rl_sim")]
#[test]
fn a_door_opens_under_a_rollback_host_and_not_only_a_fixed_tick_one() {
    use ambition_app::rl_sim::{
        AmbitionSim, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode,
    };

    let mut sim = Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            // A rollback host: this is what puts `ConfirmedFrameBoundary` in the
            // world and sends the transition down the DEFERRED branch.
            .with_sync_test_rollback_settings(4, 10),
    )
    .expect("the GGRS sync-test harness builds");

    // Settle, so the room and its zones exist before anything is measured.
    for _ in 0..20 {
        sim.step(base());
    }

    let before = active_room(&mut sim);

    //  `teleport_player`, NOT a `world_mut()` write. The body's position is ROLLBACK
    // STATE: a direct write is restored out from under itself on the next resim, so the body
    // never overlaps the zone on a frame the system sees. `teleport_player` rebases the
    // baseline, which is what folds the new position into history.
    let door = {
        let world = sim.world_mut();
        let mut query = world.query::<&ambition_platformer2d::world::rooms::RoomSet>();
        let room_set = query
            .iter(world)
            .next()
            .expect("the active room has a RoomSet");
        room_set
            .active_loading_zones()
            .iter()
            .find(|zone| {
                zone.activation == ambition_platformer2d::world::rooms::LoadingZoneActivation::Door
            })
            .cloned()
    };
    let Some(door) = door else {
        panic!(
            "the start room '{before}' authors no `Door` loading zone, so this \
             test is measuring nothing — point it at a room that has one"
        );
    };
    let centre = door.aabb.center();
    sim.teleport_player((centre.x, centre.y));

    // Longer than the eager case on purpose: a deferred transition waits for a
    // CONFIRMED frame, and the sync-test host confirms behind the prediction
    // window.
    for _ in 0..240 {
        sim.step(interact());
        if active_room(&mut sim) != before {
            return;
        }
    }

    panic!(
        "held interact inside the `{}` door of '{before}' for 240 frames under a \
         ROLLBACK host and the room never changed — while the same door opens on \
         a fixed-tick sim. That is the deferred branch: the press is consumed by \
         `slot_gestures.primary_mut().clear()` and the confirmed commit never \
         delivers the transition",
        door.name
    );
}

/// THE SECOND HOST LEDGER ROW NAMED, ASKED THE SAME WAY — the registry out of the finished
/// world, not a log line nobody can see.
///
///  a `warn!` cannot settle it: no `LogPlugin`, no output, and this row has
/// already been fooled once by a green run that captured its own probe.
#[test]
#[cfg(feature = "rl_sim")]
fn the_rollback_door_host_publishes_a_prepared_cast() {
    use ambition_app::rl_sim::{
        AmbitionSim, Platformer2dSimHarness, Platformer2dSimHarnessOptions, TimestepMode,
    };
    use ambition_platformer2d::characters::prepared::PreparedCharacterRegistry;

    let mut sim = Platformer2dSimHarness::new_with_options(
        Platformer2dSimHarnessOptions::default()
            .with_timestep(TimestepMode::fixed_60hz())
            .with_sync_test_rollback_settings(4, 10),
    )
    .expect("the GGRS sync-test harness builds");
    for _ in 0..20 {
        sim.step(base());
    }

    let ids: Vec<String> = sim
        .world_mut()
        .get_resource::<PreparedCharacterRegistry>()
        .map(|registry| registry.ids().map(str::to_string).collect())
        .unwrap_or_default();
    assert!(
        !ids.is_empty(),
        "the rollback door host publishes NO prepared cast, so every \
         character-named placement in the rooms it loads falls back to a \
         generic — ledger D75, live"
    );
    //  THE OTHER TERM: this harness composes AMBITION, so its own protagonist must be there.
    assert!(
        ids.iter().any(|id| id == "player_robot_v3"),
        "the Ambition sim harness published a cast without Ambition's \
         protagonist: {ids:?}"
    );
}
