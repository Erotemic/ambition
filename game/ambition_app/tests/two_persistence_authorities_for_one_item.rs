//! Cross-check the two persistence authorities for one physical item.
//! Durable save state restores catalog counts by item id; checkpoint state restores
//! concrete custody instances by `SimId`. Equipping and throwing can move one item
//! through both representations, so the two restore paths must agree.

use ambition_app::{AgentAction, Platformer2dSimHarness};
use ambition_platformer2d::actors::session::durable_horizon::SaveRestored;
use ambition_platformer2d::item::Item;
use ambition_platformer2d::item::ItemGrantRequested;
use ambition_platformer2d::item::OwnedItems;
use ambition_platformer2d::persistence::save::AmbitionGameSave;
use ambition_platformer2d::platformer::construction::SpawnOrigin;
use ambition_platformer2d::platformer::sim_id::SimId;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::Entity;

use crate::common::{base, fixed_60hz_room_sim};

/// The same room `death_restores_the_checkpoint` drives, for the same reason: it
/// authors more than one `GroundItem`, so a fixture can name a specific one.
const ROOM: &str = "central_hub_complex";

/// The authored object used by the second fixture — a held weapon with a catalog
/// slot, so picking it up crosses BOTH authorities in one press.
const AUTHORED_REWARD: &str = "ground_gun_sword";
const AUTHORED_REWARD_ITEM: Item = Item::GunSword;

/// The item the first fixture saves a COUNT of and then turns into an INSTANCE.
/// A javelin's authored `use_behavior` is `ThrowOnUse`, so a plain `Attack`
/// throws it — the mint happens on the ordinary pressed action.
const COUNTED_ITEM: Item = Item::Javelin;

type Custody = ambition_platformer2d::actors::items::pickup::ItemCustody;
type Ground = ambition_platformer2d::actors::items::pickup::GroundItem;
type Held = ambition_platformer2d::combat::held_items::HeldItem;

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

/// All live occurrences of one identity and their custody. Return a collection so duplicates fail.
fn occurrences(sim: &mut Platformer2dSimHarness, id: &SimId) -> Vec<(Entity, Custody)> {
    let mut query = sim.world_mut().query::<(Entity, &SimId, &Custody)>();
    query
        .iter(sim.world())
        .filter(|(_, sim_id, _)| *sim_id == id)
        .map(|(entity, _, custody)| (entity, *custody))
        .collect()
}

/// Simulation-minted occurrences, identified by `SpawnOrigin` rather than id spelling.
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

/// Unique in-world position for this identity; panic if the fixture is absent or duplicated.
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
        "exactly one `{}` must be lying in the world at this point",
        id.as_str()
    );
    found[0]
}

/// Durable catalog count for one item.
/// What the GRID shows for `item`: the bag plus the primary hand, projected
/// through `Inventory` (I1) — the bag alone no longer counts a wielded weapon.
fn catalog_count(sim: &mut Platformer2dSimHarness, item: Item) -> u32 {
    let in_hand = hand_item(sim);
    ambition_platformer2d::item::Inventory::new(sim.world().resource::<OwnedItems>(), in_hand)
        .count(item)
}

/// The catalog item in the primary body's hand.
fn hand_item(sim: &mut Platformer2dSimHarness) -> Option<Item> {
    let world = sim.world_mut();
    let held = world
        .query_filtered::<Option<&Held>, ambition_platformer2d::platformer::markers::PrimaryPlayerOnly>()
        .single(world)
        .ok()
        .flatten()
        .cloned();
    #[cfg(feature = "portal")]
    let gun = world
        .query_filtered::<
            Option<&ambition_platformer2d::portal::PortalGun>,
            ambition_platformer2d::platformer::markers::PrimaryPlayerOnly,
        >()
        .single(world)
        .ok()
        .flatten()
        .cloned();
    ambition_platformer2d::actors::items::pickup::item_in_hand(
        held.as_ref(),
        #[cfg(feature = "portal")]
        gun.as_ref(),
    )
}

/// Count serialized into `AmbitionGameSave`, read independently of the live `OwnedItems` mirror.
fn saved_count(sim: &Platformer2dSimHarness, item: Item) -> u32 {
    sim.world()
        .resource::<AmbitionGameSave>()
        .data()
        .items
        .iter()
        .find(|persisted| persisted.id == item.dialog_id())
        .map(|persisted| persisted.count)
        .unwrap_or(0)
}

/// Stand on an object and press Attack until it is in hand. Edge-triggered, so
/// the press is released between attempts.
fn pick_up(sim: &mut Platformer2dSimHarness, at: (f32, f32), id: &SimId) {
    sim.teleport_player(at);
    for _ in 0..40 {
        sim.step(AgentAction {
            attack: true,
            ..base()
        });
        sim.step(base());
        if occurrences(sim, id)
            .iter()
            .any(|(_, custody)| !custody.in_world())
        {
            return;
        }
    }
    panic!(
        "pressed Attack on `{}` for 40 frames and never picked it up. body at {:?}, \
         item at {at:?}, occurrences {:?}",
        id.as_str(),
        body_pos(sim),
        occurrences(sim, id),
    );
}

/// Rest at a shrine: spawn one where the body stands, press Interact, let the
/// commit land, then take the prop away again.
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
    sim.step_n(base(), 240);
}

/// THE SAVE. Step a frame, which is all it takes now: the shipped
/// `persist_inventory_to_save` runs every `Update` and mirrors the live catalog +
/// wallet into `AmbitionGameSave` for the autosave to write out.
fn save_the_inventory(sim: &mut Platformer2dSimHarness) {
    sim.step(base());
}

/// THE LOAD. Clear the "already applied" latch and step, which is precisely
/// what a fresh boot does with the latch starting `false`.
fn load_the_save(sim: &mut Platformer2dSimHarness) {
    sim.world_mut().resource_mut::<SaveRestored>().0 = false;
    sim.step(base());
    assert!(
        sim.world().resource::<SaveRestored>().0,
        "the restore must have LANDED — it waits for a body with a `BodyWallet`, \
         and a latch still false means it returned early and this fixture is \
         measuring a load that never happened"
    );
}

/// The inventory menu's equip, as a one-shot system: resolve the catalog item's
/// spec and hand it to the ONE take-custody operation. This is the body of
/// `MenuAction::Equip` in `menu::effects` with the portal fork removed.
fn equip_the_counted_item(
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
    let spec = ambition_platformer2d::actors::items::pickup::held_spec_for_item(COUNTED_ITEM)
        .expect("the javelin is a wired weapon with a held spec");
    ambition_platformer2d::actors::items::pickup::equip_held_spec(
        &mut commands,
        player,
        &mut action_set,
        spec,
    );
}

// ───────────────────────────────────────────────────────────────────────────
// FIXTURE 1 — THE BRIEF'S SCENARIO, END TO END.
// ───────────────────────────────────────────────────────────────────────────

/// Save a COUNT of one, load it, equip it, throw it (which MINTS an instance),
/// pick it up, bank it at a shrine, die — and ask both authorities what the
/// player has.
///
/// The three questions this exists to answer, with assertions rather than prose:
///
/// ```text
/// 1. holding it AND owning it?      an instance in hand + a non-zero catalog row
/// 2. decremented once/twice/never?  the count at every beat
/// 3. does the save agree with hand?  what a second round-trip would write to disk
/// ```
#[test]
fn a_saved_count_becomes_an_instance_and_the_two_authorities_are_compared() {
    let mut sim = fixed_60hz_room_sim(ROOM);
    sim.step_n(base(), 8);

    // ── the boot load, over a fresh save: keeps the live starter and unlatches
    //    the mirror, which is gated on the restore having happened ────────────
    load_the_save(&mut sim);

    // ── SAVE, with a count of 1 for a held-weapon-class item ─────────────────
    //
    // Granted through the channel `<<give_item>>` uses, then mirrored into the
    // save by the shipped `persist_inventory_to_save`.
    sim.world_mut().write_message(ItemGrantRequested {
        item: COUNTED_ITEM,
        count: 1,
    });
    sim.step_n(base(), 8);
    assert_eq!(
        catalog_count(&mut sim, COUNTED_ITEM),
        1,
        "the grant channel must put exactly one in the catalog"
    );
    save_the_inventory(&mut sim);
    assert_eq!(
        saved_count(&sim, COUNTED_ITEM),
        1,
        "and the durable authority must have mirrored it — the whole fixture is \
         about what happens to THIS row"
    );

    // ── LOAD ─────────────────────────────────────────────────────────────────
    load_the_save(&mut sim);
    let after_load = catalog_count(&mut sim, COUNTED_ITEM);
    assert_eq!(
        after_load, 1,
        "the load restores the saved count, which is the quantity the menu then \
         equips out of"
    );

    // ── EQUIP out of the count table ─────────────────────────────────────────
    let before_mint = dynamic_occurrences(&mut sim);
    sim.world_mut()
        .run_system_once(equip_the_counted_item)
        .expect("the equip runs");
    sim.step_n(base(), 2);
    let player = body(&mut sim);
    assert!(
        sim.world().get::<Held>(player).is_some(),
        "the body must be holding the equipped spec"
    );
    // THE NON-VACUITY GUARD FOR THE WHOLE ROAD: there is NO OBJECT behind
    // this hand, so the throw below has to mint rather than hand one back.
    assert_eq!(
        dynamic_occurrences(&mut sim),
        before_mint,
        "equipping out of the catalog must not create a world instance — that is \
         the whole reason throwing one has to mint"
    );
    let after_equip = catalog_count(&mut sim, COUNTED_ITEM);

    // ── THROW: the quantity becomes an instance ──────────────────────────────
    sim.step(AgentAction {
        attack: true,
        ..base()
    });
    sim.step_n(base(), 120);
    let minted: Vec<SimId> = dynamic_occurrences(&mut sim)
        .into_iter()
        .filter(|id| !before_mint.contains(id))
        .collect();
    assert_eq!(
        minted.len(),
        1,
        "throwing with no object behind the hand mints exactly one instance"
    );
    let minted = minted.into_iter().next().expect("one mint");
    let after_throw = catalog_count(&mut sim, COUNTED_ITEM);

    // ── PICK IT UP ───────────────────────────────────────────────────────────
    let landed = resting_place(&mut sim, &minted);
    pick_up(&mut sim, landed, &minted);
    let after_pickup = catalog_count(&mut sim, COUNTED_ITEM);

    // ── BANK IT, then DIE ────────────────────────────────────────────────────
    commit_a_checkpoint(&mut sim);
    die(&mut sim);

    let live = occurrences(&mut sim, &minted);
    let after_death = catalog_count(&mut sim, COUNTED_ITEM);
    // The second round-trip: mirror whatever the reset left behind, so the save
    // row below is what would actually land on disk.
    sim.step_n(base(), 8);
    save_the_inventory(&mut sim);
    let saved_after_death = saved_count(&sim, COUNTED_ITEM);

    eprintln!(
        "MEASURED  after_load={after_load} after_equip={after_equip} \
         after_throw={after_throw} after_pickup={after_pickup} \
         after_death={after_death} saved_after_death={saved_after_death} \
         live_occurrences={live:?}"
    );

    // ── ANSWER 1: holding it AND owning it? ──────────────────────────────────
    assert_eq!(
        live.len(),
        1,
        "⛔ the banked instance must come back exactly once. Got {live:?}"
    );
    let custodian = body(&mut sim);
    assert!(
        live[0].1.held_by(custodian),
        "the checkpoint saw it in a hand, so the death owes it back to that hand"
    );
    assert_eq!(
        after_death, 1,
        "⭐ ANSWER 1 — BOTH, and as of 2026-08-19 that is CORRECT rather than the \
         open half it used to be. The player holds the minted instance and the \
         catalog shows one, but no longer because the mint failed to spend the \
         row: the mint DOES spend it, and the death put it back from \
         `OwnedItemsBaseline`. ⇒ the number is the same and its meaning is the \
         opposite — a restored entitlement behind a restored object, not two \
         claims on one thing."
    );

    // ── ANSWER 2: decremented once, twice, or never? ─────────────────────────
    assert_eq!(
        [
            after_load,
            after_equip,
            after_throw,
            after_pickup,
            after_death
        ],
        [1, 1, 0, 1, 1],
        "⭐ ANSWER 2 — ONCE, AT THE THROW, and the shape of the row is the whole \
         story. Equipping still does not spend it (the hand is not the ledger); \
         the THROW spends it, because that is the beat a QUANTITY becomes an \
         INSTANCE and it must stop being both; the pickup reads 1 again as a \
         PROJECTION of the equipped slot rather than a stored row (`count` is \
         `stored.max(equipped)`); and the death reads 1 because \
         `OwnedItemsBaseline` put the stored row back when it retracted the \
         instance. ⛔ this array was [1,1,1,1,1] until 2026-08-19 and that was \
         D132's surviving defect, not its design: one granted javelin could be \
         thrown twice and manifest two objects."
    );

    // ── ANSWER 3: does a second save round-trip agree with the hand? ─────────
    assert_eq!(
        saved_after_death, 0,
        "⭐ ANSWER 3 — the save says 0 and the hand holds 1, AND THAT IS THE FORK \
         CLOSING rather than the two authorities disagreeing. ⛔ this read 1 and \
         1 until 2026-08-19, and the old note said why that was bad: they agreed \
         'because the row was never spent… one javelin is being described \
         twice.' Now the quantity became the object and stopped being a \
         quantity, so the file describes it ONCE — as an object in a hand, \
         restored by custody — and the stored row is honestly empty. ⚠ the \
         checkpoint here is committed AFTER the throw, so the baseline remembers \
         the row already spent and the death restores that; commit BEFORE the \
         throw and the row comes back, which is the sibling test."
    );
}

// ───────────────────────────────────────────────────────────────────────────
// FIXTURE 2 — THE CROSSING THE FIRST ONE DOES NOT REACH.
// ───────────────────────────────────────────────────────────────────────────

/// If death restores an authored object to the world, the inventory catalog
/// must stop claiming it. The fixture checkpoints before acquisition, then
/// verifies the held object projects as owned without becoming a durable count,
/// and that death retracts the projected count when the same object returns to
/// its authored placement.
#[test]
fn a_death_that_returns_the_object_leaves_nothing_in_the_catalog_claiming_it() {
    let mut sim = fixed_60hz_room_sim(ROOM);
    sim.step_n(base(), 8);

    let reward = SimId::placement(AUTHORED_REWARD);
    // The boot load, which is also what unlatches the mirror.
    load_the_save(&mut sim);

    // NON-VACUITY: the starter roster must not already own this, or every
    // reading below is about the starter set rather than about this acquisition.
    assert_eq!(
        catalog_count(&mut sim, AUTHORED_REWARD_ITEM),
        0,
        "the catalog must start without a `{}`",
        AUTHORED_REWARD_ITEM.dialog_id()
    );

    // ── the checkpoint FIRST, with empty hands ───────────────────────────────
    commit_a_checkpoint(&mut sim);

    // ── and only THEN the acquisition ────────────────────────────────────────
    let pedestal = resting_place(&mut sim, &reward);
    pick_up(&mut sim, pedestal, &reward);

    // ── CLAIM 1: held reads as owned, because the hand is projected ──────────
    assert_eq!(
        catalog_count(&mut sim, AUTHORED_REWARD_ITEM),
        1,
        "⭐ the grid must still show the weapon you are carrying. Deleting the \
         grant without projecting the hand would leave a player holding an axe \
         whose inventory screen says they have none"
    );

    // ── CLAIM 2: and it is NOT a quantity, so the save does not write it ─────
    save_the_inventory(&mut sim);
    assert_eq!(
        saved_count(&sim, AUTHORED_REWARD_ITEM),
        0,
        "⛔ a HELD object must not reach the save as a count. It would come back \
         on the next load as a row while the room re-authors the object — one \
         weapon saved, two loaded, which is this whole defect arriving by the \
         durable road instead of the checkpoint one"
    );

    // ── die HOLDING it, so the retraction arm genuinely runs ─────────────────
    die(&mut sim);

    let live = occurrences(&mut sim, &reward);
    assert_eq!(
        live.len(),
        1,
        "exactly one occurrence answers to `{}`. Got {live:?}",
        reward.as_str()
    );
    assert!(
        live[0].1.in_world(),
        "⭐⭐ NON-VACUITY: the acquisition happened after the checkpoint, so the \
         death must genuinely have put the object back in the world — otherwise \
         the count below is 0 for the wrong reason. Got {:?}",
        live[0].1
    );

    // ── CLAIM 3: the catalog went with it ────────────────────────────────────
    sim.step_n(base(), 8);
    save_the_inventory(&mut sim);
    let still_owned = catalog_count(&mut sim, AUTHORED_REWARD_ITEM);
    let still_saved = saved_count(&sim, AUTHORED_REWARD_ITEM);
    eprintln!("MEASURED  still_owned={still_owned} still_saved={still_saved}");
    assert_eq!(
        still_owned,
        0,
        "⛔ the object is back on its pedestal, so nothing may still claim it. A \
         `{}` that survives here is a phantom the menu will equip and mint a \
         second real one from",
        AUTHORED_REWARD_ITEM.dialog_id()
    );
    assert_eq!(
        still_saved, 0,
        "and the durable authority must not write the phantom to disk, where it \
         would outlive the session that invented it"
    );
}

/// THE POISON: A GRANTED QUANTITY SURVIVES THE SAME DEATH THAT RETRACTS
/// THE OBJECT MINTED OUT OF IT.
///
/// this is what stops the cheap wrong answer, and fixture 2 above does NOT stop it. The
/// obvious way to make fixture 2 green is to have `restore_custody_to_checkpoint`'s retraction
/// arm call `owned.take(item, 1)` beside the despawn — a reconciliation step between two
/// stores, which is a bigger problem wearing a smaller one's clothes. It passes fixture 2
/// exactly.
///
/// the state this reaches is one where only the wrong implementation can
/// act. The object really is retracted — asserted, not assumed — so the
/// retraction arm ran to completion; the granted row is the only thing left for
/// it to be wrong about.
#[test]
fn a_granted_quantity_survives_the_death_that_retracts_the_instance_minted_from_it() {
    let mut sim = fixed_60hz_room_sim(ROOM);
    sim.step_n(base(), 8);
    load_the_save(&mut sim);

    // ── the entitlement, conferred with NO object behind it ──────────────────
    sim.world_mut().write_message(ItemGrantRequested {
        item: COUNTED_ITEM,
        count: 1,
    });
    sim.step_n(base(), 8);
    save_the_inventory(&mut sim);
    assert_eq!(
        saved_count(&sim, COUNTED_ITEM),
        1,
        "⭐⭐ NON-VACUITY: this really is a STORED quantity, not a projected hand \
         — the save writes it, which the held-object case above proves it would \
         not do for a projection"
    );

    // ── the checkpoint sees the quantity and no object ───────────────────────
    commit_a_checkpoint(&mut sim);

    // ── and only THEN the quantity becomes an instance ───────────────────────
    let before_mint = dynamic_occurrences(&mut sim);
    sim.world_mut()
        .run_system_once(equip_the_counted_item)
        .expect("the equip runs");
    sim.step_n(base(), 2);
    sim.step(AgentAction {
        attack: true,
        ..base()
    });
    sim.step_n(base(), 120);
    let minted: Vec<SimId> = dynamic_occurrences(&mut sim)
        .into_iter()
        .filter(|id| !before_mint.contains(id))
        .collect();
    assert_eq!(minted.len(), 1, "the throw mints exactly one instance");
    let minted = minted.into_iter().next().expect("one mint");

    // Acquire it, so the death has a custody row to retract rather than an
    // object lying on a floor it was never asked about.
    let landed = resting_place(&mut sim, &minted);
    pick_up(&mut sim, landed, &minted);

    die(&mut sim);

    // THE STATE ONLY THE WRONG IMPLEMENTATION CAN ACT ON: the retraction arm
    // ran, and what it did to the catalog is the whole question.
    assert!(
        occurrences(&mut sim, &minted).is_empty(),
        "the instance was minted AFTER the checkpoint, so the death owes it to \
         nobody — and if it is still here the retraction never ran and the claim \
         below is vacuous. Found {:?}",
        occurrences(&mut sim, &minted)
    );

    sim.step_n(base(), 8);
    save_the_inventory(&mut sim);
    let granted = catalog_count(&mut sim, COUNTED_ITEM);
    let granted_saved = saved_count(&sim, COUNTED_ITEM);
    eprintln!("MEASURED  granted={granted} granted_saved={granted_saved}");
    assert_eq!(
        granted, 1,
        "⛔ the javelin was GIVEN before the checkpoint. A reset that retracts \
         the entitlement along with the object has confused a quantity with an \
         occurrence and taken away something the checkpoint saw the player owning"
    );
    assert_eq!(
        granted_saved, 1,
        "and it stays a stored quantity, so the durable save still carries it"
    );
}

/// A LOAD THEN A DEATH MUST NOT EMPTY THE BAG.
///
/// But a fresh process starts with the DEFAULT baseline, an empty bag, and the durable load
/// adopted only three of the four baselines. So the first death after a load restored *nothing*
/// over everything the file remembered.
///
/// this now also pins the ordering that the original version accidentally
/// hid. Reloading inside one process leaves the live bag equal to the file, so
/// a baseline captured BEFORE the restore looks correct. The second harness below
/// starts from a deliberately different bag; only post-load adoption can pass.
#[test]
fn a_load_then_a_death_keeps_what_the_file_remembered() {
    let mut sim = fixed_60hz_room_sim(ROOM);
    sim.step_n(base(), 8);
    load_the_save(&mut sim);

    // Something the file remembers, through the channel the game uses.
    sim.world_mut()
        .write_message(ambition_platformer2d::item::ItemGrantRequested {
            item: COUNTED_ITEM,
            count: 1,
        });
    sim.step_n(base(), 4);
    save_the_inventory(&mut sim);
    let before = catalog_count(&mut sim, COUNTED_ITEM);
    assert_eq!(before, 1, "the grant must land, or this measures nothing");

    // Copy only the file into a second harness.
    let persisted = sim.world().resource::<AmbitionGameSave>().clone();
    let mut reloaded = fixed_60hz_room_sim(ROOM);
    reloaded.step_n(base(), 8);
    assert_eq!(
        catalog_count(&mut reloaded, COUNTED_ITEM),
        0,
        "the fresh-process poison requires pre-load live state to differ from disk",
    );
    *reloaded.world_mut().resource_mut::<AmbitionGameSave>() = persisted;
    load_the_save(&mut reloaded);
    reloaded.step_n(base(), 4);

    die(&mut reloaded);
    reloaded.step_n(base(), 60);

    assert_eq!(
        catalog_count(&mut reloaded, COUNTED_ITEM),
        before,
        "the first death after a fresh-process load took back what the file remembered —          the entitlement baseline captured the pre-load bag instead of the restored one"
    );
}
