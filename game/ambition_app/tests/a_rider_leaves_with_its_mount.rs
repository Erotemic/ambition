#![cfg(feature = "rl_sim")]
//! REL3: a crossing starts at its actual root. A room-scoped NPC rider sent
//! through a door into another live room takes the mount it rides and the
//! mount's limbs, and the mount stands beneath it at the arrival.
//!
//! The rider is room-scoped, so no custody edge runs to its mount
//! (`project_body_custody` grows custody only beneath a root that travels
//! already). The crossing asks the mount owner for the attachments of its
//! root instead, and the mount owner places the mount so its seat meets the
//! rider's soles; the test moves no body but the door.

use crate::common::{base, fixed_60hz_room_options};
use ambition_app::rl_sim::AmbitionSim as _;
use ambition_platformer2d::engine_core as ae;
use ae::AabbExt as _;
use bevy::prelude::Entity;

const ARENA: &str = "gnu_ton_arena";

fn room_of(sim: &ambition_app::Platformer2dSimHarness, entity: Entity) -> Option<ambition_platformer2d::platformer::lifecycle::LiveRoomInstance> {
    sim.world().get::<ambition_platformer2d::platformer::lifecycle::InRoomInstance>(entity).map(|room| room.0)
}

fn pos(sim: &ambition_app::Platformer2dSimHarness, body: Entity) -> ae::Vec2 {
    sim.world().get::<ae::BodyKinematics>(body).expect("a body").pos
}

/// How far the rider's soles are from the mount's seat, in world units.
fn seat_gap(sim: &ambition_app::Platformer2dSimHarness, rider: Entity, mount: Entity) -> f32 {
    let world = sim.world();
    let mountable = world.get::<ambition_platformer2d::mount::Mountable>(mount).expect("a mount");
    let mount_kin = world.get::<ae::BodyKinematics>(mount).expect("a mount body");
    let unmirrored = world.get::<ae::Unmirrored>(mount).is_some();
    let seat = ambition_platformer2d::mount::seat_in_world(mountable.seat, mount_kin, unmirrored).expect("a seat");
    let rider_kin = world.get::<ae::BodyKinematics>(rider).expect("a rider body");
    let soles = rider_kin
        .material_map()
        .and_then(|map| map.point(ae::Vec2::new(0.0, rider_kin.canonical_size.y * 0.5)))
        .expect("the rider's soles");
    (soles - seat).length()
}

/// Move the arena's door `door` onto `body`, without telling the room set it
/// changed (a changed set reloads the room).
fn bring_the_door_to(sim: &mut ambition_app::Platformer2dSimHarness, body: Entity, door: &str) {
    use ambition_platformer2d::world::rooms::RoomSet;
    let at = pos(sim, body);
    let world = sim.world_mut();
    let mut sets = world.query::<&mut RoomSet>();
    let mut set = sets.iter_mut(world).next().expect("a room set");
    let set = bevy::ecs::change_detection::DetectChangesMut::bypass_change_detection(&mut set);
    let arena = set.definition_by_id(ARENA).expect("the arena is authored").index();
    let zone = set.rooms[arena].loading_zones.iter_mut().find(|zone| zone.id == door).expect("the door");
    zone.aabb = ae::Aabb::new(at, zone.aabb.half_size());
}

/// The arena's first door that leads to another room, and that room.
fn a_door_out(sim: &mut ambition_app::Platformer2dSimHarness) -> (String, String, ae::Vec2) {
    use ambition_platformer2d::world::rooms::LoadingZoneActivation;
    let world = sim.world_mut();
    let rooms = ambition_platformer2d::platformer::lifecycle::session_world_component::<ambition_platformer2d::world::rooms::RoomSet>(world)
        .expect("the room set");
    let from = rooms.definition_by_id(ARENA).expect("the arena");
    rooms
        .spec(from)
        .loading_zones
        .iter()
        .filter(|zone| zone.activation == LoadingZoneActivation::Door)
        .find_map(|zone| {
            let transition = rooms.transition_through(from, &zone.id)?;
            let to = rooms.rooms.get(transition.target_room)?.id.clone();
            (to != ARENA).then(|| (zone.id.clone(), to, transition.arrival))
        })
        .expect("the arena has a door out")
}

/// ⭐ A RIDER SENT THROUGH A DOOR TAKES ITS MOUNT, AND THE MOUNT STANDS UNDER
/// IT. A pirate rides the giant GNU (two fists and a hoof, each a body of its
/// own) that `gnu_ton_arena` authors: a room-scoped rider on an authored
/// limbed mount (a runtime spawn may not build a limbed host). Bob holds the
/// arena; Alice goes through its door, so both rooms are live. The pirate is
/// sent through the same door. The authorities: the room stamps
/// of rider, mount and limbs, the rider's position against the door's
/// arrival, and the saddle relation (the rider's soles on the mount's
/// seat), right after the crossing and after the saddle pin has run again.
#[test]
fn a_rider_sent_through_a_door_takes_its_mount_and_stands_where_it_arrived() {
    use ambition_platformer2d::actors::body::direction::Leaving;
    use ambition_platformer2d::characters::actor::limb::LimbRig;
    use ambition_platformer2d::characters::brain::Brain;
    use ambition_platformer2d::mount::RidingOn;
    use crate::common::walk_through_the_door_to;

    let mut sim = ambition_app::Platformer2dSimHarness::new_with_options(fixed_60hz_room_options(ARENA)).expect("the arena boots");
    crate::common::doors_cross_at_once(&mut sim);
    let arena = crate::two_players_two_live_rooms::bob_beside_alice(
        &mut sim,
        ARENA,
        Some(ambition_platformer2d::characters::control::PlayerSlot(1)),
    );
    // The arena authors the giant (limbs and all) with GNU-ton, a boss the
    // saddle pin does not pose. Its saddle is given to an ordinary NPC.
    let (gnu_ton, mount) = {
        let world = sim.world_mut();
        let mut q = world.query::<(Entity, &RidingOn, &Brain)>();
        q.iter(world).next().map(|(entity, riding, _)| (entity, riding.mount)).expect("the arena authors a rider on a mount")
    };
    sim.world_mut().entity_mut(gnu_ton).remove::<RidingOn>();
    sim.world_mut().get_mut::<ambition_platformer2d::mount::MountSlot>(mount).expect("a saddle").rider = None;
    let seat_at = pos(&sim, mount) + ae::Vec2::new(0.0, -500.0);
    sim.spawn_enemy_character_at(
        "rel3_rider",
        "Pirate Raider",
        (seat_at.x, seat_at.y),
        (22.0, 39.0),
        ambition_platformer2d::entity_catalog::placements::CharacterBrain::Passive,
        "npc_pirate_raider",
    );
    let rider = {
        let world = sim.world_mut();
        world
            .query::<(Entity, &ambition_platformer2d::combat::components::FeatureId)>()
            .iter(world)
            .find(|(_, feature)| feature.as_str() == "rel3_rider")
            .map(|(entity, _)| entity)
            .expect("the pirate spawned")
    };
    // A staged spawn names no live room (it is built beside the one room the
    // session had), so the pirate is given the arena's stamp, as an authored
    // body of the arena has.
    let arena_stamp = room_of(&sim, mount).expect("the giant is the arena's");
    sim.world_mut().entity_mut(rider).insert(ambition_platformer2d::platformer::lifecycle::InRoomInstance(arena_stamp));
    // The pirate holds the giant's licence (the arena gives it to GNU-ton).
    let class = sim.world().get::<ambition_platformer2d::mount::Mountable>(mount).expect("a mount").class.clone();
    sim.world_mut().entity_mut(rider).insert((Brain::stand_still(), ambition_platformer2d::mount::CanPilot { classes: vec![class] }));
    let refusal = ambition_platformer2d::mount::BoardCase::in_world(sim.world(), rider, mount).refusal();
    assert_eq!(refusal, None, "setup: the giant refuses the pirate");
    assert!(ambition_platformer2d::mount::board(sim.world_mut(), rider, mount), "setup: the pirate boards the giant");
    sim.step_n(base(), 5);
    assert_eq!(ambition_platformer2d::mount::active_mount_in(sim.world(), rider), Some(mount), "setup: the pirate rides the giant");
    let limbs: Vec<Entity> = sim
        .world()
        .get::<LimbRig>(mount)
        .map(|rig| rig.limbs.values().copied().collect())
        .unwrap_or_default();
    assert!(!limbs.is_empty(), "setup: the giant has limbs");
    let setup_gap = seat_gap(&sim, rider, mount);
    assert!(setup_gap < 2.0, "setup: the pirate sits on the giant's seat (gap {setup_gap}; rider {:?} mount {:?})", pos(&sim, rider), pos(&sim, mount));

    let (door, next_door, arrival) = a_door_out(&mut sim);
    assert_eq!(walk_through_the_door_to(&mut sim, &next_door), next_door, "setup: Alice goes next door");
    sim.step_n(base(), 30);
    let alices_room = crate::two_players_two_live_rooms::where_they_are(&mut sim).0.expect("Alice's room");
    let live: std::collections::BTreeSet<String> =
        crate::two_players_two_live_rooms::live_rooms(&mut sim).into_iter().map(|(_, id)| id).collect();
    assert_eq!(live, [ARENA.to_string(), next_door.clone()].into_iter().collect(), "setup: the arena and Alice's room are live");
    for body in [rider, mount].into_iter().chain(limbs.iter().copied()) {
        assert_eq!(room_of(&sim, body), Some(arena), "setup: the ride is in the arena ({body:?})");
    }
    assert!(
        sim.world().get::<ambition_platformer2d::platformer::lifecycle::InCustodyOf>(mount).is_none(),
        "setup: a room-scoped rider's mount is in no custody (the case this witness is about)"
    );

    let source = pos(&sim, rider);
    assert!((source - arrival).length() > 200.0, "setup: the arrival is far from where the ride stands");
    bring_the_door_to(&mut sim, rider, &door);
    // Sent as the `leave_through` verb sends a body: a visit to the zone,
    // which is already under it.
    let at = pos(&sim, rider);
    sim.world_mut().entity_mut(rider).insert((
        ambition_platformer2d::actors::body::visit::Visit::to(at),
        Leaving { zone: door.clone() },
    ));

    let mut crossed = false;
    for _ in 0..10 {
        sim.step(base());
        if room_of(&sim, rider) == Some(alices_room) {
            crossed = true;
            break;
        }
    }
    assert!(crossed, "the pirate never went through the door");
    let left: Vec<Entity> =
        [mount].into_iter().chain(limbs.iter().copied()).filter(|body| room_of(&sim, *body) != Some(alices_room)).collect();
    assert!(left.is_empty(), "the pirate went through and {left:?} of its mount and limbs stayed behind");
    let arrived = pos(&sim, rider);
    assert!((arrived - arrival).length() < 48.0, "the pirate came out at {arrived:?}, not the door's arrival {arrival:?}");
    assert!(seat_gap(&sim, rider, mount) < 2.0, "the pirate came out off its seat: the giant did not stand beneath it");
    for limb in &limbs {
        let apart = (pos(&sim, *limb) - pos(&sim, mount)).length();
        assert!(apart < 1000.0, "a limb {limb:?} stayed at the arena's coordinates, {apart} from its giant");
    }

    // The giant stands under a rider placed at a doorway, so it may settle
    // onto the floor and the rider with it. The pin must not take the rider
    // back to where the ride stood in the arena, `source`.
    sim.step_n(base(), 3);
    let held = pos(&sim, rider);
    assert!(
        (held - arrival).length() < (source - arrival).length() * 0.25,
        "the saddle pin pulled the pirate from its arrival {arrival:?} to {held:?} (it was at {source:?} in the arena)"
    );
    assert!(seat_gap(&sim, rider, mount) < 2.0, "the pirate is off its seat after the pin");
}
