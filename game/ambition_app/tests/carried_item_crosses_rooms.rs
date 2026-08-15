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
