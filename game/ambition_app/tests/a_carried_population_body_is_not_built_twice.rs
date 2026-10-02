//! A population occurrence carried into a room another player holds is not
//! built a second time by its home room (Q38, OW3).
//!
//! The ruling: a respawning population occurrence stays where it is carried
//! while it lives, and its replacement comes from its authored room. So while
//! the carried body lives in a live room, its home room must not author it
//! again. When that room retires, the body retires with it, and the home room
//! authors the replacement.
//!
//! The fixture: Bob (slot 1) holds the hub. Alice goes to `vertical_shaft`,
//! possesses an authored enemy there, walks it into the hub, releases it, and
//! goes back to `vertical_shaft`, which is built again.

use crate::common::{base, door_to, walk_through_the_door_to};
use crate::two_players_two_live_rooms::alice_leaves_bob_in;
use ambition_app::{AgentAction, Platformer2dSimHarness};
use ambition_platformer2d::characters::control::PlayerSlot;
use ambition_platformer2d::engine_core::BodyKinematics;
use ambition_platformer2d::platformer::sim_id::SimId;
use bevy::prelude::Entity;

const HUB: &str = "central_hub_complex";
const HOME: &str = "vertical_shaft";

/// Every live body that carries `id`.
fn occurrences(sim: &mut Platformer2dSimHarness, id: &SimId) -> Vec<Entity> {
    let world = sim.world_mut();
    let mut query = world.query::<(Entity, &SimId, &BodyKinematics)>();
    let mut found: Vec<_> = query
        .iter(world)
        .filter(|(_, sim_id, _)| *sim_id == id)
        .map(|(entity, ..)| entity)
        .collect();
    found.sort();
    found
}

fn possessed(sim: &mut Platformer2dSimHarness) -> Option<Entity> {
    sim.world_mut()
        .resource::<ambition_platformer2d::actors::control::possession::PossessionState>()
        .possessed
}

fn down_interact(press: bool) -> AgentAction {
    AgentAction {
        move_y: 1.0,
        interact: press,
        interact_held: true,
        ..base()
    }
}

/// Put `body` in the door to `target` and hold interact until Alice's room
/// changes.
fn carry_through_the_door(sim: &mut Platformer2dSimHarness, body: Entity, target: &str) {
    use ambition_platformer2d::engine_core::AabbExt as _;
    let before = sim.observation().active_room.clone();
    let door = door_to(sim, target).aabb.center();
    {
        let mut kin = sim.world_mut().get_mut::<BodyKinematics>(body).expect("the body has kinematics");
        kin.pos = door;
        kin.vel = ambition_platformer2d::engine_core::Vec2::ZERO;
    }
    for _ in 0..120 {
        let room = sim
            .step(AgentAction {
                interact: true,
                interact_held: true,
                ..base()
            })
            .active_room;
        if room != before {
            assert_eq!(room, target, "setup: the door led somewhere else");
            return;
        }
    }
    panic!("setup: held interact in the door of '{before}' to '{target}' and the room never changed");
}

/// Alice carries an authored enemy of `vertical_shaft` into the hub and
/// releases it there, with Bob's slot on Bob (`bob`) or on nobody. She then
/// goes back to `vertical_shaft`. Returns the enemy's identity and the live
/// bodies that carry it when she is back.
///
/// `persistent` makes the enemy `DeadStaysDead` before anything happens, the
/// policy a content pack can author for an enemy (no shipped one does).
fn carry_an_enemy_into_the_hub_and_go_back(
    bob: Option<PlayerSlot>,
    persistent: bool,
) -> (SimId, Vec<Entity>, Vec<String>) {
    let (mut sim, id) = carry_an_enemy_into_the_hub_and_go_back_in(bob, persistent);
    let found = occurrences(&mut sim, &id);
    let rooms = live_room_ids(&mut sim);
    (id, found, rooms)
}

/// The ids of the live rooms, sorted.
fn live_room_ids(sim: &mut Platformer2dSimHarness) -> Vec<String> {
    let world = sim.world_mut();
    let definitions: Vec<_> = world
        .query_filtered::<
            &ambition_platformer2d::world::rooms::LiveRoomDefinition,
            bevy::prelude::With<ambition_platformer2d::platformer::lifecycle::RoomInstanceRoot>,
        >()
        .iter(world)
        .copied()
        .collect();
    let rooms = ambition_platformer2d::platformer::lifecycle::session_world_component::<
        ambition_platformer2d::world::rooms::RoomSet,
    >(world)
    .expect("the session keeps its room set");
    let mut named: Vec<String> = definitions.into_iter().map(|definition| rooms.spec(definition).id.clone()).collect();
    named.sort();
    named
}

/// [`carry_an_enemy_into_the_hub_and_go_back`], returning the running game.
fn carry_an_enemy_into_the_hub_and_go_back_in(bob: Option<PlayerSlot>, persistent: bool) -> (Platformer2dSimHarness, SimId) {
    let (mut sim, _hub) = alice_leaves_bob_in(HUB, HOME, bob, walk_through_the_door_to);
    sim.step_n(base(), 20);
    let (enemy, id, respawn) = {
        let world = sim.world_mut();
        let mut q = world.query::<(Entity, &SimId, &ambition_platformer2d::combat::actor_tuning::ActorConfig)>();
        let mut found: Vec<_> = q
            .iter(world)
            .filter(|(_, id, _)| id.as_str().starts_with("placement:EnemySpawn"))
            .map(|(e, id, config)| (e, id.clone(), config.tuning.respawn))
            .collect();
        found.sort_by(|a, b| a.1.cmp(&b.1));
        found.into_iter().next().expect("'vertical_shaft' authors an enemy with a placement identity")
    };
    assert_ne!(
        respawn,
        ambition_platformer2d::entity_catalog::placements::RespawnPolicy::DeadStaysDead,
        "premise: the enemy is a respawning population occurrence, not a persistent character"
    );
    if persistent {
        sim.world_mut()
            .get_mut::<ambition_platformer2d::combat::actor_tuning::ActorConfig>(enemy)
            .expect("the enemy has a config")
            .tuning
            .respawn = ambition_platformer2d::entity_catalog::placements::RespawnPolicy::DeadStaysDead;
    }
    assert_eq!(occurrences(&mut sim, &id).len(), 1, "setup: one occurrence before anything happens");

    for i in 0..900 {
        // The enemy fights back; keep Alice on it until the hold takes.
        if let Some(at) = sim.world().get::<BodyKinematics>(enemy).map(|k| (k.pos.x, k.pos.y)) {
            sim.teleport_player(at);
        }
        sim.step(down_interact(i == 0));
        if possessed(&mut sim) == Some(enemy) {
            break;
        }
    }
    assert_eq!(possessed(&mut sim), Some(enemy), "setup: Alice never possessed the enemy");

    carry_through_the_door(&mut sim, enemy, HUB);
    sim.step_n(base(), 30);
    sim.step(base());
    sim.step(down_interact(true));
    sim.step_n(base(), 10);
    assert_eq!(possessed(&mut sim), None, "setup: the release did not take");
    assert_eq!(occurrences(&mut sim, &id), vec![enemy], "setup: the released enemy is the one occurrence");

    assert_eq!(walk_through_the_door_to(&mut sim, HOME), HOME, "setup: Alice did not go back");
    sim.step_n(base(), 30);
    (sim, id)
}

/// Bob holds the hub, so the carried enemy lives on there. When Alice goes
/// back, `vertical_shaft` must not author it again: one identity, one body.
/// The control is the same run with Bob's slot on nobody: the hub retires
/// when Alice leaves it, the carried enemy retires with it, and
/// `vertical_shaft` authors the replacement.
#[test]
fn a_population_body_left_in_a_room_another_player_holds_is_not_built_at_home() {
    let (id, control, rooms) = carry_an_enemy_into_the_hub_and_go_back(None, false);
    assert_eq!(rooms, vec![HOME.to_string()], "control: with Bob undriven the hub retires");
    assert_eq!(
        control.len(),
        1,
        "control: the hub retired with the carried enemy, and '{HOME}' authors its replacement ({id})"
    );

    let (id, found, rooms) = carry_an_enemy_into_the_hub_and_go_back(Some(PlayerSlot(1)), false);
    assert_eq!(
        rooms,
        vec![HUB.to_string(), HOME.to_string()],
        "setup: Bob holds the hub, so both rooms are live"
    );
    assert_eq!(
        found.len(),
        1,
        "'{HOME}' authored a second body for {id} while the carried one lives in the hub Bob holds: {found:?}"
    );
}

/// The same for a persistent (`DeadStaysDead`) enemy. A persistent body gets
/// a durable row only when a room can build it again somewhere else (an NPC
/// placement), so an enemy has no row; while it lives in the hub Bob holds,
/// it must be held as carried like a population body, or its home room
/// authors it again. The control is the same run with Bob's slot on nobody:
/// the hub retires with the enemy, and its home authors it.
#[test]
fn a_persistent_enemy_left_in_a_room_another_player_holds_is_not_built_at_home() {
    let (id, control, rooms) = carry_an_enemy_into_the_hub_and_go_back(None, true);
    assert_eq!(rooms, vec![HOME.to_string()], "control: with Bob undriven the hub retires");
    assert_eq!(control.len(), 1, "control: the hub retired with the carried enemy, and '{HOME}' authors {id}");

    let (id, found, rooms) = carry_an_enemy_into_the_hub_and_go_back(Some(PlayerSlot(1)), true);
    assert_eq!(
        rooms,
        vec![HUB.to_string(), HOME.to_string()],
        "setup: Bob holds the hub, so both rooms are live"
    );
    assert_eq!(
        found.len(),
        1,
        "'{HOME}' authored a second body for the persistent {id} while the carried one lives in the hub Bob holds: {found:?}"
    );
}

/// A checkpoint reset in Alice's room does not build a second body of an
/// occurrence that lives in the room Bob holds. The reset rebuilds
/// `vertical_shaft` from the checkpoint's occurrence ledger, which is older
/// than the carry, so the ledger alone says "build it at home". But the hub
/// is not rewound and the carried enemy still lives there. The control is
/// the setup: one body before the reset.
#[test]
fn a_checkpoint_reset_does_not_rebuild_a_body_that_lives_in_another_players_room() {
    let (mut sim, id) = carry_an_enemy_into_the_hub_and_go_back_in(Some(PlayerSlot(1)), false);
    let before = occurrences(&mut sim, &id);
    assert_eq!(before.len(), 1, "control: one body of {id} before the reset: {before:?}");
    sim.world_mut().write_message(ambition_platformer2d::platformer::lifecycle::ResetToCheckpoint);
    sim.step_n(base(), 30);
    let owed = sim
        .world_mut()
        .resource::<ambition_platformer2d::actors::session::checkpoint::OutstandingCheckpointRequest>()
        .0;
    assert_eq!(owed, None, "setup: the session is still owed the checkpoint reset");
    assert_eq!(
        live_room_ids(&mut sim),
        vec![HUB.to_string(), HOME.to_string()],
        "setup: the reset must leave Bob's hub live and rebuild Alice's room"
    );
    let after = occurrences(&mut sim, &id);
    assert_eq!(
        after, before,
        "the checkpoint reset of '{HOME}' built a second body of {id}, which lives in the hub Bob holds"
    );
    // The commit restored the checkpoint's ledger; the custody projection
    // holds the carried body again.
    assert_eq!(
        sim.world_mut()
            .resource::<ambition_platformer2d::platformer::lifecycle::AuthoredOccurrences>()
            .whereabouts(&id)
            .cloned(),
        Some(ambition_platformer2d::platformer::lifecycle::OccurrenceWhereabouts::InCustody),
        "after the reset the ledger does not hold {id} as carried"
    );
}
