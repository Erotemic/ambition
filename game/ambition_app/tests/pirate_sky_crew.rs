//! The cove's sky lookout (`pirate_sky_lookout`): a crew on flying sharks, and
//! parrots. What a player standing in it should see, measured.
//!
//! Before (MEASURED by `room_census`): three raiders posted high never saw a
//! player on the floor and hung motionless for the whole fight, while Iron Mary
//! was pushed by her own shark into the ceiling — buried 97% of the fight and
//! sounding a landing on every tick.

use crate::common::fixed_60hz_room_sim;
use ambition_platformer2d::characters::actor::{BodyHealth, WornCharacter};
use ambition_platformer2d::characters::control::ActorControl;
use ambition_platformer2d::engine_core as ae;
use ambition_platformer2d::mount::RidingOn;
use ambition_platformer2d::platformer::markers::PrimaryPlayerOnly;
use bevy::prelude::Entity;
use std::collections::BTreeMap;

const ROOM: &str = "pirate_sky_lookout";

#[derive(Default, Debug)]
struct Rider {
    who: String,
    alive_ticks: usize,
    shots: usize,
    buried: usize,
    /// Player y minus rider y, each live tick: how far above the player it rides.
    above_player: Vec<f32>,
}

/// A player standing still on the floor for twenty seconds: every rider of the
/// crew joins the fight at the player's level — not pinned under the ceiling
/// by its own shark (MEASURED with that push restored: every rider ~630 px
/// above the player, against 170-215 px) — and none rides inside solid.
#[test]
fn the_cove_crew_fights_together_and_nobody_rides_inside_the_ceiling() {
    let mut sim = fixed_60hz_room_sim(ROOM);
    let solids: Vec<ae::Aabb> = {
        let world = sim.world_mut();
        let mut rooms = world.query::<&ae::RoomGeometry>();
        let room = rooms.iter(world).next().expect("the lookout has its geometry");
        room.0
            .blocks
            .iter()
            .filter(|block| matches!(block.kind, ae::BlockKind::Solid))
            .map(|block| block.aabb)
            .collect()
    };
    let inside_solid = |pos: ae::Vec2, size: ae::Vec2| {
        let (min, max) = (pos - size * 0.5, pos + size * 0.5);
        solids.iter().any(|b| b.min.x < max.x - 1.0 && b.max.x > min.x + 1.0 && b.min.y < max.y - 1.0 && b.max.y > min.y + 1.0)
    };
    let mut riders: BTreeMap<Entity, Rider> = BTreeMap::new();
    for _ in 0..20 * 60 {
        sim.step(ambition_app::AgentAction::default());
        let world = sim.world_mut();
        let mut players = world.query_filtered::<&ae::BodyKinematics, PrimaryPlayerOnly>();
        let player_y = players.iter(world).next().expect("a player stands in the room").pos.y;
        let mut query = world.query::<(Entity, &WornCharacter, &ae::BodyKinematics, &BodyHealth, &ActorControl, &RidingOn)>();
        for (entity, worn, kin, health, control, _) in query.iter(world) {
            let rider = riders.entry(entity).or_insert_with(|| Rider { who: worn.0.to_string(), ..Default::default() });
            if !health.alive() {
                continue;
            }
            rider.alive_ticks += 1;
            rider.shots += usize::from(control.0.fire.is_some());
            rider.buried += usize::from(inside_solid(kin.pos, kin.size));
            rider.above_player.push(player_y - kin.pos.y);
        }
    }
    let raiders: Vec<&Rider> = riders.values().filter(|r| r.who == "npc_pirate_raider").collect();
    assert_eq!(raiders.len(), 3, "the lookout posts three raiders on sharks: {riders:?}");
    assert!(riders.len() >= 4, "and Iron Mary: {riders:?}");
    for rider in riders.values() {
        assert!(rider.alive_ticks > 0, "{} never lived in the fight: {riders:?}", rider.who);
        // Posted high, a raider cannot SEE a player on the floor; the crew calls
        // the sighting, and every one of them comes and fires.
        assert!(rider.shots > 0, "{} never fired at a player standing in the room: {riders:?}", rider.who);
        let buried = rider.buried as f32 / rider.alive_ticks as f32;
        assert!(buried < 0.10, "{} rode inside solid {:.0}% of the fight: {riders:?}", rider.who, buried * 100.0);
        // A skirmisher orbits at most ~0.65 of its standoff above its target
        // (<= ~270 px for this crew); pinned under the ceiling is ~630.
        let mut above = rider.above_player.clone();
        above.sort_by(f32::total_cmp);
        let median = above[above.len() / 2];
        assert!(median < 400.0, "{} rode a median {median:.0} px above the player: pinned high, not fighting", rider.who);
    }
}

/// Every enemy body in the room: id, entity, alive, riding something.
fn crew(sim: &mut ambition_app::Platformer2dSimHarness) -> BTreeMap<String, (Entity, bool, bool)> {
    use ambition_platformer2d::combat::components::ActorIdentity;
    let world = sim.world_mut();
    let mut q = world.query::<(Entity, &ActorIdentity, &BodyHealth, Option<&RidingOn>)>();
    q.iter(world)
        .filter(|(_, identity, ..)| identity.id.starts_with("EnemySpawn"))
        .map(|(entity, identity, health, riding)| (identity.id.clone(), (entity, health.alive(), riding.is_some())))
        .collect()
}

/// Defeat every body in `ids` through the real damage channel (a hand-zeroed
/// health bar skips the death pass that records a fate).
fn defeat(sim: &mut ambition_app::Platformer2dSimHarness, bodies: &[(Entity, ae::Vec2)]) {
    use ambition_platformer2d::combat::events::{HitEvent, HitMode, HitSource, HitTarget};
    for (entity, pos) in bodies {
        let volume: ae::CombatVolume = ae::Aabb::new(*pos, ae::Vec2::new(48.0, 48.0)).into();
        sim.world_mut().write_message(HitEvent {
            strike_sfx: None,
            volume,
            damage: 9_999,
            source: HitSource::Projectile,
            attacker: None,
            target: HitTarget::Body(*entity),
            mode: HitMode::Knockback,
            knockback: None,
            ignored_targets: Vec::new(),
            attacker_move_instance: None,
        });
    }
}

/// Defeat the whole crew, leave, come back: they are ALL back — the riders on
/// their sharks with the sharks and the parrots.
///
/// Jon: "only the sharks and parrots respawn, the riders don't". It was
/// authored so — the riders `OnRest`, the rest `OnRoomReenter`, placement by
/// placement. The room now authors its enemy ZONE's policy once
/// (`enemy_respawn: OnRoomReenter`); a placement may still state its own.
#[test]
fn defeat_the_crew_leave_and_come_back_and_riders_and_mounts_are_all_back() {
    let mut sim = fixed_60hz_room_sim(ROOM);
    sim.step_n(crate::common::base(), 60);
    let before = crew(&mut sim);
    let riders: Vec<&String> = before.iter().filter(|(_, (_, _, riding))| *riding).map(|(id, _)| id).collect();
    assert_eq!(before.len(), 10, "the premise: the crew is four riders, four sharks and two parrots: {before:?}");
    assert_eq!(riders.len(), 4, "the premise: four of them ride: {before:?}");
    assert!(before.values().all(|(_, alive, _)| *alive), "the premise: all alive on entry");
    {
        let world = sim.world_mut();
        let mut q = world.query::<(&ambition_platformer2d::combat::components::ActorIdentity, &ambition_platformer2d::actor::ActorConfig)>();
        for (identity, config) in q.iter(world).filter(|(identity, _)| identity.id.starts_with("EnemySpawn")) {
            assert!(
                matches!(config.tuning.respawn, ambition_platformer2d::actors::features::RespawnPolicy::OnRoomReenter),
                "{} is built {:?}: the zone's policy did not reach it",
                identity.id,
                config.tuning.respawn
            );
        }
    }

    let bodies: Vec<(Entity, ae::Vec2)> = {
        let world = sim.world_mut();
        let mut q = world.query::<(Entity, &ambition_platformer2d::combat::components::ActorIdentity, &ae::BodyKinematics)>();
        q.iter(world)
            .filter(|(_, identity, _)| identity.id.starts_with("EnemySpawn"))
            .map(|(entity, _, kin)| (entity, kin.pos))
            .collect()
    };
    defeat(&mut sim, &bodies);
    for _ in 0..120 {
        sim.step(crate::common::base());
        if crew(&mut sim).values().all(|(_, alive, _)| !*alive) {
            break;
        }
    }
    assert!(crew(&mut sim).values().all(|(_, alive, _)| !*alive), "the premise: the whole crew is down");

    assert_eq!(crate::common::walk_through_the_door_to(&mut sim, "pirate_cove"), "pirate_cove");
    assert_eq!(crate::common::walk_through_the_door_to(&mut sim, ROOM), ROOM);
    sim.step_n(crate::common::base(), 60);
    let after = crew(&mut sim);
    assert_eq!(
        after.keys().collect::<Vec<_>>(),
        before.keys().collect::<Vec<_>>(),
        "the same crew comes back"
    );
    for (id, (_, alive, riding)) in &after {
        assert!(*alive, "{id} did not come back");
        assert_eq!(*riding, before[id].2, "{id} came back {} its mount", if *riding { "on" } else { "off" });
    }
}
