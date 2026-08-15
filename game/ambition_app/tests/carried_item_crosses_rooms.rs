//! **An object you are carrying goes through the door with you.**
//!
//! `ItemCustody` made a picked-up item keep its entity and its `SimId` across
//! world → held → world — and that was true only inside ONE room. An authored
//! `GroundItem` is spawned `RoomScopedEntity`; it kept that scope while `Held`;
//! and `RoomConstructionPlan::retire_outgoing` despawns every room-scoped entity
//! except the transiting body. So the axe you carried through a door was
//! destroyed AT the door: the same destroyed identity `ItemCustody` exists to
//! prevent, reached through the room boundary instead of through the pickup.
//!
//! ⛔ **the fix is not in the transition, and this test would pass either way —
//! which is why it is worth saying here.** Teaching the room commit to walk a
//! body's held items would make the boundary know that inventories exist. What
//! landed instead is that CUSTODY OWNS RESIDENCY
//! (`items::pickup::project_custody_onto_residency`): an object in a travelling
//! body's custody is scoped to a room and resident in NONE, so the roster a room
//! change retires (`RoomResident`) simply does not contain it. The transition is
//! unchanged except for the word "resident".
//!
//! What is driven here is the real machinery end to end: the authored LDtk item,
//! the pressed pickup, an authored `Door` loading zone held open with the real
//! interact action, the `RoomTransitionApplication` commit, a real Shield+Attack
//! throw through `ControlFrame`, and a SECOND real transition out of the
//! destination.
//!
//! Three claims, in order:
//!
//! 1. the object SURVIVES the crossing — same entity, same `SimId`, still held;
//! 2. thrown down in the destination it becomes a resident THERE;
//! 3. so the next transition out of that room retires it, exactly like anything
//!    else lying on that room's floor. ⭐ the third is the half that makes the
//!    first honest: an object that stopped being retirable would also pass (1).

use ambition_app::{AgentAction, Platformer2dSimHarness};
use ambition_platformer2d::engine_core::{AabbExt, ControlFrame};
use ambition_platformer2d::platformer::sim_id::SimId;
use bevy::prelude::{Entity, With};

use crate::common::{base, fixed_60hz_room_sim};

/// `blink_run` authors exactly one `GroundItem` (`blink_run_pickup`) and exactly
/// one `Door` loading zone (to `portal_bridge`, which authors three doors back
/// out). One item and one exit is what makes the assertions below unambiguous.
const SOURCE_ROOM: &str = "blink_run";
const TARGET_ROOM: &str = "portal_bridge";

/// The one authored ground item in the room, with the identity it was authored
/// with. Panics rather than returning `None`: a room that stopped authoring the
/// item is a test measuring nothing, and that must be loud.
fn authored_item(sim: &mut Platformer2dSimHarness) -> (Entity, SimId) {
    let mut query = sim.world_mut().query::<(
        Entity,
        &SimId,
        &ambition_platformer2d::actors::items::pickup::ItemCustody,
    )>();
    let found: Vec<(Entity, SimId)> = query
        .iter(sim.world())
        .filter(|(_, _, custody)| custody.in_world())
        .map(|(entity, id, _)| (entity, id.clone()))
        .collect();
    assert_eq!(
        found.len(),
        1,
        "'{SOURCE_ROOM}' should author exactly one ground item lying in the world"
    );
    found[0].clone()
}

/// Where the object thinks it is, and who has it.
fn custody(
    sim: &Platformer2dSimHarness,
    item: Entity,
) -> Option<ambition_platformer2d::actors::items::pickup::ItemCustody> {
    sim.world()
        .get::<ambition_platformer2d::actors::items::pickup::ItemCustody>(item)
        .copied()
}

fn item_pos(sim: &Platformer2dSimHarness, item: Entity) -> (f32, f32) {
    let ground = sim
        .world()
        .get::<ambition_platformer2d::actors::items::pickup::GroundItem>(item)
        .expect("the item is still a ground item");
    (ground.pos.x, ground.pos.y)
}

/// Every live occurrence claiming one authored identity.
///
/// ⭐ **the whole defect is a COUNT**, so the measurement is a count and never a
/// lookup: `get::<SimId>(entity)` on the object you are holding answers "still
/// itself" and says nothing at all about the copy on the floor beside it.
fn occurrences(sim: &mut Platformer2dSimHarness, authored: &SimId) -> Vec<Entity> {
    let mut query = sim.world_mut().query::<(Entity, &SimId)>();
    query
        .iter(sim.world())
        .filter(|(_, sim_id)| *sim_id == authored)
        .map(|(entity, _)| entity)
        .collect()
}

/// Walk the controlled body through the authored `Door` of the active room that
/// leads to `target`, and return the room it arrived in.
///
/// The door is chosen by asking the room graph where each `Door` zone actually
/// GOES — `transition_for_player` is the same resolver the crossing itself uses
/// — because a room with several doors makes "the first one" a coin flip.
fn walk_through_the_door_to(sim: &mut Platformer2dSimHarness, target: &str) -> String {
    let before = sim.observation().active_room.clone();
    let door = {
        let world = sim.world_mut();
        let mut query = world.query::<&ambition_platformer2d::actors::rooms::RoomSet>();
        let room_set = query
            .iter(world)
            .next()
            .expect("the session has an active room set");
        let mut reachable: Vec<String> = Vec::new();
        let mut chosen = None;
        for zone in room_set.active_loading_zones() {
            if zone.activation != ambition_platformer2d::world::rooms::LoadingZoneActivation::Door {
                continue;
            }
            // The zone's own box as the body's box: a path of zero length from
            // the centre of a rectangle is inside that rectangle, so this asks
            // the resolver about exactly this door.
            let Some(transition) = room_set.transition_for_player(
                zone.aabb,
                ambition_platformer2d::engine_core::Vec2::ZERO,
                true,
            ) else {
                continue;
            };
            let Some(destination) = room_set.rooms.get(transition.target_room) else {
                continue;
            };
            reachable.push(destination.id.clone());
            if destination.id == target {
                chosen = Some(zone.clone());
                break;
            }
        }
        chosen.unwrap_or_else(|| {
            panic!("'{before}' has no Door to '{target}'; its doors reach {reachable:?}")
        })
    };
    let center = door.aabb.center();
    sim.teleport_player((center.x, center.y));
    for _ in 0..60 {
        let room = sim
            .step(AgentAction {
                interact: true,
                interact_held: true,
                ..base()
            })
            .active_room;
        if room != before {
            return room;
        }
    }
    panic!(
        "held interact inside the '{}' door of '{before}' for 60 frames and the \
         room never changed",
        door.name
    );
}

/// Pick the one authored ground item up with the pressed pickup, and answer with
/// the body now holding it.
fn pick_it_up(sim: &mut Platformer2dSimHarness, item: Entity) -> Entity {
    let (x, y) = item_pos(sim, item);
    sim.teleport_player((x, y));
    sim.step(AgentAction {
        attack: true,
        ..base()
    });
    sim.step(base());
    match custody(sim, item) {
        Some(ambition_platformer2d::actors::items::pickup::ItemCustody::Held { holder }) => holder,
        other => panic!("the pressed pickup should have taken custody, got {other:?}"),
    }
}

/// Shield+Attack: the only input that puts a `UseSystem` item back in the world.
fn throw_it_down(sim: &mut Platformer2dSimHarness) {
    sim.step_frame(ControlFrame {
        attack_pressed: true,
        shield_held: true,
        ..ControlFrame::default()
    });
    for _ in 0..30 {
        sim.step(base());
    }
}

/// Stand the controlled body in the middle of any authored `Door` zone of the
/// active room and hold interact until the room actually changes. Returns the
/// room it arrived in.
///
/// This is the REAL transition: the press goes through the interaction buffer
/// and `detect_room_transition_system`, and the commit lands a frame or two
/// later (a rollback host defers it to a confirmed boundary).
fn walk_through_a_door(sim: &mut Platformer2dSimHarness) -> String {
    let before = sim.observation().active_room.clone();
    let door = {
        let world = sim.world_mut();
        let mut query = world.query::<&ambition_platformer2d::actors::rooms::RoomSet>();
        let room_set = query
            .iter(world)
            .next()
            .expect("the session has an active room set");
        room_set
            .active_loading_zones()
            .iter()
            .find(|zone| {
                zone.activation == ambition_platformer2d::world::rooms::LoadingZoneActivation::Door
            })
            .cloned()
            .unwrap_or_else(|| panic!("'{before}' authors no Door loading zone to walk through"))
    };
    let center = door.aabb.center();
    sim.teleport_player((center.x, center.y));
    for _ in 0..60 {
        let room = sim
            .step(AgentAction {
                interact: true,
                interact_held: true,
                ..base()
            })
            .active_room;
        if room != before {
            return room;
        }
    }
    panic!(
        "held interact inside the '{}' door of '{before}' for 60 frames and the \
         room never changed",
        door.name
    );
}

#[test]
fn an_item_carried_through_a_door_survives_and_belongs_to_the_room_it_is_dropped_in() {
    let mut sim = fixed_60hz_room_sim(SOURCE_ROOM);
    // Let the room finish constructing before anything is read out of it.
    for _ in 0..10 {
        sim.step(base());
    }
    assert_eq!(sim.observation().active_room.as_str(), SOURCE_ROOM);

    let (item, authored) = authored_item(&mut sim);

    // ── PICK IT UP (the pressed pickup, on the body being driven) ────────────
    let (x, y) = item_pos(&sim, item);
    sim.teleport_player((x, y));
    sim.step(AgentAction {
        attack: true,
        ..base()
    });
    sim.step(base());
    let holder = match custody(&sim, item) {
        Some(ambition_platformer2d::actors::items::pickup::ItemCustody::Held { holder }) => holder,
        other => panic!("the pressed pickup should have taken custody, got {other:?}"),
    };

    // ── CROSS A REAL ROOM BOUNDARY ──────────────────────────────────────────
    let arrived = walk_through_a_door(&mut sim);
    assert_eq!(
        arrived, TARGET_ROOM,
        "the authored door out of '{SOURCE_ROOM}' leads to '{TARGET_ROOM}'"
    );

    // CLAIM 1: the same object, not a replacement. Before residency followed
    // custody, this entity was despawned by `retire_outgoing` on the way out and
    // the body arrived holding a `HeldItem` with nothing behind it.
    assert!(
        sim.world().get_entity(item).is_ok(),
        "the object in the body's hands was destroyed by the room transition"
    );
    assert_eq!(
        sim.world().get::<SimId>(item),
        Some(&authored),
        "and it is the object it was authored as — no despawn, no re-mint"
    );
    assert!(
        matches!(
            custody(&sim, item),
            Some(ambition_platformer2d::actors::items::pickup::ItemCustody::Held { holder: h })
                if h == holder
        ),
        "still in the same hands on the far side of the door"
    );

    // ── THROW IT DOWN HERE (Shield + Attack, the real input) ────────────────
    // `blink` is a `UseSystem` item, so only the explicit Shield+Attack throws
    // it — `AgentAction` has no shield, which is why this drives a raw
    // `ControlFrame`, the same value a device produces.
    sim.step_frame(ControlFrame {
        attack_pressed: true,
        shield_held: true,
        ..ControlFrame::default()
    });
    for _ in 0..30 {
        sim.step(base());
    }
    assert!(
        custody(&sim, item).is_some_and(|custody| custody.in_world()),
        "Shield+Attack puts the object back in the world"
    );
    assert_eq!(
        sim.world().get::<SimId>(item),
        Some(&authored),
        "the thrown object is the carried object, still"
    );

    // CLAIM 2/3: it belongs to THIS room now. Asserted the only way that means
    // anything — by leaving, and finding it retired with everything else that
    // was lying on this room's floor.
    let after = walk_through_a_door(&mut sim);
    assert_ne!(after, TARGET_ROOM);
    assert!(
        sim.world().get_entity(item).is_err(),
        "an object dropped in '{TARGET_ROOM}' is a resident of it, and leaving \
         '{TARGET_ROOM}' must retire it exactly like anything else lying on that \
         floor. Surviving here would mean residency was SUSPENDED rather than \
         restored — an object nothing in the engine can ever collect",
    );
}

/// **The carrier is not assumed to be the player**, and the population that can
/// carry an object across a boundary is not the player population.
///
/// The body driven above happens to be the primary player. What the projection
/// actually asks is where the HOLDER lives: a body promoted out of room scope
/// (the home avatar, or any body possession has taken over) carries its objects
/// with it, and a body that is still a fixture of the room does not. This pins
/// the half the room-crossing case cannot see — that the carried object is still
/// room-SCOPED, so nothing has become immortal.
#[test]
fn a_carried_object_keeps_the_room_lifetime_it_stopped_being_resident_in() {
    let mut sim = fixed_60hz_room_sim(SOURCE_ROOM);
    for _ in 0..10 {
        sim.step(base());
    }
    let (item, _) = authored_item(&mut sim);
    let (x, y) = item_pos(&sim, item);
    sim.teleport_player((x, y));
    sim.step(AgentAction {
        attack: true,
        ..base()
    });
    sim.step(base());

    assert!(
        !custody(&sim, item).expect("the item exists").in_world(),
        "the body is carrying it"
    );
    assert!(
        sim.world()
            .get::<ambition_platformer2d::platformer::lifecycle::RoomScopedEntity>(item)
            .is_some(),
        "a carried object KEEPS its room scope — residency is suspended, the \
         lifetime is not retracted, so every sweep that culls on the scope \
         (the sandbox reset) still sees it",
    );
    assert!(
        sim.world()
            .get::<ambition_platformer2d::platformer::lifecycle::InCustodyOf>(item)
            .is_some(),
        "and it says WHOSE residency it has instead of the room's",
    );

    // The holder the projection named is the body actually driving it, whoever
    // that is — read back rather than assumed to be the player.
    let holder = sim
        .world()
        .get::<ambition_platformer2d::platformer::lifecycle::InCustodyOf>(item)
        .expect("in custody")
        .0;
    let mut driven = sim
        .world_mut()
        .query_filtered::<Entity, With<ambition_platformer2d::actors::features::HeldItem>>();
    let hands: Vec<Entity> = driven.iter(sim.world()).collect();
    assert!(
        hands.contains(&holder),
        "the entity named as the holder is the one with the object in its hand"
    );
}

/// **AN AUTHORED PLACEMENT IS NOT AUTHORED TWICE.**
///
/// Custody let a carried object cross a room boundary alive (the tests above).
/// That opened the hazard `ItemCustody` recorded on the way out: the object
/// survives, and re-entering the room it came from re-runs authored
/// construction, which mints a SECOND occurrence stamped with the same
/// `SimId::placement(..)`. Two live things behind one identity is precisely the
/// failure `SimId` exists to make impossible, and nothing detected it.
///
/// ⭐ **the question construction now asks is a DISPOSITION, not a visit count.**
/// `AuthoredOccurrences` records what became of the occurrence a record minted;
/// re-entry authors a fresh one only for the records whose last occurrence is
/// neither alive elsewhere nor deliberately gone. This drives that end to end:
/// the real pickup, two real transitions, and the real construction commit.
///
/// The measurement is a COUNT over the whole world, because "is the thing in my
/// hands still itself" was already true while the bug was live.
#[test]
fn re_entering_a_room_does_not_re_author_a_placement_that_is_still_in_custody() {
    let mut sim = fixed_60hz_room_sim(SOURCE_ROOM);
    for _ in 0..10 {
        sim.step(base());
    }
    let (item, authored) = authored_item(&mut sim);
    assert_eq!(
        occurrences(&mut sim, &authored),
        vec![item],
        "the room authors its placement exactly once to begin with"
    );

    let holder = pick_it_up(&mut sim, item);

    // ── OUT, AND BACK IN AGAIN ──────────────────────────────────────────────
    assert_eq!(walk_through_the_door_to(&mut sim, TARGET_ROOM), TARGET_ROOM);
    let arrived = walk_through_the_door_to(&mut sim, SOURCE_ROOM);
    assert_eq!(arrived, SOURCE_ROOM, "back where the object was authored");
    for _ in 0..10 {
        sim.step(base());
    }

    // THE CLAIM. Before the ledger, this room had just been rebuilt from its
    // authored records and there were TWO.
    assert_eq!(
        occurrences(&mut sim, &authored),
        vec![item],
        "re-entering the room that authored this placement must not mint a \
         second occurrence of it: the first one is alive, in somebody's hands, \
         and both would claim {authored:?}"
    );
    assert!(
        matches!(
            custody(&sim, item),
            Some(ambition_platformer2d::actors::items::pickup::ItemCustody::Held { holder: h })
                if h == holder
        ),
        "and the one occurrence is the one still being carried"
    );

    // ── PUT IT DOWN AT HOME ─────────────────────────────────────────────────
    // Landing in its own room is the moment a naive "has this room been
    // visited" rule and a disposition disagree: the room was rebuilt without
    // it, and it must not be conjured back the instant it touches the floor.
    throw_it_down(&mut sim);
    assert!(
        custody(&sim, item).is_some_and(|custody| custody.in_world()),
        "Shield+Attack puts the object back in the world"
    );
    assert_eq!(
        occurrences(&mut sim, &authored),
        vec![item],
        "still exactly one, and it is the object that was dropped — nothing \
         resurrects a placement just because it came home"
    );

    // ── AND THE MEMORY IS RETRACTED, NOT LEAKED ─────────────────────────────
    // An object put down is a RESIDENT again, so leaving retires it with
    // everything else on this floor — and the next entry must author it, from
    // its record, exactly once. A ledger that retracted by leaking a row would
    // pass every assertion above and produce ZERO here, which is why the count
    // is asserted rather than "no duplicate".
    assert_eq!(walk_through_the_door_to(&mut sim, TARGET_ROOM), TARGET_ROOM);
    assert_eq!(walk_through_the_door_to(&mut sim, SOURCE_ROOM), SOURCE_ROOM);
    for _ in 0..10 {
        sim.step(base());
    }
    let live = occurrences(&mut sim, &authored);
    assert_eq!(
        live.len(),
        1,
        "an object dropped in its own room is retired with that room and \
         authored again on the next entry — exactly once, and never zero"
    );
    assert!(
        sim.world().get_entity(item).is_err(),
        "the dropped occurrence was retired by leaving, so the one live now is \
         the freshly authored one — the disposition was reset when the object \
         stopped being carried, not remembered forever"
    );
}

/// **THE ORDINARY CASE IS UNTOUCHED**, which is the half that makes the fix
/// above a fix rather than a way to lose an object.
///
/// A placement nobody ever picked up has no disposition, so every entry authors
/// it — and a RESET rebuilds the room from its authored records even while
/// something IS being carried, because a reset destroys the world those
/// occurrences live in, hands included.
#[test]
fn an_untouched_placement_is_authored_on_every_entry_and_a_reset_rebuilds_it() {
    let mut sim = fixed_60hz_room_sim(SOURCE_ROOM);
    for _ in 0..10 {
        sim.step(base());
    }
    let (_, authored) = authored_item(&mut sim);

    // Out and back without ever touching it.
    assert_eq!(walk_through_the_door_to(&mut sim, TARGET_ROOM), TARGET_ROOM);
    assert_eq!(walk_through_the_door_to(&mut sim, SOURCE_ROOM), SOURCE_ROOM);
    for _ in 0..10 {
        sim.step(base());
    }
    assert_eq!(
        occurrences(&mut sim, &authored).len(),
        1,
        "a placement nobody has interacted with is authored on every entry"
    );

    // Now carry it, and reset. ⛔ the reset road states NO dispositions on
    // purpose: it wipes the room AND the hand, so remembering "that one is
    // alive elsewhere" would rebuild the room permanently short of it.
    let (item, _) = authored_item(&mut sim);
    pick_it_up(&mut sim, item);
    sim.reset_episode();
    for _ in 0..10 {
        sim.step(base());
    }
    assert_eq!(
        sim.observation().active_room.as_str(),
        SOURCE_ROOM,
        "the reset returns to the room this session starts in"
    );
    assert!(
        sim.world().get_entity(item).is_err(),
        "a reset destroys a carried object: it never lost its room SCOPE, only \
         its residency"
    );
    assert_eq!(
        occurrences(&mut sim, &authored).len(),
        1,
        "and the room is rebuilt from its authored records, with the placement \
         back on the floor exactly once"
    );
}
