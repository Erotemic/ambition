//! Who is a foe of a body, asked of a bare World: each test pairs a foe with
//! a body that must not be one.

use super::*;
use ambition_characters::actor::ActorFaction;

fn fighter(world: &mut World, id: &str, x: f32, team: &str) -> Entity {
    world
        .spawn((
            ae::BodyKinematics { pos: ae::Vec2::new(x, 0.0), ..Default::default() },
            ActorFaction::Player,
            crate::targeting::MatchTeam::new(team),
            SimId::placement(id),
        ))
        .id()
}

fn positions(world: &World, scope: Entity) -> Vec<f32> {
    foes_of(world, scope).expect("a body has foes, if none").foes.iter().map(|foe| foe.position[0]).collect()
}

/// An opponent is a foe; a teammate is not, and neither is a neutral
/// bystander, which is damageable and still not a target.
#[test]
fn an_opponent_is_a_foe_and_a_teammate_or_bystander_is_not() {
    let mut world = World::new();
    let me = fighter(&mut world, "me", 0.0, "red");
    fighter(&mut world, "opponent", 10.0, "blue");
    fighter(&mut world, "teammate", 20.0, "red");
    world.spawn((
        ae::BodyKinematics { pos: ae::Vec2::new(30.0, 0.0), ..Default::default() },
        ActorFaction::Npc,
        SimId::placement("bystander"),
    ));
    assert_eq!(positions(&world, me), vec![10.0], "the foes are not the opponent alone");
}

/// A body out of play is not a foe: it cannot be hit.
#[test]
fn a_body_out_of_play_is_not_a_foe() {
    let mut world = World::new();
    let me = fighter(&mut world, "me", 0.0, "red");
    fighter(&mut world, "in_play", 10.0, "blue");
    let gone = fighter(&mut world, "gone", 20.0, "blue");
    world.entity_mut(gone).insert(crate::death_rules::OutOfPlay);
    assert_eq!(positions(&world, me), vec![10.0], "a body out of play was a foe");
}

/// A body of another live room is not a foe, however close: it stands in
/// another world.
#[test]
fn a_body_of_another_live_room_is_not_a_foe() {
    let mut world = World::new();
    let first = LiveRoomInstance::ACTIVATION;
    let me = fighter(&mut world, "me", 0.0, "red");
    let here = fighter(&mut world, "here", 10.0, "blue");
    let elsewhere = fighter(&mut world, "elsewhere", 20.0, "blue");
    world.entity_mut(me).insert(InRoomInstance(first));
    world.entity_mut(here).insert(InRoomInstance(first));
    world.entity_mut(elsewhere).insert(InRoomInstance(first.next()));
    assert_eq!(positions(&world, me), vec![10.0], "a body of another live room was a foe");
}

/// Foes are in the order of their simulation identity, not of their entity,
/// so a module that keeps the first of equals breaks a tie the same way on
/// every peer. Three foes, so the identity order is neither the spawn order
/// nor its reverse (`Entity` order runs either way across Bevy versions).
#[test]
fn foes_are_in_identity_order() {
    let mut world = World::new();
    let me = fighter(&mut world, "me", 0.0, "red");
    fighter(&mut world, "bb", 10.0, "blue");
    fighter(&mut world, "cc", 20.0, "green");
    fighter(&mut world, "aa", 30.0, "gold");
    assert_eq!(positions(&world, me), vec![30.0, 10.0, 20.0], "the foes are not in identity order");
}
