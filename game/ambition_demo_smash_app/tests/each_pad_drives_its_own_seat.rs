//! Each controller drives its own hand on the select screen and its own
//! fighter in the match, through the real device road: a `Gamepad` entity, the
//! pad table, the seat's input map and leafwing.
//!
//! The other select-screen tests write `SeatMenuFrames`, which is after the
//! road these tests are about.
//!
//! These tests need the `input` feature, which the default cell of this crate
//! does not have:
//!
//! ```text
//! cargo test -p ambition_demo_smash_app --features input --test smash_it -- each_pad_drives_its_own_seat
//! ```

#![cfg(feature = "input")]

use ambition_demo_smash::select::{SlotOccupant, SlotPick, SmashSelect};
use ambition_demo_smash_app::build_demo_app;
use ambition_platformer2d::input::{LocalDeviceOrder, SeatMenuFrames};
use bevy::input::gamepad::{Gamepad, GamepadAxis};
use bevy::prelude::*;

/// The demo app, on the select screen, with these controllers connected.
fn lobby(pads: &[&str]) -> (App, Vec<Entity>) {
    let mut app = build_demo_app();
    app.update();
    let pads = pads
        .iter()
        .map(|name| {
            app.world_mut()
                .spawn((Gamepad::default(), Name::new(name.to_string())))
                .id()
        })
        .collect();
    settle(&mut app);
    (app, pads)
}

fn settle(app: &mut App) {
    for _ in 0..4 {
        app.update();
    }
}

/// Disconnect a controller as Bevy does: the entity stays and the component
/// goes.
fn unplug(app: &mut App, pad: Entity) {
    app.world_mut().entity_mut(pad).remove::<Gamepad>();
    settle(app);
}

/// Connect a controller again on its own entity, as Bevy does.
fn plug_back(app: &mut App, pad: Entity) {
    app.world_mut().entity_mut(pad).insert(Gamepad::default());
    settle(app);
}

/// Hold this controller's stick to the right for some frames, give back what
/// `read` saw while it was held, and let go.
fn while_pushing<T>(app: &mut App, pad: Entity, read: impl Fn(&App) -> T) -> T {
    let set = |app: &mut App, x: f32| {
        app.world_mut()
            .get_mut::<Gamepad>(pad)
            .expect("a connected pad")
            .analog_mut()
            .set(GamepadAxis::LeftStickX, x);
    };
    set(app, 1.0);
    for _ in 0..3 {
        app.update();
    }
    let seen = read(app);
    set(app, 0.0);
    for _ in 0..2 {
        app.update();
    }
    seen
}

/// The select-screen seats whose stick is deflected.
fn hands_moved(app: &App) -> Vec<u8> {
    app.world()
        .resource::<SeatMenuFrames>()
        .seats()
        .filter(|(_, frame)| frame.analog.x != 0.0)
        .map(|(seat, _)| seat)
        .collect()
}

/// The fighter slots whose raw frame holds a direction.
fn fighters_moved(app: &App) -> Vec<u8> {
    app.world()
        .resource::<ambition_platformer2d::characters::control::SeatRawFrames>()
        .seats()
        .filter(|(_, frame)| frame.axis_x != 0.0)
        .map(|(slot, _)| slot.0)
        .collect()
}

/// Start a match in which each `(slot, source)` pair is a person and each
/// slot of `cpus` is a CPU. A source is the number of a select-screen seat:
/// 0 is the keyboard and `n` is pad `n - 1`.
fn start_match(app: &mut App, people: &[(usize, usize)], cpus: &[usize]) {
    {
        let mut select = app.world_mut().resource_mut::<SmashSelect>();
        for (slot, source) in people {
            assert!(
                select.assign_controller(*slot, *source),
                "source {source} could not take slot {slot}"
            );
            select.set_pick(*slot, SlotPick::Random);
        }
        for slot in cpus {
            select.set_occupant(*slot, SlotOccupant::Cpu);
            select.set_pick(*slot, SlotPick::Random);
        }
        assert!(select.ready(), "the fixture's lobby is not ready");
    }
    app.world_mut()
        .resource_mut::<ambition_demo_smash::select_screen::StartRequested>()
        .0 = true;
    for _ in 0..240 {
        app.update();
    }
    let route = app
        .world()
        .resource::<ambition_platformer2d::game_shell::ShellRouter>()
        .active
        .as_ref()
        .map(|active| active.route_id.as_str().to_owned());
    assert_eq!(
        route.as_deref(),
        Some(ambition_demo_smash::SMASH_GAMEPLAY_ROUTE),
        "the match did not start"
    );
}

/// Four controllers and no keyboard player. Each pad moves one hand, and in
/// the match each pad moves the fighter of the slot it took.
///
/// Measured before: the screen offered four seats and the keyboard held the
/// first, so the fourth pad moved no hand and could take no slot.
#[test]
fn four_pads_fill_the_four_slots_with_the_keyboard_in_none() {
    let (mut app, pads) = lobby(&["pad a", "pad b", "pad c", "pad d"]);
    for (index, pad) in pads.iter().enumerate() {
        assert_eq!(
            while_pushing(&mut app, *pad, hands_moved),
            [index as u8 + 1],
            "pad {index} moves the hand of its own seat and no other"
        );
    }
    start_match(&mut app, &[(0, 1), (1, 2), (2, 3), (3, 4)], &[]);
    for (index, pad) in pads.iter().enumerate() {
        assert_eq!(
            while_pushing(&mut app, *pad, fighters_moved),
            [index as u8],
            "pad {index} moves the fighter of the slot it took and no other"
        );
    }
}

/// Jon's report, 2026-10-08: one stick moved every hand. The middle of three
/// pads disconnects.
///
/// Measured before: pad a moved the hands of seats 1 and 2, and pad c moved
/// no hand. The seat of the missing pad had no pad association, and leafwing
/// gives such a seat the first connected pad.
#[test]
fn a_disconnect_on_the_select_screen_moves_no_other_controller() {
    let (mut app, pads) = lobby(&["pad a", "pad b", "pad c"]);
    let (a, b, c) = (pads[0], pads[1], pads[2]);
    unplug(&mut app, b);
    assert_eq!(while_pushing(&mut app, a, hands_moved), [1]);
    assert_eq!(while_pushing(&mut app, c, hands_moved), [3]);

    plug_back(&mut app, b);
    assert_eq!(while_pushing(&mut app, a, hands_moved), [1]);
    assert_eq!(while_pushing(&mut app, b, hands_moved), [2]);
    assert_eq!(while_pushing(&mut app, c, hands_moved), [3]);
}

/// A pad disconnects on the select screen, and then two people start a match.
/// Each fighter answers the controller whose hand took its slot.
///
/// Measured before: the hands and the roster named the pads by two different
/// records, so after a disconnect pad b drove the fighter that pad c's hand
/// chose, and pad c drove the other.
#[test]
fn a_fighter_answers_the_controller_whose_hand_took_its_slot() {
    let (mut app, pads) = lobby(&["pad a", "pad b", "pad c"]);
    let (a, b, c) = (pads[0], pads[1], pads[2]);
    unplug(&mut app, a);
    let hand_of_c = while_pushing(&mut app, c, hands_moved);
    let hand_of_b = while_pushing(&mut app, b, hands_moved);
    assert_eq!((hand_of_c.len(), hand_of_b.len()), (1, 1));

    // Pad c's hand takes the first slot and pad b's the second.
    start_match(
        &mut app,
        &[(0, hand_of_c[0] as usize), (1, hand_of_b[0] as usize)],
        &[],
    );
    assert_eq!(while_pushing(&mut app, c, fighters_moved), [0]);
    assert_eq!(while_pushing(&mut app, b, fighters_moved), [1]);
}

/// One person holds the second pad and plays a CPU.
///
/// Measured before: the first pad drove the fighter and the pad in the
/// player's hands did nothing.
#[test]
fn a_lone_player_on_the_second_pad_drives_the_fighter() {
    let (mut app, pads) = lobby(&["pad a", "pad b"]);
    start_match(&mut app, &[(0, 2)], &[1]);
    assert_eq!(while_pushing(&mut app, pads[0], fighters_moved), [] as [u8; 0]);
    assert_eq!(while_pushing(&mut app, pads[1], fighters_moved), [0]);
}

/// The keyboard takes a slot beside three pads.
#[test]
fn the_keyboard_and_three_pads_fill_the_four_slots() {
    let (mut app, pads) = lobby(&["pad a", "pad b", "pad c"]);
    start_match(&mut app, &[(0, 0), (1, 1), (2, 2), (3, 3)], &[]);
    for (index, pad) in pads.iter().enumerate() {
        assert_eq!(
            while_pushing(&mut app, *pad, fighters_moved),
            [index as u8 + 1],
            "the keyboard has slot 0, so pad {index} has the next one"
        );
    }
}

/// In a match a controller that disconnects leaves its fighter with no
/// input. A different controller that connects does not get the fighter, and
/// the controller that left gets it back.
#[test]
fn a_fighter_whose_controller_disconnects_answers_no_other_controller() {
    let (mut app, pads) = lobby(&["pad a", "pad b"]);
    let (a, b) = (pads[0], pads[1]);
    start_match(&mut app, &[(0, 1), (1, 2)], &[]);

    unplug(&mut app, a);
    let other = app
        .world_mut()
        .spawn((Gamepad::default(), Name::new("another pad")))
        .id();
    settle(&mut app);
    assert_eq!(
        app.world().resource::<LocalDeviceOrder>().pad(0),
        Some(other),
        "premise: the new controller has the number of the one that left"
    );
    assert_eq!(
        while_pushing(&mut app, other, fighters_moved),
        [] as [u8; 0],
        "a different controller took over the fighter"
    );
    assert_eq!(while_pushing(&mut app, b, fighters_moved), [1]);

    app.world_mut().entity_mut(other).despawn();
    plug_back(&mut app, a);
    assert_eq!(while_pushing(&mut app, a, fighters_moved), [0]);
}
