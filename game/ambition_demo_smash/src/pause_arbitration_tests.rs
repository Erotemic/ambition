//! Input-arbitration tests for the select screen, kept in a sibling file.
//!
//! The module-size gate counts inline `#[cfg(test)]` code toward its file but
//! excludes a sibling test file.

use super::*;
use ambition_platformer2d::input::participant::{
    context_priority, resolve_active_input_context, ContextClaim, ParticipantContexts,
};
use ambition_platformer2d::input::{
    InputParticipant, MenuControlFrame, SeatInputContexts, SeatMenuFrames, PAUSE_CONTEXT,
};
use bevy::prelude::*;

/// A seat that is browsing this screen, plus whatever else is claiming.
fn app_with(pause_open: bool) -> App {
    let mut app = App::new();
    app.init_resource::<SeatInputContexts>();
    app.init_resource::<SeatMenuFrames>();
    app.init_resource::<select::SmashSelect>();
    app.init_resource::<ambition_platformer2d::game_shell::ShellRouter>();
    app.init_resource::<select_screen::cursor::SelectCursors>();
    app.init_resource::<select_screen::SelectPage>();
    app.init_resource::<select_screen::SelectInteractionPolicy>();
    // The screen's driver writes the stage the START press will use.
    app.init_resource::<crate::SmashStageChoice>();
    app.init_resource::<crate::SmashStockChoice>();
    // `drive_the_cursor` integrates a held stick against `Time`, so the
    // fixture needs a clock (a real composition has `TimePlugin`).
    app.init_resource::<Time>();
    app.init_resource::<select_screen::StartRequested>();
    app.init_resource::<select_screen::LeaveRequested>();
    app.add_message::<ambition_platformer2d::game_shell::ShellCommand>();
    app.init_resource::<ambition_platformer2d::game_shell::ShellHostConfiguration>();
    app.world_mut()
        .resource_mut::<ambition_platformer2d::game_shell::ShellHostConfiguration>()
        .spec = Some(ambition_platformer2d::game_shell::ShellHostSpec::new(
        SMASH_SELECT_ROUTE,
        "ambition_launcher",
    ));
    // The default roster (this demo's own fighters): no catalog is needed,
    // only a non-empty grid for the cursor.
    app.init_resource::<select::SmashRoster>();
    app.add_systems(
        Update,
        (
            resolve_active_input_context,
            select_screen::drive_the_cursor.run_if(the_select_screen_owns_its_input),
            // The real consumer, in the real order: the flag matters only if
            // the system that spends it runs too.
            leave_the_select_screen_when_asked,
        )
            .chain(),
    );

    // On the select route, with this screen's own claim declared — the same
    // claim `declare_the_select_input_context` writes in production.
    let mut contexts = ParticipantContexts::default();
    contexts.declare(ContextClaim::capturing(
        ambition_platformer2d::input::SELECT_CONTEXT,
        context_priority::SELECT,
    ));
    // The pause menu's claim, at its real priority. This test stands in for
    // the host; neither the screen nor the pause menu names the other.
    if pause_open {
        contexts.declare(ContextClaim::capturing(
            PAUSE_CONTEXT,
            context_priority::PAUSE,
        ));
    }
    app.world_mut().spawn((
        InputParticipant {
            id: ambition_platformer2d::input::ParticipantId(0),
        },
        contexts,
    ));

    app.world_mut()
        .resource_mut::<ambition_platformer2d::game_shell::ShellRouter>()
        .active = Some(ambition_platformer2d::game_shell::ActiveShellExperience {
        activation_id: ambition_platformer2d::game_shell::ShellActivationId(1),
        route_id: ambition_platformer2d::game_shell::ShellRouteId::new(SMASH_SELECT_ROUTE),
        experience_id: ambition_platformer2d::game_shell::ShellExperienceId::new(
            SMASH_SELECT_EXPERIENCE,
        ),
        parameters: Default::default(),
        load_authorization: None,
        prepared_session: None,
    });

    // The cursor is on slot 1's button. With no window, the rectangles come
    // from `select_screen::layout` at `HEADLESS_VIEWPORT`, so the test
    // presses a real button. The control below proves the press lands.
    let button = select_screen::layout::SelectLayout::for_viewport(
        None,
        select::SmashRoster::default().cell_count(),
    )
    .role_button(0);
    app.world_mut()
        .resource_mut::<select_screen::cursor::SelectCursors>()
        .seat_mut(0)
        .expect("seat 0")
        .move_to(button.center());

    // Seat 0 presses confirm on that button, which cycles the slot.
    app.world_mut().resource_mut::<SeatMenuFrames>().set(
        0,
        MenuControlFrame {
            select: true,
            ..Default::default()
        },
    );
    app
}

/// What one seat is holding this frame, replacing what [`app_with`] armed.
/// BACK tests press through the same `SeatMenuFrames` channel a pad, a
/// keyboard and the touch overlay's "Back" button all use.
fn seat_presses(app: &mut App, seat: u8, frame: MenuControlFrame) {
    app.world_mut()
        .resource_mut::<SeatMenuFrames>()
        .set(seat, frame);
}

/// Which shell commands this frame produced. Drains, so a caller reads one
/// frame.
fn commands_sent(app: &mut App) -> Vec<ambition_platformer2d::game_shell::ShellCommand> {
    app.world_mut()
        .resource_mut::<Messages<ambition_platformer2d::game_shell::ShellCommand>>()
        .drain()
        .collect()
}

/// Did this frame ask the shell to go home (the pause menu's "Quit to Title"
/// command, which this screen also writes)?
fn asked_to_go_home(app: &mut App) -> bool {
    commands_sent(app).iter().any(|command| {
        matches!(
            command,
            ambition_platformer2d::game_shell::ShellCommand::QuitToHome
        )
    })
}

/// Tap-B is an in-screen token operation: with an empty hand, the cursor
/// returns to its own placed token and starts carrying it. It does not
/// navigate out of the character-select screen.
#[test]
fn tap_back_recalls_the_owners_token_without_leaving() {
    let mut app = app_with(false);
    // Spend the fixture's initial confirm: seat 0 becomes a controller on
    // Random and therefore owns a placed token.
    app.update();
    assert!(commands_sent(&mut app).is_empty());
    seat_presses(&mut app, 0, MenuControlFrame::default());

    let layout = select_screen::layout::SelectLayout::for_viewport(
        None,
        select::SmashRoster::default().cell_count(),
    );
    let token = select_screen::token_rect(
        &layout,
        app.world().resource::<select::SmashSelect>(),
        app.world().resource::<select::SmashRoster>(),
        0,
    )
    .expect("seat 0 joined on Random, so it owns a placed token");
    app.world_mut()
        .resource_mut::<select_screen::cursor::SelectCursors>()
        .seat_mut(0)
        .expect("seat 0")
        .move_to(layout.portrait(0).expect("a portrait").center());

    seat_presses(
        &mut app,
        0,
        MenuControlFrame {
            back: true,
            ..Default::default()
        },
    );
    app.update();

    let cursor = app
        .world()
        .resource::<select_screen::cursor::SelectCursors>()
        .seat(0)
        .expect("seat 0");
    assert_eq!(
        cursor.carrying,
        Some(0),
        "tap-B did not pick up the owner's token"
    );
    assert_eq!(
        cursor.position,
        token.center(),
        "tap-B moved the token to the hand instead of returning the hand to the token"
    );
    assert!(
        commands_sent(&mut app).is_empty(),
        "tap-B recalled a token and also left the lobby"
    );
}

/// The explicit Back control remains a shared way out. A connected input
/// seat does not need to own a match card merely to choose this UI action.
#[test]
fn a_later_seat_may_activate_the_back_control() {
    let mut app = app_with(false);
    seat_presses(&mut app, 0, MenuControlFrame::default());
    let back = select_screen::layout::SelectLayout::for_viewport(
        None,
        select::SmashRoster::default().cell_count(),
    )
    .back_button();
    app.world_mut()
        .resource_mut::<select_screen::cursor::SelectCursors>()
        .seat_mut(2)
        .expect("seat 2")
        .move_to(back.center());
    seat_presses(
        &mut app,
        2,
        MenuControlFrame {
            select: true,
            ..Default::default()
        },
    );
    app.update();
    assert!(
        asked_to_go_home(&mut app),
        "seat 3 could not activate the shared Back control"
    );
}

/// B while already carrying a token is a no-op. It neither drops the token
/// nor leaves the lobby.
#[test]
fn back_is_a_noop_while_carrying_a_token() {
    let mut app = app_with(false);
    app.world_mut()
        .resource_mut::<select_screen::cursor::SelectCursors>()
        .try_grab(0, 0);
    seat_presses(
        &mut app,
        0,
        MenuControlFrame {
            back: true,
            ..Default::default()
        },
    );
    app.update();
    assert_eq!(
        app.world()
            .resource::<select_screen::cursor::SelectCursors>()
            .seat(0)
            .expect("seat 0")
            .carrying,
        Some(0),
        "BACK dropped a carried token"
    );
    assert!(
        commands_sent(&mut app).is_empty(),
        "BACK while carrying also quit the lobby"
    );
}

/// Holding B is the navigation gesture. Unlike tap-B, it leaves the
/// character-select route once the hold threshold is crossed.
#[test]
fn holding_back_leaves_the_character_select_screen() {
    let mut app = app_with(false);
    app.update();
    commands_sent(&mut app);

    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_millis(600));
    seat_presses(
        &mut app,
        0,
        MenuControlFrame {
            back_held: true,
            ..Default::default()
        },
    );
    app.update();

    assert!(
        asked_to_go_home(&mut app),
        "holding B past the CSS threshold did not leave the lobby"
    );
}

/// An unseated connected participant may join by doing the thing they came
/// here to do: choosing a fighter. The press claims the first absent match
/// card and the same press chooses the portrait; no role-button preflight is
/// required.
#[test]
fn an_unseated_connected_cursor_claims_a_slot_when_it_selects_a_fighter() {
    let mut app = app_with(false);
    // Seat P1 through the fixture's real role-button press, then make seat 1
    // present in the same per-seat input table production fills for a second
    // connected participant.
    app.update();
    seat_presses(&mut app, 0, MenuControlFrame::default());
    seat_presses(&mut app, 1, MenuControlFrame::default());

    let layout = select_screen::layout::SelectLayout::for_viewport(
        None,
        select::SmashRoster::default().cell_count(),
    );
    let face = layout.portrait(1).expect("a grid with a second cell");
    app.world_mut()
        .resource_mut::<select_screen::cursor::SelectCursors>()
        .seat_mut(1)
        .expect("seat 1")
        .move_to(face.center());
    seat_presses(
        &mut app,
        1,
        MenuControlFrame {
            select: true,
            ..Default::default()
        },
    );
    app.update();

    let select = app.world().resource::<select::SmashSelect>();
    assert_eq!(
        select.slot(1).occupant,
        select::SlotOccupant::Controller { device: 1 },
        "the second connected cursor selected a fighter but never joined"
    );
    assert_eq!(
        select.slot(1).pick,
        Some(select::SlotPick::Fighter(1)),
        "the join press was consumed by seating instead of also choosing its fighter"
    );
}

/// A seated player may explicitly open an empty human card for another
/// connected participant. This is distinct from implicit join-on-selection:
/// the requester chooses the roster POSITION, while the model assigns the
/// first connected source that is not already seated.
#[test]
fn player_one_can_enable_a_slot_for_a_connected_second_player() {
    let mut app = app_with(false);
    app.update(); // the fixture seats source 0 in slot 0
    seat_presses(&mut app, 0, MenuControlFrame::default());
    // A neutral row is still evidence that source 1 exists; production
    // `populate_seat_menu_frames` writes one row per InputParticipant.
    seat_presses(&mut app, 1, MenuControlFrame::default());

    let role = select_screen::layout::SelectLayout::for_viewport(
        None,
        select::SmashRoster::default().cell_count(),
    )
    .role_button(1);
    app.world_mut()
        .resource_mut::<select_screen::cursor::SelectCursors>()
        .seat_mut(0)
        .expect("seat 0")
        .move_to(role.center());
    seat_presses(
        &mut app,
        0,
        MenuControlFrame {
            select: true,
            ..Default::default()
        },
    );
    app.update();

    assert_eq!(
        app.world()
            .resource::<select::SmashSelect>()
            .slot(1)
            .occupant,
        select::SlotOccupant::Controller { device: 1 },
        "enabling the second card ignored the connected, unseated second participant"
    );
}

/// A lobby with a CPU between two people routes the second one's presses to
/// the second one's card.
///
/// The roster is sparse and not in input-seat order:
///
/// ```text
/// card 0   Controller { device: 0 }
/// card 1   Cpu
/// card 2   Controller { device: 1 }
/// ```
///
/// The second person reports on input seat one; the screen must not use that
/// as a card index.
#[test]
fn a_cpu_between_two_people_does_not_swallow_the_second_ones_presses() {
    let mut app = app_with(false);
    // `app_with` arms seat 0 on a role button; this test is about seat 1.
    app.world_mut().resource_mut::<SeatMenuFrames>().clear();
    {
        let mut select = app.world_mut().resource_mut::<select::SmashSelect>();
        select.set_occupant(0, select::SlotOccupant::Controller { device: 0 });
        select.set_occupant(1, select::SlotOccupant::Cpu);
        select.set_occupant(2, select::SlotOccupant::Controller { device: 1 });
    }

    let layout = select_screen::layout::SelectLayout::for_viewport(
        None,
        select::SmashRoster::default().cell_count(),
    );
    let face = layout.portrait(1).expect("a grid with a second cell");
    app.world_mut()
        .resource_mut::<select_screen::cursor::SelectCursors>()
        .seat_mut(1)
        .expect("seat 1")
        .move_to(face.center());
    seat_presses(
        &mut app,
        1,
        MenuControlFrame {
            select: true,
            ..Default::default()
        },
    );
    app.update();

    let select = app.world().resource::<select::SmashSelect>();
    assert_eq!(
        select.slot(2).pick,
        Some(select::SlotPick::Fighter(1)),
        "the second person's press did not reach the card their controller drives"
    );
    // The other half: landing on card 2 is right only if it did not also land
    // on the CPU's card.
    assert_eq!(
        select.slot(1).pick,
        Some(select::SlotPick::Random),
        "the second person chose the CPU's fighter"
    );
    assert_eq!(
        select.slot(0).pick,
        Some(select::SlotPick::Random),
        "seat 1's press reached seat 0's card"
    );
}

/// One token has at most one carrier.
///
/// A human may pick up a CPU's token, but two cursors must not carry the same
/// one. The incumbent keeps it, resolved in seat order, so the result is
/// deterministic.
#[test]
fn two_people_reaching_for_one_cpu_token_do_not_both_get_it() {
    let mut app = app_with(false);
    app.world_mut().resource_mut::<SeatMenuFrames>().clear();
    {
        let mut select = app.world_mut().resource_mut::<select::SmashSelect>();
        select.set_occupant(0, select::SlotOccupant::Controller { device: 0 });
        select.set_occupant(1, select::SlotOccupant::Controller { device: 1 });
        select.set_occupant(2, select::SlotOccupant::Cpu);
    }

    let layout = select_screen::layout::SelectLayout::for_viewport(
        None,
        select::SmashRoster::default().cell_count(),
    );
    let token = select_screen::token_rect(
        &layout,
        app.world().resource::<select::SmashSelect>(),
        app.world().resource::<select::SmashRoster>(),
        2,
    )
    .expect("the machine is in the lobby, so it owns a token");
    {
        let mut cursors = app
            .world_mut()
            .resource_mut::<select_screen::cursor::SelectCursors>();
        cursors.seat_mut(0).expect("seat 0").move_to(token.center());
        cursors.seat_mut(1).expect("seat 1").move_to(token.center());
    }
    let press = MenuControlFrame {
        select: true,
        ..Default::default()
    };
    seat_presses(&mut app, 0, press);
    seat_presses(&mut app, 1, press);
    app.update();

    let cursors = *app
        .world()
        .resource::<select_screen::cursor::SelectCursors>();
    assert_eq!(
        (
            cursors.seat(0).expect("seat 0").carrying,
            cursors.seat(1).expect("seat 1").carrying
        ),
        (Some(2), None),
        "both hands closed on the machine's one token"
    );
    assert_eq!(
        cursors.carrier_of(2),
        Some(0),
        "the token names a carrier the cursors disagree with"
    );
}

/// Escape opens the pause menu and does not also quit.
///
/// `presets.rs` binds Escape to both `Start` and `MenuBack` (tested in
/// `rebind.rs`). The pause menu opens on `start` in the same unordered
/// `InputSet::Consume` set, so a bare `back` read would quit the lobby from
/// under the menu. The Back-control test above proves the screen still has a
/// way out.
#[test]
fn escape_does_not_quit_the_lobby_out_from_under_the_pause_menu_it_opens() {
    let mut app = app_with(false);
    seat_presses(
        &mut app,
        0,
        // What Escape produces: both edges, one frame.
        MenuControlFrame {
            back: true,
            start: true,
            ..Default::default()
        },
    );
    app.update();
    assert!(
        commands_sent(&mut app).is_empty(),
        "Escape quit the lobby as well as opening the pause menu over it"
    );
}

/// A composition whose home is this screen draws no way out.
///
/// The standalone demo uses `SMASH_SELECT_ROUTE` as its home route, so
/// `QuitToHome` would re-enter the same route: a dead button.
#[test]
fn there_is_no_way_out_when_the_lobby_is_itself_home() {
    let mut app = app_with(false);
    app.world_mut()
        .resource_mut::<ambition_platformer2d::game_shell::ShellHostConfiguration>()
        .spec = Some(ambition_platformer2d::game_shell::ShellHostSpec::new(
        SMASH_SELECT_ROUTE,
        SMASH_SELECT_ROUTE,
    ));
    seat_presses(
        &mut app,
        0,
        MenuControlFrame {
            back: true,
            ..Default::default()
        },
    );
    app.update();
    assert!(
        commands_sent(&mut app).is_empty(),
        "the standalone demo asked to leave for the screen it is already on"
    );
}

/// The screen drives when it owns its seat. The control for the test below.
#[test]
fn the_select_screen_reads_its_seat_when_nothing_is_over_it() {
    let mut app = app_with(false);
    app.update();
    assert_eq!(
        app.world()
            .resource::<select::SmashSelect>()
            .participating(),
        1,
        "a click on slot 1's button did nothing while this screen owned the seat"
    );
}

/// One press moves one thing.
///
/// With the pause menu open over this screen, both read the arrows through
/// different channels (`MenuControlFrame` and `SeatMenuFrames`), and this demo
/// cannot name `ShellPauseMenu` (`basic_shell_presentation` is not in
/// `all_capabilities`). So the claim system arbitrates: a capturing claim
/// above `SELECT` closes this screen's context. Neither side names the other.
#[test]
fn a_pause_claim_takes_the_arrows_away_from_the_select_screen() {
    let mut app = app_with(true);
    app.update();
    assert_eq!(
        app.world()
            .resource::<select::SmashSelect>()
            .participating(),
        0,
        "the pause menu owns the presses; the screen underneath must not \
         also act on them"
    );
}

/// The screen publishes its submit verb while it is up, and takes it back
/// when it leaves. A stale cue would tell the next screen's player to choose
/// a fighter.
#[test]
fn the_select_screen_publishes_its_cue_and_retracts_it_on_the_way_out() {
    use ambition_platformer2d::input::{ActiveUiCues, SELECT_CONTEXT};

    let mut app = app_with(false);
    app.init_resource::<ActiveUiCues>();
    app.add_systems(Update, publish_the_select_ui_cue);
    app.update();
    assert_eq!(
        app.world()
            .resource::<ActiveUiCues>()
            .for_context(SELECT_CONTEXT)
            .map(|cue| cue.submit_label.as_str()),
        Some("Choose"),
        "the lobby is up and nothing says what confirming does"
    );

    // Leave the route: the only change.
    app.world_mut()
        .resource_mut::<ambition_platformer2d::game_shell::ShellRouter>()
        .active = None;
    app.update();
    assert!(
        app.world()
            .resource::<ActiveUiCues>()
            .for_context(SELECT_CONTEXT)
            .is_none(),
        "a cue left behind outlives its surface"
    );
}
