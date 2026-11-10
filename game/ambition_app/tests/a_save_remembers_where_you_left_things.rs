//! THE DURABLE HORIZON, DRIVEN — a save file that remembers where the player
//! left things, and a load that puts them there.
//!
//! ```text
//! current world truth    AuthoredOccurrences + ItemCustody
//! checkpoint truth       OccurrenceBaseline + CustodyBaseline + MintedItemBaseline
//! durable save truth     ← THIS FILE
//! ```
//!
//! It carries three lists now, and they are the same three values the checkpoint copies rather
//! than a second description of them.
//!
//! # What is driven, and what is constructed
//!
//! the one manufactured beat is the BOOT. A load is two facts inside one
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

use crate::common::{base, door_to, fixed_60hz_room_sim, possess_the_authored_enemy, walk_through_the_door_to};

type Custody = ambition_platformer2d::held_items::ItemCustody;
type Held = ambition_platformer2d::combat::held_items::HeldItem;
type SaveRestored = ambition_platformer2d::platformer::lifecycle::SaveRestored;

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
/// a COUNT, never a lookup. Two live things behind one `SimId` and zero
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

/// Where the one occurrence of `id` is lying.
fn resting_place(sim: &mut Platformer2dSimHarness, id: &SimId) -> (f32, f32) {
    let mut query = sim.world_mut().query::<(&SimId, &ambition_platformer2d::engine_core::BodyKinematics, &Custody)>();
    let found: Vec<(f32, f32)> = query
        .iter(sim.world())
        .filter(|(sim_id, _, custody)| *sim_id == id && custody.in_world())
        .map(|(_, ground, _)| (ground.pos.x, ground.pos.y))
        .collect();
    if found.len() != 1 {
        // Where every occurrence is, and where the body stands: a thing in a
        // hand and a thing in another room read the same from the count.
        let all = occurrences(sim, id);
        let at = sim.observation();
        panic!(
            "exactly one `{}` must be lying in the world here; found {} lying, every \
             occurrence {all:?}, the body at {:?} in `{}`",
            id.as_str(),
            found.len(),
            at.player_pos,
            at.active_room,
        );
    }
    found[0]
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

// ───────────────────────────────────────────────────────────────────────────
// The save and the load.
// ───────────────────────────────────────────────────────────────────────────

/// THE FILE. What `persist_occurrence_horizon_to_save` has mirrored into
/// `AmbitionGameSave` — the exact value the autosave would commit to disk.
///
/// read out of the save rather than out of the live ledger, because the
/// question is what would land on the file. A fixture that read
/// `AuthoredOccurrences` would be asserting the thing it means to compare
/// against.
fn the_file(sim: &Platformer2dSimHarness) -> AmbitionGameSaveData {
    sim.world().resource::<AmbitionGameSave>().data().clone()
}

/// Boot a fresh room with a supplied save and wait for the production restore latch.
fn boot_with(room: &str, file: &AmbitionGameSaveData) -> Platformer2dSimHarness {
    let mut sim = fixed_60hz_room_sim(room);
    sim.step_n(base(), 8);
    sim.world_mut().resource_mut::<AmbitionGameSave>().0 = file.clone();
    sim.world_mut().resource_mut::<SaveRestored>().0 = false;
    for _ in 0..90 {
        sim.step(base());
        if sim.world().resource::<SaveRestored>().0 {
            break;
        }
    }
    // A few frames past the latch, so the rebuild it requested has landed.
    sim.step_n(base(), 4);
    assert!(
        sim.world().resource::<SaveRestored>().0,
        "the load must have LANDED — a latch still false means it returned early \
         and everything below is measuring a load that never happened"
    );
    sim
}

/// A save taken while possessing an enemy must not persist transient body custody.
/// On a fresh boot nobody is possessing it, so its authored room must recreate exactly one body.
#[test]
fn a_save_taken_mid_possession_does_not_delete_the_enemy_in_a_fresh_process() {
    use ambition_platformer2d::actors::control::possession::PossessionState;
    use ambition_platformer2d::platformer::lifecycle::AuthoredOccurrences;

    let mut sim = fixed_60hz_room_sim("vertical_shaft");
    let (_actor, id) = possess_the_authored_enemy(&mut sim);
    // Let the mirror run: it is gated on the restore latch and value-compared, so
    // it reaches the save on the first tick after custody settles.
    sim.step_n(base(), 4);

    // Confirm the live occurrence is in custody before testing persistence.
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

    // Transient possession custody must not reach either durable save table.
    let file = the_file(&sim);
    assert!(
        !file.occurrences().iter().any(|row| row.id == id.as_str()),
        "the save carries a whereabouts for a body whose custodian is possession \
         state the save does not hold. On load nobody is driving it, and the only \
         thing standing between that row and the enemy's permanent deletion is a \
         live retraction winning a race. Saved occurrences were {:?}",
        file.occurrences()
    );
    assert!(
        !file.custody().iter().any(|row| row.occurrence == id.as_str()),
        "the save carries a custody row naming a hand it cannot reconstruct. \
         Saved custody was {:?}",
        file.custody()
    );

    // Fresh boot with no possession must restore exactly one authored enemy.
    let mut fresh = boot_with("vertical_shaft", &file);
    assert_eq!(
        fresh.world().resource::<PossessionState>().get(ambition_platformer2d::characters::control::PlayerSlot::PRIMARY).possessed,
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

/// Count live entities with this identity so both deletion and duplication fail.
fn live_bodies_named(sim: &mut Platformer2dSimHarness, id: &SimId) -> usize {
    let mut query = sim.world_mut().query::<&SimId>();
    query.iter(sim.world()).filter(|found| *found == id).count()
}

/// An authored object moved to another room must reload there exactly once, while
/// the source room remains suppressed. The control is the source room before the
/// pickup: it authors the object, so the empty pedestal after the load is about
/// the save.
#[test]
fn an_object_left_in_another_room_is_lying_there_after_a_load() {
    let mut sim = fixed_60hz_room_sim(SOURCE_ROOM);
    sim.step_n(base(), 10);
    let (_, authored) = the_only_authored_item(&mut sim);
    let pedestal = resting_place(&mut sim, &authored);

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

    let file = the_file(&sim);
    assert_eq!(
        file.occurrences(),
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
}

// ───────────────────────────────────────────────────────────────────────────
// FALSIFIER B — CUSTODY SURVIVES A LOAD, AND A TERMINAL ROW STAYS TERMINAL.
// ───────────────────────────────────────────────────────────────────────────

/// Save while holding a weapon and restore the same occurrence to the same hand.
/// The source room suppresses its authored copy; checkpoint reconstruction
/// materializes the held occurrence from the saved custody identity.
///
/// And a `Consumed` occurrence stays absent after a load, while an unrelated
/// authored occurrence stays on its pedestal. That file is constructed directly
/// because no live producer emits a terminal `Consumed` row for an authored
/// ground item.
///
/// The control for both loads is the room before anything happens: it authors
/// both objects, on their pedestals, and the hand is empty.
#[test]
fn a_weapon_in_your_hands_is_still_in_your_hands_after_a_load() {
    let reward = SimId::placement(ENDED);
    let untouched = SimId::placement(UNTOUCHED);

    let mut sim = fixed_60hz_room_sim(TWO_ITEM_ROOM);
    sim.step_n(base(), 10);
    // ── THE CONTROL ─────────────────────────────────────────────────────────
    assert!(
        occurrences(&mut sim, &reward)
            .first()
            .is_some_and(|(_, custody)| custody.in_world()),
        "'{TWO_ITEM_ROOM}' authors `{ENDED}` on its pedestal, so its absence or \
         its place in a hand below is about the save and not about the room"
    );
    let empty_handed = body(&mut sim);
    assert!(
        sim.world().get::<Held>(empty_handed).is_none(),
        "the room starts with an empty hand, so 'still holding it' below is a \
         claim about the SAVE"
    );
    let untouched_pedestal = resting_place(&mut sim, &untouched);

    pick_up(&mut sim, &reward);
    sim.step_n(base(), 4);

    let file = the_file(&sim);
    assert_eq!(
        file.occurrences(),
        vec![PersistedOccurrence::new(
            reward.as_str(),
            PersistedWhereabouts::InCustody
        )],
        "⭐ NON-VACUITY: the file must record the object as carried. Anything else \
         and the load below is not being asked the question this fixture is about"
    );
    assert_eq!(
        file.custody().len(),
        1,
        "⚠ and it must name the HAND separately: an `InCustody` row says somebody \
         has it, which is enough to stop a room minting a second one and NOT \
         enough to put it back. Got {:?}",
        file.custody()
    );
    assert_eq!(file.custody()[0].occurrence, reward.as_str());

    // ── LOAD: the weapon is back in the hand ────────────────────────────────
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

    // ── LOAD a file that marks the same occurrence consumed ────────────────
    let mut ended = AmbitionGameSaveData::new();
    ended.set_durable_horizon(vec![PersistedOccurrence::new(
        reward.as_str(),
        PersistedWhereabouts::Consumed,
    )], Vec::new());
    let mut consumed = boot_with(TWO_ITEM_ROOM, &ended);
    assert!(
        occurrences(&mut consumed, &reward).is_empty(),
        "⛔ the world remembered ending this occurrence and the file carried that \
         memory. Bringing it back is a load overruling a terminal disposition — \
         the consumed key back on its pedestal, the destroyed mechanism whole \
         again. Got {:?}",
        occurrences(&mut consumed, &reward)
    );
    let still = occurrences(&mut consumed, &untouched);
    assert_eq!(
        still.len(),
        1,
        "⛔ a record with no row is authored as written, every time. Losing it \
         here is a load that treats 'the file mentions occurrences' as 'the file \
         is the whole world'. Got {still:?}"
    );
    assert!(still[0].1.in_world());
    let where_it_is = resting_place(&mut consumed, &untouched);
    let drift =
        (where_it_is.0 - untouched_pedestal.0).abs() + (where_it_is.1 - untouched_pedestal.1).abs();
    assert!(
        drift < 2.0,
        "and it is on its own pedestal at {untouched_pedestal:?}, not somewhere a \
         restored row put it. Found at {where_it_is:?}"
    );
}

/// The boss whose profile authors a `signature_gauntlet`, and the id that
/// gauntlet reaches the world under. ⛔ `volley` has NO `Item` row: it resolves
/// only through `ambition_characters`' `HELD_ITEMS`, which is the point.
const GAUNTLET_BOSS: &str = "saved_gauntlet_boss";
const GAUNTLET: &str = "volley";

/// The identity of the gauntlet the BOSS dropped, told apart from the one the
/// world authors by its provenance.
fn dropped_gauntlet(sim: &mut Platformer2dSimHarness) -> Vec<SimId> {
    let mut query = sim.world_mut().query::<(
        &ambition_platformer2d::held_items::GroundItem,
        &SimId,
        &ambition_platformer2d::platformer::construction::SpawnOrigin,
    )>();
    query
        .iter(sim.world())
        .filter(|(ground, _, origin)| {
            ground.spec.id == GAUNTLET
                && matches!(
                    origin,
                    ambition_platformer2d::platformer::construction::SpawnOrigin::Dynamic { .. }
                )
        })
        .map(|(_, sim_id, _)| sim_id.clone())
        .collect()
}

/// A RUNTIME MINT THE ITEM CATALOG HAS NEVER HEARD OF SURVIVES A FRESH PROCESS.
///
/// ⛔⛔ THE POINT IS THE REGISTRY, NOT THE ITEM. `held_spec_by_id` consults the
/// item catalog AND `ambition_characters`' `HELD_ITEMS`, and its own comment
/// says consulting one alone "silently loses half the items". Every other arm in
/// this file and its siblings carries a spec the CATALOG knows — the axe, the
/// gun-sword, the grapple, the menu-minted javelin. `volley` has no `Item` row
/// at all, so `ItemCatalog::item_by_held_item_id` answers `None` and only the second
/// registry can rebuild it.
///
/// The durable road this proves, end to end and across a process boundary:
///
/// ```text
/// mint     a real boss kill leaves the gauntlet on the floor with an identity
/// hand     the ordinary pressed pickup
/// bank     a shrine rest, so the MINTED description reaches the save file
/// load     a fresh harness, the file, and the production restore latch
/// ```
///
/// ⇒ If the brain-registry arm of `held_spec_by_id` were ever dropped, the load
/// would warn "no item spec answers to that id" and the hand would come back
/// empty.
///
/// The same file is also where the two durable writers must agree: every
/// minted-item row (`set_minted_items`, `items/pickup/minted_horizon.rs`) must
/// name an occurrence the save holds (`set_durable_horizon`,
/// `session/durable_horizon.rs`). They are separate authorities, each
/// idempotent against its own field, and a unit test that constructs both
/// cannot witness them disagreeing; this drives both production writers.
#[test]
fn a_gauntlet_the_item_catalog_never_heard_of_is_still_in_your_hands_after_a_load() {
    let mut sim = fixed_60hz_room_sim(TWO_ITEM_ROOM);
    sim.step_n(base(), 30);
    assert!(
        dropped_gauntlet(&mut sim).is_empty(),
        "precondition: the authored `{GAUNTLET}` on the hub's shelf is a \
         different road and must not be counted as the drop"
    );

    crate::boss_lifecycle::spawn_mockingbird(&mut sim, GAUNTLET_BOSS);
    crate::boss_lifecycle::kill_boss_with_a_real_hit(&mut sim, GAUNTLET_BOSS, 600);
    sim.step_n(base(), 120);

    let dropped = dropped_gauntlet(&mut sim);
    assert_eq!(
        dropped.len(),
        1,
        "the kill must leave exactly one dropped gauntlet, or the load below \
         proves nothing"
    );
    let occurrence = dropped.into_iter().next().expect("one drop");

    pick_up(&mut sim, &occurrence);
    crate::death_restores_the_checkpoint::commit_a_hub_checkpoint(&mut sim);
    sim.step_n(base(), 4);
    let file = the_file(&sim);

    // ── THE FILE: every minted row names an occurrence the save holds ───────
    // A file with no minted rows satisfies "no orphaned minted row" for free.
    assert!(
        !file.minted_items().is_empty(),
        "the save carries NO minted-item rows after a mint taken into a hand and \
         banked, so nothing below is being checked. occurrences={:?}",
        file.occurrences().len()
    );
    let known: Vec<&str> = file.occurrences().iter().map(|o| o.id.as_str()).collect();
    for row in file.minted_items() {
        assert!(
            known.contains(&row.occurrence.as_str()),
            "minted-item row names occurrence {:?}, which the REAL save does not \
             hold: the minted-item writer and the durable-horizon writer disagree \
             on the file. occurrences={known:?}",
            row.occurrence
        );
    }
    for row in file.custody() {
        assert!(
            known.contains(&row.occurrence.as_str()),
            "custody row names occurrence {:?}, which the REAL save does not hold \
             — these two share a setter, so its pairing was bypassed. \
             occurrences={known:?}",
            row.occurrence
        );
    }

    // ── THE LOAD ────────────────────────────────────────────────────────────
    let mut fresh = boot_with(TWO_ITEM_ROOM, &file);

    let after = occurrences(&mut fresh, &occurrence);
    assert_eq!(
        after.len(),
        1,
        "exactly one live occurrence of `{}` after the load: zero means the \
         durable road could not rebuild a spec the item catalog does not know, \
         two means it built one beside the one it restored. got {after:?}",
        occurrence.as_str()
    );
    assert!(
        !after[0].1.in_world(),
        "and it is in the HAND the save recorded, not lying on the floor"
    );
}

/// Mint a gauntlet off a real boss kill in the hub, take it in hand, bank it,
/// throw it down and walk out to the shaft. The gauntlet's id and where it
/// fell. The hub is then not live, so the gauntlet is a dormant record: a
/// `Placed` row in the occurrence ledger and a minted description.
fn a_banked_gauntlet_left_in_the_hub(sim: &mut Platformer2dSimHarness, boss: &str) -> (SimId, (f32, f32)) {
    sim.step_n(base(), 30);
    crate::boss_lifecycle::spawn_mockingbird(sim, boss);
    crate::boss_lifecycle::kill_boss_with_a_real_hit(sim, boss, 600);
    sim.step_n(base(), 120);
    let dropped = dropped_gauntlet(sim);
    assert_eq!(dropped.len(), 1, "the kill must leave exactly one gauntlet");
    let occurrence = dropped.into_iter().next().expect("one drop");
    pick_up(sim, &occurrence);
    crate::death_restores_the_checkpoint::commit_a_hub_checkpoint(sim);
    throw_it_down(sim);
    sim.step_n(base(), 120);
    let where_it_fell = resting_place(sim, &occurrence);
    leave_the_hub(sim, &[&occurrence]);
    (occurrence, where_it_fell)
}

/// Walk out of the hub to the shaft, so the hub retires and `dormant` are
/// records rather than live objects.
fn leave_the_hub(sim: &mut Platformer2dSimHarness, dormant: &[&SimId]) {
    assert_eq!(
        walk_through_the_door_to(sim, "vertical_shaft"),
        "vertical_shaft",
        "precondition: the body did not leave the hub, so the hub is still live"
    );
    for occurrence in dormant {
        assert!(
            occurrences(sim, occurrence).is_empty(),
            "precondition: `{}` is still live, so it is not a dormant record",
            occurrence.as_str()
        );
    }
}

/// The gauntlet is in the hub again, once, lying where it fell.
fn the_gauntlet_is_back(sim: &mut Platformer2dSimHarness, occurrence: &SimId, where_it_fell: (f32, f32)) {
    let back = occurrences(sim, occurrence);
    assert_eq!(
        back.len(),
        1,
        "exactly one live occurrence of `{}` in the hub: zero means the hub was \
         rebuilt without the dormant gauntlet. got {back:?}",
        occurrence.as_str()
    );
    assert!(back[0].1.in_world(), "and it is lying in the room, not in a hand");
    let now = resting_place(sim, occurrence);
    assert!(
        (now.0 - where_it_fell.0).abs() < 4.0 && (now.1 - where_it_fell.1).abs() < 4.0,
        "and it is where it fell: dropped at {where_it_fell:?}, rebuilt at {now:?}"
    );
}

/// OW3: a checkpoint taken in another room keeps a dormant room's minted
/// item; a checkpoint that described only the live mints would forget the
/// gauntlet lying in the hub, and the hub would be rebuilt without it.
#[test]
fn a_gauntlet_left_in_a_room_outlives_a_checkpoint_taken_in_another() {
    let mut sim = fixed_60hz_room_sim(TWO_ITEM_ROOM);
    let (occurrence, where_it_fell) = a_banked_gauntlet_left_in_the_hub(&mut sim, "dormant_checkpoint_gauntlet_boss");
    // Off the door: the shrine is used with the interact press, and on the
    // door that press goes back to the hub.
    let door = door_to(&mut sim, TWO_ITEM_ROOM).aabb;
    let at = door.center();
    sim.teleport_player((at.x + door.half_size().x + 48.0, at.y));
    sim.step_n(base(), 30);
    crate::death_restores_the_checkpoint::commit_a_checkpoint(&mut sim);
    assert_eq!(
        sim.observation().active_room,
        "vertical_shaft",
        "precondition: the checkpoint was not taken in the shaft"
    );
    walk_through_the_door_to(&mut sim, TWO_ITEM_ROOM);
    sim.step_n(base(), 60);
    the_gauntlet_is_back(&mut sim, &occurrence, where_it_fell);
}

/// OW3: a save taken in another room keeps a dormant room's minted item. The
/// file must place the gauntlet in the hub AND hold a description to build it
/// from, and the load's rebuild must accept a reinstated row whose parent, the
/// boss that dropped it, is in no room of a fresh process. (The door witnesses
/// do not see that rule: their boss is staged room content that the rebuild
/// plans again.)
#[test]
fn a_gauntlet_left_in_a_room_outlives_a_save_taken_in_another() {
    let mut sim = fixed_60hz_room_sim(TWO_ITEM_ROOM);
    let (occurrence, where_it_fell) = a_banked_gauntlet_left_in_the_hub(&mut sim, "dormant_save_gauntlet_boss");
    sim.step_n(base(), 30);
    let file = the_file(&sim);
    assert!(
        file.occurrences().iter().any(|row| row.id == occurrence.as_str()
            && matches!(&row.whereabouts, PersistedWhereabouts::Placed { room, .. } if room == TWO_ITEM_ROOM)),
        "precondition: the file does not place the gauntlet in the hub, so the \
         load below measures nothing. occurrences={:?}",
        file.occurrences()
    );
    assert!(
        file.minted_items().iter().any(|row| row.occurrence == occurrence.as_str()),
        "the file places the gauntlet in the hub but holds no minted row to \
         build it from: the save forgot a dormant mint. minted={:?}",
        file.minted_items()
    );
    // The load puts the body in the hub, its checkpoint room, and builds the
    // hub from the file.
    let mut fresh = boot_with(TWO_ITEM_ROOM, &file);
    the_gauntlet_is_back(&mut fresh, &occurrence, where_it_fell);
}

/// OW3: runtime mints no checkpoint ever saw are still in the room they were
/// left in when it is live again: one taken up and put down, and one nobody
/// ever carried.
///
/// The room build rebuilds a dormant mint from the ledger's `at` and the mint's
/// description, through `held_spec_by_id` (`features/ecs/spawn`); a boss
/// gauntlet resolves only through `HELD_ITEMS`, so a describer narrowed to the
/// item catalog loses it while every catalog item is fine. The carried one
/// needs a description no checkpoint wrote; the uncarried one needs a ledger
/// row that custody never admitted.
#[test]
fn a_gauntlet_no_checkpoint_saw_is_still_where_it_was_left() {
    let mut sim = fixed_60hz_room_sim(TWO_ITEM_ROOM);
    sim.step_n(base(), 30);

    // Nobody carries this one.
    crate::boss_lifecycle::spawn_mockingbird(&mut sim, "uncarried_gauntlet_boss");
    crate::boss_lifecycle::kill_boss_with_a_real_hit(&mut sim, "uncarried_gauntlet_boss", 600);
    sim.step_n(base(), 120);
    let dropped = dropped_gauntlet(&mut sim);
    assert_eq!(dropped.len(), 1, "the first kill must leave exactly one gauntlet");
    let uncarried = dropped.into_iter().next().expect("one drop");
    let uncarried_fell = resting_place(&mut sim, &uncarried);

    // This one is taken up and put down, with no checkpoint between. Its boss
    // stands well clear of the first gauntlet, so the pickup cannot take that.
    crate::boss_lifecycle::spawn_mockingbird_beside(&mut sim, "unbanked_gauntlet_boss", 240.0);
    crate::boss_lifecycle::kill_boss_with_a_real_hit(&mut sim, "unbanked_gauntlet_boss", 600);
    sim.step_n(base(), 120);
    let carried = dropped_gauntlet(&mut sim)
        .into_iter()
        .find(|id| *id != uncarried)
        .expect("the second kill must leave a second gauntlet");
    pick_up(&mut sim, &carried);
    throw_it_down(&mut sim);
    sim.step_n(base(), 120);
    let carried_fell = resting_place(&mut sim, &carried);
    assert_eq!(
        resting_place(&mut sim, &uncarried),
        uncarried_fell,
        "precondition: the first gauntlet did not move while the second was handled"
    );

    leave_the_hub(&mut sim, &[&uncarried, &carried]);
    walk_through_the_door_to(&mut sim, TWO_ITEM_ROOM);
    sim.step_n(base(), 60);
    the_gauntlet_is_back(&mut sim, &uncarried, uncarried_fell);
    the_gauntlet_is_back(&mut sim, &carried, carried_fell);
}
