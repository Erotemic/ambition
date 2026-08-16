//! **The checkpoint decides what a death takes back — not what kind of thing it
//! is.**
//!
//! The maintainer's rule, 2026-08-15:
//!
//! ```text
//! C0: a reward is authored on its pedestal
//!   pick it up, die before committing      → it is back on the pedestal, and you do not have it
//!   pick it up again, commit C1, die       → you still have it, and the pedestal stays empty
//!   after C1 pick up something else, die   → you still have the first, the second went back
//! ```
//!
//! ⛔⛔ **the third line is the whole test, and it is why `KeyItem => survives
//! death` is the wrong shape.** An item-kind rule satisfies the first two lines
//! and fails the third: the thing that decides is WHEN the acquisition happened
//! relative to the last committed checkpoint, and the kind of object never enters
//! the question. Encoding a kind rule would put a second authority beside the
//! checkpoint, and the two start disagreeing the first time content changes.
//!
//! # What this drives, and what it refuses to drive
//!
//! Every beat goes through an ordinary road: the authored LDtk ground item, the
//! real pressed pickup, a real `HealShrine` touched with a real `Interact`, and
//! a real death report. ⛔ **nothing here writes `CheckpointCommitted`,
//! `ResetToCheckpoint`, `OccurrenceBaseline` or `CustodyBaseline` directly.**
//! Writing the baseline by hand and then asserting the baseline would be a test
//! of `clone()`; the claim under test is that the death road and the checkpoint
//! road MEET, and only production wiring can be wrong about that.
//!
//! ⭐ the one thing constructed rather than authored is the shrine, because no
//! room in this world authors one next to a ground item. It is spawned as the
//! ordinary component the LDtk lowering produces, at the body's own position, so
//! `heal_save_shrine_system` runs against it exactly as it would in a room that
//! authored it.

use ambition_app::{AgentAction, Platformer2dSimHarness};
use ambition_platformer2d::engine_core::ControlFrame;
use ambition_platformer2d::platformer::sim_id::SimId;
use bevy::prelude::Entity;

use crate::common::{base, fixed_60hz_room_sim};

/// `central_hub_basement` authors fourteen `GroundItem`s on one shelf, which is
/// what lets this fixture name TWO objects and hold them at different horizons
/// in one life. ⛔ that is beat 7's whole requirement and no other authored room
/// can meet it — a one-item room can show an object reverting and an object
/// persisting, but never both at once, which is the shape an item-kind rule
/// cannot produce.
const ROOM: &str = "central_hub_basement";

/// The object the player BANKS: acquired, then a checkpoint committed over it.
const REWARD: &str = "ground_gun_sword";
/// The object acquired AFTER that checkpoint, and therefore owed back on death.
///
/// ⚠ picked for being inert while held. The maintainer's word was
/// "temporary/disposable", but that is a description of when it was acquired,
/// not a property the engine reads — which is exactly the point of the fixture.
const TEMPORARY: &str = "ground_grapple";

type Custody = ambition_platformer2d::actors::items::pickup::ItemCustody;
type Ground = ambition_platformer2d::actors::items::pickup::GroundItem;

/// Every live occurrence of `authored`, and whether each is in a hand.
///
/// ⭐ **a COUNT, never a lookup.** "Do I still have the key" is answered by the
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
        .query_filtered::<Entity, ambition_platformer2d::actors::actor::PrimaryPlayerOnly>();
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
/// ⚠ **the shrine is despawned again afterwards.** It is a fixture prop, and one
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
    sim.world_mut()
        .write_message(ambition_platformer2d::actors::ActorDiedMessage {
            victim,
            pos: ambition_platformer2d::engine_core::Vec2::new(x, y),
            cause: ambition_platformer2d::actors::DeathCause {
                source: ambition_platformer2d::combat::HitSource::Hazard,
                attacker: None,
            },
        });
    // The interlude counts down on the sim clock and only then asks the roster
    // for the consequence; the rebuild it requests takes several more frames.
    sim.step_n(base(), 240);
}

/// **THE FIXTURE. Seven beats, in the order the rule states them.**
///
/// ⭐ **one test rather than seven, because the beats are a HISTORY.** Beat 6's
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

    // ── 3. death BEFORE any checkpoint restores C0 ───────────────────────────
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

    // ── 7. ⭐⭐ THE BEAT THAT KILLS THE ITEM-KIND READING. ────────────────────
    //
    // A second object is acquired AFTER C1 and never banked. One death now has
    // to reach two OPPOSITE answers about two objects of the same kind, held by
    // the same body, in the same frame — and the only thing that separates them
    // is which side of the checkpoint each acquisition fell on.
    //
    // ⛔ no `KeyItem => survives` rule can produce this: both are ordinary
    // authored ground items, and a kind rule has exactly one answer for them.
    //
    // ⚠ the reward has to be put down first, because a body has one hand. That
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
    // ⭐⭐ **AND THE REWARD IS BACK IN THE HAND, not merely still one object.**
    // Putting it down happened AFTER C1 and was never banked, so it is undone by
    // exactly the same rule that takes the temporary object back — the reset
    // restores the state at C1, and at C1 this was in a hand.
    //
    // ⛔ counting occurrences alone would pass with the reward lying on the
    // floor, which is a different world from C1 and the wrong one.
    assert_still_held(
        &mut sim,
        &reward,
        "putting the reward down happened after C1 and was never banked, so the \
         reset owes it back to the hand it was in at C1",
    );
}
