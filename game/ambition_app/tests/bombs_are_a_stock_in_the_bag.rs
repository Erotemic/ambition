//! Bombs are a stock in the bag.
//!
//! A bomb equipped from the inventory is one of the bag's stock in the hand.
//! Each throw takes one from the bag, and the hand takes the next one while
//! the bag holds more. When the bag has none, the hand is empty and Attack is
//! the body's own attack again. The equip is the menu's: the same verb
//! (`equip_held_spec`) on the primary body.
//!
//! A bomb is heavy and it bounces. A hard throw at a surface sets it off where
//! it strikes; a soft throw lands, bounces and goes off on its fuse.

#![cfg(feature = "rl_sim")]

use crate::common::{base, fixed_60hz_sim};
use ambition_app::{AgentAction, Platformer2dSimHarness};
use ambition_platformer2d::combat::held_items::HeldItem;
use ambition_platformer2d::held_items::{GroundItem, ItemCustody};
use ambition_platformer2d::items::{Item, OwnedItems};
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::{Entity, With};

fn primary(sim: &mut Platformer2dSimHarness) -> Entity {
    let world = sim.world_mut();
    let mut query = world.query_filtered::<Entity, With<ambition_platformer2d::platformer::body::PrimaryBody>>();
    query.single(world).expect("one primary body")
}

fn bag(sim: &Platformer2dSimHarness) -> u32 {
    sim.world().resource::<OwnedItems>().count(Item::Bomb)
}

/// Make the bag hold exactly `n` bombs.
fn stock(sim: &mut Platformer2dSimHarness, n: u32) {
    let catalog = ambition_platformer2d::items::item_catalog(sim.world()).clone();
    let mut owned = sim.world_mut().resource_mut::<OwnedItems>();
    owned.take(Item::Bomb, u32::MAX);
    owned.grant(&catalog, Item::Bomb, n);
}

/// What the menu does to equip a held item from the bag.
fn equip_a_bomb_from_the_bag(sim: &mut Platformer2dSimHarness) {
    sim.world_mut()
        .run_system_once(
            |mut commands: bevy::prelude::Commands,
             items: ambition_platformer2d::items::ItemCatalogRead,
             mut bodies: bevy::prelude::Query<
                (Entity, ambition_platformer2d::combat::hand::RepertoireQuery),
                ambition_platformer2d::platformer::markers::PrimaryPlayerOnly,
            >| {
                let (player, mut repertoire) = bodies.single_mut().expect("one primary body");
                let spec = ambition_platformer2d::held_items::held_spec_for_item(items.get(), Item::Bomb)
                    .expect("the bomb is a held item");
                ambition_platformer2d::held_items::empty_hand(&mut commands, player, &mut repertoire);
                ambition_platformer2d::held_items::equip_held_spec(&mut commands, player, &mut repertoire, spec);
            },
        )
        .expect("the equip verb runs");
    sim.step(base());
}

fn hand(sim: &Platformer2dSimHarness, body: Entity) -> Option<String> {
    sim.world().get::<HeldItem>(body).map(|held| held.id().to_string())
}

/// The bombs loose in the world.
fn loose_bombs(sim: &mut Platformer2dSimHarness) -> Vec<Entity> {
    let world = sim.world_mut();
    let mut query = world.query::<(Entity, &GroundItem, &ItemCustody)>();
    let mut bombs: Vec<Entity> = query
        .iter(world)
        .filter(|(_, ground, custody)| ground.spec.id == "bomb" && custody.in_world())
        .map(|(entity, ..)| entity)
        .collect();
    bombs.sort();
    bombs
}

/// Is the body playing a move of its own (its attack)?
fn attacking(sim: &Platformer2dSimHarness, body: Entity) -> bool {
    sim.world().get::<ambition_platformer2d::combat::moveset::MovePlayback>(body).is_some()
}

/// Press Attack with `action` and return the bomb the press threw, if any,
/// and whether the body started a move of its own in the next ticks.
fn press_attack(sim: &mut Platformer2dSimHarness, body: Entity, action: AgentAction) -> (Option<Entity>, bool) {
    let before = loose_bombs(sim);
    sim.step(AgentAction { attack: true, ..action });
    let thrown = loose_bombs(sim).into_iter().find(|bomb| !before.contains(bomb));
    let mut moved = attacking(sim, body);
    for _ in 0..6 {
        sim.step(base());
        moved |= attacking(sim, body);
    }
    (thrown, moved)
}

/// The ticks until `bomb` goes off (it leaves the world), up to `limit`.
fn ticks_to_blast(sim: &mut Platformer2dSimHarness, bomb: Entity, limit: usize) -> Option<usize> {
    for tick in 0..limit {
        if sim.world().get::<GroundItem>(bomb).is_none() {
            return Some(tick);
        }
        sim.step(base());
    }
    None
}

#[test]
fn each_throw_of_an_equipped_bomb_takes_one_from_the_bag_until_the_hand_is_empty() {
    let mut sim = fixed_60hz_sim();
    sim.step_n(base(), 120);
    let body = primary(&mut sim);
    assert_eq!(
        bag(&sim),
        ambition_platformer2d::items::STARTER_BOMBS,
        "premise: the demo's bag starts with a stock of bombs"
    );
    stock(&mut sim, 2);

    // Control: an empty hand attacks with the body's own move.
    let (thrown, moved) = press_attack(&mut sim, body, base());
    assert_eq!((thrown, moved), (None, true), "control: Attack with an empty hand is the body's own attack");
    sim.step_n(base(), 60);

    equip_a_bomb_from_the_bag(&mut sim);
    assert_eq!(hand(&sim, body).as_deref(), Some("bomb"), "the equip puts a bomb in the hand");
    assert_eq!(bag(&sim), 2, "the bomb in the hand is still the bag's");

    let (first, moved) = press_attack(&mut sim, body, base());
    let first = first.expect("Attack with a bomb in the hand throws it");
    assert!(!moved, "the throw is the press's action: the body does not also attack");
    assert_eq!(bag(&sim), 1, "the throw took one bomb from the bag");
    assert_eq!(hand(&sim, body).as_deref(), Some("bomb"), "the hand took the next bomb from the bag");
    assert!(ticks_to_blast(&mut sim, first, 120).is_some(), "the thrown bomb goes off");

    let (second, _) = press_attack(&mut sim, body, base());
    assert!(second.is_some(), "the next Attack throws the next bomb");
    assert_eq!(bag(&sim), 0, "the second throw took the last bomb");
    assert_eq!(hand(&sim, body), None, "with the bag empty, the hand is empty");
    sim.step_n(base(), 120);

    let (none, moved) = press_attack(&mut sim, body, base());
    assert_eq!(none, None, "an empty bag throws no bomb");
    assert!(moved, "with the bag empty, Attack is the body's own attack again");
    assert_eq!(bag(&sim), 0);
}

/// A smash throw down from the top of a jump strikes the floor hard and goes
/// off where it strikes, long before the fuse. The control is a tilt throw
/// forward from the floor: it lands soft, bounces, and goes off on its fuse.
#[test]
fn a_hard_throw_goes_off_on_contact_and_a_soft_one_on_its_fuse() {
    let throw_and_time = |jump_first: bool, action: AgentAction| -> usize {
        let mut sim = fixed_60hz_sim();
        sim.step_n(base(), 120);
        let body = primary(&mut sim);
        stock(&mut sim, 1);
        equip_a_bomb_from_the_bag(&mut sim);
        if jump_first {
            // Up, and over the floor: a down press there does not drop the
            // body through the one-way floor it stands on.
            sim.step(AgentAction { jump: true, jump_held: true, ..base() });
            sim.step_n(AgentAction { jump_held: true, ..base() }, 12);
        }
        let before = loose_bombs(&mut sim);
        sim.step(AgentAction { attack: true, ..action });
        let bomb = loose_bombs(&mut sim)
            .into_iter()
            .find(|bomb| !before.contains(bomb))
            .expect("the press threw the bomb");
        assert_eq!(hand(&sim, body), None, "the last bomb left the hand");
        ticks_to_blast(&mut sim, bomb, 240).expect("the bomb goes off")
    };
    let fuse_ticks = (ambition_platformer2d::abilities::ranged::bomb::BOMB_FUSE_SECS * 60.0) as usize;
    let hard = throw_and_time(
        true,
        AgentAction {
            move_y: 1.0,
            attack_strength: ambition_platformer2d::sim::AttackStrengthHint::Smash,
            ..base()
        },
    );
    assert!(hard <= 8, "a smash throw down at the floor goes off on contact: {hard} ticks");
    let soft = throw_and_time(false, base());
    assert!(
        soft > 12 && soft <= fuse_ticks + 2,
        "a tilt throw lands, bounces and goes off on its fuse ({fuse_ticks} ticks at most): {soft} ticks"
    );
}

/// The bombs some body carries.
fn carried_bombs(sim: &mut Platformer2dSimHarness) -> usize {
    let world = sim.world_mut();
    let mut query = world.query::<(&GroundItem, &ItemCustody)>();
    query.iter(world).filter(|(ground, custody)| ground.spec.id == "bomb" && !custody.in_world()).count()
}

/// The menu's stow, as the host intent it writes.
fn stow(sim: &mut Platformer2dSimHarness) {
    ambition_platformer2d::platformer::host_intents::write_host_intent(
        sim.world_mut(),
        ambition_platformer2d::items::ItemStowRequested { participant: ambition_platformer2d::characters::control::PlayerSlot(0) },
    );
    sim.step_n(base(), 3);
}

/// The room's authored floor bomb (the sandbox places one), moved to the
/// body's feet and picked up with one Attack press (an empty hand picks up
/// what it overlaps), so the hand carries an object, not a quantity of the
/// bag.
fn carry_the_floor_bomb(sim: &mut Platformer2dSimHarness, body: Entity) -> Entity {
    let bombs = loose_bombs(sim);
    assert_eq!((hand(sim, body), bombs.len()), (None, 1), "premise: one bomb on the floor and the hand empty");
    let at = sim.world().get::<ambition_platformer2d::engine_core::BodyKinematics>(body).expect("the body has kinematics").pos;
    sim.world_mut().get_mut::<ambition_platformer2d::engine_core::BodyKinematics>(bombs[0]).expect("a bomb has kinematics").pos = at;
    sim.step_n(base(), 2);
    sim.step(AgentAction { attack: true, ..base() });
    sim.step_n(base(), 3);
    assert_eq!(carried_bombs(sim), 1, "premise: the press picked the floor bomb up");
    bombs[0]
}

fn row(sim: &Platformer2dSimHarness, id: &ambition_platformer2d::platformer::sim_id::SimId) -> Option<String> {
    sim.world()
        .resource::<ambition_platformer2d::platformer::lifecycle::AuthoredOccurrences>()
        .whereabouts(id)
        .map(|row| format!("{row:?}"))
}

/// ⭐ A7: A BOMB PICKED UP OFF THE FLOOR CAN BE POCKETED. A stow puts the
/// carried object into the bag (+1), the object ends, and its row is
/// `Consumed`, so its room does not build it again. The control: with the
/// bag at its stack there is no room, and the stow puts the bomb down.
#[test]
fn a_carried_floor_bomb_is_pocketed_by_a_stow() {
    let mut sim = fixed_60hz_sim();
    sim.step_n(base(), 120);
    let body = primary(&mut sim);
    let catalog = ambition_platformer2d::items::item_catalog(sim.world()).clone();
    let full = catalog.stack(Item::Bomb).expect("the bomb is a catalog item");

    // Control: no room in the bag.
    stock(&mut sim, full);
    carry_the_floor_bomb(&mut sim, body);
    stow(&mut sim);
    assert_eq!(
        (bag(&sim), carried_bombs(&mut sim), loose_bombs(&mut sim).len()),
        (full, 0, 1),
        "(bag, carried, loose) after a stow with a full bag: the bomb is put down"
    );

    stock(&mut sim, 2);
    let bomb = carry_the_floor_bomb(&mut sim, body);
    let id = sim.world().get::<ambition_platformer2d::platformer::sim_id::SimId>(bomb).expect("the authored bomb has an id").clone();
    assert_eq!(bag(&sim), 2, "premise: a pickup off the floor is not the bag's");
    stow(&mut sim);
    assert_eq!(
        (bag(&sim), carried_bombs(&mut sim), loose_bombs(&mut sim).len(), hand(&sim, body)),
        (3, 0, 0, None),
        "(bag, carried, loose, hand) after a stow of the carried bomb: it is in the bag, and nowhere else"
    );
    assert_eq!(row(&sim, &id).as_deref(), Some("Consumed"), "the pocketed bomb's row ends, so its room does not build it again");
}
