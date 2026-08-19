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
    let door = door_to(sim, target);
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

/// The authored `Door` zone of the ACTIVE room that leads to `target`.
///
/// Split out of [`walk_through_the_door_to`] because a door is also the one
/// position in a room a test can name without reading its geometry: it is
/// authored, in-bounds, and stands somewhere the body can be. The relocation
/// test stands there to put an object down somewhere the room does not author
/// it.
fn door_to(
    sim: &mut Platformer2dSimHarness,
    target: &str,
) -> ambition_platformer2d::world::rooms::LoadingZone {
    let before = sim.observation().active_room.clone();
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
}

/// **THE SANDBOX RESET — the road that rebuilds the room.**
///
/// ⛔ **not `reset_episode()`, and the difference is the whole reason this helper
/// exists.** `reset_episode` presses `ControlFrame::reset_pressed`, which
/// `apply_player_reset_input_system` turns into `reset_sandbox` plus a
/// `ResetRoomFeaturesEvent`: the body returns to spawn and the room's FEATURE
/// state is reset IN PLACE (collected pickups un-collected, actors re-posed,
/// this-attempt loot despawned). It never sweeps `RoomScopedEntity`, never
/// empties a hand, and never re-runs authored construction. A test that drives it
/// and then asserts "the room was rebuilt" is measuring a road it did not take.
///
/// The reset this file's last test is about is `process_new_game_reset_request`:
/// it sweeps `With<RoomScopedEntity>` (deliberately NOT `RoomResident`), commits
/// a fresh start-room plan stating NO dispositions, and its paired
/// `clear_transient_on_sandbox_reset` strips `HeldItem`. Requesting it by its own
/// resource is the only way to execute those two.
fn request_sandbox_reset(sim: &mut Platformer2dSimHarness) {
    sim.world_mut()
        .resource_mut::<ambition_platformer2d::actors::session::reset::NewGameResetRequested>()
        .request();
    // The harness contract for a `world_mut` mutation: a change GGRS input cannot
    // reproduce may not sit behind the rollback cursor. A no-op when this fixture
    // runs without rollback, which is the case here — stated anyway so the fixture
    // can gain a sync-test session without silently going wrong.
    sim.rebase_rollback_history()
        .expect("the pending sandbox reset folds into the rollback baseline");
    // One frame to run `ResetProcessing`, one to flush its deferred commands.
    sim.step(base());
    sim.step(base());
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
/// room-SCOPED.
///
/// ⚠ **it pins the COMPONENT and stops there.** It used to go on to conclude
/// that therefore "nothing has become immortal" — a claim about a SWEEP, from a
/// test that runs no sweep. What actually collects a scoped object is asserted by
/// executing the reset, in
/// [`an_untouched_placement_is_authored_on_every_entry_and_a_reset_rebuilds_it`].
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
         lifetime is not retracted. ⚠ this reads the COMPONENT and infers \
         nothing further: whether the sandbox reset's `With<RoomScopedEntity>` \
         sweep actually collects it is a question only running that reset can \
         answer, and the last test in this file runs it",
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
/// ⭐ **the question construction now asks is WHERE, not a visit count.**
/// `AuthoredOccurrences` records where the occurrence a record minted actually
/// is; re-entry authors a fresh one only for the records whose occurrence is
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

    // ── AND THE MEMORY IS NEITHER LEAKED NOR LOST ───────────────────────────
    // An object put down is a RESIDENT again, so leaving retires it with
    // everything else on this floor — and the next entry must produce it, from
    // its record, exactly once. A ledger that retracted by leaking a row would
    // pass every assertion above and produce ZERO here, which is why the count
    // is asserted rather than "no duplicate".
    //
    // ⚠ **exactly once, and never zero — this says nothing about WHERE.** Since
    // the whereabouts ledger, the object comes back where it was left rather
    // than where the record puts it; that is
    // [`a_relocated_placement_comes_back_where_it_was_left`]'s claim and it is
    // asserted there, against a position this test never measures.
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
         reconstituted on the next entry — exactly once, and never zero"
    );
    assert!(
        sim.world().get_entity(item).is_err(),
        "the dropped occurrence was retired by leaving, so the one live now is \
         a freshly built one — the ledger remembers WHERE the occurrence is, \
         and a room unload does not turn that into a second live copy"
    );
}

/// **AN OBJECT COMES BACK WHERE YOU LEFT IT.**
///
/// The tests above are about an occurrence's EXISTENCE: one, never two, never
/// zero. This one is about its POSITION, which is the half that makes a
/// whereabouts ledger a whereabouts ledger rather than a suppression list. Carry
/// an authored object across the room, put it down somewhere the room does not
/// author it, leave — which destroys it with everything else on that floor — and
/// come back. The occurrence that returns must be the same one, at the place it
/// was left.
///
/// ⭐ **run against a do-nothing implementation, this fails on its last
/// assertion and only there.** Construction that ignores the ledger's `Placed`
/// row rebuilds the room from the authored record, which puts the object back at
/// the coordinates LDtk gives it — so the object exists, is unique, is lying in
/// the world, and is in the WRONG PLACE. The precondition below is what makes
/// that a failure rather than a coincidence: it refuses to run the claim at all
/// unless the drop actually moved the object.
///
/// ⚠ **the returning occurrence is a DIFFERENT ENTITY, and must be.** The room
/// unload destroyed the one that was lying there; what identifies it as the same
/// occurrence is its `SimId`, which is the whole distinction between authored
/// definition identity and runtime occurrence identity.
#[test]
fn a_relocated_placement_comes_back_where_it_was_left() {
    let mut sim = fixed_60hz_room_sim(SOURCE_ROOM);
    for _ in 0..10 {
        sim.step(base());
    }
    let (item, authored) = authored_item(&mut sim);
    let authored_pos = item_pos(&sim, item);

    // ── CARRY IT ACROSS THE ROOM AND PUT IT DOWN ────────────────────────────
    // The door is used as a POSITION here, not as an exit: it is the one spot in
    // a room a test can name without reading the room's geometry — authored,
    // in-bounds, and somewhere a body can stand.
    pick_it_up(&mut sim, item);
    let door = door_to(&mut sim, TARGET_ROOM);
    let stand = door.aabb.center();
    sim.teleport_player((stand.x, stand.y));
    throw_it_down(&mut sim);
    assert!(
        custody(&sim, item).is_some_and(|custody| custody.in_world()),
        "Shield+Attack puts the object back in the world"
    );
    let dropped_pos = item_pos(&sim, item);

    // THE PRECONDITION. Everything below measures "back where it was left"
    // against "back where it is authored", so a fixture in which those two are
    // the same place proves nothing at all and must say so loudly rather than
    // pass.
    let moved = (dropped_pos.0 - authored_pos.0).abs() + (dropped_pos.1 - authored_pos.1).abs();
    assert!(
        moved > 8.0,
        "this test needs the object to end up somewhere the room does NOT \
         author it: authored at {authored_pos:?}, dropped at {dropped_pos:?}. \
         Nothing below can fail while those are the same place"
    );

    // ── LEAVE (which destroys it) AND COME BACK ─────────────────────────────
    assert_eq!(walk_through_the_door_to(&mut sim, TARGET_ROOM), TARGET_ROOM);
    assert!(
        sim.world().get_entity(item).is_err(),
        "an object lying on this room's floor is a resident of it and is \
         retired by leaving — if it survived, the test below would be measuring \
         a survivor rather than a reconstitution"
    );
    assert_eq!(walk_through_the_door_to(&mut sim, SOURCE_ROOM), SOURCE_ROOM);
    for _ in 0..10 {
        sim.step(base());
    }

    // THE CLAIM.
    let live = occurrences(&mut sim, &authored);
    assert_eq!(
        live.len(),
        1,
        "the room owes exactly one occurrence of this record — never two, and \
         never zero"
    );
    let back = live[0];
    assert_ne!(
        back, item,
        "the returning occurrence is a fresh entity built from the record: the \
         one that was lying here died with the room"
    );
    assert!(
        custody(&sim, back).is_some_and(|custody| custody.in_world()),
        "and it is lying in the world, collectible, not stranded in a hand"
    );
    let back_pos = item_pos(&sim, back);
    let drift = (back_pos.0 - dropped_pos.0).abs() + (back_pos.1 - dropped_pos.1).abs();
    assert!(
        drift < 1.0,
        "an object put down at {dropped_pos:?} must come back THERE, not at the \
         {authored_pos:?} its record names. It came back at {back_pos:?} — the \
         room was rebuilt from its authored records and the world's memory of \
         where the occurrence actually is was not consulted"
    );
}

/// **AN OBJECT LEFT IN ANOTHER ROOM STAYS IN THAT ROOM.**
///
/// Its sibling above proves the whole mechanism for a room that AUTHORS the
/// record. This one drops the object in a room that does NOT author it, which is
/// the case that forced room construction to stop being a pure function of one
/// `RoomSpec`: reinstating this occurrence in `portal_bridge` means building a
/// record that lives in `blink_run`, so construction reconstructs current
/// residency from the world's DEFINITIONS plus the authoritative disposition of
/// every occurrence, rather than from the one room in front of it.
///
/// ⛔⛔ **THE TWO HALVES ARE ONE CLAIM, AND THIS TEST ASSERTS BOTH.** Suppressing
/// the record in `blink_run` without reconstituting the occurrence in
/// `portal_bridge` leaves the object in NO room, permanently — a deletion bug
/// traded for a duplication bug. So:
///
/// * a run that reinstates but forgets to suppress fails at `blink_run`, which
///   would hold a second live occurrence of one identity;
/// * a run that suppresses but forgets to reinstate fails at `portal_bridge`,
///   which would hold none.
///
/// Neither half can regress alone and leave this green, which is the only reason
/// it is safe to have landed them together.
#[test]
fn a_placement_dropped_in_another_room_stays_there() {
    let mut sim = fixed_60hz_room_sim(SOURCE_ROOM);
    for _ in 0..10 {
        sim.step(base());
    }
    let (item, authored) = authored_item(&mut sim);
    let authored_pos = item_pos(&sim, item);

    // ── CARRY IT NEXT DOOR AND DROP IT THERE ────────────────────────────────
    pick_it_up(&mut sim, item);
    assert_eq!(walk_through_the_door_to(&mut sim, TARGET_ROOM), TARGET_ROOM);
    // Let go standing in the door back to `blink_run`: it is the one position in
    // `portal_bridge` this test can name without reading the room's geometry —
    // authored, in bounds, somewhere a body can stand — and it is where the body
    // leaves from anyway. Dropping wherever the crossing happened to deposit the
    // body would put the object at an arrival point that may be mid-air.
    let stand = door_to(&mut sim, SOURCE_ROOM).aabb.center();
    sim.teleport_player((stand.x, stand.y));
    throw_it_down(&mut sim);
    assert!(
        custody(&sim, item).is_some_and(|custody| custody.in_world()),
        "Shield+Attack puts the object back in the world"
    );

    // ── LET IT COME TO REST, AND PROVE IT HAS ───────────────────────────────
    // What the world remembers is where the object last WAS when the room
    // unloaded, so a position measured mid-fall would make the claim below a
    // race against the physics rather than a claim about reconstruction.
    for _ in 0..60 {
        sim.step(base());
    }
    let dropped_pos = item_pos(&sim, item);
    for _ in 0..10 {
        sim.step(base());
    }
    let still = item_pos(&sim, item);
    let creep = (still.0 - dropped_pos.0).abs() + (still.1 - dropped_pos.1).abs();
    assert!(
        creep < 0.5,
        "the dropped object must be at rest before the room is unloaded: it was \
         at {dropped_pos:?} and ten frames later at {still:?}. Nothing below can \
         mean anything while it is still moving"
    );
    // And it is somewhere the AUTHORED record does not put it, so "came back
    // where it was left" and "came back where the record says" are different
    // answers and the last assertion can fail.
    let moved = (dropped_pos.0 - authored_pos.0).abs() + (dropped_pos.1 - authored_pos.1).abs();
    assert!(
        moved > 8.0,
        "authored at {authored_pos:?}, dropped at {dropped_pos:?} — this test \
         needs those to be different places"
    );

    // ── UNLOAD THE ROOM IT IS NOW IN, AND VISIT THE ONE THAT AUTHORS IT ─────
    assert_eq!(walk_through_the_door_to(&mut sim, SOURCE_ROOM), SOURCE_ROOM);
    for _ in 0..10 {
        sim.step(base());
    }
    assert!(
        occurrences(&mut sim, &authored).is_empty(),
        "the room that AUTHORS this placement must not mint it again: the \
         occurrence it minted is lying in '{TARGET_ROOM}', and re-authoring it \
         here would put two live things behind one identity the moment the \
         player walks back"
    );

    // ── AND BACK NEXT DOOR: THE SAME OCCURRENCE, WHERE IT WAS LEFT ──────────
    assert_eq!(walk_through_the_door_to(&mut sim, TARGET_ROOM), TARGET_ROOM);
    for _ in 0..10 {
        sim.step(base());
    }
    let live = occurrences(&mut sim, &authored);
    assert_eq!(
        live.len(),
        1,
        "the occurrence was left lying in '{TARGET_ROOM}' and must reconstitute \
         there — exactly once. ZERO here is the deletion this whole design \
         exists to prevent: '{SOURCE_ROOM}' has already been told not to author \
         it, so if this room does not build it the object is in no room at all"
    );
    let back = live[0];
    assert_ne!(
        back, item,
        "the returning occurrence is a fresh entity built from the record — the \
         one that was lying here died when '{TARGET_ROOM}' unloaded, and its \
         `SimId` is what makes this the same occurrence"
    );
    assert!(
        custody(&sim, back).is_some_and(|custody| custody.in_world()),
        "lying in the world, collectible, the way it was left"
    );
    let back_pos = item_pos(&sim, back);
    let drift = (back_pos.0 - dropped_pos.0).abs() + (back_pos.1 - dropped_pos.1).abs();
    assert!(
        drift < 1.0,
        "it must come back at {dropped_pos:?}, where it was dropped; it came \
         back at {back_pos:?}. A record built at the coordinates ITS OWN room \
         gives it ({authored_pos:?}) is the failure this measures"
    );
}

/// **THE ORDINARY CASE IS UNTOUCHED**, which is the half that makes the fix
/// above a fix rather than a way to lose an object.
///
/// A placement nobody ever picked up has no disposition, so every entry authors
/// it — and a RESET rebuilds the room from its authored records even while
/// something IS being carried, because a reset destroys the world those
/// occurrences live in, hands included.
///
/// ⛔ **"a reset" here means the SANDBOX reset and only that** — see
/// [`request_sandbox_reset`]. This test first drove `reset_episode()`, which is a
/// different product (reset the room's feature state and put the body back at
/// spawn, in place) and correctly leaves a carried object alone; the failure that
/// produced was the test naming a road it was not on, not the engine keeping an
/// object it should have destroyed.
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
    let holder = pick_it_up(&mut sim, item);
    request_sandbox_reset(&mut sim);
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
    // ⭐ AND THE HAND WENT WITH IT. Destroying the object while leaving a
    // `HeldItem` pointing at the hole is the failure the reset's own comment
    // argues against ("this same function empties the hand a few lines below"),
    // and it is invisible to a count over `SimId`. Asked of THE carrier rather
    // than of the world, because a rebuilt room may legitimately author other
    // bodies with something in hand.
    assert!(
        sim.world()
            .get::<ambition_platformer2d::actors::features::HeldItem>(holder)
            .is_none(),
        "the reset that destroyed the object also emptied the hand holding it"
    );
    // ⭐ **AND THE ROOM CAME BACK WHOLE.** Counted AND identified, because a
    // count alone is satisfied by a reset that did nothing at all: the object
    // still in the hand would be the one occurrence, which is exactly how a
    // sibling assertion in this file once passed vacuously. A do-nothing reset
    // fails both halves below — the survivor would be `item`, and it would be
    // `Held` rather than lying in the world.
    let rebuilt = occurrences(&mut sim, &authored);
    assert_eq!(
        rebuilt.len(),
        1,
        "and the room is rebuilt from its authored records, with the placement \
         back on the floor exactly once"
    );
    assert_ne!(
        rebuilt[0], item,
        "the one occurrence is a FRESH one built from the authored record, not \
         the carried object surviving a reset that did nothing"
    );
    assert!(
        custody(&sim, rebuilt[0]).is_some_and(|custody| custody.in_world()),
        "the rebuilt placement is lying in the world, the way the record \
         authors it — not in somebody's hand"
    );
}

// ---------------------------------------------------------------------------
// **The three-way composition nothing exercised** (2026-08-19).
//
// Every test above drives the HOME AVATAR: it picks the item up, walks it
// through the door, and the object survives because
// `project_custody_onto_residency` makes a held object non-resident when its
// holder is not `RoomScopedEntity` — which the session-scoped home avatar never
// is.
//
// ⚠ **I predicted this would FAIL and it passed, for a reason that then turned
// out to be a bug of its own** — the whole sequence is worth keeping because
// each step corrected the one before it.
//
// 1. PREDICTED: an authored actor is room-scoped, so the projection reads it as
//    *"a room fixture's hand"*, leaves the object resident, and the room change
//    retires it while `carry_body` carries the body.
// 2. MEASURED: `holder_room_scoped=false`. `possess_target` was PROMOTING the
//    body out of room scope into session scope, and that promotion — made to
//    save the BODY — was what made the custody rule true of the ITEM.
// 3. AND THAT PROMOTION WAS ITSELF THE DEFECT. `InCustodyOf`'s doc says *"the
//    LIFETIME is unchanged, and that is deliberate … no query that requires the
//    scope silently loses sight of it"*, and the occurrence ledger's projection
//    requires exactly that scope. So a possessed body was invisible to the
//    ledger and its home room authored a SECOND copy of it —
//    `an_authored_actor_carried_out_of_its_room_and_back_does_not_meet_a_copy`
//    counted two.
//
// ⭐ **possession suspends RESIDENCY now, like everything else that travels**:
// it keeps `RoomScopedEntity` and adds `InCustodyOf`, and the residency
// projection asks the `RoomResident` roster instead of restating it — which
// makes custody TRANSITIVE, and is what carries this test's object.
//
// ⛔⛔ two subsystems that never name each other still agree by way of one fact,
// and this test is what makes that agreement visible: it went RED the moment the
// promotion was removed, and stayed red until the projection stopped asking a
// proxy question. Neither change could land alone.
// ---------------------------------------------------------------------------

const POSSESS_TARGET_ID: &str = "carry_while_possessed";

/// Possess an actor standing beside the home avatar, and return its entity.
///
/// Hold Down+Interact across several commit windows — the mechanic commits on a
/// whole hold window, and the target weaves around its own attack range, so a
/// single window can land just out of the possession radius.
fn possess_an_actor(sim: &mut Platformer2dSimHarness) -> Entity {
    use ambition_platformer2d::actors::abilities::traversal::possession::PossessionState;
    use ambition_platformer2d::actors::features::FeatureId;
    use ambition_platformer2d::entity_catalog::placements::CharacterBrain;

    let here = {
        let world = sim.world_mut();
        let mut q = world
            .query_filtered::<&ambition_platformer2d::actors::actor::BodyKinematics,
                ambition_platformer2d::actors::actor::PrimaryPlayerOnly>();
        q.single(world).expect("primary player").pos
    };
    sim.spawn_enemy_character_at(
        POSSESS_TARGET_ID,
        "Carry Target",
        (here.x + 60.0, here.y),
        (14.0, 23.0),
        CharacterBrain::Custom("cellular_automaton_fighter".to_string()),
        "perfect_cellular_automaton",
    );
    let actor = {
        let world = sim.world_mut();
        let mut q = world.query::<(Entity, &FeatureId)>();
        q.iter(world)
            .find(|(_, id)| id.as_str() == POSSESS_TARGET_ID)
            .map(|(entity, _)| entity)
            .expect("the spawned actor is present")
    };
    for i in 0..900 {
        sim.step(AgentAction {
            move_y: 1.0,
            interact: i == 0,
            interact_held: true,
            ..base()
        });
        if sim.world_mut().resource::<PossessionState>().possessed == Some(actor) {
            return actor;
        }
    }
    panic!("setup: holding Down+Interact never possessed the actor");
}

/// Put a body somewhere, through its kinematics — the possessed body is not the
/// primary player, so `teleport_player` moves the wrong entity.
fn place_body(sim: &mut Platformer2dSimHarness, body: Entity, at: (f32, f32)) {
    let world = sim.world_mut();
    let mut kin = world
        .get_mut::<ambition_platformer2d::actors::actor::BodyKinematics>(body)
        .expect("the body has kinematics");
    kin.pos = ambition_platformer2d::engine_core::Vec2::new(at.0, at.1);
    kin.vel = ambition_platformer2d::engine_core::Vec2::ZERO;
}

/// **An object carried by a POSSESSED body goes through the door with it.**
///
/// ⛔ the failure this catches is silent and asymmetric: the body arrives, the
/// room is right, the hand still says it is holding something, and the OBJECT is
/// gone — retired by the room change because its holder was room-scoped. A test
/// asserting only "the room changed" or only "the body arrived" passes straight
/// through it, which is why both of those are asserted here as SETUP and the
/// object's survival is the claim.
#[test]
fn an_item_carried_by_a_possessed_body_survives_the_door_too() {
    use ambition_platformer2d::actors::items::pickup::ItemCustody;

    let mut sim = fixed_60hz_room_sim(SOURCE_ROOM);
    for _ in 0..10 {
        sim.step(base());
    }
    assert_eq!(sim.observation().active_room.as_str(), SOURCE_ROOM);

    let (item, authored) = authored_item(&mut sim);
    let actor = possess_an_actor(&mut sim);

    // ── THE POSSESSED BODY picks it up ──────────────────────────────────────
    let (x, y) = item_pos(&sim, item);
    place_body(&mut sim, actor, (x, y));
    sim.step(AgentAction {
        attack: true,
        ..base()
    });
    sim.step(base());
    let holder = match custody(&sim, item) {
        Some(ItemCustody::Held { holder }) => holder,
        other => panic!(
            "the possessed body pressed Attack on the item and did not take custody \
             ({other:?}) — the pickup resolves through `ControlledSubject`, so this is \
             a different defect from the one under test and must not be read as one"
        ),
    };
    assert_eq!(
        holder, actor,
        "the POSSESSED body must be the custodian, not the vacated home avatar — \
         otherwise this test crosses the door holding the wrong thing and proves \
         nothing about a room-scoped holder"
    );

    // ── AND CARRIES IT THROUGH A REAL DOOR ──────────────────────────────────
    let before = sim.observation().active_room.clone();
    let door = door_to(&mut sim, TARGET_ROOM);
    let centre = door.aabb.center();
    place_body(&mut sim, actor, (centre.x, centre.y));
    let mut arrived = None;
    for _ in 0..60 {
        let room = sim
            .step(AgentAction {
                interact: true,
                interact_held: true,
                ..base()
            })
            .active_room;
        if room != before {
            arrived = Some(room);
            break;
        }
    }
    let arrived = arrived.expect(
        "the possessed body held interact in the door for 60 frames and the room never \
         changed; it cannot use a door at all while carrying, which is a different bug",
    );
    assert_eq!(arrived, TARGET_ROOM);

    // CLAIM: the object survived, as itself, still held.
    assert!(
        sim.world().get_entity(item).is_ok(),
        "the object a POSSESSED body carried was destroyed by the room transition. \
         `project_custody_onto_residency` makes a held object non-resident only when \
         its holder is not a `RoomResident`, and what makes that true of a possessed \
         body is `possess_target` giving it `InCustodyOf`. If either half moved, the \
         body still crosses (carry_body carries it) and the thing in its hand is \
         retired at the door"
    );
    assert_eq!(
        sim.world().get::<SimId>(item),
        Some(&authored),
        "and it is the object it was authored as — no despawn, no re-mint"
    );
    assert!(
        matches!(custody(&sim, item), Some(ItemCustody::Held { holder: h }) if h == actor),
        "it arrived still in the possessed body's custody"
    );
    assert_eq!(
        occurrences(&mut sim, &authored).len(),
        1,
        "exactly one occurrence claims the authored identity after the crossing"
    );
}
/// **An authored ACTOR carried out of its room and back does not meet a second
/// copy of itself.**
///
/// ⛔⛔ **THE OCCURRENCE LEDGER IS ITEMS-ONLY, and possession makes a body
/// travel exactly like a carried object.** `record_placed_ground_items` is the
/// single writer of `AuthoredOccurrences`; nothing records where a POSSESSED
/// actor went. So the actor's home room, rebuilt on re-entry, consults an outlook
/// that has never heard of it, authors it again, and the world holds two live
/// entities behind one `SimId::placement(..)` — the state
/// `ActorConstructionPlan::prepare` refuses as `IdentityAlreadyLive` when it is
/// told, and cannot refuse when it is not.
///
/// ⚠ **possession is CUSTODY OF A BODY**, which is why the fix is the same
/// vocabulary and not a new one: while a participant is driving an authored
/// actor, that occurrence is `InCustody` and its home room must not re-author it
/// — precisely the rule a carried axe already follows.
///
/// ⚠ the player is re-anchored onto the target every frame during the hold: the
/// enemy fights back, and a staggered player drifts out of the 150 px possession
/// radius. That is a spacing race, not the thing under test.
#[test]
fn an_authored_actor_carried_out_of_its_room_and_back_does_not_meet_a_copy() {
    use ambition_platformer2d::actors::abilities::traversal::possession::PossessionState;
    use ambition_platformer2d::actors::actor::BodyKinematics;
    use ambition_platformer2d::characters::brain::Brain;

    const HOME: &str = "vertical_shaft";
    const AWAY: &str = "central_hub_complex";

    let mut sim = fixed_60hz_room_sim(HOME);
    for _ in 0..20 {
        sim.step(base());
    }
    let (actor, id) = {
        let world = sim.world_mut();
        let mut q = world.query::<(Entity, &SimId, &Brain, &BodyKinematics)>();
        q.iter(world)
            .find(|(_, id, _, _)| id.as_str().starts_with("placement:EnemySpawn"))
            .map(|(e, id, _, _)| (e, id.clone()))
            .expect("'vertical_shaft' authors an enemy with a placement identity")
    };
    assert_eq!(
        occurrences(&mut sim, &id).len(),
        1,
        "setup: exactly one occurrence of the authored actor before anything happens"
    );

    let mut possessed = false;
    for i in 0..900 {
        if let Some(here) = sim
            .world()
            .get::<BodyKinematics>(actor)
            .map(|k| (k.pos.x, k.pos.y))
        {
            sim.teleport_player(here);
        }
        sim.step(AgentAction {
            move_y: 1.0,
            interact: i == 0,
            interact_held: true,
            ..base()
        });
        if sim.world_mut().resource::<PossessionState>().possessed == Some(actor) {
            possessed = true;
            break;
        }
    }
    assert!(
        possessed,
        "setup: holding Down+Interact on the authored actor never possessed it, so \
         nothing below is about a travelling body"
    );

    for (target, leg) in [(AWAY, "out"), (HOME, "back")] {
        let before = sim.observation().active_room.clone();
        let door = door_to(&mut sim, target);
        let centre = door.aabb.center();
        place_body(&mut sim, actor, (centre.x, centre.y));
        let mut arrived = None;
        for _ in 0..90 {
            let room = sim
                .step(AgentAction {
                    interact: true,
                    interact_held: true,
                    ..base()
                })
                .active_room;
            if room != before {
                arrived = Some(room);
                break;
            }
        }
        assert_eq!(
            arrived.as_deref(),
            Some(target),
            "setup: the {leg} leg never left '{before}', so the round trip did not happen"
        );
    }

    assert!(
        sim.world().get_entity(actor).is_ok(),
        "the possessed body did not survive its own round trip"
    );
    assert_eq!(
        occurrences(&mut sim, &id).len(),
        1,
        "'{HOME}' authored a SECOND copy of the actor you are driving. The occurrence \
         ledger is written only by `record_placed_ground_items`, so a possessed body \
         that leaves its authoring room is remembered nowhere, and the rebuild on \
         re-entry has no disposition telling it the occurrence is already alive in \
         somebody's hands"
    );
}

/// **A checkpoint taken while possessing does not turn the driven body into an
/// item.**
///
/// ⭐ **this is a consequence of the fix above, measured rather than assumed.**
/// A possessed body wears `InCustodyOf` now, and `capture_custody_baseline`
/// records `(&SimId, &InCustodyOf)` for ANY room-scoped entity — so the driven
/// actor genuinely enters the checkpoint's custody baseline, as
/// `placement:EnemySpawn-4513 <- slot:0`. That row is TRUE (a participant does
/// have custody of that body), and the population it joins is one the ITEM
/// domain's restore was written for.
///
/// ⛔ the risk that had to be checked: `restore_custody_to_checkpoint` has a
/// materialization arm for baseline rows with no live occurrence behind them,
/// and an actor is not a `GroundItem`, so the row looks "missing" to it. It is
/// harmless because the arm's two describers — the checkpoint's minted
/// descriptions and the world's authored ground-item records — can describe
/// neither an enemy placement, so it falls through. This test is what keeps that
/// harmless: a describer that grew a broader arm would start manufacturing a
/// ground item for a body.
#[test]
fn a_checkpoint_taken_while_possessing_does_not_manufacture_an_item() {
    use ambition_platformer2d::actors::abilities::traversal::possession::PossessionState;
    use ambition_platformer2d::actors::actor::BodyKinematics;
    use ambition_platformer2d::characters::brain::Brain;
    use ambition_platformer2d::platformer::lifecycle::{
        CheckpointCommitted, CustodyBaseline, ResetToCheckpoint,
    };

    let mut sim = fixed_60hz_room_sim("vertical_shaft");
    for _ in 0..20 {
        sim.step(base());
    }
    let (actor, id) = {
        let world = sim.world_mut();
        let mut q = world.query::<(Entity, &SimId, &Brain, &BodyKinematics)>();
        q.iter(world)
            .find(|(_, id, _, _)| id.as_str().starts_with("placement:EnemySpawn"))
            .map(|(e, id, _, _)| (e, id.clone()))
            .expect("'vertical_shaft' authors an enemy with a placement identity")
    };
    for i in 0..900 {
        if let Some(here) = sim
            .world()
            .get::<BodyKinematics>(actor)
            .map(|k| (k.pos.x, k.pos.y))
        {
            sim.teleport_player(here);
        }
        sim.step(AgentAction {
            move_y: 1.0,
            interact: i == 0,
            interact_held: true,
            ..base()
        });
        if sim.world_mut().resource::<PossessionState>().possessed == Some(actor) {
            break;
        }
    }
    assert_eq!(
        sim.world_mut().resource::<PossessionState>().possessed,
        Some(actor),
        "setup: nothing below is about a driven body unless one is being driven"
    );

    sim.world_mut().write_message(CheckpointCommitted);
    sim.step(base());
    sim.step(base());

    // ⚠ asserted, not assumed: if the row stopped forming, the reset below would
    // prove nothing about what the restore does with one.
    let rows: Vec<String> = sim
        .world()
        .resource::<CustodyBaseline>()
        .rows()
        .map(|(occurrence, custodian)| format!("{} <- {}", occurrence.as_str(), custodian.as_str()))
        .collect();
    assert!(
        rows.iter().any(|row| row.starts_with(id.as_str())),
        "the driven body did not enter the custody baseline at all, so this test is \
         not exercising the arm it exists for; rows were {rows:?}"
    );

    sim.world_mut().write_message(ResetToCheckpoint);
    for _ in 0..5 {
        sim.step(base());
    }

    assert!(
        sim.world().get_entity(actor).is_ok(),
        "the reset destroyed the body that was being driven"
    );
    assert_eq!(
        occurrences(&mut sim, &id).len(),
        1,
        "the checkpoint restore manufactured a second occurrence for the driven body \
         — its materialization arm found a describer for an ACTOR and built a ground \
         item out of it"
    );
}

/// **A save taken WHILE POSSESSING cannot suppress the enemy on load.**
///
/// ⛔⛔ **this is the save/load consequence of making possession a custody**, and
/// it had to be measured because the failure would be silent and permanent. A
/// driven body's occurrence goes into `AuthoredOccurrences` as `InCustody` —
/// that is the fix, it is what stops the home room authoring a second copy — and
/// `persist_durable_horizon_to_save` mirrors the ledger to disk. So a save taken
/// mid-possession carries a row saying an enemy is in somebody's hands. A FRESH
/// PROCESS then adopts that row while nobody is possessing anything, and if the
/// row survived, the room build would suppress an enemy that nobody is holding
/// and it would be gone from the world.
///
/// ⭐ **it does not survive, and the reason is `republish_custody`'s own
/// contract**: *"RETRACT BY RESETTING, NEVER BY REMOVING … the whole leg is
/// replaced by what is true now"*. `project_custody_onto_authored_occurrences`
/// runs every tick, ungated, and republishes the custody leg from LIVE state —
/// so a row with no live holder is dropped on the first tick, before any room
/// build can act on it. Measured at one tick.
///
/// ⚠ **the assertion is the RETRACTION, not the absence.** A test that only
/// checked "the enemy exists after loading" would pass on a world that never
/// wrote the row in the first place — i.e. against the bug this whole change
/// fixes.
#[test]
fn a_custody_row_with_nobody_holding_it_is_retracted_before_a_room_can_act_on_it() {
    use ambition_platformer2d::actors::abilities::traversal::possession::PossessionState;
    use ambition_platformer2d::actors::actor::BodyKinematics;
    use ambition_platformer2d::characters::brain::Brain;
    use ambition_platformer2d::platformer::lifecycle::AuthoredOccurrences;

    let mut sim = fixed_60hz_room_sim("vertical_shaft");
    for _ in 0..30 {
        sim.step(base());
    }
    let (actor, id) = {
        let world = sim.world_mut();
        let mut q = world.query::<(Entity, &SimId, &Brain, &BodyKinematics)>();
        q.iter(world)
            .find(|(_, id, _, _)| id.as_str().starts_with("placement:EnemySpawn"))
            .map(|(e, id, _, _)| (e, id.clone()))
            .expect("'vertical_shaft' authors an enemy with a placement identity")
    };
    for i in 0..900 {
        if let Some(here) = sim
            .world()
            .get::<BodyKinematics>(actor)
            .map(|k| (k.pos.x, k.pos.y))
        {
            sim.teleport_player(here);
        }
        sim.step(AgentAction {
            move_y: 1.0,
            interact: i == 0,
            interact_held: true,
            ..base()
        });
        if sim.world_mut().resource::<PossessionState>().possessed == Some(actor) {
            break;
        }
    }
    assert_eq!(
        sim.world_mut().resource::<PossessionState>().possessed,
        Some(actor),
        "setup: nothing below is about a driven body unless one is being driven"
    );
    sim.step(base());
    sim.step(base());

    // ⭐ **BOTH TERMS, and this is the one that matters**: the row is WRITTEN.
    // Without it there is nothing for the retraction to retract, and this test
    // would agree with the duplication bug.
    let written: Vec<String> = sim
        .world()
        .resource::<AuthoredOccurrences>()
        .rows()
        .map(|(occurrence, whereabouts)| format!("{} = {whereabouts:?}", occurrence.as_str()))
        .collect();
    assert!(
        written
            .iter()
            .any(|row| row.starts_with(id.as_str()) && row.ends_with("InCustody")),
        "the driven body's occurrence is not recorded as being in custody, so the \
         home room is free to author a second copy of it; ledger was {written:?}"
    );

    // A FRESH PROCESS: the file's row is adopted while nobody possesses anything.
    *sim.world_mut().resource_mut::<PossessionState>() = PossessionState::default();
    sim.step(base());

    let after: Vec<String> = sim
        .world()
        .resource::<AuthoredOccurrences>()
        .rows()
        .map(|(occurrence, whereabouts)| format!("{} = {whereabouts:?}", occurrence.as_str()))
        .collect();
    assert!(
        !after.iter().any(|row| row.starts_with(id.as_str())),
        "a custody row survived a tick with nobody holding the occurrence. On a load \
         that row suppresses the enemy in the room build and it is gone from the \
         world permanently; ledger was {after:?}"
    );
    assert_eq!(
        occurrences(&mut sim, &id).len(),
        1,
        "and exactly one occurrence remains"
    );
}

/// **A MOUNT YOU ARE RIDING GOES THROUGH THE DOOR WITH YOU.**
///
/// ⛔⛔ **the third population of one rule, and it was the last one still
/// broken.** An object in a travelling body's custody rides across; a possessed
/// body itself rides across; a MOUNT the possessed body is sitting on did not —
/// it is an authored room actor, nothing suspended its residency, and the room
/// change retired it at the door. Measured in `pirate_sky_lookout`: the rider
/// arrived in `pirate_cove`, `mount_alive=false`, and the rider still carried
/// `RidingOn` naming the dead mount.
///
/// ⭐ **the rule is TRANSITIVE and this is what makes it so.** The mount is in
/// its rider's custody exactly when that rider is itself not a `RoomResident` —
/// which is true while a participant is driving it. An AI-piloted sky rider is
/// room furniture and stays resident, mount and all.
///
/// ⚠ the fixture aims the MOUNT so the RIDER lands in the door: the transition
/// tests the controlled subject's box, and `sync_riders_to_mounts` snaps the
/// rider to a saddle 77 px above the mount's centre. Placing the mount in the
/// door puts the rider above it, which is why a one-shot placement never crosses.
#[test]
fn a_mount_you_are_riding_crosses_the_door_with_you() {
    use ambition_platformer2d::actors::abilities::traversal::possession::PossessionState;
    use ambition_platformer2d::actors::actor::BodyKinematics;
    use ambition_platformer2d::actors::features::RidingOn;
    use ambition_platformer2d::characters::brain::Brain;

    let mut sim = fixed_60hz_room_sim("pirate_sky_lookout");
    for _ in 0..30 {
        sim.step(base());
    }

    let (rider, mount) = {
        let world = sim.world_mut();
        let mut q = world.query::<(Entity, &SimId, &RidingOn, &Brain)>();
        q.iter(world)
            .next()
            .map(|(entity, _, riding, _)| (entity, riding.mount))
            .expect("'pirate_sky_lookout' authors a rider on a mount")
    };
    let mount_id = sim
        .world()
        .get::<SimId>(mount)
        .cloned()
        .expect("the authored mount has a placement identity");

    let mut possessed = false;
    for i in 0..900 {
        if let Some(here) = sim
            .world()
            .get::<BodyKinematics>(rider)
            .map(|k| (k.pos.x, k.pos.y))
        {
            sim.teleport_player(here);
        }
        sim.step(AgentAction {
            move_y: 1.0,
            interact: i == 0,
            interact_held: true,
            ..base()
        });
        if sim.world_mut().resource::<PossessionState>().possessed == Some(rider) {
            possessed = true;
            break;
        }
    }
    assert!(
        possessed,
        "setup: the rider was never possessed, so nothing below is about a PILOTED \
         mount and an AI-piloted one is supposed to stay put"
    );

    let before = sim.observation().active_room.clone();
    let door = {
        let world = sim.world_mut();
        let mut q = world.query::<&ambition_platformer2d::actors::rooms::RoomSet>();
        let set = q.iter(world).next().expect("a room set");
        set.active_loading_zones()
            .iter()
            .find(|zone| {
                zone.activation == ambition_platformer2d::world::rooms::LoadingZoneActivation::Door
            })
            .cloned()
            .expect("'pirate_sky_lookout' authors a door")
    };
    let centre = door.aabb.center();
    place_body(&mut sim, mount, (centre.x, centre.y));
    sim.step(base());
    let saddle = {
        let m = sim
            .world()
            .get::<BodyKinematics>(mount)
            .map(|k| k.pos)
            .unwrap();
        let r = sim
            .world()
            .get::<BodyKinematics>(rider)
            .map(|k| k.pos)
            .unwrap();
        (r.x - m.x, r.y - m.y)
    };
    let aim = (centre.x - saddle.0, centre.y - saddle.1);

    let mut arrived = None;
    for _ in 0..120 {
        place_body(&mut sim, mount, aim);
        let room = sim
            .step(AgentAction {
                interact: true,
                interact_held: true,
                ..base()
            })
            .active_room;
        if room != before {
            arrived = Some(room);
            break;
        }
    }
    let arrived = arrived
        .expect("the piloted mount never left the room, so nothing below is about a crossing");

    assert!(
        sim.world().get_entity(rider).is_ok(),
        "the rider did not survive its own crossing into '{arrived}'"
    );
    assert!(
        sim.world().get_entity(mount).is_ok(),
        "the MOUNT was destroyed at the door of '{before}' while the rider crossed \
         into '{arrived}'. A mount whose rider is travelling is in that rider's \
         custody and must not be a `RoomResident` — the same rule that carries a \
         held axe and a possessed body, one link further along"
    );
    assert_eq!(
        sim.world().get::<SimId>(mount),
        Some(&mount_id),
        "and it is the mount it was authored as — no despawn, no re-mint"
    );
    assert_eq!(
        occurrences(&mut sim, &mount_id).len(),
        1,
        "exactly one occurrence claims the mount's authored identity after the crossing"
    );
}

/// **EVERYTHING ATTACHED TO A TRAVELLER CROSSES WITH IT, THREE LINKS DEEP.**
///
/// `gnu_ton_arena` authors a boss riding a mount, and that mount has hands. So
/// possessing the boss makes the chain **rider → mount → limbs**, and each link
/// is a separate relation (`RidingOn`, then `Limb`). Measured before this test
/// passed: the rider and the mount crossed into `hall_of_bosses` and the mount
/// arrived HANDLESS — `limbs_alive=[false, false]`.
///
/// ⭐ **this is the test that forced a FIXPOINT rather than a third ordered
/// arm.** An ordered pass (possessed, then mounts, then limbs) happens to be
/// right for this depth and encodes it; content chooses the depth, so the
/// closure iterates until nothing changes.
///
/// ⚠ the fixture aims the MOUNT so the RIDER lands in the door — the transition
/// tests the controlled subject's box and the rider is snapped to a saddle above
/// the mount.
#[test]
fn a_limbed_mount_crosses_the_door_with_all_of_its_parts() {
    use ambition_platformer2d::actors::abilities::traversal::possession::PossessionState;
    use ambition_platformer2d::actors::actor::BodyKinematics;
    use ambition_platformer2d::actors::features::{LimbRig, RidingOn};
    use ambition_platformer2d::characters::brain::Brain;

    let mut sim = fixed_60hz_room_sim("gnu_ton_arena");
    for _ in 0..30 {
        sim.step(base());
    }
    let (rider, mount) = {
        let world = sim.world_mut();
        let mut q = world.query::<(Entity, &RidingOn, &Brain)>();
        q.iter(world)
            .next()
            .map(|(entity, riding, _)| (entity, riding.mount))
            .expect("'gnu_ton_arena' authors a rider on a mount")
    };
    let limbs: Vec<Entity> = sim
        .world()
        .get::<LimbRig>(mount)
        .map(|rig| rig.limbs.values().copied().collect())
        .unwrap_or_default();
    assert_eq!(
        limbs.len(),
        2,
        "setup: the mount must have limbs, or this test is the mount test again"
    );

    let mut possessed = false;
    for i in 0..900 {
        if let Some(here) = sim
            .world()
            .get::<BodyKinematics>(rider)
            .map(|k| (k.pos.x, k.pos.y))
        {
            sim.teleport_player(here);
        }
        sim.step(AgentAction {
            move_y: 1.0,
            interact: i == 0,
            interact_held: true,
            ..base()
        });
        if sim.world_mut().resource::<PossessionState>().possessed == Some(rider) {
            possessed = true;
            break;
        }
    }
    assert!(possessed, "setup: the rider was never possessed");

    let before = sim.observation().active_room.clone();
    let door = {
        let world = sim.world_mut();
        let mut q = world.query::<&ambition_platformer2d::actors::rooms::RoomSet>();
        let set = q.iter(world).next().expect("a room set");
        set.active_loading_zones()
            .iter()
            .find(|zone| {
                zone.activation == ambition_platformer2d::world::rooms::LoadingZoneActivation::Door
            })
            .cloned()
            .expect("'gnu_ton_arena' authors a door")
    };
    let centre = door.aabb.center();
    place_body(&mut sim, mount, (centre.x, centre.y));
    sim.step(base());
    let saddle = {
        let m = sim
            .world()
            .get::<BodyKinematics>(mount)
            .map(|k| k.pos)
            .unwrap();
        let r = sim
            .world()
            .get::<BodyKinematics>(rider)
            .map(|k| k.pos)
            .unwrap();
        (r.x - m.x, r.y - m.y)
    };
    let aim = (centre.x - saddle.0, centre.y - saddle.1);
    let mut arrived = None;
    for _ in 0..120 {
        place_body(&mut sim, mount, aim);
        let room = sim
            .step(AgentAction {
                interact: true,
                interact_held: true,
                ..base()
            })
            .active_room;
        if room != before {
            arrived = Some(room);
            break;
        }
    }
    let arrived = arrived.expect("the piloted mount never left the room");

    assert!(
        sim.world().get_entity(rider).is_ok(),
        "the rider did not survive"
    );
    assert!(
        sim.world().get_entity(mount).is_ok(),
        "the mount did not survive into '{arrived}'"
    );
    let lost: Vec<Entity> = limbs
        .iter()
        .copied()
        .filter(|limb| sim.world().get_entity(*limb).is_err())
        .collect();
    assert!(
        lost.is_empty(),
        "the mount crossed into '{arrived}' and {} of its limbs did not ({lost:?}). \
         An attachment travels with its anchor, and the chain here is three links — \
         rider, mount, limbs — so a rule that walks a fixed depth arrives handless",
        lost.len()
    );
}

/// **THE WHOLE ATTACHMENT CLOSURE JOINS THE OCCURRENCE LEDGER, so no part of it
/// can be re-authored while somebody is carrying it.**
///
/// ⭐ **this is the property that makes the duplication fix cover every
/// population at once, and it is worth pinning separately from the crossing.**
/// `project_driven_body_custody` marks the closure `InCustodyOf`;
/// `project_custody_onto_authored_occurrences` then records every marked
/// room-scoped occurrence as `InCustody`; and a room rebuild consults that
/// outlook and declines to author what somebody is holding. So the chain
/// rider → mount → limbs is protected by the same sentence that protects a
/// carried axe, with nothing per-relation anywhere in the ledger.
///
/// Measured in `gnu_ton_arena`, whose boss rides a mount with two hands: all
/// FOUR identities appear, including the limbs' own
/// `placement:EnemySpawn-6836/0` and `/1`.
///
/// ⚠ the second crossing is deliberate. The door out of `hall_of_bosses` does
/// not lead back, so this is not a round trip — it is two transitions while
/// ridden, which is the stronger statement the geometry actually supports: the
/// mount is still one occurrence after both.
#[test]
fn the_whole_attachment_closure_is_recorded_as_being_in_custody() {
    use ambition_platformer2d::actors::abilities::traversal::possession::PossessionState;
    use ambition_platformer2d::actors::actor::BodyKinematics;
    use ambition_platformer2d::actors::features::{LimbRig, RidingOn};
    use ambition_platformer2d::characters::brain::Brain;
    use ambition_platformer2d::platformer::lifecycle::AuthoredOccurrences;

    let mut sim = fixed_60hz_room_sim("gnu_ton_arena");
    for _ in 0..30 {
        sim.step(base());
    }
    let (rider, mount) = {
        let world = sim.world_mut();
        let mut q = world.query::<(Entity, &RidingOn, &Brain)>();
        q.iter(world)
            .next()
            .map(|(entity, riding, _)| (entity, riding.mount))
            .expect("'gnu_ton_arena' authors a rider on a mount")
    };
    let mount_id = sim
        .world()
        .get::<SimId>(mount)
        .cloned()
        .expect("the authored mount has a placement identity");
    let limb_ids: Vec<SimId> = sim
        .world()
        .get::<LimbRig>(mount)
        .map(|rig| {
            rig.limbs
                .values()
                .filter_map(|limb| sim.world().get::<SimId>(*limb).cloned())
                .collect()
        })
        .unwrap_or_default();
    assert_eq!(
        limb_ids.len(),
        2,
        "setup: the mount's limbs must carry identities of their own, or the ledger \
         assertion below is about a shorter chain than it claims"
    );

    for i in 0..900 {
        if let Some(here) = sim
            .world()
            .get::<BodyKinematics>(rider)
            .map(|k| (k.pos.x, k.pos.y))
        {
            sim.teleport_player(here);
        }
        sim.step(AgentAction {
            move_y: 1.0,
            interact: i == 0,
            interact_held: true,
            ..base()
        });
        if sim.world_mut().resource::<PossessionState>().possessed == Some(rider) {
            break;
        }
    }
    assert_eq!(
        sim.world_mut().resource::<PossessionState>().possessed,
        Some(rider),
        "setup: the rider was never possessed"
    );

    let recorded: Vec<String> = sim
        .world()
        .resource::<AuthoredOccurrences>()
        .rows()
        .filter(|(_, whereabouts)| {
            matches!(
                whereabouts,
                ambition_platformer2d::platformer::lifecycle::OccurrenceWhereabouts::InCustody
            )
        })
        .map(|(occurrence, _)| occurrence.as_str().to_string())
        .collect();
    for held in std::iter::once(&mount_id).chain(limb_ids.iter()) {
        assert!(
            recorded.iter().any(|row| row == held.as_str()),
            "`{}` is being carried and the occurrence ledger does not say so, so the \
             room that authored it is free to mint a second one behind the same \
             `SimId::placement(..)`. Recorded: {recorded:?}",
            held.as_str()
        );
    }

    // Two crossings while ridden — the door out of `hall_of_bosses` does not
    // lead home, so this is not a round trip and does not pretend to be.
    for _ in 0..2 {
        let before = sim.observation().active_room.clone();
        let door = {
            let world = sim.world_mut();
            let mut q = world.query::<&ambition_platformer2d::actors::rooms::RoomSet>();
            let set = q.iter(world).next().expect("a room set");
            set.active_loading_zones()
                .iter()
                .find(|zone| {
                    zone.activation
                        == ambition_platformer2d::world::rooms::LoadingZoneActivation::Door
                })
                .cloned()
        };
        let Some(door) = door else { break };
        let centre = door.aabb.center();
        place_body(&mut sim, mount, (centre.x, centre.y));
        sim.step(base());
        let saddle = {
            let m = sim
                .world()
                .get::<BodyKinematics>(mount)
                .map(|k| k.pos)
                .unwrap();
            let r = sim
                .world()
                .get::<BodyKinematics>(rider)
                .map(|k| k.pos)
                .unwrap();
            (r.x - m.x, r.y - m.y)
        };
        let aim = (centre.x - saddle.0, centre.y - saddle.1);
        let mut crossed = false;
        for _ in 0..120 {
            place_body(&mut sim, mount, aim);
            let room = sim
                .step(AgentAction {
                    interact: true,
                    interact_held: true,
                    ..base()
                })
                .active_room;
            if room != before {
                crossed = true;
                break;
            }
        }
        assert!(crossed, "the ridden mount never left '{before}'");
    }

    assert_eq!(
        occurrences(&mut sim, &mount_id).len(),
        1,
        "after two crossings while ridden, more than one occurrence claims the mount's \
         authored identity"
    );
}

/// Every authored id in a room set, with the rooms that claim it.
///
/// ⚠ **`LoadingZone` ids are deliberately excluded**, matching
/// `validate.placement_id_collision`'s own exemption: a zone's `target_zone`
/// resolves within its `target_room`, so `return_door` naming a zone in seven
/// rooms is correct rather than a collision. Every other authored kind lands in
/// the ONE global `SimId::placement(..)` namespace.
fn authored_id_owners(
    set: &ambition_platformer2d::actors::rooms::RoomSet,
) -> std::collections::BTreeMap<String, Vec<String>> {
    let mut owners: std::collections::BTreeMap<String, Vec<String>> = Default::default();
    for room in &set.rooms {
        let ids = room
            .placements
            .iter()
            .map(|placement| placement.id.0.clone())
            .chain(room.enemy_spawns.iter().map(|enemy| enemy.id.clone()))
            .chain(room.boss_spawns.iter().map(|boss| boss.id.clone()))
            .chain(room.ground_items.iter().map(|item| item.id.clone()))
            .chain(room.gravity_zones.iter().map(|zone| zone.id.clone()))
            .chain(room.shrines.iter().map(|shrine| shrine.id.clone()));
        for id in ids {
            owners.entry(id).or_default().push(room.id.clone());
        }
    }
    owners
}

/// **NO TWO ROOMS ANYWHERE IN THE LOADED WORLD AUTHOR ONE ID — checked on the
/// MERGED set, which is the half the file validator cannot see.**
///
/// ⛔⛔ **`SimId::placement(id)` is a GLOBAL namespace and uniqueness was only
/// ever checked PER FILE.** D125 records the gap in its own words: *"a
/// cross-WORLD collision is possible in principle (measured 0), and checking it
/// would need every world loaded at once, which this validator does not do."*
/// The RUNTIME does exactly that — the boot log says
/// `merged 11 level(s) from secondary world 'world.intro_ldtk'` — so the merged
/// `RoomSet` is the artifact where the question is answerable, and this asks it
/// there.
///
/// What a collision would cost: `OccurrenceContinuity` keys dispositions on
/// `SimId`, so one id claimed by two rooms means one row speaking for two
/// things. Carry either out of its room and the ledger suppresses BOTH on
/// rebuild — the object in your hands and a stranger two worlds away.
///
/// ⚠ **the self-check below is not decoration.** A guard that is green on all
/// real data is indistinguishable from one that cannot fire, and this one is
/// green on every shipped world. So the same detector is run against a synthetic
/// pair that DOES collide, in the same test, and must report it.
#[test]
fn no_two_rooms_in_the_merged_world_author_the_same_id() {
    let mut sim = fixed_60hz_room_sim("vertical_shaft");
    for _ in 0..20 {
        sim.step(base());
    }
    let (rooms, owners) = {
        let world = sim.world_mut();
        let mut query = world.query::<&ambition_platformer2d::actors::rooms::RoomSet>();
        let set = query
            .iter(world)
            .next()
            .expect("the session publishes a merged room set");
        (set.rooms.len(), authored_id_owners(set))
    };

    // ⭐ the zero floor, in BOTH dimensions: a scan that loaded one room, or a
    // room set whose authored ids stopped being readable, would otherwise report
    // "no collisions" over almost nothing.
    assert!(
        rooms >= 50,
        "only {rooms} rooms were merged — the secondary worlds did not load, so this \
         is not a CROSS-WORLD check at all"
    );
    assert!(
        owners.len() >= 300,
        "only {} authored ids were read across {rooms} rooms; the collection above has \
         stopped seeing a kind that carries an id",
        owners.len()
    );

    let collisions: Vec<String> = owners
        .iter()
        .filter(|(_, claimants)| claimants.len() > 1)
        .map(|(id, claimants)| format!("`{id}` in {claimants:?}"))
        .collect();
    assert!(
        collisions.is_empty(),
        "two rooms author one id, and `SimId::placement(..)` is a GLOBAL namespace — \
         so one occurrence row speaks for both, and carrying either out of its room \
         suppresses the other on rebuild: {collisions:?}"
    );

    // ── the detector can fire ────────────────────────────────────────────────
    let mut synthetic: std::collections::BTreeMap<String, Vec<String>> = Default::default();
    synthetic.insert("shared_id".into(), vec!["room_a".into(), "room_b".into()]);
    synthetic.insert("unique_id".into(), vec!["room_a".into()]);
    let found: Vec<&String> = synthetic
        .iter()
        .filter(|(_, claimants)| claimants.len() > 1)
        .map(|(id, _)| id)
        .collect();
    assert_eq!(
        found,
        vec!["shared_id"],
        "the collision predicate cannot detect a collision it is handed directly, so \
         its silence on the real world means nothing"
    );
}
