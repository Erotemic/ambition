//! A LOADING SCREEN SHOWS ONLY WHEN REAL WORK BLOCKS THE DOOR.
//!
//! Jon, 2026-10-04: if the hardware is fast enough a loading screen never
//! appears; on a slower machine it tells the player the game has not frozen.
//! `docs/planning/engine/readiness-driven-room-transitions.md` makes that the
//! contract: the foreground is a consequence of a missed readiness deadline,
//! never a timer, a ration or a minimum display time.
//!
//! * A destination the neighbour prefetch has already prepared commits with
//!   the foreground never visible.
//! * The control: a destination nothing prepared (the hall of characters,
//!   crossed straight after boot) does show it, and only while the readiness
//!   gate is still unsettled. Without the control, an instrument that could
//!   never see a loading screen would pass the first arm.

use ambition_app::app::{build_visible_app, VisibleRenderMode};
use ambition_platformer2d::load_presentation::LoadPresentationModel;
use ambition_platformer2d::runtime::room_transition::RoomTransitionLoadState;

use crate::neighbor_prefetch_prepares_rooms::{alice, neighbours_of};

use ambition_platformer2d::characters::control::PlayerSlot;
type LiveBodyId = ambition_platformer2d::platformer::lifecycle::LiveBodyId;

/// One frame of 1/60 s on the app's clocks, with real time for the async
/// decoders to run in.
fn step(app: &mut bevy::prelude::App) {
    app.update();
    std::thread::sleep(std::time::Duration::from_millis(4));
}

/// A host whose frames each take `frame` on the app's clocks: 1/60 s models
/// fast hardware, 250 ms a machine slow enough that work outlasts the
/// loading screen's reveal grace.
fn gameplay_after_startup(frame: std::time::Duration) -> bevy::prelude::App {
    let mut app = build_visible_app(VisibleRenderMode::NoWindow, false);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(frame));
    for _ in 0..ambition_app::app::shared_host_startup_ticks() {
        app.update();
    }
    app
}

const FAST: std::time::Duration = std::time::Duration::from_nanos(16_666_667);
const SLOW: std::time::Duration = std::time::Duration::from_millis(250);

fn live_room(app: &bevy::prelude::App) -> String {
    let live = ambition_platformer2d::world::rooms::sole_live_room_definition(app.world()).expect("a live room");
    let rooms = ambition_platformer2d::platformer::lifecycle::session_world_component::<ambition_platformer2d::world::rooms::RoomSet>(app.world())
        .expect("the session keeps its room set");
    rooms.rooms.get(live.index()).expect("the live room is in the set").id.clone()
}

/// What a crossing looked like, frame by frame.
#[derive(Debug, Default)]
struct Crossing {
    prefetch_hit: bool,
    /// Frames the loading foreground was visible.
    foreground_frames: usize,
    /// Frames the foreground was visible while the readiness gate had already
    /// settled and the transition had committed: a screen kept up with no work
    /// behind it.
    foreground_frames_after_ready: usize,
    frames: usize,
}

/// Stage Alice's crossing to `target` and run it to its end.
fn cross(app: &mut bevy::prelude::App, target: &str) -> Crossing {
    use ambition_platformer2d::actors::session::lifecycle_commit::{LifecycleIntent, PendingLifecycleCommit, RoomTransitionIntent};
    use ambition_platformer2d::runtime::room_transition::RoomTransitionLoadPhase;

    let body = alice(app);
    let subject = LiveBodyId::of_entity(app.world(), body).expect("the body has a SimId");
    let intent = LifecycleIntent::Transition(RoomTransitionIntent {
        subject,
        target_room: target.to_owned(),
        arrival: ambition_platformer2d::engine_core::Vec2::new(200.0, 200.0),
        edge_exit: false,
        zone_sfx: None,
        participant: Some(PlayerSlot(0)),
    });
    assert!(
        app.world_mut().resource_mut::<PendingLifecycleCommit>().record(0, intent).admitted(),
        "the lifecycle slot refused the crossing to `{target}`"
    );
    let mut crossing = Crossing::default();
    let mut began = false;
    for _ in 0..1200 {
        step(app);
        crossing.frames += 1;
        let visible = app.world().resource::<LoadPresentationModel>().visible;
        let state = app.world().resource::<RoomTransitionLoadState>();
        match state.active.as_ref() {
            Some(active) => {
                began = true;
                crossing.prefetch_hit = active.prefetch_hit;
                if visible {
                    crossing.foreground_frames += 1;
                    if active.asset_readiness_complete && active.phase == RoomTransitionLoadPhase::Committed {
                        crossing.foreground_frames_after_ready += 1;
                    }
                }
            }
            None if began => {
                assert!(!visible, "the loading foreground outlived its transition");
                return crossing;
            }
            None => {}
        }
    }
    panic!("the crossing to `{target}` did not end in 1200 frames: {crossing:?}");
}

#[test]
fn a_prepared_neighbour_is_entered_with_no_loading_screen() {
    let mut app = gameplay_after_startup(FAST);
    let source = live_room(&app);
    let target = neighbours_of(&app, &source).into_iter().next().expect("the start room has a neighbour");
    // At its door: the prefetch prepares the nearest doors first.
    stand_at_the_door_to(&mut app, &target);
    // Let the neighbour prefetch prepare it, plan and art, in the open.
    for _ in 0..360 {
        step(&mut app);
    }
    let crossing = cross(&mut app, &target);
    // ⛔ Premise: the destination was prepared ahead, or this measures the
    // other arm.
    assert!(crossing.prefetch_hit, "`{target}` was not prefetched; nothing here was ready at the door: {crossing:?}");
    assert_eq!(
        crossing.foreground_frames, 0,
        "a loading screen appeared for `{target}`, which was prepared before the door: {crossing:?}"
    );
}

#[test]
fn an_unprepared_room_shows_the_loading_screen_only_while_work_remains() {
    // A slow machine: each frame is 250 ms, so the 250 ms reveal grace runs out
    // while the crossing still has frames of work (cover, readiness, commit,
    // first draw). On fast hardware the same crossing finishes inside the grace
    // and shows nothing (11 frames, measured 2026-10-05, once the start ration
    // was gone).
    let mut app = gameplay_after_startup(SLOW);
    // Straight after boot: the hall of characters (137 characters) is not
    // prepared.
    let crossing = cross(&mut app, "hall_of_characters");
    eprintln!("unprepared hall on a slow machine: {crossing:?}");
    // ⛔ The control: the instrument can see a loading screen.
    assert!(
        crossing.foreground_frames > 0,
        "the hall was entered unprepared on a slow machine and no loading screen appeared; the \
         instrument cannot see one: {crossing:?}"
    );
    // Once committed with its work done, the screen goes as soon as the room
    // is presentable (measured: 1 frame). ⚠ This cannot see a re-added
    // minimum display time shorter than the screen was already up (a 300 ms
    // floor poisoned in stayed green here: the screen had been visible longer
    // than that by commit); the floor's removal is held by the code and the
    // plan, not by this arm.
    assert!(
        crossing.foreground_frames_after_ready <= 2,
        "the loading screen stayed up {} frames after the hall was ready and committed: {crossing:?}",
        crossing.foreground_frames_after_ready
    );
}

/// Stand Alice, at rest, at the centre of the door of the live room that leads
/// to `target`, through the motion authority (ADR 0024).
fn stand_at_the_door_to(app: &mut bevy::prelude::App, target: &str) {
    use ambition_platformer2d::engine_core as ae;
    use ambition_platformer2d::engine_core::AabbExt;
    let door = {
        let world = app.world_mut();
        let live = ambition_platformer2d::world::rooms::sole_live_room_definition(world).expect("a live room");
        let rooms = ambition_platformer2d::platformer::lifecycle::session_world_component::<ambition_platformer2d::world::rooms::RoomSet>(world)
            .expect("the session keeps its room set");
        rooms
            .spec(live)
            .loading_zones
            .iter()
            .find(|zone| {
                rooms
                    .transition_for_player(live, zone.aabb, ae::Vec2::ZERO, true)
                    .is_some_and(|transition| rooms.rooms[transition.target_room].id == target)
            })
            .unwrap_or_else(|| panic!("the live room has no door to `{target}`"))
            .aabb
            .center()
    };
    let body = alice(app);
    let mut state = bevy::ecs::system::SystemState::<
        bevy::prelude::Query<(ae::BodyClusterQueryData, &mut ambition_platformer2d::actor::MotionModel)>,
    >::new(app.world_mut());
    let mut bodies = state.get_mut(app.world_mut()).expect("the body query is valid");
    let (mut clusters, mut model) = bodies.get_mut(body).expect("Alice has a body");
    let mut clusters = clusters.as_clusters_mut();
    ae::movement::transit_body(&mut model, &mut clusters, door, ae::movement::TransitVelocity::Zero);
    state.apply(app.world_mut());
}

/// Jon, 2026-10-04: through the central hub's door, the hall of characters
/// still showed a loading bar. The hub has 21 doors and the neighbour
/// prefetch prepares 4; by room index the hall was never among them. Ranked by
/// the door the player stands at (`RoomSet::neighbors_nearest_first`), a player
/// at the hall's door has the hall prepared, and enters it with no screen.
#[test]
fn a_player_at_the_halls_door_enters_a_prepared_hall() {
    let mut app = gameplay_after_startup(FAST);
    let source = live_room(&app);
    let neighbours = neighbours_of(&app, &source);
    // ⛔ Premise: the hall is behind a door of this room, and the room has
    // more neighbours than the prefetch prepares, so an unranked prefetch could
    // leave it out.
    assert!(neighbours.iter().any(|room| room == "hall_of_characters"), "`{source}` has no door to the hall");
    assert!(neighbours.len() > 4, "`{source}` has {} neighbours: every one is prefetched, so this tests nothing", neighbours.len());
    stand_at_the_door_to(&mut app, "hall_of_characters");
    for _ in 0..360 {
        step(&mut app);
    }
    let crossing = cross(&mut app, "hall_of_characters");
    assert!(crossing.prefetch_hit, "the hall was not prepared while Alice stood at its door: {crossing:?}");
    assert_eq!(crossing.foreground_frames, 0, "a loading screen for a hall prepared at its door: {crossing:?}");
}
