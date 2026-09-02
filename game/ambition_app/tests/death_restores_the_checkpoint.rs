//! Checkpoint restoration is temporal, not item-kind-specific.
//!
//! Items acquired after the current checkpoint return to their authored state on
//! death; items already present in the committed checkpoint remain where that
//! baseline recorded them. The test drives production pickup, checkpoint, and
//! death paths rather than mutating baseline resources directly.

use ambition_app::{AgentAction, Platformer2dSimHarness};
use ambition_platformer2d::engine_core::{AabbExt, ControlFrame};
use ambition_platformer2d::platformer::construction::SpawnOrigin;
use ambition_platformer2d::platformer::sim_id::SimId;
use bevy::prelude::Entity;

use crate::common::{base, fixed_60hz_room_sim};

/// The hub authors several `GroundItem`s on one shelf, which is what lets this
/// fixture name TWO objects and hold them at different horizons in one life.
/// that is beat 7's whole requirement and no other authored room can meet it —
/// a one-item room can show an object reverting and an object persisting, but
/// never both at once, which is the shape an item-kind rule cannot produce.
const ROOM: &str = "central_hub_complex";

/// The object the player BANKS: acquired, then a checkpoint committed over it.
const REWARD: &str = "ground_gun_sword";
/// The object acquired AFTER that checkpoint, and therefore owed back on death.
///
/// picked for being inert while held. The maintainer's word was
/// "temporary/disposable", but that is a description of when it was acquired,
/// not a property the engine reads — which is exactly the point of the fixture.
const TEMPORARY: &str = "ground_grapple";

type Custody = ambition_platformer2d::actors::items::pickup::ItemCustody;
type Ground = ambition_platformer2d::actors::items::pickup::GroundItem;

/// Every live occurrence of `authored`, and whether each is in a hand.
///
/// a COUNT, never a lookup. "Do I still have the key" is answered by the
/// entity in your hand and says nothing about the copy the pedestal may have
/// minted beside it — and a duplicate is one of the two ways this can fail.
fn occurrences(sim: &mut Platformer2dSimHarness, authored: &SimId) -> Vec<(Entity, Custody)> {
    let mut query = sim.world_mut().query::<(Entity, &SimId, &Custody)>();
    query
        .iter(sim.world())
        .filter(|(_, sim_id, _)| *sim_id == authored)
        .map(|(entity, _, custody)| (entity, *custody))
        .collect()
}

/// Where the named authored object is lying right now.
///
/// Panics rather than returning `None`: a room that stopped authoring one of
/// these is a fixture measuring nothing, and that has to be loud.
fn resting_place(sim: &mut Platformer2dSimHarness, authored: &SimId) -> (f32, f32) {
    let mut query = sim.world_mut().query::<(&SimId, &Ground, &Custody)>();
    let found: Vec<(f32, f32)> = query
        .iter(sim.world())
        .filter(|(id, _, custody)| *id == authored && custody.in_world())
        .map(|(_, ground, _)| (ground.pos.x, ground.pos.y))
        .collect();
    assert_eq!(
        found.len(),
        1,
        "'{ROOM}' must have exactly one `{}` lying in the world at this point",
        authored.as_str()
    );
    found[0]
}

/// Assert that `authored` is lying in the world, exactly once.
fn assert_returned(sim: &mut Platformer2dSimHarness, authored: &SimId, why: &str) {
    let live = occurrences(sim, authored);
    assert_eq!(
        live.len(),
        1,
        "exactly one occurrence must carry `{}`; found {live:?} ({why})",
        authored.as_str()
    );
    assert!(live[0].1.in_world(), "{why} — but it is {:?}", live[0].1);
}

/// Assert that `authored` is in a hand, exactly once.
fn assert_still_held(sim: &mut Platformer2dSimHarness, authored: &SimId, why: &str) {
    let live = occurrences(sim, authored);
    assert_eq!(
        live.len(),
        1,
        "⛔ the identity must not be duplicated: suppressing the pedestal and \
         keeping the held object are ONE decision. `{}` found {live:?} ({why})",
        authored.as_str()
    );
    assert!(!live[0].1.in_world(), "{why} — but it is {:?}", live[0].1);
}

fn body(sim: &mut Platformer2dSimHarness) -> Entity {
    let mut query = sim
        .world_mut()
        .query_filtered::<Entity, ambition_platformer2d::platformer::markers::PrimaryPlayerOnly>();
    query
        .iter(sim.world())
        .next()
        .expect("the session has a primary body")
}

fn body_pos(sim: &mut Platformer2dSimHarness) -> (f32, f32) {
    let entity = body(sim);
    let kin = sim
        .world()
        .get::<ambition_platformer2d::engine_core::BodyKinematics>(entity)
        .expect("the primary body has kinematics");
    (kin.pos.x, kin.pos.y)
}

/// Stand on the authored item and press Attack until it is in hand.
///
/// The pressed pickup is edge-triggered, so the press is released between
/// attempts rather than held — holding it reads as one press and the second
/// pickup in this fixture would never fire.
fn pick_up(sim: &mut Platformer2dSimHarness, at: (f32, f32), authored: &SimId) {
    sim.teleport_player(at);
    for _ in 0..40 {
        sim.step(AgentAction {
            attack: true,
            ..base()
        });
        sim.step(base());
        if occurrences(sim, authored)
            .iter()
            .any(|(_, custody)| !custody.in_world())
        {
            return;
        }
    }
    let entity = body(sim);
    panic!(
        "pressed Attack on the authored item for 40 frames and never picked it up.\n\
         body at {:?}, item at {at:?}, occurrences {:?}\n\
         out of play: {}, holding: {:?}",
        body_pos(sim),
        occurrences(sim, authored),
        sim.world()
            .get::<ambition_platformer2d::combat::death_rules::OutOfPlay>(entity)
            .is_some(),
        sim.world()
            .get::<ambition_platformer2d::combat::held_items::HeldItem>(entity)
            .is_some(),
    );
}

/// Rest at a shrine: spawn one where the body stands, press Interact, and let
/// the commit land.
///
/// the shrine is despawned again afterwards. It is a fixture prop, and one
/// left standing would re-commit a checkpoint every time a later beat presses
/// Interact for some other reason.
fn commit_a_checkpoint(sim: &mut Platformer2dSimHarness) {
    let (x, y) = body_pos(sim);
    let shrine = sim
        .world_mut()
        .spawn(ambition_platformer2d::actors::shrine::HealShrine {
            pos: ambition_platformer2d::engine_core::Vec2::new(x, y),
            half_extent: ambition_platformer2d::engine_core::Vec2::new(48.0, 48.0),
        })
        .id();
    for _ in 0..8 {
        sim.step(AgentAction {
            interact: true,
            ..base()
        });
        sim.step(base());
    }
    sim.world_mut().entity_mut(shrine).despawn();
}

/// Kill the primary body through the ordinary death report and run out the
/// interlude, so the consequence the roster decides actually fires.
fn die(sim: &mut Platformer2dSimHarness) {
    let victim = body(sim);
    let (x, y) = body_pos(sim);
    sim.world_mut().write_message(
        ambition_platformer2d::combat::death_rules::ActorDiedMessage {
            victim,
            pos: ambition_platformer2d::engine_core::Vec2::new(x, y),
            cause: ambition_platformer2d::combat::death_rules::DeathCause {
                source: ambition_platformer2d::combat::HitSource::Hazard,
                attacker: None,
            },
        },
    );
    // The interlude counts down on the sim clock and only then asks the roster
    // for the consequence; the rebuild it requests takes several more frames.
    sim.step_n(base(), 240);
}

/// THE FIXTURE. Seven beats, in the order the rule states them.
///
/// one test rather than seven, because the beats are a HISTORY. Beat 6's
/// claim ("still held") is only meaningful given beat 5 happened, and splitting
/// them would need each to reconstruct the world state of the last — which is
/// precisely the direct state-writing this file refuses.
#[test]
fn a_death_returns_what_was_not_banked_and_keeps_what_was() {
    let mut sim = fixed_60hz_room_sim(ROOM);
    sim.step_n(base(), 8);

    let reward = SimId::placement(REWARD);
    let temporary = SimId::placement(TEMPORARY);

    // ── C0: both objects are authored, lying where their records put them ────
    let pedestal = resting_place(&mut sim, &reward);

    // ── 2. the body acquires the reward ──────────────────────────────────────
    pick_up(&mut sim, pedestal, &reward);

    // ── 3.
    die(&mut sim);
    assert_returned(
        &mut sim,
        &reward,
        "the reward was acquired before any checkpoint, so a death owes it back \
         to the world",
    );

    // ── 4. acquire it again ──────────────────────────────────────────────────
    let pedestal_again = resting_place(&mut sim, &reward);
    pick_up(&mut sim, pedestal_again, &reward);

    // ── 5. commit C1 WITH the reward in hand ─────────────────────────────────
    commit_a_checkpoint(&mut sim);

    // ── 6. death restores C1: still held, and the pedestal stays empty ───────
    die(&mut sim);
    assert_still_held(
        &mut sim,
        &reward,
        "acquiring the reward was COMMITTED at C1, so a death must leave it in \
         hand and must not re-author it on its pedestal",
    );

    // ── 7. THE BEAT THAT KILLS THE ITEM-KIND READING. ────────────────────
    //
    // A second object is acquired AFTER C1 and never banked. One death now has
    // to reach two OPPOSITE answers about two objects of the same kind, held by
    // the same body, in the same frame — and the only thing that separates them
    // is which side of the checkpoint each acquisition fell on.
    //
    // no `KeyItem => survives` rule can produce this: both are ordinary
    // authored ground items, and a kind rule has exactly one answer for them.
    //
    // the reward has to be put down first, because a body has one hand. That
    // is a relocation, not a give-up: the reward's baseline row still says the
    // checkpoint saw it in custody, so beat 7's claim about it is that the
    // reset does NOT hand it back to the room.
    // Shield+Attack is the only input that puts a held item back in the world.
    // `AgentAction` hardcodes `shield_held: false`, so this goes through the
    // control frame directly — the same road `carried_item_crosses_rooms` uses.
    sim.step_frame(ControlFrame {
        attack_pressed: true,
        shield_held: true,
        ..ControlFrame::default()
    });
    sim.step_n(base(), 30);

    let temporary_pedestal = resting_place(&mut sim, &temporary);
    pick_up(&mut sim, temporary_pedestal, &temporary);
    die(&mut sim);

    assert_returned(
        &mut sim,
        &temporary,
        "the temporary object was acquired AFTER C1 and never banked, so the \
         same death that keeps the reward must take this one back",
    );
    // AND THE REWARD IS BACK IN THE HAND, not merely still one object.
    // Putting it down happened AFTER C1 and was never banked, so it is undone by
    // exactly the same rule that takes the temporary object back — the reset
    // restores the state at C1, and at C1 this was in a hand.
    //
    // counting occurrences alone would pass with the reward lying on the
    // floor, which is a different world from C1 and the wrong one.
    assert_still_held(
        &mut sim,
        &reward,
        "putting the reward down happened after C1 and was never banked, so the \
         reset owes it back to the hand it was in at C1",
    );
}

/// The room next door, used only to take an object somewhere and leave it there.
const NEIGHBOUR: &str = "duel_arena";

/// Stand in the `Door` zone of the active room that leads to `target` and hold
/// Interact until the room actually changes.
///
/// The door is chosen by asking the room graph where each zone GOES —
/// `transition_for_player` is the same resolver the crossing itself uses —
/// because this room authors eighteen of them and "the first one" is a coin flip.
fn walk_to(sim: &mut Platformer2dSimHarness, target: &str) {
    let before = sim.observation().active_room.clone();
    let zone = {
        let world = sim.world_mut();
        let mut query = world.query::<&ambition_platformer2d::world::rooms::RoomSet>();
        let room_set = query
            .iter(world)
            .next()
            .expect("the session has an active room set");
        room_set
            .active_loading_zones()
            .iter()
            .find(|zone| {
                room_set
                    .transition_for_player(
                        zone.aabb,
                        ambition_platformer2d::engine_core::Vec2::ZERO,
                        true,
                    )
                    .and_then(|t| room_set.rooms.get(t.target_room))
                    .is_some_and(|destination| destination.id == target)
            })
            .cloned()
            .unwrap_or_else(|| panic!("'{before}' has no door to '{target}'"))
    };
    let center = zone.aabb.center();
    // STEP OUT OF THE ZONE FIRST. Arriving through a door leaves the body
    // standing INSIDE the return zone, and a transition that fired the moment
    // you landed in it would ping-pong; so the crossing wants the player to
    // enter the zone rather than to already be in it. Without this the return
    // trip held Interact for ninety frames and never fired — measured
    // and it is why every earlier test only ever walked OUTWARD.
    let away = zone.aabb.center() + ambition_platformer2d::engine_core::Vec2::new(0.0, -400.0);
    sim.teleport_player((away.x, away.y));
    sim.step_n(base(), 10);
    sim.teleport_player((center.x, center.y));
    for frame in 0..90 {
        // RE-PLACED EVERY FRAME, because gravity takes the body straight
        // back out of the zone. A single teleport plus ninety frames of
        // Interact never fired the return crossing and produced no diagnostic
        // either — the system's own `warn_once` is for a body TOUCHING a zone,
        // and this body was not touching it after the first frame. Standing
        // still in the zone is also the discrete case the swept test preserves
        // exactly: a zero-length delta degrades to the overlap it always was.
        sim.teleport_player((center.x, center.y));
        // the press is an EDGE, so it is released between attempts.
        // `wants_interact` reads `slot_gestures.primary().buffered()`; holding
        // Interact down forever arms the gesture once and never again, which is
        // why ninety frames of a held button crossed nothing while the body was
        // demonstrably standing in the zone.
        let pressed = frame % 2 == 0;
        let room = sim
            .step(AgentAction {
                interact: pressed,
                interact_held: pressed,
                ..base()
            })
            .active_room;
        if room != before {
            sim.step_n(base(), 30);
            return;
        }
    }
    panic!("held Interact in the '{before}' door to '{target}' for 90 frames and never crossed");
}

/// A BANKED OBJECT WHOSE ENTITY THE WORLD DESTROYED COMES BACK INTO THE HAND
/// THAT BANKED IT — the same occurrence, MATERIALIZED, not re-authored.
///
/// The player lost the "still acquired" property they had banked — recoverable, and wrong. What
/// closed it is materialization: the reset now reaches the record BY IDENTITY, wherever in the
/// world that record lives, and rebuilds the occurrence directly into the custodian's hand.
///
/// # The four claims, and why each of them is a different way to fail
///
/// ```text
/// same occurrence   exactly one entity answers to `SimId::placement(REWARD)`
/// in the hand       it is in CUSTODY, and of the body the checkpoint named
/// pedestal empty    the home room was rebuilt and did NOT author a second one
/// not annihilated   the count is one rather than zero
/// ```
///
/// the NON-VACUITY guard is load-bearing and is asserted, not assumed: the
/// occurrence's entity really is gone before the death. Without it the whole
/// scenario can pass while the ordinary re-assignment arm does all the work and
/// materialization is never exercised at all.
#[test]
fn a_banked_object_whose_room_unloaded_returns_to_the_hand_that_banked_it() {
    let mut sim = fixed_60hz_room_sim(ROOM);
    sim.step_n(base(), 8);

    let reward = SimId::placement(REWARD);
    let pedestal = resting_place(&mut sim, &reward);
    pick_up(&mut sim, pedestal, &reward);

    // Banked: the checkpoint sees it in hand.
    commit_a_checkpoint(&mut sim);

    // Carried next door and put down there. Custody crosses rooms, so it
    // arrives still held.
    walk_to(&mut sim, NEIGHBOUR);
    assert_still_held(
        &mut sim,
        &reward,
        "custody crosses a room boundary, so the object arrives still in hand",
    );
    sim.step_frame(ControlFrame {
        attack_pressed: true,
        shield_held: true,
        ..ControlFrame::default()
    });
    sim.step_n(base(), 30);

    // Back out again. The neighbour unloads and takes the object's ENTITY with
    // it; only the ledger's `Placed` row remembers where it lies.
    walk_to(&mut sim, ROOM);
    // THE NON-VACUITY GUARD. Everything below is about an occurrence with no
    // entity behind it; if one is still resident here, the reset's ordinary
    // re-assignment arm answers the whole scenario and this test proves nothing
    // about materialization.
    assert!(
        occurrences(&mut sim, &reward).is_empty(),
        "the neighbour must actually have unloaded and taken the object's ENTITY \
         with it, or this test measures nothing"
    );

    die(&mut sim);

    // The death resumes at the checkpoint, which is in the room whose record
    // MINTED the reward — so the pedestal is genuinely rebuilt, and "the
    // pedestal stayed empty" is a claim about a room that actually ran its
    // authored construction rather than one nobody looked at.
    assert_eq!(
        sim.observation().active_room,
        ROOM,
        "the death must resume at the checkpoint, in the room that authors the \
         reward, or the empty-pedestal claim below is vacuous"
    );

    let live = occurrences(&mut sim, &reward);
    assert_eq!(
        live.len(),
        1,
        "⛔ exactly one occurrence must answer to this identity. ZERO means it was \
         ANNIHILATED — the baseline said a hand, the world said an unloaded room, \
         and neither could produce it. TWO means the materialization and the home \
         room's authoring disagreed about who owed it. Got {live:?}"
    );
    assert!(
        !live[0].1.in_world(),
        "⭐ the checkpoint saw this in a hand, so a death owes it back to that hand \
         — not to the pedestal it was authored on, which is a world the player had \
         already banked their way out of. Got {:?}",
        live[0].1
    );
    // read AFTER the death: a restart reuses the body, but the claim being made
    // is about the body that is playing now, which is the one the checkpoint's
    // custodian identity has to resolve to.
    let custodian = body(&mut sim);
    assert!(
        live[0].1.held_by(custodian),
        "and back into the hand the checkpoint NAMED: a restore that put it \
         anywhere else — or into nobody's hand at all — satisfies every count above"
    );
}

// ───────────────────────────────────────────────────────────────────────────
// THE OTHER HALF OF THE HORIZON: AN OCCURRENCE NO RECORD DESCRIBES
//
// Everything above is about AUTHORED occurrences — a room states `ground_axe`
// lies here, so a checkpoint that remembers one in a hand can always reach the
// record that minted it. That bounds materialization by *"some room authors a
// record with this id"*, and the boundary has a real inhabitant on the other
// side: a RUNTIME-MINTED instance. It is room-scoped and carryable, so it can
// enter the custody baseline, and no record anywhere can rebuild it.
//
// the production road that mints one is the inventory leg. `OwnedItems`
// is a COUNT TABLE — a quantity, not an object — and the inventory menu equips
// straight out of it, so the body ends up holding a spec with no world instance
// behind it. Throwing that turns the quantity into an INSTANCE, and
// `throw_held_item_system` mints `SimId::spawned(thrower, counter.next())` with
// a `SpawnOrigin::Dynamic` naming the thrower. Its own comment calls that arm
// "the visible edge of the unclosed inventory leg".
// ───────────────────────────────────────────────────────────────────────────

type Item = ambition_platformer2d::item::Item;

/// The item this pair MINTS. A javelin's authored `use_behavior` is
/// `ThrowOnUse`, so a plain `Attack` throws it — no shield modifier needed, and
/// the throw is the ordinary pressed action rather than a special case.
const MINTED_ITEM: Item = Item::Javelin;

/// Every live occurrence the SIMULATION minted, by identity.
///
/// it asks the `SpawnOrigin`, never the spelling of the id. `SimId`'s own
/// doc is explicit that the string is a legibility convenience and that nothing
/// may recover a fact from it — provenance is a component precisely so a change
/// to the id grammar cannot silently change what reconstruction believes. A test
/// that pattern-matched `"slot:0/0"` would be asserting the grammar.
fn dynamic_occurrences(sim: &mut Platformer2dSimHarness) -> Vec<SimId> {
    let mut query = sim.world_mut().query::<(&SimId, &SpawnOrigin, &Ground)>();
    let mut found: Vec<SimId> = query
        .iter(sim.world())
        .filter(|(_, origin, _)| matches!(origin, SpawnOrigin::Dynamic { .. }))
        .map(|(sim_id, _, _)| sim_id.clone())
        .collect();
    found.sort();
    found
}

/// Turn a QUANTITY into an INSTANCE on the production road, and return the
/// identity the simulation minted for it.
///
/// Three beats, all of them the game's own:
///
/// ```text
/// 1. entitlement   ItemGrantRequested — the `<<give_item>>` channel
/// 2. custody       equip_held_spec    — THE take-custody operation, shared by
///                                       the inventory menu and the world pickup
/// 3. the mint      a pressed Attack   — throw_held_item_system finds no object
///                                       behind the hand and mints one
/// ```
///
/// beat 2 calls the production verb directly rather than driving the menu
/// UI, for the same reason this file constructs the shrine: the claim under
/// test is what the CHECKPOINT does with a minted instance, and the inventory
/// grid's cursor navigation is a different subsystem that no other test here
/// drives. `equip_held_spec` is not a test shim — `menu::effects` reaches this
/// exact function with these exact arguments, and it is the one place a body
/// comes to hold anything.
///
/// beat 3 is NOT called directly, and that is the part that must not be
/// faked. The mint is the thing under test.
fn mint_a_dynamic_item(sim: &mut Platformer2dSimHarness) -> SimId {
    use ambition_platformer2d::item::ItemGrantRequested;
    use ambition_platformer2d::item::OwnedItems;
    use bevy::ecs::system::RunSystemOnce;

    let before = dynamic_occurrences(sim);

    // ── 1. entitlement, through the channel `<<give_item>>` uses ─────────────
    sim.world_mut().write_message(ItemGrantRequested {
        item: MINTED_ITEM,
        count: 1,
    });
    sim.step_n(base(), 4);
    assert!(
        sim.world().resource::<OwnedItems>().has(MINTED_ITEM),
        "the grant channel must have put the item in the catalog, or there is \
         nothing to equip out of"
    );

    // ── 2. equip out of the count table ─────────────────────────────────────
    sim.world_mut()
        .run_system_once(equip_the_minted_item)
        .expect("the equip runs");
    sim.step_n(base(), 2);
    let player = body(sim);
    assert!(
        sim.world()
            .get::<ambition_platformer2d::combat::held_items::HeldItem>(player)
            .is_some(),
        "the body must be holding the equipped spec"
    );
    // THE NON-VACUITY GUARD FOR THE WHOLE ROAD: there is NO OBJECT behind
    // this hand. If the equip had produced one, the throw would hand that object
    // back instead of minting, and neither fixture below would exercise a
    // runtime mint at all.
    assert_eq!(
        dynamic_occurrences(sim),
        before,
        "equipping out of the catalog must not create a world instance — that is \
         the whole reason throwing one has to mint"
    );

    // ── 3. throw it: the mint ───────────────────────────────────────────────
    sim.step(AgentAction {
        attack: true,
        ..base()
    });
    // Long enough for the arc to finish and the object to settle, so the pickup
    // below has a resting position to stand on.
    sim.step_n(base(), 120);

    let after = dynamic_occurrences(sim);
    let minted: Vec<SimId> = after
        .into_iter()
        .filter(|id| !before.contains(id))
        .collect();
    assert_eq!(
        minted.len(),
        1,
        "throwing an item with no object behind the hand must mint exactly one \
         instance; before {before:?}"
    );
    minted.into_iter().next().expect("one mint")
}

/// The inventory menu's equip, as a one-shot system.
///
/// This is the body of `MenuAction::Equip` in `menu::effects` with the portal
/// fork removed: resolve the catalog item's spec, then hand it to the ONE
/// take-custody operation.
fn equip_the_minted_item(
    mut commands: bevy::prelude::Commands,
    mut bodies: bevy::prelude::Query<
        (
            Entity,
            &mut ambition_platformer2d::characters::brain::ActionSet,
        ),
        ambition_platformer2d::platformer::markers::PrimaryPlayerOnly,
    >,
) {
    let (player, mut action_set) = bodies.single_mut().expect("one primary body");
    let spec = ambition_platformer2d::actors::items::pickup::held_spec_for_item(MINTED_ITEM)
        .expect("the javelin is a wired weapon with a held spec");
    ambition_platformer2d::actors::items::pickup::equip_held_spec(
        &mut commands,
        player,
        &mut action_set,
        spec,
    );
}

/// A: A RUNTIME-MINTED INSTANCE BANKED AT A CHECKPOINT COMES BACK TO THE HAND,
/// EVEN AFTER THE WORLD DESTROYED IT.
///
/// The authored twin of this scenario is
/// `a_banked_object_whose_room_unloaded_returns_to_the_hand_that_banked_it`, and
/// the difference is the whole point: there, the reset reaches the record that
/// authored the object. Here there is no record. Nothing in any room describes
/// this instance, and the only thing that can rebuild it is the description the
/// checkpoint captured of it — provenance plus the authored id of its spec.
///
/// ```text
/// mint      equip out of the count table, throw it → SimId::spawned + Dynamic
/// acquire   the ordinary pressed pickup
/// bank      a real shrine rest, with it in hand
/// destroy   carry it next door, drop it, LEAVE so that room unloads
/// die       and it is back in the hand, same identity, exactly once
/// ```
#[test]
fn a_banked_runtime_mint_returns_to_the_hand_that_banked_it() {
    let mut sim = fixed_60hz_room_sim(ROOM);
    sim.step_n(base(), 8);

    let minted = mint_a_dynamic_item(&mut sim);

    let landed = resting_place(&mut sim, &minted);
    pick_up(&mut sim, landed, &minted);
    assert_still_held(
        &mut sim,
        &minted,
        "the minted instance is picked up like any other object in the world",
    );

    // Banked: the checkpoint sees the mint in hand, and captures both the
    // custody row and the description that can rebuild it.
    commit_a_checkpoint(&mut sim);

    // Carried next door and put down there.
    walk_to(&mut sim, NEIGHBOUR);
    assert_still_held(
        &mut sim,
        &minted,
        "custody crosses a room boundary, so the mint arrives still in hand",
    );
    sim.step_frame(ControlFrame {
        attack_pressed: true,
        shield_held: true,
        ..ControlFrame::default()
    });
    sim.step_n(base(), 30);
    // dropping it must RETURN the object, not mint a second one: there is an
    // instance behind the hand now, and the throw's whole first arm exists to
    // hand it back.
    assert_returned(
        &mut sim,
        &minted,
        "putting the mint down returns the object it already is",
    );

    // Back out again. The neighbour unloads and takes the entity with it.
    walk_to(&mut sim, ROOM);
    // THE NON-VACUITY GUARD. Everything below is about an occurrence with no
    // entity behind it; if one were still resident, the ordinary re-assignment
    // arm would answer the scenario and this would prove nothing.
    assert!(
        occurrences(&mut sim, &minted).is_empty(),
        "the neighbour must actually have unloaded and taken the minted object's \
         ENTITY with it, or this test measures nothing"
    );

    die(&mut sim);

    let live = occurrences(&mut sim, &minted);
    assert_eq!(
        live.len(),
        1,
        "⛔ exactly one occurrence must answer to `{}`. ZERO means it was \
         ANNIHILATED — the checkpoint said a hand, the world said an unloaded \
         room, and no authored record could ever have described it. Got {live:?}",
        minted.as_str()
    );
    assert!(
        !live[0].1.in_world(),
        "⭐ the checkpoint saw this in a hand, so a death owes it back to that \
         hand. Got {:?}",
        live[0].1
    );
    let custodian = body(&mut sim);
    assert!(
        live[0].1.held_by(custodian),
        "and back into the hand the checkpoint NAMED"
    );
    // AND IT COMES BACK RECONSTRUCTABLE. A rebuilt instance that stated
    // no provenance would be invisible to the NEXT capture — it would survive
    // exactly one death and then be unrecoverable — and `SpawnOrigin::Dynamic`'s
    // own doc refuses to let "dynamic, parent unknown" be spelled at all.
    assert!(
        dynamic_occurrences(&mut sim).contains(&minted),
        "the materialized instance must state the same dynamic provenance the \
         original did, or the checkpoint can only ever restore it once"
    );
}

/// B1: A RUNTIME-MINTED INSTANCE STILL IN THE HAND AT DEATH, ACQUIRED AFTER
/// THE CHECKPOINT, IS TAKEN BACK.
///
/// The ordinary retraction arm, reached by an object that has no authored record
/// behind it — so the despawn cannot be softened into "let the room re-author
/// it", because no room will.
///
/// the identity is asserted absent EVERYWHERE, not merely absent from the
/// hand: a restore that put it back on some floor would satisfy "you are not
/// holding it" and still be a world the player had never been in.
#[test]
fn a_runtime_mint_acquired_after_the_checkpoint_is_gone_after_a_death() {
    let mut sim = fixed_60hz_room_sim(ROOM);
    sim.step_n(base(), 8);

    // ── the checkpoint FIRST, with empty hands ──────────────────────────────
    commit_a_checkpoint(&mut sim);

    // ── and only THEN the mint ──────────────────────────────────────────────
    let minted = mint_a_dynamic_item(&mut sim);
    let landed = resting_place(&mut sim, &minted);
    pick_up(&mut sim, landed, &minted);
    // THE NON-VACUITY GUARD: it really is in the hand going into the death,
    // so "gone afterwards" is a claim about a retraction that happened rather
    // than about a scenario that never got started.
    assert_still_held(
        &mut sim,
        &minted,
        "the mint must be in hand at the moment of death, or the claim below is \
         vacuous",
    );

    die(&mut sim);

    assert!(
        occurrences(&mut sim, &minted).is_empty(),
        "⛔ `{}` was minted AFTER the checkpoint, so it did not exist at the \
         checkpoint and a death owes it to nobody — not to the hand, and not to \
         any floor. Found {:?}",
        minted.as_str(),
        occurrences(&mut sim, &minted)
    );
    let player = body(&mut sim);
    assert!(
        sim.world()
            .get::<ambition_platformer2d::combat::held_items::HeldItem>(player)
            .is_none(),
        "and the hand is empty: retracting the object without retracting the \
         hand leaves the body wielding a ghost it can never put down"
    );
}

/// this is the half that stops the cheap wrong answer, and B1 above does
/// NOT stop it. In B1 the object is still a live entity when the death lands,
/// so the ordinary retraction arm despawns it and any implementation passes.
/// Here the entity is already gone — destroyed with the room it was left in —
/// which is precisely the state that made fixture A need materialization at all.
/// So the only thing standing between this object and a resurrection is that
/// nothing remembers it, and "rebuild everything the engine remembers minting"
/// is exactly the forbidden design: a growing registry of every instance that
/// ever existed.
///
/// The commit-time snapshot is what it is measuring.
#[test]
fn a_runtime_mint_the_checkpoint_never_saw_is_not_resurrected_by_a_death() {
    let mut sim = fixed_60hz_room_sim(ROOM);
    sim.step_n(base(), 8);

    // THE ONE MOVED LINE. In fixture A this rest happens with the mint
    // already in hand.
    commit_a_checkpoint(&mut sim);

    let minted = mint_a_dynamic_item(&mut sim);
    let landed = resting_place(&mut sim, &minted);
    pick_up(&mut sim, landed, &minted);

    // Carried next door and left there, exactly as in fixture A.
    walk_to(&mut sim, NEIGHBOUR);
    sim.step_frame(ControlFrame {
        attack_pressed: true,
        shield_held: true,
        ..ControlFrame::default()
    });
    sim.step_n(base(), 30);
    walk_to(&mut sim, ROOM);

    // THE NON-VACUITY GUARD, and it is the same one fixture A needs: the
    // entity really is gone before the death, so what happens next is a decision
    // about REBUILDING rather than about re-assigning something still resident.
    assert!(
        occurrences(&mut sim, &minted).is_empty(),
        "the neighbour must actually have unloaded and taken the minted object's \
         ENTITY with it, or this test measures nothing"
    );

    die(&mut sim);

    assert!(
        occurrences(&mut sim, &minted).is_empty(),
        "⛔ `{}` was minted AFTER the checkpoint, so the checkpoint never saw it \
         and a death owes it to nobody. Coming back here means the engine is \
         rebuilding from a record of everything it ever minted rather than from \
         a snapshot of what was true at the commit. Found {:?}",
        minted.as_str(),
        occurrences(&mut sim, &minted)
    );
    let player = body(&mut sim);
    assert!(
        sim.world()
            .get::<ambition_platformer2d::combat::held_items::HeldItem>(player)
            .is_none(),
        "and nothing was put into the hand either"
    );
}

/// A runtime-minted item that was present at the checkpoint is rebuilt at its
/// recorded resting place after its room entity is destroyed.
///
/// The room unload is required so this exercises reconstruction rather than an
/// already-resident entity. Runtime mints resolve through the item catalog, and
/// reinstatement is settled during room construction because the checkpoint reset
/// may run while a different room is active.
#[test]
fn a_mint_banked_where_it_fell_comes_back_where_it_fell() {
    let mut sim = fixed_60hz_room_sim(ROOM);
    sim.step_n(base(), 8);

    let minted = mint_a_dynamic_item(&mut sim);

    // PICKED UP AND PUT DOWN AGAIN, and that is not ceremony. The whereabouts ledger tracks
    // only occurrences it ALREADY remembers — the condition is `remembers(sim_id)`, so the
    // population is exactly "things somebody carried", never "every object in the room". Its own
    // doc refuses to be the universal instance registry that would take.
    let landed = resting_place(&mut sim, &minted);
    pick_up(&mut sim, landed, &minted);
    assert_still_held(&mut sim, &minted, "the mint is picked up like any object");
    sim.step_frame(ControlFrame {
        attack_pressed: true,
        shield_held: true,
        ..ControlFrame::default()
    });
    sim.step_n(base(), 30);
    assert_returned(
        &mut sim,
        &minted,
        "putting it down must RETURN the object, not mint a second one",
    );
    let fell_at = resting_place(&mut sim, &minted);

    // Banked while it lies there: the checkpoint sees an object in nobody's
    // custody and must still be able to describe it.
    commit_a_checkpoint(&mut sim);

    // DIE WITHOUT LEAVING, so the room resets IN PLACE. The death road
    // writes `ResetToCheckpoint` and the room is torn down and rebuilt around
    // the body — `room-reset reasons=[Manual]` — which is exactly the rebuild
    // whose reinstatement debt this arm settles. Walking next door and back
    // would test the same thing through a door the fixture cannot re-cross
    // (six eliminations recorded above); staying put needs no door at all.
    die(&mut sim);
    sim.step_n(base(), 90);

    assert_returned(
        &mut sim,
        &minted,
        "a mint banked where it fell must survive its room being rebuilt — it is \
         described by nobody else, and no authored record can rebuild what the \
         simulation invented",
    );
    let back_at = resting_place(&mut sim, &minted);
    assert!(
        (back_at.0 - fell_at.0).abs() < 8.0 && (back_at.1 - fell_at.1).abs() < 8.0,
        "it came back at {back_at:?} rather than where it fell, {fell_at:?} — a \
         rebuild at the origin is the materializer's `Vec2::ZERO` arm, which is \
         honest only for something a hand supplies the position for"
    );
}

/// ONE ENTITLEMENT MUST NOT MANIFEST TWO OBJECTS.
///
/// What that could not reach is an entitlement that never had an object behind it: a granted
/// quantity is `stored`, the throw mints an INSTANCE, and nothing spends the row.
#[test]
fn one_granted_quantity_mints_exactly_one_object() {
    use ambition_platformer2d::item::ItemGrantRequested;
    use ambition_platformer2d::item::OwnedItems;
    use bevy::ecs::system::RunSystemOnce;

    let mut sim = fixed_60hz_room_sim(ROOM);
    sim.step_n(base(), 8);
    let before = dynamic_occurrences(&mut sim);

    // ONE entitlement, through the channel `<<give_item>>` uses.
    sim.world_mut().write_message(ItemGrantRequested {
        item: MINTED_ITEM,
        count: 1,
    });
    sim.step_n(base(), 4);
    assert!(
        sim.world().resource::<OwnedItems>().has(MINTED_ITEM),
        "the grant did not land, so this measures nothing"
    );

    // Try to spend it TWICE, the way the grid would: the menu offers an item the
    // player owns, so a round is attempted only while the entitlement is there.
    //
    // the ownership check is the menu's rule, restated — it is not this test
    // being polite. `equip_held_spec` is the take-custody verb and equips
    // whatever it is handed; what decides that a row is offerable is the grid,
    // and the whole point of spending the row at the mint is that the grid stops
    // offering it. A round that equipped regardless would be measuring a
    // pathway no player has.
    let mut rounds = 0;
    for _ in 0..2 {
        if !sim.world().resource::<OwnedItems>().has(MINTED_ITEM) {
            break;
        }
        rounds += 1;
        let _ = sim
            .world_mut()
            .run_system_once(equip_the_minted_item)
            .expect("the equip verb runs");
        sim.step(AgentAction {
            attack: true,
            ..base()
        });
        sim.step_n(base(), 120);
    }
    // the zero floor: a run that never equipped at all would satisfy "one
    // object" by minting none.
    assert_eq!(rounds, 1, "the entitlement was offerable {rounds} time(s)");

    let minted: Vec<SimId> = dynamic_occurrences(&mut sim)
        .into_iter()
        .filter(|id| !before.contains(id))
        .collect();
    assert_eq!(
        minted.len(),
        1,
        "one granted javelin manifested {} objects: {minted:?} — the entitlement \
         survived its own mint, so the grid can equip a phantom and throw it \
         again",
        minted.len()
    );
}

/// AND A DEATH PUTS THE ENTITLEMENT BACK — the half that makes spending it
/// safe.
///
/// The throw's own comment said exactly that, and it is why the gate stood.
///
/// ```text
/// grant     one javelin, through the <<give_item>> channel
/// bank      a real shrine rest, with the entitlement unspent
/// mint      equip and throw: the row is spent, the object exists
/// die       and the row is back, because the object is gone with it
/// ```
#[test]
fn a_death_puts_back_the_entitlement_its_mint_spent() {
    use ambition_platformer2d::item::ItemGrantRequested;
    use ambition_platformer2d::item::OwnedItems;
    use bevy::ecs::system::RunSystemOnce;

    let mut sim = fixed_60hz_room_sim(ROOM);
    sim.step_n(base(), 8);

    sim.world_mut().write_message(ItemGrantRequested {
        item: MINTED_ITEM,
        count: 1,
    });
    sim.step_n(base(), 4);

    commit_a_checkpoint(&mut sim);
    assert!(
        sim.world().resource::<OwnedItems>().has(MINTED_ITEM),
        "the checkpoint must see the entitlement, or this measures nothing"
    );

    let _ = sim
        .world_mut()
        .run_system_once(equip_the_minted_item)
        .expect("the equip verb runs");
    sim.step(AgentAction {
        attack: true,
        ..base()
    });
    sim.step_n(base(), 120);
    assert!(
        !sim.world().resource::<OwnedItems>().has(MINTED_ITEM),
        "the mint did not spend the row, so the restore below proves nothing"
    );

    die(&mut sim);
    sim.step_n(base(), 90);

    assert!(
        sim.world().resource::<OwnedItems>().has(MINTED_ITEM),
        "the death retracted the minted instance and did NOT put the quantity \
         back — the player lost a javelin for having thrown it, which is the \
         annihilation the checkpoint baseline exists to prevent"
    );
}
