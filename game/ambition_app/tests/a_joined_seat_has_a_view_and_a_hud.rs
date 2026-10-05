//! A seat that joined has a HUD and a view (Q153).
//!
//! The Q153 survey said "views and the HUD already follow a second body". The
//! witnesses of that sentence used a second body that is a placement of its
//! room (`two_players_two_live_rooms`). The body of the join road is not a
//! placement: it is session-scoped, and it is built while the game runs. These
//! arms ask the same questions of that body, in the shipped composition:
//!
//! - In one room, the shared view has a HUD row for the joined seat.
//! - When the primary leaves the room, a view opens that follows the joined
//!   seat, shows its meters, and frames its body.
//! - The facts that are one per session stay the primary's. That is the
//!   measured edge of the sentence, and the last arm holds it.
//!
//! These are the observation facts that presentation reads. No arm here draws
//! a frame.

#![cfg(feature = "rl_sim")]

use ambition_app::rl_sim::{AgentAction, AmbitionSim as _, Platformer2dSimHarness, TimestepMode};
use ambition_platformer2d::characters::actor::BodyWallet;
use ambition_platformer2d::engine_core::{ControlFrame, Vec2};
use ambition_platformer2d::sim_view::affordances::{InteractVariant, NearestInteractable};
use ambition_platformer2d::sim_view::{
    LocalView, LocalViewId, PlayerHudFacts, SharedViewHudFacts, ViewHudFacts, ViewHudSeat,
};
use bevy::prelude::*;

use crate::a_second_seat_joins_the_session::{bodies_of_the_seat, die, join, jump, place_of, room_of, the_primary};
use crate::common::{a_save_that_has_seen_the_hub_intro, fixed_60hz_room_options, walk_through_the_door_to};
use crate::two_players_two_live_rooms::{live_rooms, the_views, view_frame, ROOM};

const SEAT: u8 = 1;
const HUB: &str = "central_hub_complex";
/// The purses that tell the two bodies apart on a HUD.
const PRIMARY_PURSE: i32 = 3;
const SEAT_PURSE: i32 = 11;

/// One HUD: (the view, it shows a body, the purse it shows, the seat it is
/// for, the other seats on the view and their purses).
type Hud = (u8, bool, i32, Option<u8>, Vec<(u8, i32)>);

fn the_huds(sim: &mut Platformer2dSimHarness) -> Vec<Hud> {
    let world = sim.world_mut();
    let mut huds: Vec<Hud> = world
        .query_filtered::<(&LocalViewId, &ViewHudFacts, &ViewHudSeat, &SharedViewHudFacts), With<LocalView>>()
        .iter(world)
        .map(|(id, own, seat, shared)| {
            let others = shared.0.iter().map(|(slot, facts)| (slot.0, facts.balance)).collect();
            (id.0, own.0.present, own.0.balance, seat.0.map(|seat| seat.0), others)
        })
        .collect();
    huds.sort();
    huds
}

fn give_purse(sim: &mut Platformer2dSimHarness, body: Entity, balance: i32) {
    sim.world_mut().entity_mut(body).insert(BodyWallet { balance });
}

/// Seat 1 joins, and its press is cleared: a seat that is driven by hand keeps
/// its last frame, so the body would jump on each tick after the join.
fn seat_one_joins(sim: &mut Platformer2dSimHarness) -> Entity {
    let (body, _) = join(sim, 4).expect("seat 1 pressed Jump and no body was built");
    sim.drive_seat(SEAT, ControlFrame::default());
    body
}

/// Move `body` to `at`, at rest.
fn put_body_at(sim: &mut Platformer2dSimHarness, body: Entity, at: Vec2) {
    let world = sim.world_mut();
    let mut bodies = world.query::<(
        ambition_platformer2d::engine_core::BodyClusterQueryData,
        &mut ambition_platformer2d::actor::MotionModel,
    )>();
    let (mut clusters, mut model) = bodies.get_mut(world, body).expect("the body has its movement clusters");
    let mut clusters = clusters.as_clusters_mut();
    ambition_platformer2d::engine_core::movement::transit_body(
        &mut model,
        &mut clusters,
        at,
        ambition_platformer2d::engine_core::movement::TransitVelocity::Zero,
    );
}

/// ONE ROOM. The joined seat is on the primary's view, with a HUD row of its
/// own. Control: before the join the view shows the primary and no other
/// seat. The split does not open: the two bodies are in one live room.
#[test]
fn a_joined_seat_has_its_own_hud_on_the_shared_view() {
    let mut sim =
        Platformer2dSimHarness::new_with_timestep(TimestepMode::fixed_60hz()).expect("sandbox sim builds");
    sim.step_n(AgentAction::default(), 15);
    let primary = the_primary(&mut sim);
    give_purse(&mut sim, primary, PRIMARY_PURSE);
    sim.step_n(AgentAction::default(), 2);
    assert_eq!(
        (the_views(&mut sim), the_huds(&mut sim)),
        (vec![(0, None, false)], vec![(0, true, PRIMARY_PURSE, Some(0), vec![])]),
        "control, before the join: (the views, the HUDs)"
    );

    let body = seat_one_joins(&mut sim);
    give_purse(&mut sim, body, SEAT_PURSE);
    sim.step_n(AgentAction::default(), 2);
    assert_eq!(room_of(&sim, body), room_of(&sim, primary), "precondition: seat 1 joined in the primary's room");
    assert_eq!(
        (the_views(&mut sim), the_huds(&mut sim)),
        (
            vec![(0, None, false)],
            vec![(0, true, PRIMARY_PURSE, Some(0), vec![(SEAT, SEAT_PURSE)])]
        ),
        "(the views, the HUDs) with the joined seat beside the primary"
    );
}

/// TWO ROOMS. The primary goes through the door and the joined seat stays. A
/// view opens for seat 1: it shows seat 1's meters and it frames seat 1's
/// body. Seat 1 then dies and comes back beside the primary, and the view
/// closes. Control: before the crossing there is one view.
#[test]
fn a_joined_seat_gets_its_own_view_when_the_primary_leaves_its_room() {
    let mut sim = Platformer2dSimHarness::new_with_options(
        fixed_60hz_room_options(ROOM).with_save(a_save_that_has_seen_the_hub_intro()),
    )
    .unwrap_or_else(|error| panic!("{ROOM} boots: {error:?}"));
    sim.step_n(AgentAction::default(), 10);
    seat_one_joins(&mut sim);
    sim.step_n(AgentAction::default(), 2);
    assert_eq!(the_views(&mut sim), vec![(0, None, false)], "control: one room, one view");

    assert_eq!(walk_through_the_door_to(&mut sim, HUB), HUB);
    sim.step_n(AgentAction::default(), 30);
    let primary = the_primary(&mut sim);
    let body = bodies_of_the_seat(&mut sim).first().copied().expect("seat 1 has a body after the crossing");
    give_purse(&mut sim, primary, PRIMARY_PURSE);
    give_purse(&mut sim, body, SEAT_PURSE);
    sim.step_n(AgentAction::default(), 2);
    assert_eq!(live_rooms(&mut sim).len(), 2, "precondition: the room seat 1 is in did not stay live");
    assert_eq!(
        (the_views(&mut sim), the_huds(&mut sim)),
        (
            vec![(0, None, false), (1, Some(SEAT), true)],
            vec![
                (0, true, PRIMARY_PURSE, Some(0), vec![]),
                (1, true, SEAT_PURSE, Some(SEAT), vec![])
            ]
        ),
        "(the views, the HUDs) with the primary in the hub and the joined seat in {ROOM}"
    );

    // The view of seat 1 frames seat 1's body: its follow point moves as the
    // body moves.
    let at = |sim: &Platformer2dSimHarness| {
        let (x, y) = place_of(sim, body);
        Vec2::new(x as f32, y as f32)
    };
    let (from, frame_from) = (at(&sim), view_frame(&mut sim, LocalViewId(1)));
    for _ in 0..30 {
        sim.drive_seat(
            SEAT,
            ControlFrame {
                axis_x: -1.0,
                ..ControlFrame::default()
            },
        );
        sim.step(AgentAction::default());
    }
    sim.drive_seat(SEAT, ControlFrame::default());
    let (moved, frame_to) = (at(&sim) - from, view_frame(&mut sim, LocalViewId(1)));
    assert!(moved.x.abs() > 20.0, "precondition: seat 1 did not run ({moved:?})");
    let lag = match (frame_from, frame_to) {
        (Some((from, _)), Some((to, _))) => Some(((to - from) - moved).length()),
        _ => None,
    };
    assert!(
        lag.is_some_and(|lag| lag < 4.0),
        "the view of seat 1 did not follow its body: the body moved {moved:?}, the lag is {lag:?}"
    );

    die(&mut sim, body);
    sim.step_n(AgentAction::default(), 2);
    assert_eq!(room_of(&sim, body), room_of(&sim, primary), "precondition: seat 1 came back in the primary's room");
    let huds = the_huds(&mut sim);
    assert_eq!(
        (
            the_views(&mut sim),
            huds.iter().map(|(view, .., others)| (*view, others.iter().map(|(seat, _)| *seat).collect())).collect::<Vec<(u8, Vec<u8>)>>()
        ),
        (vec![(0, None, false)], vec![(0, vec![SEAT])]),
        "(the views, the other seats on each) after seat 1 came back beside the primary"
    );
}

/// WHAT STAYS THE PRIMARY'S. Two facts are one per session, and a joined seat
/// does not change them:
///
/// - `PlayerHudFacts`, which the declared readouts read, shows the primary's
///   purse.
/// - The first field of `NearestInteractable`, the one prompt on the screen,
///   is the primary's answer. Seat 1 stands on a switch and the prompt says
///   nothing. Its own answer is in the map by body, and no HUD reads it.
///
/// Control: with the primary on the same switch, the prompt names it.
#[test]
fn the_facts_that_are_one_per_session_stay_the_primarys_after_a_join() {
    let mut sim = Platformer2dSimHarness::new_with_options(
        fixed_60hz_room_options(ROOM).with_save(a_save_that_has_seen_the_hub_intro()),
    )
    .unwrap_or_else(|error| panic!("{ROOM} boots: {error:?}"));
    sim.step_n(AgentAction::default(), 10);
    let primary = the_primary(&mut sim);
    let body = seat_one_joins(&mut sim);
    give_purse(&mut sim, primary, PRIMARY_PURSE);
    give_purse(&mut sim, body, SEAT_PURSE);

    // A switch that the primary does not stand on.
    let home = place_of(&sim, primary);
    let switch = {
        let world = sim.world_mut();
        let mut switches = world.query_filtered::<
            &ambition_platformer2d::engine_core::CenteredAabb,
            With<ambition_platformer2d::encounter::switches::SwitchFeature>,
        >();
        let mut centers: Vec<Vec2> = switches.iter(world).map(|aabb| aabb.center).collect();
        centers.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
        centers
            .into_iter()
            .find(|center| (center.x - home.0 as f32).abs() > 64.0)
            .unwrap_or_else(|| panic!("{ROOM} has no switch away from the primary at {home:?}"))
    };
    let prompts = |sim: &Platformer2dSimHarness| {
        let nearest = sim.world().resource::<NearestInteractable>();
        (nearest.0.clone(), nearest.for_body(primary), nearest.for_body(body))
    };

    put_body_at(&mut sim, body, switch);
    sim.step_n(AgentAction::default(), 2);
    assert_eq!(
        (prompts(&sim), sim.world().resource::<PlayerHudFacts>().balance),
        (
            (InteractVariant::None, InteractVariant::None, InteractVariant::Activate),
            PRIMARY_PURSE
        ),
        "((the screen's prompt, the primary's answer, seat 1's answer), the purse of the session's readout) \
         with seat 1 on a switch"
    );

    put_body_at(&mut sim, body, Vec2::new(home.0 as f32, home.1 as f32));
    put_body_at(&mut sim, primary, switch);
    sim.step_n(AgentAction::default(), 2);
    assert_eq!(
        prompts(&sim),
        (InteractVariant::Activate, InteractVariant::Activate, InteractVariant::None),
        "control, (the screen's prompt, the primary's answer, seat 1's answer) with the primary on the switch"
    );
}

/// When the second pad connects, for the rendered arm.
#[derive(Clone, Copy, Debug)]
enum SecondPad {
    BeforeTheSession,
    AfterTheSessionStarted,
    Never,
}

/// The rendered host (the visible composition with no window) starts an
/// Ambition session, and seat 1 presses Jump for 120 frames or until it has a
/// body. Returns (the seat of each drawn HUD, the bodies of seat 1) before
/// the presses and after them.
#[allow(clippy::type_complexity)]
fn drawn_huds_around_a_press_of_seat_one(second_pad: SecondPad) -> [(Vec<Option<u8>>, usize); 2] {
    use ambition_platformer2d::characters::control::{DrivingParticipant, PlayerSlot};
    use ambition_platformer2d::game_shell::ShellCommand;
    use ambition_platformer2d::render::hud::{HudOfSeat, PlayerHudRoot};
    use crate::shell_host_rendered::{rendered_app, settle};

    fn measure(app: &mut App) -> (Vec<Option<u8>>, usize) {
        let world = app.world_mut();
        let mut seats: Vec<Option<u8>> = world
            .query_filtered::<Option<&HudOfSeat>, With<PlayerHudRoot>>()
            .iter(world)
            .map(|seat| seat.map(|seat| seat.0 .0))
            .collect();
        seats.sort();
        let bodies = world
            .query::<&DrivingParticipant>()
            .iter(world)
            .filter(|driver| driver.0 == PlayerSlot(SEAT))
            .count();
        (seats, bodies)
    }
    fn connect_a_pad(app: &mut App) {
        app.world_mut().spawn(bevy::input::gamepad::Gamepad::default());
        app.update();
    }

    let mut app = rendered_app();
    settle(&mut app);
    connect_a_pad(&mut app);
    if matches!(second_pad, SecondPad::BeforeTheSession) {
        connect_a_pad(&mut app);
    }
    app.world_mut().write_message(ShellCommand::GoTo(
        ambition_content::provider::AMBITION_GAMEPLAY_ROUTE.into(),
    ));
    settle(&mut app);
    if matches!(second_pad, SecondPad::AfterTheSessionStarted) {
        connect_a_pad(&mut app);
    }
    for _ in 0..30 {
        app.update();
    }
    let before = measure(&mut app);
    for _ in 0..120 {
        if measure(&mut app).1 > 0 {
            break;
        }
        ambition_platformer2d::rollback::drive_slot_frame(app.world_mut(), PlayerSlot(SEAT), jump());
        app.update();
    }
    for _ in 0..6 {
        ambition_platformer2d::rollback::drive_slot_frame(app.world_mut(), PlayerSlot(SEAT), ControlFrame::default());
        app.update();
    }
    [before, measure(&mut app)]
}

/// THE DRAWN HUD. Two pads are connected before the session starts, so the
/// session has two seats. Seat 1 presses Jump and joins, and the screen has a
/// second HUD, the one of seat 1. Controls: before the press there is one
/// HUD; with one pad the same presses build no body and no HUD; and a second
/// pad that connects after the session started has no seat in it, because a
/// session is not resized.
#[test]
fn a_joined_seat_has_a_drawn_hud_in_the_rendered_host() {
    let alone = (vec![None], 0);
    assert_eq!(
        drawn_huds_around_a_press_of_seat_one(SecondPad::BeforeTheSession),
        [alone.clone(), (vec![None, Some(SEAT)], 1)],
        "[before, after] (the seat of each drawn HUD, the bodies of seat 1), two pads from the start"
    );
    assert_eq!(
        drawn_huds_around_a_press_of_seat_one(SecondPad::Never),
        [alone.clone(), alone.clone()],
        "control, one pad: [before, after] (the seat of each drawn HUD, the bodies of seat 1)"
    );
    assert_eq!(
        drawn_huds_around_a_press_of_seat_one(SecondPad::AfterTheSessionStarted),
        [alone.clone(), alone],
        "a pad that connected after the session started: [before, after] (the seat of each drawn HUD, the bodies of seat 1)"
    );
}
