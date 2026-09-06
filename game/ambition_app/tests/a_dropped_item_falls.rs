//! A released object is under gravity; an authored one is where its author put it.
//!
//! ⭐ THESE TWO PULL IN OPPOSITE DIRECTIONS, and that is the whole reason the
//! file exists. `ground_item_physics` used to skip any object whose velocity was
//! zero, which made a Z-drop — a release that launches at EXACTLY zero — hang
//! wherever the hand let go. The obvious repair, deleting the early-out, was
//! measured to take the authored population with it: a room rebuild came back
//! with zero ground items where it had fifteen, because an authored placement is
//! NOT necessarily resting on collision geometry the step can see. `blink_run`
//! shows it directly — its object is authored at y=124 and the step rests a free
//! object at y≈109.8, so the authored one is inside the floor.
//!
//! So sleep became explicit (`SettledItem`) rather than derived from velocity,
//! and both halves are pinned here on the production path: the real pickup, the
//! real `grab_pressed` release, the real authored room.
//!
//! ⛔ THE ASSERTIONS ARE ABOUT VELOCITY AND THE MARKER, NOT DISTANCE. A position
//! that DIFFERS is not a position that is still CHANGING — reading a drop by how
//! far it travelled misdiagnosed this defect twice, and the room's own ceiling
//! decides the distance anyway.

use ambition_app::{AgentAction, Platformer2dSimHarness};
use ambition_platformer2d::engine_core::ControlFrame;
use ambition_platformer2d::platformer::sim_id::SimId;
use bevy::prelude::Entity;

use crate::common::{base, fixed_60hz_room_sim};

/// `blink_run` authors exactly one ground item, which is what makes "the
/// authored object" an unambiguous phrase in this file.
const ROOM: &str = "blink_run";

type GroundItem = ambition_platformer2d::held_items::GroundItem;
type ItemCustody = ambition_platformer2d::held_items::ItemCustody;
type SettledItem = ambition_platformer2d::held_items::SettledItem;

/// Every object lying in the world right now, with where it thinks it is.
fn items_in_world(sim: &mut Platformer2dSimHarness) -> Vec<(Entity, (f32, f32))> {
    let mut query = sim
        .world_mut()
        .query::<(Entity, &GroundItem, &ItemCustody)>();
    let mut found: Vec<(Entity, (f32, f32))> = query
        .iter(sim.world())
        .filter(|(_, _, custody)| custody.in_world())
        .map(|(entity, ground, _)| (entity, (ground.pos.x, ground.pos.y)))
        .collect();
    found.sort_by_key(|(entity, _)| *entity);
    found
}

fn ground(sim: &Platformer2dSimHarness, item: Entity) -> &GroundItem {
    sim.world()
        .get::<GroundItem>(item)
        .expect("the object is still a ground item")
}

fn is_settled(sim: &Platformer2dSimHarness, item: Entity) -> bool {
    sim.world().get::<SettledItem>(item).is_some()
}

fn held_by(sim: &Platformer2dSimHarness, item: Entity) -> Option<Entity> {
    match sim.world().get::<ItemCustody>(item) {
        Some(ItemCustody::Held { holder }) => Some(*holder),
        _ => None,
    }
}

/// The authored object, taken up by the pressed pickup the player actually uses.
fn pick_the_authored_object_up(sim: &mut Platformer2dSimHarness) -> Entity {
    let lying = items_in_world(sim);
    assert_eq!(
        lying.len(),
        1,
        "'{ROOM}' should author exactly one ground item lying in the world"
    );
    let (item, (x, y)) = lying[0];
    assert!(
        is_settled(sim, item),
        "an authored placement is declared at rest by its construction — that \
         declaration is what keeps it out of the floor"
    );
    sim.teleport_player((x, y));
    sim.step(AgentAction {
        attack: true,
        ..base()
    });
    sim.step(base());
    assert!(
        held_by(sim, item).is_some(),
        "the pressed pickup should have taken custody of the authored object"
    );
    item
}

/// Let go with `grab_pressed` — the Z-drop, whose launch is EXACTLY zero — and
/// answer with how far the object descended before it stopped.
fn drop_it_and_let_it_land(sim: &mut Platformer2dSimHarness, item: Entity) -> f32 {
    sim.step_frame(ControlFrame {
        grab_pressed: true,
        ..ControlFrame::default()
    });
    assert!(
        held_by(sim, item).is_none(),
        "the grab press should have let go of the held object"
    );
    let released = ground(sim, item).pos;
    assert!(
        !is_settled(sim, item),
        "the object came out of a hand, so it is not at rest: the release owes \
         the settled mark back, or the drop hangs where the hand let go"
    );

    // ⭐ THE RELEASE FRAME IS THE WHOLE QUESTION. `Release::Drop` adds NOTHING
    // to the launch — the object leaves the hand at exactly zero — so it either
    // picks up gravity on the very step that releases it or it never will. This
    // reads the velocity AFTER that step, which is why it is already non-zero:
    // a hanging object reads exactly (0, 0) here, forever.
    let launched = ground(sim, item).vel;
    assert!(
        launched.y > 0.0 && launched.x == 0.0,
        "a Z-drop launches at zero and falls straight down from there; this \
         object left the hand with velocity {launched:?}"
    );
    sim.step(base());
    let moving = ground(sim, item);
    assert!(
        moving.vel.y > launched.y && moving.pos.y > released.y,
        "the next frame should be faster and lower, and it is {:?} at {:?} \
         (released at {released:?} doing {launched:?})",
        moving.vel,
        moving.pos
    );

    for _ in 0..180 {
        sim.step(base());
    }
    let resting = ground(sim, item).pos;
    assert!(
        is_settled(sim, item),
        "three seconds later the object should have landed on something and said \
         so; it is at {resting:?} with velocity {:?}",
        ground(sim, item).vel
    );
    for _ in 0..60 {
        sim.step(base());
    }
    assert_eq!(
        ground(sim, item).pos,
        resting,
        "a settled object holds still — a fall that never ends is the opposite \
         failure, and the explicit mark is what ends it"
    );
    resting.y - released.y
}

/// AN AUTHORED PLACEMENT IS ALREADY AT REST — and it stays there.
///
/// The poison for the wrong repair. Nothing goes near this room; if the step
/// treats an authored object as unsupported and walks it downward, or drops it
/// out of the world entirely, this reddens.
#[test]
fn an_authored_object_nobody_touches_stays_exactly_where_it_was_authored() {
    let mut sim = fixed_60hz_room_sim(ROOM);
    let before = items_in_world(&mut sim);
    assert_eq!(
        before.len(),
        1,
        "'{ROOM}' should author exactly one ground item lying in the world"
    );

    for _ in 0..120 {
        sim.step(base());
    }

    let after = items_in_world(&mut sim);
    assert_eq!(
        after, before,
        "two seconds passed and nobody went near the authored object: it should \
         still be the same object at the same place"
    );
}

/// A Z-DROP LAUNCHES AT EXACTLY ZERO, and it still has to fall.
#[test]
fn a_zero_velocity_release_falls_and_then_rests() {
    let mut sim = fixed_60hz_room_sim(ROOM);
    let item = pick_the_authored_object_up(&mut sim);
    let fell = drop_it_and_let_it_land(&mut sim, item);
    assert!(
        fell > 0.0,
        "the object was released above what it landed on, so the descent should \
         be positive, and it was {fell}px"
    );
}

/// TAKING A SETTLED OBJECT BACK UP UN-SETTLES IT.
///
/// The mark is state about an object lying in the world; carrying it away and
/// letting go again must give the same answer the first release did. Without the
/// release clearing it, the SECOND drop hangs where the first one landed — and
/// that is the failure the first test alone cannot see, because an authored
/// object happens to start marked.
#[test]
fn an_object_released_twice_falls_twice() {
    let mut sim = fixed_60hz_room_sim(ROOM);
    let item = pick_the_authored_object_up(&mut sim);
    let identity = sim
        .world()
        .get::<SimId>(item)
        .expect("the authored object carries its authored identity")
        .clone();

    let first = drop_it_and_let_it_land(&mut sim, item);

    // Same object, taken back up from where it landed and let go again.
    let (x, y) = {
        let pos = ground(&sim, item).pos;
        (pos.x, pos.y)
    };
    sim.teleport_player((x, y));
    sim.step(AgentAction {
        attack: true,
        ..base()
    });
    sim.step(base());
    assert!(
        held_by(&sim, item).is_some(),
        "an object that has come to rest should be pick-up-able again"
    );

    let second = drop_it_and_let_it_land(&mut sim, item);
    assert!(
        second > 0.0,
        "the first release descended {first}px and the second {second}px: a \
         second release has to fall too"
    );

    assert_eq!(
        sim.world().get::<SimId>(item),
        Some(&identity),
        "two releases and a pickup are custody changes, not a new object"
    );
}
