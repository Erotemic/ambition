//! An effect is drawn in its own live room (view half, cut V2f). Two live
//! rooms of different sizes; one effect for each, at one simulation position.
//! Each effect is placed by its own room's flip into Bevy space, at spawn and
//! by its clock. A sole-room read did not run while two rooms were live, so no
//! effect was drawn.

use super::*;
use ambition_platformer2d_shared_tangle::lifecycle::{
    insert_live_room_component, spawn_live_room, InRoomInstance, LiveRoomInstance,
};

const AT: ae::Vec2 = ae::Vec2::new(100.0, 200.0);
const BIG: ae::Vec2 = ae::Vec2::new(800.0, 600.0);
const SMALL: ae::Vec2 = ae::Vec2::new(400.0, 300.0);

fn room(size: ae::Vec2) -> ae::RoomGeometry {
    ae::RoomGeometry(ae::World::new("fx room", size, ae::Vec2::new(40.0, 40.0), Vec::new()))
}

/// Where the flip of a room of `size` puts `AT`.
fn flipped(size: ae::Vec2) -> BVec2 {
    BVec2::new(AT.x - size.x * 0.5, size.y * 0.5 - AT.y)
}

/// The spawn subscriber and the particle clock, as the host wires them.
fn app() -> App {
    let mut app = App::new();
    app.init_resource::<Time>();
    app.add_message::<VfxInRoom>();
    app.add_systems(Update, (vfx_spawn_messages, update_particles).chain());
    app
}

fn coin_pop(app: &mut App, room: Option<LiveRoomInstance>) {
    app.world_mut().write_message(VfxInRoom {
        room,
        vfx: VfxMessage::CoinPop { pos: AT },
    });
}

/// (room stamp, Bevy position) of every coin drawn, by room ordinal.
fn drawn(app: &mut App) -> Vec<(Option<u32>, BVec2)> {
    let mut q = app
        .world_mut()
        .query::<(&ParticleVisual, &Transform, Option<&InRoomInstance>)>();
    let mut rows: Vec<_> = q
        .iter(app.world())
        .map(|(_, transform, stamp)| (stamp.map(|stamp| stamp.0.ordinal()), transform.translation.truncate()))
        .collect();
    rows.sort_by_key(|(room, _)| *room);
    rows
}

#[test]
fn each_effect_is_drawn_in_its_own_live_room() {
    let mut app = app();
    insert_live_room_component(app.world_mut(), room(BIG));
    let second = LiveRoomInstance::ACTIVATION.next();
    spawn_live_room(app.world_mut(), second, room(SMALL));
    coin_pop(&mut app, Some(LiveRoomInstance::ACTIVATION));
    coin_pop(&mut app, Some(second));
    // An unroomed row cannot say which of two rooms it is in, so it is not drawn.
    coin_pop(&mut app, None);
    // The first update spawns the coins; the second runs their clock.
    app.update();
    app.update();
    assert_eq!(
        drawn(&mut app),
        vec![
            (Some(LiveRoomInstance::ACTIVATION.ordinal()), flipped(BIG)),
            (Some(second.ordinal()), flipped(SMALL)),
        ],
        "(room, position) of each drawn effect: each must be stamped with and placed by its own live room"
    );
}

/// The control: with one live room, an unroomed row is drawn in it, as every
/// effect was before rooms were named.
#[test]
fn an_unroomed_effect_is_drawn_in_the_sole_live_room() {
    let mut app = app();
    insert_live_room_component(app.world_mut(), room(BIG));
    coin_pop(&mut app, None);
    app.update();
    app.update();
    assert_eq!(
        drawn(&mut app),
        vec![(Some(LiveRoomInstance::ACTIVATION.ordinal()), flipped(BIG))],
        "(room, position) of an unroomed effect with one live room"
    );
}

/// The blink ring is drawn in its blinking body's live room (view half, cut
/// V2i): placed by that room's flip and stamped with it. A ring whose room
/// cannot be told is not drawn.
#[cfg(feature = "input")]
#[test]
fn the_blink_ring_is_drawn_in_its_body_s_own_live_room() {
    let mut app = App::new();
    app.init_resource::<Time>();
    insert_live_room_component(app.world_mut(), room(BIG));
    let second = LiveRoomInstance::ACTIVATION.next();
    spawn_live_room(app.world_mut(), second, room(SMALL));
    let fact = |room| ambition_sim_view::BlinkPreviewFact {
        active: true,
        target: AT,
        precision: false,
        body_min_extent: 0.0,
        room,
    };
    app.insert_resource(fact(Some(second)));
    app.add_systems(Update, update_blink_preview);
    app.update();
    app.update();
    let embers = |app: &mut App| -> Vec<(Option<u32>, BVec2)> {
        let mut q = app
            .world_mut()
            .query::<(&BlinkPreviewVisual, &Transform, Option<&InRoomInstance>)>();
        let mut rows: Vec<_> = q
            .iter(app.world())
            .map(|(_, transform, stamp)| (stamp.map(|stamp| stamp.0.ordinal()), transform.translation.truncate()))
            .collect();
        rows.dedup();
        rows
    };
    assert_eq!(
        embers(&mut app),
        vec![(Some(second.ordinal()), flipped(SMALL))],
        "the ring must be placed by its body's live room and stamped with it"
    );
    app.insert_resource(fact(None));
    app.update();
    app.update();
    assert_eq!(embers(&mut app), Vec::new(), "a ring whose room cannot be told is drawn");
}
