//! **THE DURABLE HORIZON, DRIVEN — a save file that remembers where the player
//! left things, and a load that puts them there.**
//!
//! ```text
//! current world truth    AuthoredOccurrences + ItemCustody
//! checkpoint truth       OccurrenceBaseline + CustodyBaseline + MintedItemBaseline
//! durable save truth     ← THIS FILE
//! ```
//!
//! Everything the two previous slices built survived a death and evaporated on a
//! load, because `AmbitionGameSaveData` carried counts and flags and nothing at
//! all about occurrences. It carries three lists now, and they are the same three
//! values the checkpoint copies rather than a second description of them.
//!
//! # What is driven, and what is constructed
//!
//! The world half is entirely production road: the authored LDtk ground items,
//! the pressed pickup, a real `Door` crossing, a real Shield+Attack throw, and
//! both shipped persist systems — which since 2026-08-16 are SCHEDULED in every
//! composition (`DurableSaveHorizonPlugin`) rather than living in the visible
//! binary's presentation assembly.
//!
//! ⚠ **the one manufactured beat is the BOOT.** A load is two facts inside one
//! process: `AmbitionGameSave` holds the file's bytes, and `SaveRestored` is
//! `false` because nothing has applied them yet. `load_save_at_startup` produces
//! exactly that pair before the first frame, and a second process is not
//! something a test can have. The systems that run are the shipped ones,
//! unmodified, in their shipped order.

use ambition_app::{AgentAction, Platformer2dSimHarness};
use ambition_platformer2d::engine_core::{AabbExt, ControlFrame};
use ambition_platformer2d::persistence::save::AmbitionGameSave;
use ambition_platformer2d::persistence::save_data::{
    AmbitionGameSaveData, PersistedOccurrence, PersistedWhereabouts,
};
use ambition_platformer2d::platformer::sim_id::SimId;
use bevy::prelude::Entity;

use crate::common::{base, fixed_60hz_room_sim, possess_the_authored_enemy};

type Custody = ambition_platformer2d::actors::items::pickup::ItemCustody;
type Ground = ambition_platformer2d::actors::items::pickup::GroundItem;
type Held = ambition_platformer2d::combat::held_items::HeldItem;
type SaveRestored = ambition_platformer2d::actors::session::durable_horizon::SaveRestored;

/// `blink_run` authors exactly one `GroundItem` and exactly one `Door`, and that
/// door leads to `portal_bridge`. One item and one exit is what makes "carried it
/// next door" unambiguous. Same pair `carried_item_crosses_rooms` drives.
const SOURCE_ROOM: &str = "blink_run";
const TARGET_ROOM: &str = "portal_bridge";

/// A room that authors MORE THAN ONE ground item, so the terminal-row fixture can
/// end one object and watch the other stay exactly where it was.
const TWO_ITEM_ROOM: &str = "central_hub_complex";
const ENDED: &str = "ground_gun_sword";
const UNTOUCHED: &str = "ground_grapple";

// ───────────────────────────────────────────────────────────────────────────
// Reading the world through its own vocabulary.
// ───────────────────────────────────────────────────────────────────────────

/// Every live occurrence of one identity, with where it is and who has it.
///
/// ⭐ **a COUNT, never a lookup.** Two live things behind one `SimId` and zero
/// live things behind one `SimId` are both failures this file exists to catch,
/// and a `find` sees neither.
fn occurrences(sim: &mut Platformer2dSimHarness, id: &SimId) -> Vec<(Entity, Custody)> {
    let mut query = sim.world_mut().query::<(Entity, &SimId, &Custody)>();
    query
        .iter(sim.world())
        .filter(|(_, sim_id, _)| *sim_id == id)
        .map(|(entity, _, custody)| (entity, *custody))
        .collect()
}

/// Where the one occurrence of `id` is lying. Panics unless exactly one is,
/// because a fixture that measured a position out of two occurrences would be
/// measuring a coin flip.
fn resting_place(sim: &mut Platformer2dSimHarness, id: &SimId) -> (f32, f32) {
    let mut query = sim.world_mut().query::<(&SimId, &Ground, &Custody)>();
    let found: Vec<(f32, f32)> = query
        .iter(sim.world())
        .filter(|(sim_id, _, custody)| *sim_id == id && custody.in_world())
        .map(|(_, ground, _)| (ground.pos.x, ground.pos.y))
        .collect();
    assert_eq!(
        found.len(),
        1,
        "exactly one `{}` must be lying in the world here",
        id.as_str()
    );
    found[0]
}

fn body(sim: &mut Platformer2dSimHarness) -> Entity {
    let mut query = sim
        .world_mut()
        .query_filtered::<Entity, ambition_platformer2d::actors::actor::PrimaryPlayerOnly>();
    query
        .iter(sim.world())
        .next()
        .expect("the session has a primary body")
}

/// The one authored ground item lying in `SOURCE_ROOM`, and its identity.
fn the_only_authored_item(sim: &mut Platformer2dSimHarness) -> (Entity, SimId) {
    let mut query = sim.world_mut().query::<(Entity, &SimId, &Custody)>();
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

// ───────────────────────────────────────────────────────────────────────────
// The roads.
// ───────────────────────────────────────────────────────────────────────────

/// Stand on the object and press Attack until it is in hand.
fn pick_up(sim: &mut Platformer2dSimHarness, id: &SimId) -> Entity {
    let at = resting_place(sim, id);
    sim.teleport_player(at);
    for _ in 0..40 {
        sim.step(AgentAction {
            attack: true,
            ..base()
        });
        sim.step(base());
        if let Some((_, Custody::Held { holder })) = occurrences(sim, id)
            .into_iter()
            .find(|(_, custody)| !custody.in_world())
        {
            return holder;
        }
    }
    panic!(
        "pressed Attack on `{}` for 40 frames and never picked it up",
        id.as_str()
    );
}

/// Shield+Attack: the only input that puts a `UseSystem` item back in the world.
fn throw_it_down(sim: &mut Platformer2dSimHarness) {
    sim.step_frame(ControlFrame {
        attack_pressed: true,
        shield_held: true,
        ..ControlFrame::default()
    });
    sim.step_n(base(), 60);
}

/// The authored `Door` zone of the ACTIVE room that leads to `target`.
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

/// Walk the body through the authored door that leads to `target`.
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
        "held interact in the '{}' door of '{before}' for 60 frames and the room never changed",
        door.name
    );
}

// ───────────────────────────────────────────────────────────────────────────
// The save and the load.
// ───────────────────────────────────────────────────────────────────────────

/// **THE FILE.** What `persist_occurrence_horizon_to_save` has mirrored into
/// `AmbitionGameSave` — the exact value the autosave would commit to disk.
///
/// ⚠ read out of the save rather than out of the live ledger, because the
/// question is what would land on the file. A fixture that read
/// `AuthoredOccurrences` would be asserting the thing it means to compare
/// against.
fn the_file(sim: &Platformer2dSimHarness) -> AmbitionGameSaveData {
    sim.world().resource::<AmbitionGameSave>().data().clone()
}

/// **THE BOOT.** A fresh world, handed a file, told nothing has been applied yet.
///
/// That pair — the bytes in `AmbitionGameSave`, the latch `false` — is exactly
/// what `load_save_at_startup` leaves behind before the first frame of a real
/// process. Everything after it is shipped: the durable domain adopters install
/// the three values and asks for a checkpoint resume, and the resume is the road
/// a death already takes.
fn boot_with(room: &str, file: &AmbitionGameSaveData) -> Platformer2dSimHarness {
    let mut sim = fixed_60hz_room_sim(room);
    sim.step_n(base(), 8);
    sim.world_mut().resource_mut::<AmbitionGameSave>().0 = file.clone();
    sim.world_mut().resource_mut::<SaveRestored>().0 = false;
    sim.step_n(base(), 90);
    assert!(
        sim.world().resource::<SaveRestored>().0,
        "the load must have LANDED — a latch still false means it returned early \
         and everything below is measuring a load that never happened"
    );
    sim
}

// ───────────────────────────────────────────────────────────────────────────
// ⭐⭐ FALSIFIER Z — A SAVE TAKEN MID-POSSESSION, THROUGH A REAL FRESH BOOT.
// ───────────────────────────────────────────────────────────────────────────

/// **Save while driving an enemy, quit, come back — and the enemy is still
/// there, standing in its own room.**
///
/// ⛔⛔ **A RELATIONSHIP MAY NOT CROSS THE DURABLE HORIZON WITHOUT ITS
/// AUTHORITY, and possession is the case that found the rule.** Possession became
/// custody, so a driven body wears `InCustodyOf` and its occurrence enters the
/// ledger as `InCustody` — that IS the fix, it is what stops the home room minting
/// a second copy of a body somebody is driving. `persist_occurrence_horizon_to_save`
/// then mirrored the ledger and every live custody row to disk, because it queries
/// the generic component and a possessed body now answers it.
///
/// ⚠ **but `PossessionState` is NOT durable save state.** The file said *"this
/// enemy is in somebody's hands"* with no hand on the other side of the boot, and
/// the only reader of that claim is a room build deciding whether to author the
/// enemy at all.
///
/// ⭐ **it did not fail, and it was one line deep.** The live projection
/// republishes the custody leg from live state every tick, so the row was
/// retracted before any room build could act on it. Stopping that retraction from
/// reaching empty — the exact thing `republish_custody`'s own contract forbids —
/// makes THIS test report **zero** bodies behind the identity: the enemy deleted
/// from the world, permanently, by a save taken while somebody was driving it.
/// The other three falsifiers in this file stayed green.
///
/// ⇒ the mirror now writes an `InCustody` claim only for occurrences whose
/// custody the durable road can RESTORE, which is the item road. A body's
/// occurrence is simply absent from the file, and absent is right: on load its
/// room authors it, which is what a world with nobody possessing anything should
/// contain.
///
/// ⭐⭐ **AND THIS IS THE TEST THAT WAS CLAIMED AND NOT WRITTEN.**
/// `a_custody_row_with_nobody_holding_it_is_retracted_before_a_room_can_act_on_it`
/// asserts something real and narrower — the LIVE projection retracts a row once
/// possession disappears — but it does it by assigning `PossessionState::default()`
/// into the same running world and stepping once. Its comment called that "A FRESH
/// PROCESS"; it is not one. No save is written, nothing is serialised, no second
/// app is built, and no durable adopter runs. This one uses the file
/// ([`the_file`]) and the real boot ([`boot_with`]), the pair every other
/// falsifier here is built on.
#[test]
fn a_save_taken_mid_possession_does_not_delete_the_enemy_in_a_fresh_process() {
    use ambition_platformer2d::actors::abilities::traversal::possession::PossessionState;
    use ambition_platformer2d::platformer::lifecycle::AuthoredOccurrences;

    let mut sim = fixed_60hz_room_sim("vertical_shaft");
    let (_actor, id) = possess_the_authored_enemy(&mut sim);
    // Let the mirror run: it is gated on the restore latch and value-compared, so
    // it reaches the save on the first tick after custody settles.
    sim.step_n(base(), 4);

    // ⭐ **TERM ONE, and without it the rest is about nothing**: the LIVE ledger
    // does say the body is in custody. That is the state whose durability is in
    // question, and a fixture that skipped this would be asserting the file is
    // clean of a row nothing ever produced.
    let live: Vec<String> = sim
        .world()
        .resource::<AuthoredOccurrences>()
        .rows()
        .map(|(occurrence, whereabouts)| format!("{} = {whereabouts:?}", occurrence.as_str()))
        .collect();
    assert!(
        live.iter()
            .any(|row| row.starts_with(id.as_str()) && row.ends_with("InCustody")),
        "the live ledger does not record the driven body as being in custody, so \
         nothing below is about a relationship reaching the file. Ledger was {live:?}"
    );

    // ⭐ **TERM TWO**: and the file does NOT carry it, in either leg.
    let file = the_file(&sim);
    assert!(
        !file.occurrences.iter().any(|row| row.id == id.as_str()),
        "the save carries a whereabouts for a body whose custodian is possession \
         state the save does not hold. On load nobody is driving it, and the only \
         thing standing between that row and the enemy's permanent deletion is a \
         live retraction winning a race. Saved occurrences were {:?}",
        file.occurrences
    );
    assert!(
        !file.custody.iter().any(|row| row.occurrence == id.as_str()),
        "the save carries a custody row naming a hand it cannot reconstruct. \
         Saved custody was {:?}",
        file.custody
    );

    // ⭐ **TERM THREE**: the fresh process — a new world, handed that file, with
    // nobody driving anything. This is the boot every other falsifier here uses.
    let mut fresh = boot_with("vertical_shaft", &file);
    assert_eq!(
        fresh.world().resource::<PossessionState>().possessed,
        None,
        "a fresh boot must not be possessing anything — if it were, the claim would \
         be true again and this test would prove nothing"
    );

    let count = live_bodies_named(&mut fresh, &id);
    assert_eq!(
        count,
        1,
        "after loading a save taken mid-possession, `vertical_shaft` holds {count} \
         bodies behind `{}` and it must hold exactly one. Zero is the permanent \
         deletion this test exists for; two is the duplication the ledger exists to \
         prevent.",
        id.as_str()
    );
}

/// How many live entities carry one identity. A COUNT, for the same reason
/// [`occurrences`] is one — zero and two are both failures, and a `find` sees
/// neither. Separate from `occurrences` because a BODY has no `ItemCustody`.
fn live_bodies_named(sim: &mut Platformer2dSimHarness, id: &SimId) -> usize {
    let mut query = sim.world_mut().query::<&SimId>();
    query.iter(sim.world()).filter(|found| *found == id).count()
}

// ───────────────────────────────────────────────────────────────────────────
// ⭐⭐ FALSIFIER A — AUTHORED OCCURRENCE CONTINUITY SURVIVES A LOAD.
// ───────────────────────────────────────────────────────────────────────────

/// **Carry an authored object into the next room, put it down, save, quit, come
/// back — and it is lying where you left it, with its pedestal still empty.**
///
/// ⛔⛔ **BOTH HALVES ARE ONE FACT AND BOTH ARE ASSERTED, from ONE file.** The
/// room the object lies in owes the world that occurrence; the room whose record
/// minted it owes nothing, and authoring it again would put two live things
/// behind one `SimId`. A load that reinstates and forgets to suppress fails at
/// `SOURCE_ROOM`; one that suppresses and forgets to reinstate fails at
/// `TARGET_ROOM` — and that failure is a permanent DELETION traded for a
/// duplication, which is the worse of the two.
///
/// ⭐ **and the control run is what makes the emptiness mean anything.** The same
/// fresh world booted with a DEFAULT file has the object on its pedestal, so
/// "zero at `SOURCE_ROOM`" cannot be satisfied by a room that stopped authoring
/// it, a harness that failed to build, or a query that matches nothing.
#[test]
fn an_object_left_in_another_room_is_lying_there_after_a_load() {
    let mut sim = fixed_60hz_room_sim(SOURCE_ROOM);
    sim.step_n(base(), 10);
    let (_, authored) = the_only_authored_item(&mut sim);
    let pedestal = resting_place(&mut sim, &authored);

    // ── carry it next door and drop it there ────────────────────────────────
    pick_up(&mut sim, &authored);
    assert_eq!(walk_through_the_door_to(&mut sim, TARGET_ROOM), TARGET_ROOM);
    let stand = door_to(&mut sim, SOURCE_ROOM).aabb.center();
    sim.teleport_player((stand.x, stand.y));
    throw_it_down(&mut sim);
    sim.step_n(base(), 60);
    let dropped = resting_place(&mut sim, &authored);
    let moved = (dropped.0 - pedestal.0).abs() + (dropped.1 - pedestal.1).abs();
    assert!(
        moved > 8.0,
        "this fixture needs the object to end up somewhere its record does NOT \
         put it: authored at {pedestal:?}, dropped at {dropped:?}. Nothing below \
         can fail while those are the same place"
    );

    // ── SAVE: the shipped mirror has been running every frame ───────────────
    let file = the_file(&sim);
    assert_eq!(
        file.occurrences,
        vec![PersistedOccurrence::new(
            authored.as_str(),
            PersistedWhereabouts::Placed {
                room: TARGET_ROOM.to_string(),
                x: dropped.0.round() as i32,
                y: dropped.1.round() as i32,
            },
        )],
        "⭐⭐ NON-VACUITY: the file must actually SAY where the object is, and say \
         it about exactly one occurrence. A save that wrote nothing here would \
         make both loads below pass for the reason this slice exists to remove"
    );

    // ── LOAD into the room it was left in ───────────────────────────────────
    let mut there = boot_with(TARGET_ROOM, &file);
    let live = occurrences(&mut there, &authored);
    assert_eq!(
        live.len(),
        1,
        "the occurrence was left lying in '{TARGET_ROOM}' and must be there after \
         a load — exactly once. ZERO is the deletion this whole design exists to \
         prevent, because '{SOURCE_ROOM}' has been told not to author it. Got {live:?}"
    );
    assert!(
        live[0].1.in_world(),
        "lying in the world, collectible, the way it was left — not stranded in \
         a hand. Got {:?}",
        live[0].1
    );
    let back = resting_place(&mut there, &authored);
    let drift = (back.0 - dropped.0).abs() + (back.1 - dropped.1).abs();
    assert!(
        drift < 2.0,
        "it must come back at {dropped:?}, where it was dropped; it came back at \
         {back:?}. A room rebuilt from its authored records puts it at {pedestal:?}, \
         which is the failure this measures. (⚠ up to one pixel of the difference \
         is the format's own rounding — see `PersistedWhereabouts::Placed`.)"
    );

    // ── LOAD into the room that AUTHORS it: the pedestal is empty ───────────
    let mut home = boot_with(SOURCE_ROOM, &file);
    assert!(
        occurrences(&mut home, &authored).is_empty(),
        "the room whose record minted this placement must NOT author a second \
         one: the occurrence exists, next door, and the file says so. Got {:?}",
        occurrences(&mut home, &authored)
    );

    // ── THE CONTROL: the same room, booted with a file that remembers nothing ─
    let mut fresh = boot_with(SOURCE_ROOM, &AmbitionGameSaveData::new());
    assert_eq!(
        occurrences(&mut fresh, &authored).len(),
        1,
        "⭐⭐ and the emptiness above is about the SAVE. A default file leaves the \
         object on its pedestal, so a room that had simply stopped authoring it — \
         or a harness that had failed to build one — cannot pass this pair"
    );
}

// ───────────────────────────────────────────────────────────────────────────
// ⭐⭐ FALSIFIER B — CUSTODY SURVIVES A LOAD.
// ───────────────────────────────────────────────────────────────────────────

/// **Save while holding a weapon, load, and you are still holding it — the same
/// occurrence, in the same hand.**
///
/// ⭐ **this is a REGRESSION FIX as well as a feature.** D132 measured the trade
/// the durable save was making: a held weapon reached disk as nothing at all,
/// because `to_persisted` reads the stored quantity and never the projection of
/// the hand. The object was therefore LOST across save/load (it used to be
/// duplicated, which is worse), and that trade was recorded as explicitly not a
/// resting state.
///
/// ⛔ **the object has no live entity at load time and that is the point.** The
/// room that authors it has been told to suppress it, so nothing rebuilds it into
/// a room; what puts it back is `restore_custody_to_checkpoint`'s materialization
/// arm, reaching the record BY IDENTITY from the file's custody row. A load that
/// merely restored the ledger would leave the player holding nothing and the
/// pedestal empty — the object in no place at all.
///
/// ⭐ **the control run pins that**: a default file leaves the weapon on its
/// pedestal and the hand empty, so neither claim below can pass by accident.
#[test]
fn a_weapon_in_your_hands_is_still_in_your_hands_after_a_load() {
    let reward = SimId::placement(ENDED);

    let mut sim = fixed_60hz_room_sim(TWO_ITEM_ROOM);
    sim.step_n(base(), 10);
    pick_up(&mut sim, &reward);
    sim.step_n(base(), 4);

    let file = the_file(&sim);
    assert_eq!(
        file.occurrences,
        vec![PersistedOccurrence::new(
            reward.as_str(),
            PersistedWhereabouts::InCustody
        )],
        "⭐ NON-VACUITY: the file must record the object as carried. Anything else \
         and the load below is not being asked the question this fixture is about"
    );
    assert_eq!(
        file.custody.len(),
        1,
        "⚠ and it must name the HAND separately: an `InCustody` row says somebody \
         has it, which is enough to stop a room minting a second one and NOT \
         enough to put it back. Got {:?}",
        file.custody
    );
    assert_eq!(file.custody[0].occurrence, reward.as_str());

    // ── LOAD ────────────────────────────────────────────────────────────────
    let mut loaded = boot_with(TWO_ITEM_ROOM, &file);
    let holder = body(&mut loaded);
    let live = occurrences(&mut loaded, &reward);
    assert_eq!(
        live.len(),
        1,
        "⛔ exactly one occurrence answers to `{}` after the load. ZERO is the \
         D132 regression — the object reaching disk as nothing and being lost — \
         and TWO is the duplication that preceded it. Got {live:?}",
        reward.as_str()
    );
    assert!(
        live[0].1.held_by(holder),
        "the file said this body was carrying it, so the load owes it back to \
         that hand rather than to the floor. Got {:?}",
        live[0].1
    );
    assert!(
        loaded.world().get::<Held>(holder).is_some(),
        "⭐ and BOTH HALVES of the forked relation: the object says it is held, \
         and the body says it is holding something. Only one of the two is a \
         body permanently unable to pick anything else up"
    );

    // ── THE CONTROL ─────────────────────────────────────────────────────────
    let mut fresh = boot_with(TWO_ITEM_ROOM, &AmbitionGameSaveData::new());
    let empty_handed = body(&mut fresh);
    assert!(
        fresh.world().get::<Held>(empty_handed).is_none(),
        "⭐⭐ a default file leaves the hand empty, so 'still holding it' above is \
         a claim about the SAVE rather than about how this room starts"
    );
    assert!(
        occurrences(&mut fresh, &reward)
            .first()
            .is_some_and(|(_, custody)| custody.in_world()),
        "and it leaves the weapon on its pedestal"
    );
}

// ───────────────────────────────────────────────────────────────────────────
// ⛔⛔ FALSIFIER C — THE POISON.
// ───────────────────────────────────────────────────────────────────────────

/// **A TERMINAL DISPOSITION IS NOT UNDONE BY A LOAD, AND AN UNTOUCHED RECORD IS
/// NOT TOUCHED BY ONE.**
///
/// ```text
/// Consumed before the save  → the object must NOT come back
/// never picked up at all    → the object must still be on its pedestal
/// ```
///
/// ⭐⭐ **the two halves fail different implementations, which is why they are in
/// one test.** A load that restores "everything the ledger ever mentioned" —
/// materializing a row rather than reading its disposition — resurrects the
/// consumed object and passes the second half perfectly. A load that drops rows
/// it does not understand, or writes the whole world into the file and reinstates
/// all of it, fails the second half and passes the first.
///
/// ⚠ **THE STATE ONLY THE WRONG IMPLEMENTATION CAN ACT ON.** The consumed
/// occurrence has no live entity anywhere: it is not in a hand, not lying in a
/// room, not held by anything the reset can reconcile against. Every ordinary arm
/// of the restore is a no-op for it, so the only thing that decides whether it
/// comes back is how `outlook_for` reads the row the file carried. The control
/// run below proves the room really does author it otherwise.
///
/// ⚠ **the file is written by hand, and that is the honest way to reach this
/// state.** `OccurrenceWhereabouts::Consumed` has no live producer — deliberately,
/// so that ephemeral/resettable stays the default — so no world can be driven
/// into producing one. What a load consumes IS a file, so a file is what this
/// drives it with; the systems that read it are the shipped ones.
#[test]
fn a_consumed_occurrence_is_not_resurrected_by_a_load_and_an_untouched_one_is_untouched() {
    let ended = SimId::placement(ENDED);
    let untouched = SimId::placement(UNTOUCHED);

    // ── THE CONTROL FIRST: this room authors both, on an empty file ──────────
    let mut fresh = boot_with(TWO_ITEM_ROOM, &AmbitionGameSaveData::new());
    assert_eq!(
        occurrences(&mut fresh, &ended).len(),
        1,
        "⭐⭐ NON-VACUITY: '{TWO_ITEM_ROOM}' authors `{ENDED}`, so the absence \
         asserted below is about the terminal row and not about the room"
    );
    let untouched_pedestal = resting_place(&mut fresh, &untouched);

    // ── a file that remembers ONE occurrence, and remembers it as ENDED ──────
    let mut file = AmbitionGameSaveData::new();
    file.occurrences = vec![PersistedOccurrence::new(
        ended.as_str(),
        PersistedWhereabouts::Consumed,
    )];

    let mut loaded = boot_with(TWO_ITEM_ROOM, &file);

    // ── CLAIM 1: the terminal row is honoured ───────────────────────────────
    assert!(
        occurrences(&mut loaded, &ended).is_empty(),
        "⛔ the world remembered ending this occurrence and the file carried that \
         memory. Bringing it back is a load overruling a terminal disposition — \
         the consumed key back on its pedestal, the destroyed mechanism whole \
         again. Got {:?}",
        occurrences(&mut loaded, &ended)
    );

    // ── CLAIM 2: and the record nobody touched is exactly as authored ───────
    let still = occurrences(&mut loaded, &untouched);
    assert_eq!(
        still.len(),
        1,
        "⛔ a record with no row is authored as written, every time. Losing it \
         here is a load that treats 'the file mentions occurrences' as 'the file \
         is the whole world'. Got {still:?}"
    );
    assert!(still[0].1.in_world());
    let where_it_is = resting_place(&mut loaded, &untouched);
    let drift =
        (where_it_is.0 - untouched_pedestal.0).abs() + (where_it_is.1 - untouched_pedestal.1).abs();
    assert!(
        drift < 2.0,
        "and it is on its own pedestal at {untouched_pedestal:?}, not somewhere a \
         restored row put it. Found at {where_it_is:?}"
    );
}
