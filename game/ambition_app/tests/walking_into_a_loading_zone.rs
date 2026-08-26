//! Exercise room transitions by moving through authored zones under the real
//! movement/control pipeline rather than teleporting the body into a trigger.
//!
//! Walk-activated zones fire on overlap; interact doors require overlap plus the
//! interaction edge. The test distinguishes failure to physically reach the zone
//! from failure of the transition once overlap occurs.

use ambition_app::{AgentAction, Platformer2dSimHarness};
use ambition_platformer2d::engine_core::AabbExt;
use ambition_platformer2d::world::rooms::{LoadingZone, LoadingZoneActivation};
use bevy::prelude::With;

use crate::common::{base, fixed_60hz_sim};

/// How many frames a body may spend walking to a zone before we call it stuck.
///
/// a liveness backstop, not a measurement. At 60Hz this is ten seconds of
/// walking, which is far past any authored zone in a start room; the assertions
/// are about ARRIVAL and the ROOM, never about when.
const WALK_CAP: usize = 600;

fn active_room(sim: &mut Platformer2dSimHarness) -> String {
    sim.observation().active_room.clone()
}

/// Where the controlled body is right now.
fn body_pos(sim: &mut Platformer2dSimHarness) -> ambition_platformer2d::engine_core::Vec2 {
    let world = sim.world_mut();
    let mut q = world.query_filtered::<&ambition_platformer2d::platformer::body::BodyKinematics, With<ambition_platformer2d::platformer::markers::PrimaryPlayer>>();
    q.single(world)
        .expect("the session has a controlled body")
        .pos
}

/// Every authored zone of this activation kind in the active room, nearest first.
fn zones_by_distance(
    sim: &mut Platformer2dSimHarness,
    activation: LoadingZoneActivation,
) -> Vec<LoadingZone> {
    let from = body_pos(sim);
    let world = sim.world_mut();
    let mut query = world.query::<&ambition_platformer2d::world::rooms::RoomSet>();
    let Some(room_set) = query.iter(world).next() else {
        return Vec::new();
    };
    let mut candidates: Vec<LoadingZone> = room_set
        .active_loading_zones()
        .iter()
        .filter(|zone| zone.activation == activation)
        .cloned()
        .collect();
    candidates.sort_by(|a, b| {
        let da = (a.aabb.center() - from).length();
        let db = (b.aabb.center() - from).length();
        da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
    });
    candidates
}

/// Hold a direction toward `target_x`, one frame, plus whatever else is asked.
fn walk_toward(target_x: f32, from_x: f32, extra: AgentAction) -> AgentAction {
    let dir = if target_x > from_x { 1.0 } else { -1.0 };
    AgentAction {
        move_x: dir,
        right_pressed: dir > 0.0,
        left_pressed: dir < 0.0,
        ..extra
    }
}

/// Walk until the body overlaps `zone`, holding `extra` the whole way.
///
/// Returns the room id the moment it changes, or `None` if the walk finished
/// without a transition. Stops early if the room changes — a `Walk` zone fires
/// the instant the rectangles touch, and stepping past that would measure the
/// NEXT room's geometry.
fn walk_into(
    sim: &mut Platformer2dSimHarness,
    zone: &LoadingZone,
    extra: AgentAction,
) -> (bool, Option<String>) {
    let before = active_room(sim);
    let target_x = zone.aabb.center().x;
    let mut arrived = false;
    for _ in 0..WALK_CAP {
        let here = body_pos(sim);
        // the RECTANGLE is the arrival test, not a distance threshold: "inside
        // it" is what the transition itself asks. This checks the body's CENTRE,
        // which is stricter than the real rule (the transition overlaps the whole
        // body box) — deliberately, because it only decides which HALF a failure
        // blames, and a stricter arrival test never blames movement for a
        // transition that did fire.
        let inside = here.x >= zone.aabb.min.x
            && here.x <= zone.aabb.max.x
            && here.y >= zone.aabb.min.y
            && here.y <= zone.aabb.max.y;
        if inside {
            arrived = true;
        }
        let now = active_room(sim);
        if now != before {
            return (true, Some(now));
        }
        sim.step(walk_toward(target_x, here.x, extra.clone()));
    }
    let now = active_room(sim);
    (arrived, (now != before).then_some(now))
}

/// Reaching an authored `EdgeExit` by ordinary movement must activate the room
/// transition. The fixture deliberately walks rather than injecting contact so
/// movement, overlap detection, and transition activation are exercised together.
#[test]
fn reaching_a_contact_zone_under_her_own_power_changes_the_room() {
    let mut sim = fixed_60hz_sim();
    for _ in 0..10 {
        sim.step(base());
    }
    let before = active_room(&mut sim);
    let zone = zones_by_distance(&mut sim, LoadingZoneActivation::EdgeExit)
        .into_iter()
        .next()
        .unwrap_or_else(|| {
            panic!(
                "'{before}' authors no `EdgeExit` loading zone, so this test \
                 walked at nothing — point it at a room that has one"
            )
        });
    let name = zone.name.clone();
    let target_x = zone.aabb.center().x;
    for frame in 0..WALK_CAP {
        if active_room(&mut sim) != before {
            return;
        }
        let here = body_pos(&mut sim);
        let dir = if target_x > here.x { 1.0 } else { -1.0 };
        let _ = frame;
        sim.step(AgentAction {
            move_x: dir,
            right_pressed: dir > 0.0,
            left_pressed: dir < 0.0,
            ..base()
        });
    }
    panic!(
        "WALKED toward the `{name}` contact zone of '{before}' for {WALK_CAP} \
         frames and never left the room. She ended at {:?}; the zone \
         is {:?}. Contact transitions fire on overlap alone, so this names the \
         mechanism: overlap → transition_for_player → RoomTransitionRequested → \
         the room actually changing.",
        body_pos(&mut sim),
        zone.aabb,
    );
}

/// AND A DOOR OPENS FOR A BODY THAT WALKED UP TO IT.
///
/// interact is held for the WHOLE walk rather than pressed on arrival, and
/// that is deliberate: a door is buffered-interact, the buffer is what
/// `door_entry` covers, and holding it here keeps this test measuring the walk
/// and the transition rather than re-measuring the buffer's timing.
#[test]
fn walking_into_a_door_and_holding_interact_changes_the_room() {
    let mut sim = fixed_60hz_sim();
    for _ in 0..10 {
        sim.step(base());
    }
    let before = active_room(&mut sim);
    let zone = zones_by_distance(&mut sim, LoadingZoneActivation::Door)
        .into_iter()
        .next()
        .unwrap_or_else(|| {
            panic!(
                "the start room '{before}' authors no `Door` loading zone, so \
                 this test walked at nothing — point it at a room that has one"
            )
        });
    let name = zone.name.clone();
    let held = AgentAction {
        interact: true,
        interact_held: true,
        ..base()
    };
    let (arrived, after) = walk_into(&mut sim, &zone, held);
    assert!(
        arrived,
        "held a direction for {WALK_CAP} frames toward the `{name}` door of \
         '{before}' and the body never got inside it — it ended at {:?}, the \
         door is {:?}. The MOVEMENT half, not the door.",
        body_pos(&mut sim),
        zone.aabb,
    );
    assert!(
        after.is_some(),
        "the body walked into the `{name}` door of '{before}' holding interact \
         and the room never changed.",
    );
}
