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
