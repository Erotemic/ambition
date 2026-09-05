//! The select screen has to be drivable by a controller, not only by a test.
//!
//! `SmashSelect` shipped fully unit-tested and completely inert: every state
//! transition was covered, and nothing in the app ever WROTE to it, so the
//! battle could not start from the screen at all. Every one of those unit tests
//! drove the resource directly, which is exactly why they were all green over a
//! screen nobody could use.
//!
//! The rectangles come from `select_screen::layout`, a pure function of the viewport, so a headless
//! app clicks exactly where a windowed one draws.

use ambition_demo_smash::select::{
    SlotOccupant, SlotPick, SmashRoster, SmashSelect, MAX_SMASH_SEATS,
};
use ambition_demo_smash::select_screen::cursor::{HitRect, SelectCursors};
use ambition_demo_smash::select_screen::layout::SelectLayout;
use ambition_demo_smash::select_screen::{CardName, CursorNode, RoleButtonLabel, SlotToken};
use ambition_demo_smash_app::build_demo_app;
use ambition_platformer2d::input::{MenuControlFrame, SeatMenuFrames};
use bevy::prelude::*;

/// Plug in `count` controllers. The screen offers exactly as many seats as there
/// are pads — one pad is one seat, which is the right answer on a couch and the
/// reason a test has to say how many people are in the room.
fn plug_in(app: &mut App, count: usize) {
    // SPAWN PADS, do not insert the order. `track_local_device_order` rebuilds
    // `LocalDeviceOrder` from live `Gamepad` entities every frame, so a
    // hand-inserted order is clobbered on the next update — and only when the
    // `input` feature is on, which is how this passed by default and failed
    // under `--features input,visible`. The resource is derived; the pads are
    // the fact.
    let pads: Vec<Entity> = (0..count)
        .map(|_| {
            app.world_mut()
                .spawn(bevy::input::gamepad::Gamepad::default())
                .id()
        })
        .collect();
    app.update();
    // …and the tracker itself is behind the `input` feature, so under default
    // features nothing derives the order and the pads sit there unread. Seed it
    // only when that happened: seeding unconditionally would put the test back
    // to fighting the tracker in the configuration where the tracker runs.
    let derived = app
        .world()
        .get_resource::<ambition_platformer2d::input::LocalDeviceOrder>()
        .map(|order| order.devices().len())
        .unwrap_or(0);
    if derived < count {
        app.world_mut()
            .insert_resource(ambition_platformer2d::input::LocalDeviceOrder::from_devices(pads));
    }
}

/// What this test is holding down, per seat, until it releases.
#[derive(Resource, Default, Clone)]
struct Held(Vec<(u8, MenuControlFrame)>);

/// Put the press into the port AFTER the host has rebuilt it.
///
/// writing `SeatMenuFrames` and calling `update()` is not enough under `--features input`:
/// `populate_seat_menu_frames` CLEARS that resource and refills it from the live participants
/// every frame.
///
/// So the injection is a SYSTEM, ordered exactly where a real device's press
/// lands: after the producer, before the screen reads.
fn install_press_port(app: &mut App) {
    app.init_resource::<Held>();
    app.add_systems(
        Update,
        (|held: Res<Held>, mut frames: ResMut<SeatMenuFrames>| {
            for (seat, frame) in &held.0 {
                frames.set(*seat, *frame);
            }
        })
        .in_set(ambition_platformer2d::input::InputSet::Consume)
        .before(ambition_demo_smash::SmashSelectSet),
    );
}

fn press(app: &mut App, seat: u8, frame: MenuControlFrame) {
    app.world_mut().resource_mut::<Held>().0 = vec![(seat, frame)];
    app.update();
    // Release, so a held button is not a new press next frame — the screen
    // reads edges and the writer produces them.
    app.world_mut().resource_mut::<Held>().0.clear();
    app.world_mut().resource_mut::<SeatMenuFrames>().clear();
    app.update();
}

fn confirm() -> MenuControlFrame {
    MenuControlFrame {
        select: true,
        ..Default::default()
    }
}

fn back() -> MenuControlFrame {
    MenuControlFrame {
        back: true,
        ..Default::default()
    }
}

fn arrow(direction: &str) -> MenuControlFrame {
    let mut frame = MenuControlFrame::default();
    match direction {
        "left" => frame.left = true,
        "right" => frame.right = true,
        "up" => frame.up = true,
        _ => frame.down = true,
    }
    frame
}

/// A portrait index this composition actually has.
///
/// the STANDALONE demo's grid is short, because the crossover roster is
/// Ambition's own cast and this app composes none of it — `SMASH_ROSTER` is
/// filtered to what the catalog carries, and here that is the one fighter this
/// demo declares. So these tests pick by "the nth fighter, or the last one",
/// which keeps them about the SCREEN rather than about how many characters ship.
/// Two slots landing on one fighter is a mirror match, which every platform
/// fighter allows and this one should.
fn nth_of(fighters: &SmashRoster, index: usize) -> usize {
    index.min(fighters.len().saturating_sub(1))
}

fn nth(app: &App, index: usize) -> usize {
    let count = app.world().resource::<SmashRoster>().len();
    assert!(count > 0, "an empty grid is a screen that cannot be worked");
    index.min(count - 1)
}

/// The screen's own geometry, which is what it draws AND what it hit-tests.
///
/// from the app's OWN roster, not a constant. The standalone demo offers the
/// four fighters it declares; a host offers its whole tagged cast, and a layout
/// built from the wrong count would put the cursor between two cells.
fn layout(app: &App) -> SelectLayout {
    SelectLayout::for_viewport(None, app.world().resource::<SmashRoster>().cell_count())
}

/// Where a participating slot's token is placed on the current page.
fn placed_token(app: &App, slot: usize) -> HitRect {
    let geometry = layout(app);
    ambition_demo_smash::select_screen::token_rect(
        &geometry,
        app.world().resource::<SmashSelect>(),
        app.world().resource::<SmashRoster>(),
        slot,
    )
    .unwrap_or_else(|| panic!("slot {slot} has no placed token on this page"))
}

/// Put ONE SEAT'S cursor somewhere. A mouse does exactly this; so does a pad,
/// one snap at a time, and `the_arrows_alone_can_work_the_whole_screen` covers
/// that path.
fn point_at(app: &mut App, seat: u8, rect: HitRect) {
    app.world_mut()
        .resource_mut::<SelectCursors>()
        .seat_mut(seat as usize)
        .expect("seat is bounded by the caller's seat count")
        .move_to(rect.center());
}

/// Point at something and press confirm from `seat`.
fn click(app: &mut App, seat: u8, rect: HitRect) {
    point_at(app, seat, rect);
    press(app, seat, confirm());
}

fn slot(app: &App, index: usize) -> ambition_demo_smash::select::SlotCard {
    app.world().resource::<SmashSelect>().slot(index)
}

/// Every string the cards are currently showing, in slot order.
fn card_text(app: &mut App) -> Vec<(String, String)> {
    let mut roles: Vec<(usize, String)> = app
        .world_mut()
        .query::<(&RoleButtonLabel, &Text)>()
        .iter(app.world())
        .map(|(label, text)| (label.0, text.0.clone()))
        .collect();
    roles.sort_by_key(|(slot, _)| *slot);
    let mut names: Vec<(usize, String)> = app
        .world_mut()
        .query::<(&CardName, &Text)>()
        .iter(app.world())
        .map(|(name, text)| (name.0, text.0.clone()))
        .collect();
    names.sort_by_key(|(slot, _)| *slot);
    roles
        .into_iter()
        .zip(names)
        .map(|((_, role), (_, name))| (role, name))
        .collect()
}

/// Two people take controllers, drag a fighter each, and click START.
///
/// The whole loop, through the only surface a player has.
/// A PAD THAT IS PLUGGED IN GETS A HAND BEFORE IT GETS A SEAT.
///
/// at all, so a game pad cannot join, unless player 1 lets them in."* The
/// cursor was drawn only for a seat that already PARTICIPATED, and the way in
/// is to press your own card's role button — so player two had nothing to press
/// it with and had to be admitted by player one.
///
/// the other half is asserted too: a seat with no device and nobody in it
/// still draws nothing, or a one-player lobby shows three hands nobody can move.
#[test]
fn a_plugged_in_pad_has_a_cursor_before_anybody_admits_it() {
    let mut app = build_demo_app();
    install_press_port(&mut app);
    plug_in(&mut app, 2);
    app.update();
    app.update();

    // the premise, asserted. If plugging a pad in also SEATED it, every
    // assertion below would pass on the old rule too and this test would be
    // checking nothing.
    assert_eq!(
        slot(&app, 1).occupant,
        SlotOccupant::Absent,
        "seat 1 joined by itself, so this test cannot tell the two rules apart"
    );

    let shown: Vec<(usize, bool)> = {
        let world = app.world_mut();
        let mut q = world.query::<(&CursorNode, &Visibility)>();
        let mut rows: Vec<(usize, bool)> = q
            .iter(world)
            .map(|(node, visibility)| (node.0, *visibility != Visibility::Hidden))
            .collect();
        rows.sort_by_key(|(seat, _)| *seat);
        rows
    };
    assert_eq!(
        shown.len(),
        MAX_SMASH_SEATS,
        "the screen did not draw one cursor per seat: {shown:?}"
    );
    assert!(
        shown[1].1,
        "seat 1 has a pad and no cursor, so its player cannot press their own \
         role button to join: {shown:?}"
    );
    assert!(
        !shown[3].1,
        "seat 3 has no device and nobody in it, and still drew a hand: {shown:?}"
    );
}

#[test]
fn two_players_take_controllers_pick_fighters_and_the_battle_starts() {
    let mut app = build_demo_app();
    install_press_port(&mut app);
    plug_in(&mut app, 2);
    app.update();
    let layout = layout(&app);

    click(&mut app, 0, layout.role_button(0));
    click(&mut app, 1, layout.role_button(1));
    assert_eq!(
        slot(&app, 0).occupant,
        SlotOccupant::Controller { device: 0 },
        "a click on the first card's button did not seat anybody"
    );
    assert_eq!(
        slot(&app, 1).occupant,
        SlotOccupant::Controller { device: 1 },
        "the second card took the first card's controller"
    );

    // Pick up each player's placed token, then place it on a portrait.
    let token = placed_token(&app, 0);
    click(&mut app, 0, token);
    let cell = layout.portrait(nth(&app, 0)).expect("an authored portrait");
    click(&mut app, 0, cell);
    let token = placed_token(&app, 1);
    click(&mut app, 1, token);
    let cell = layout.portrait(nth(&app, 1)).expect("an authored portrait");
    click(&mut app, 1, cell);
    assert_eq!(slot(&app, 0).pick, Some(SlotPick::Fighter(nth(&app, 0))));
    assert_eq!(slot(&app, 1).pick, Some(SlotPick::Fighter(nth(&app, 1))));

    // it must NOT have started yet. A screen that launches the instant
    // the last token lands is the one nobody can look at.
    assert!(
        app.world()
            .get_resource::<ambition_platformer2d::actor::MatchParticipantRoster>()
            .is_none(),
        "the match started before anybody asked it to"
    );

    click(&mut app, 0, layout.start_button());
    let roster = app
        .world()
        .get_resource::<ambition_platformer2d::actor::MatchParticipantRoster>()
        .expect("clicking START publishes the roster the screen decided")
        .clone();
    assert_eq!(roster.participants.len(), 2);
    let fighters = app.world().resource::<SmashRoster>().clone();
    assert_eq!(
        roster.participants[0].character,
        fighters
            .get(nth_of(&fighters, 0))
            .expect("a fighter")
            .into()
    );
    assert_eq!(
        roster.participants[1].character,
        fighters
            .get(nth_of(&fighters, 1))
            .expect("a fighter")
            .into()
    );
}

/// A PLAYER WHO NEVER TOUCHED THE GRID STILL STARTS THE MATCH — ON RANDOM.
#[test]
fn a_player_who_never_touched_the_grid_starts_on_random() {
    let mut app = build_demo_app();
    install_press_port(&mut app);
    plug_in(&mut app, 2);
    app.update();
    let layout = layout(&app);

    click(&mut app, 0, layout.role_button(0));
    click(&mut app, 1, layout.role_button(1));
    // Only slot 0 chooses. Slot 1 is left exactly as joining made it.
    let token = placed_token(&app, 0);
    click(&mut app, 0, token);
    let cell = layout.portrait(nth(&app, 1)).expect("an authored portrait");
    click(&mut app, 0, cell);
    assert_eq!(
        slot(&app, 1).pick,
        Some(SlotPick::Random),
        "the untouched slot is not on random, so this test is not about random"
    );

    click(&mut app, 0, layout.start_button());
    for _ in 0..8 {
        app.update();
    }
    let fighters = app.world().resource::<SmashRoster>().clone();
    let roster = app
        .world()
        .get_resource::<ambition_platformer2d::actor::MatchParticipantRoster>()
        .expect("a decided screen with a random seat did not start a match")
        .clone();
    assert_eq!(roster.participants.len(), 2);
    assert_eq!(
        roster.participants[0].character,
        fighters.get(nth(&app, 1)).expect("a fighter").into(),
        "the slot that CHOSE did not get what it chose"
    );
    assert!(
        fighters
            .0
            .iter()
            .any(|id| id.as_str() == roster.participants[1].character.as_str()),
        "the random seat drew `{}`, which is not on the grid",
        roster.participants[1].character
    );
}

/// A token has only two presentation states: placed on its selection or carried.
/// Clicking empty space while carrying does not create an arbitrary resting
/// coordinate, and B while carrying is a no-op.
#[test]
fn a_carried_token_stays_in_hand_until_it_reaches_a_selection() {
    let mut app = build_demo_app();
    install_press_port(&mut app);
    plug_in(&mut app, 1);
    app.update();
    let layout = layout(&app);

    click(&mut app, 0, layout.role_button(0));
    let cell = layout.portrait(nth(&app, 1)).expect("an authored portrait");
    click(&mut app, 0, cell);
    assert_eq!(slot(&app, 0).pick, Some(SlotPick::Fighter(nth(&app, 1))));

    let token = placed_token(&app, 0);
    click(&mut app, 0, token);
    click(&mut app, 0, layout.title());
    assert_eq!(
        slot(&app, 0).pick,
        Some(SlotPick::Fighter(nth(&app, 1))),
        "empty-space interaction changed the token owner's selection"
    );
    assert_eq!(
        app.world()
            .resource::<SelectCursors>()
            .seat(0)
            .expect("seat 0")
            .carrying,
        Some(0),
        "empty-space interaction invented a resting token state"
    );

    press(&mut app, 0, back());
    assert_eq!(
        app.world()
            .resource::<SelectCursors>()
            .seat(0)
            .expect("seat 0")
            .carrying,
        Some(0),
        "B dropped a token that was already in hand"
    );
}

/// A screen that works and cannot be seen is the same bug one layer up.
///
/// Asserting the cards EXIST would pass over four empty boxes, so this asserts
/// what they SAY — and says it by reading the same text the player reads.
#[test]
fn the_cards_say_what_each_slot_has_decided() {
    let mut app = build_demo_app();
    install_press_port(&mut app);
    plug_in(&mut app, 2);
    app.update();
    let layout = layout(&app);

    let fresh = card_text(&mut app);
    assert_eq!(fresh.len(), MAX_SMASH_SEATS, "four cards, one per slot");
    for (role, name) in &fresh {
        assert_eq!(role, "NOT PLAYING");
        assert!(
            name.contains("no fighter"),
            "an empty card claimed a fighter"
        );
    }

    click(&mut app, 0, layout.role_button(0));
    let token = placed_token(&app, 0);
    click(&mut app, 0, token);
    let cell = layout.portrait(nth(&app, 0)).expect("an authored portrait");
    click(&mut app, 0, cell);
    click(&mut app, 1, layout.role_button(1));
    click(&mut app, 1, layout.role_button(1)); // → CPU

    let decided = card_text(&mut app);
    // gives more info for debugging."*) This read `CONTROLLER 1` — the slot's own numbering
    // said back to it — which told a person nothing about which of their two hands was seated
    // where. `plug_in(2)` gives this fixture pads and a keyboard, and the keyboard is source
    // zero under the couch policy.
    assert_eq!(decided[0].0, "KEYBOARD");
    assert_eq!(
        decided[0].1, "George Booul",
        "the card shows `{}` rather than the fighter's display name — the \
         catalog lookup the portraits also depend on did not resolve",
        decided[0].1
    );
    assert_eq!(decided[1].0, "CPU");
    assert_eq!(decided[3].0, "NOT PLAYING");
}

/// A participating slot's token is ON SCREEN, because the token is the only
/// thing tying a card to the grid and an invisible one is an unplayable screen.
#[test]
fn a_participating_slot_puts_a_visible_token_on_the_grid() {
    let mut app = build_demo_app();
    install_press_port(&mut app);
    plug_in(&mut app, 1);
    app.update();
    let layout = layout(&app);

    let visible = |app: &mut App| -> Vec<(usize, bool)> {
        let mut rows: Vec<(usize, bool)> = app
            .world_mut()
            .query::<(&SlotToken, &Visibility)>()
            .iter(app.world())
            .map(|(token, visibility)| (token.0, *visibility != Visibility::Hidden))
            .collect();
        rows.sort_by_key(|(slot, _)| *slot);
        rows
    };
    assert!(
        visible(&mut app).iter().all(|(_, shown)| !shown),
        "a screen nobody has joined is already showing tokens"
    );

    click(&mut app, 0, layout.role_button(0));
    app.update();
    let rows = visible(&mut app);
    assert!(rows[0].1, "the slot that joined has no token to drag");
    assert!(
        rows[1..].iter().all(|(_, shown)| !shown),
        "slots nobody is at grew tokens"
    );
}

/// THE STAGE BUTTON IS REACHABLE AND IT CHANGES THE MATCH.
///
/// ⛔ Two halves, and the second is the one that would rot silently. A button
/// that cycles a resource nothing reads looks completely correct on screen —
/// the label even changes — while every match still loads the same stage. So
/// this presses the real button through the real screen AND then asserts the
/// prepared world followed.
#[test]
fn the_stage_button_cycles_the_stage_the_match_will_prepare() {
    use ambition_demo_smash::SmashStageChoice;

    let mut app = build_demo_app();
    install_press_port(&mut app);
    plug_in(&mut app, 0);
    app.update();
    let layout = layout(&app);

    assert_eq!(
        *app.world().resource::<SmashStageChoice>(),
        SmashStageChoice::Flat,
        "the default is the stage every recorded measurement was taken on"
    );

    click(&mut app, 0, layout.stage_button());
    assert_eq!(
        *app.world().resource::<SmashStageChoice>(),
        SmashStageChoice::Platforms,
        "pressing the stage button did not change the stage"
    );

    // The label the player reads followed the value.
    assert!(
        stage_label_text(&mut app).contains("Platforms"),
        "the button still reads {:?} after the stage changed — a control that \
         lies about what it sets",
        stage_label_text(&mut app)
    );

    // And it cycles rather than latching.
    click(&mut app, 0, layout.stage_button());
    assert_eq!(
        *app.world().resource::<SmashStageChoice>(),
        SmashStageChoice::Flat
    );
}

/// What the stage button is currently showing.
fn stage_label_text(app: &mut App) -> String {
    use ambition_demo_smash::select_screen::StageButtonLabel;
    app.world_mut()
        .query_filtered::<&Text, With<StageButtonLabel>>()
        .iter(app.world())
        .next()
        .map(|text| text.0.clone())
        .unwrap_or_default()
}

/// One person, one keyboard, a fight.
///
/// The screen offered one seat per PAD with a floor of one, every decided seat
/// was a human, and a match needed two — so alone, at a keyboard, there was no
/// sequence of presses that started anything. Every unit test passed: they all
/// drove two seats.
#[test]
fn a_player_alone_can_add_a_cpu_and_start_the_match() {
    let mut app = build_demo_app();
    install_press_port(&mut app);
    plug_in(&mut app, 0);
    app.update();
    let layout = layout(&app);

    click(&mut app, 0, layout.role_button(0));
    assert_eq!(
        slot(&app, 0).occupant,
        SlotOccupant::Controller { device: 0 },
        "the only source in the room did not reach the first card"
    );
    // the SECOND card has no source left, so its button skips the controller
    // rung entirely — one press, not two.
    click(&mut app, 0, layout.role_button(1));
    assert_eq!(
        slot(&app, 1).occupant,
        SlotOccupant::Cpu,
        "a lone player's second card offered a controller nobody is holding"
    );

    for (slot_index, character) in [(0usize, 0usize), (1, 1)] {
        let token = placed_token(&app, slot_index);
        click(&mut app, 0, token);
        let cell = layout
            .portrait(nth(&app, character))
            .expect("an authored portrait");
        click(&mut app, 0, cell);
    }
    click(&mut app, 0, layout.start_button());

    let roster = app
        .world()
        .get_resource::<ambition_platformer2d::actor::MatchParticipantRoster>()
        .expect("one player and one CPU is a match")
        .clone();
    assert_eq!(roster.participants.len(), 2);
    assert!(
        roster.participants[1].controller.brain_profile().is_some(),
        "the second seat is a CPU on the screen and a human in the roster"
    );
}

/// The arrows alone can work the whole screen.
///
/// the piece with no precedent in this repo, and the one a mouse would hide.
/// Every stop is on something clickable, so a pad reaches the cards, the grid,
/// the tokens and START without a pointer — and if snapping ever loses a
/// direction, a whole third of the screen becomes unreachable with nothing else
/// to notice.
#[test]
fn the_arrows_alone_can_work_the_whole_screen() {
    let mut app = build_demo_app();
    install_press_port(&mut app);
    plug_in(&mut app, 1);
    app.update();

    // Down from the grid must reach the cards, then their buttons.
    for _ in 0..12 {
        press(&mut app, 0, arrow("down"));
    }
    let position = app
        .world()
        .resource::<SelectCursors>()
        .seat(0)
        .expect("seat 0")
        .position;
    let layout = layout(&app);
    assert!(
        (0..MAX_SMASH_SEATS).any(|slot| layout.role_button(slot).contains(position)),
        "pressing down twelve times never reached a card's button — it stopped \
         at {position:?}"
    );

    // And it can act there.
    press(&mut app, 0, confirm());
    assert!(
        (0..MAX_SMASH_SEATS).any(|slot| slot_participates(&app, slot)),
        "confirm on a card's button did nothing"
    );

    // Back up into the grid.
    for _ in 0..12 {
        press(&mut app, 0, arrow("up"));
    }
    let position = app
        .world()
        .resource::<SelectCursors>()
        .seat(0)
        .expect("seat 0")
        .position;
    assert!(
        (0..app.world().resource::<SmashRoster>().len()).any(|index| layout
            .portrait(index)
            .is_some_and(|cell| cell.contains(position))),
        "pressing up twelve times never got back to the portrait grid — it \
         stopped at {position:?}"
    );
}

fn slot_participates(app: &App, index: usize) -> bool {
    app.world()
        .resource::<SmashSelect>()
        .slot(index)
        .occupant
        .participates()
}

/// The select screen's own score plays in the STANDALONE demo too.
///
/// The companion to `shell_host_rendered::a_providers_own_frontend_route_plays_the_score_written_for_it`,
/// and it is the half that already worked: before the select theme
/// played here — and only here — because frontend audio was one process-global
/// resource this app happened to own outright.
///
/// The subject is the selected AUTHORITY, not the declaration. Reading a profile
/// back out of the registry it was written into would pass under either design.
#[test]
fn the_select_screen_plays_its_own_score_in_the_standalone_demo() {
    use ambition_platformer2d::audio::selection::ActiveAudioSelection;

    let mut app = build_demo_app();
    for _ in 0..6 {
        app.update();
    }

    assert_eq!(
        app.world()
            .resource::<ambition_platformer2d::game_shell::ShellRouter>()
            .active
            .as_ref()
            .map(|active| active.route_id.as_str().to_owned()),
        Some(ambition_demo_smash::SMASH_SELECT_ROUTE.to_owned()),
        "the standalone demo boots onto the select screen",
    );
    assert_eq!(
        app.world()
            .resource::<ActiveAudioSelection>()
            .preferred_track(),
        Some(ambition_demo_smash::SMASH_SELECT_TRACK),
        "the standalone demo's select screen still selects its own score",
    );
    // deliberately NOT claiming this proves the ROUTE declaration answered.
    // In this app the route declaration and the composition's host default name
    // the same track, so the outcome is identical either way and the test cannot
    // tell them apart. Which one wins is pinned where it is actually decidable:
    // `composition::a_route_that_declares_its_own_sound_is_not_overruled_by_the_default`
    // asserts the precedence directly, and the Ambition host asserts the
    // consequence — a route whose default says something else entirely.
}

/// THE STOCKS BUTTON IS REACHABLE AND THE MATCH ACTUALLY USES WHAT IT SAYS.
///
/// ⛔⛔ THE SECOND HALF IS THE ONE THAT WOULD ROT SILENTLY, and it is the half
/// this file's stage-button test claims in prose and does not take: a button
/// that cycles a resource nothing reads looks completely correct on screen —
/// the label even changes — while every match is still played at three stocks.
/// So this presses the real button through the real screen, starts the real
/// match, and reads the count off the roster the screen PUBLISHED.
///
/// ⭐ That is reachable here only because `apply_smash_match_rules` takes the
/// count as an argument. While it read a constant, no test at any level could
/// have told a wired button from an unwired one.
#[test]
fn the_stocks_button_sets_the_count_the_published_match_is_played_at() {
    use ambition_demo_smash::SmashStockChoice;

    let mut app = build_demo_app();
    install_press_port(&mut app);
    plug_in(&mut app, 2);
    app.update();
    let layout = layout(&app);

    assert_eq!(
        *app.world().resource::<SmashStockChoice>(),
        SmashStockChoice::Three,
        "the default is the count every recorded ladder number was measured at"
    );

    // One stock, and the label the player reads follows the value.
    click(&mut app, 0, layout.stocks_button());
    assert_eq!(
        *app.world().resource::<SmashStockChoice>(),
        SmashStockChoice::Five,
        "pressing the stocks button did not change the count"
    );
    click(&mut app, 0, layout.stocks_button());
    assert_eq!(
        *app.world().resource::<SmashStockChoice>(),
        SmashStockChoice::One,
        "the stocks button latched instead of cycling"
    );
    assert!(
        stocks_label_text(&mut app).contains('1'),
        "the button reads {:?} after the count changed — a control that lies \
         about what it sets",
        stocks_label_text(&mut app)
    );

    // ⭐ ANY SEAT, like the stage cycle. Seat 1 moves it on, then back to one.
    click(&mut app, 1, layout.stocks_button());
    assert_eq!(
        *app.world().resource::<SmashStockChoice>(),
        SmashStockChoice::Three,
        "only seat zero could change the count — the player-one-centric shape"
    );
    click(&mut app, 1, layout.stocks_button());
    click(&mut app, 1, layout.stocks_button());
    assert_eq!(*app.world().resource::<SmashStockChoice>(), SmashStockChoice::One);

    // Seat the two players and start.
    click(&mut app, 0, layout.role_button(0));
    click(&mut app, 1, layout.role_button(1));
    let token = placed_token(&app, 0);
    click(&mut app, 0, token);
    let cell = layout.portrait(nth(&app, 0)).expect("an authored portrait");
    click(&mut app, 0, cell);
    let token = placed_token(&app, 1);
    click(&mut app, 1, token);
    let cell = layout.portrait(nth(&app, 1)).expect("an authored portrait");
    click(&mut app, 1, cell);
    click(&mut app, 0, layout.start_button());

    let roster = app
        .world()
        .get_resource::<ambition_platformer2d::actor::MatchParticipantRoster>()
        .expect("clicking START publishes the roster the screen decided")
        .clone();
    assert_eq!(
        roster.rules.stocks,
        Some(1),
        "the lobby said one stock and the published match says {:?} — the \
         button is not wired to the match it claims to change",
        roster.rules.stocks
    );
}

/// What the stocks button is currently showing.
fn stocks_label_text(app: &mut App) -> String {
    use ambition_demo_smash::select_screen::StocksButtonLabel;
    app.world_mut()
        .query_filtered::<&Text, With<StocksButtonLabel>>()
        .iter(app.world())
        .next()
        .map(|text| text.0.clone())
        .unwrap_or_default()
}
