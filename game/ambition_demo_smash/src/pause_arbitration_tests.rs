//! Extracted from `lib.rs` on 2026-08-30, unchanged.
//!
//! ⭐ THE MODULE-SIZE GATE COUNTS INLINE `#[cfg(test)]` TOWARD ITS FILE and
//! excludes a sibling `tests.rs` centrally, so a crate's own convention —
//! `#[cfg(test)] mod tests;` in a file of its own — is the sanctioned way to
//! bring a module back under the limit without moving a line of production
//! code. `lib.rs` was 5062 lines, of which 1640 were these two modules.

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
    // the CLOCK, because the cursor roams now. `drive_the_cursor`
    // integrates a held stick against `Time`, so a hand-built app without
    // one fails validation on a resource rather than on anything this test
    // is about. A real composition always has `TimePlugin`; a fixture has
    // exactly what it says.
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
    // the DEFAULT roster (this demo's own fighters), not an assembled one:
    // there is no catalog in this fixture and none is needed. What is under
    // test is the arbitration, and the roster only has to be non-empty so
    // the layout has a grid to put a cursor on.
    app.init_resource::<select::SmashRoster>();
    app.add_systems(
        Update,
        (
            resolve_active_input_context,
            select_screen::drive_the_cursor.run_if(the_select_screen_owns_its_input),
            // the real consumer, in the real order. Asserting on
            // `LeaveRequested` alone would prove a flag was set and nothing
            // about whether anybody acts on it — the flag is this test's
            // subject only if the system that spends it is here too.
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
    // The pause menu's claim, at its real priority. this test names it
    // only because it is standing in for the host; neither the screen nor
    // the pause menu names the other.
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

    // THE CURSOR IS ON SLOT 1's BUTTON. No window and no `UiPlugin` here,
    // and it does not matter: the screen's rectangles come from
    // `select_screen::layout`, which lays out against `HEADLESS_VIEWPORT`
    // when there is no window. That is what makes this test press a real
    // button rather than reach into the value — and the control below is
    // what proves the press lands at all.
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

/// What one seat is holding down this frame, replacing whatever
/// [`app_with`] armed. Every BACK test below presses through the SAME
/// `SeatMenuFrames` channel a pad, a keyboard and the touch overlay's own
/// "Back" button all reduce to — there is no second road to fake.
fn seat_presses(app: &mut App, seat: u8, frame: MenuControlFrame) {
    app.world_mut()
        .resource_mut::<SeatMenuFrames>()
        .set(seat, frame);
}

/// Which shell commands this frame produced. Drains, so a caller reads a
/// FRAME rather than everything since boot.
fn commands_sent(app: &mut App) -> Vec<ambition_platformer2d::game_shell::ShellCommand> {
    app.world_mut()
        .resource_mut::<Messages<ambition_platformer2d::game_shell::ShellCommand>>()
        .drain()
        .collect()
}

/// Did this frame ask the shell to go home — the pause menu's own
/// "Quit to Title" command, and the one this screen now writes?
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

/// A LOBBY WITH A CPU BETWEEN TWO PEOPLE ROUTES THE SECOND ONE HOME.
///
/// The roster is SPARSE and it is not in input-seat order. Explicit join
/// ownership can therefore produce:
///
/// ```text
/// card 0   Controller { device: 0 }
/// card 1   Cpu
/// card 2   Controller { device: 1 }
/// ```
///
/// The second person reports on input seat ONE — the numbering their pad, their menu frame and
/// their cursor all share — and the screen indexed the CARDS with it too.
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
    // the other half, and the half that was actually broken. Landing on
    // card 2 is only right if it did NOT also land on the machine's.
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

/// ONE TOKEN HAS AT MOST ONE CARRIER.
///
/// a human may pick up a CPU's token — one person setting up two machine
/// opponents is this lobby's most ordinary use. but EVERY human could,
/// with nothing arbitrating, so two cursors carried the same piece and
/// `carrier_of` returned whichever the array reached first. Two people then
/// dragged one token to two different fighters and the last writer won.
///
/// the incumbent keeps it, resolved in seat order — deterministic, so
/// this cannot pass on one run and fail on the next.
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

/// ESCAPE OPENS THE PAUSE MENU AND DOES NOT ALSO QUIT.
///
/// one key, two semantic actions: `presets.rs` binds Escape to BOTH
/// `Start` and `MenuBack`, deliberately and with `rebind.rs` testing that it
/// does. The shell's pause menu opens on `start` and this screen's chain
/// runs in the SAME `InputSet::Consume` with no order between them, so a
/// bare `back` reading would have Escape open the menu AND quit the lobby
/// out from under it — the double-fire being deterministic in whichever
/// direction the schedule happened to resolve.
///
/// The explicit Back-control test above proves the screen still has a way
/// out; this guard is specifically about Escape's combined Start+Back edge.
#[test]
fn escape_does_not_quit_the_lobby_out_from_under_the_pause_menu_it_opens() {
    let mut app = app_with(false);
    seat_presses(
        &mut app,
        0,
        // What Escape actually produces: both edges, one frame.
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

/// A COMPOSITION WHOSE HOME IS THIS SCREEN DRAWS NO WAY OUT.
///
/// The standalone smash demo names `SMASH_SELECT_ROUTE` as its own home
/// route, so `QuitToHome` there re-enters the route it is already on. An
/// exit that churns the router and changes nothing on screen is a dead
/// button, and this is the term that refuses it.
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

/// The screen drives when it owns its seat. The control: without this,
/// the test below passes on a screen that never worked.
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

/// One press moves ONE thing.
///
/// With the universal pause menu open OVER this screen the arrows drove
/// BOTH — the menu's cursor and the CPU count. They read different channels
/// (`MenuControlFrame` and `SeatMenuFrames`), so neither could consume the
/// other's edge, and this demo cannot name `ShellPauseMenu` at all:
/// `basic_shell_presentation` is not in `all_capabilities`, which is the
/// oracle rule working as intended.
///
/// So the arbitration is the CLAIM system. A capturing claim above `SELECT`
/// closes this screen's context, and the screen asks whether it still owns
/// the seat. Neither side names the other.
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
/// when it leaves.
///
/// the retraction is the half that bites. A cue outlives its surface if
/// nothing withdraws it, and the next screen then inherits a prompt telling
/// the player to choose a fighter on a screen with no fighters.
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

    // Leave the route — the only change.
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
