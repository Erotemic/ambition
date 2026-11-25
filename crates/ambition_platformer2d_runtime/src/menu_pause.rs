//! Publish the pause roster from session admission and simulation authority.

use ambition_characters::control::DrivingParticipant;
use ambition_platformer2d_session::participants::SessionParticipants;
use ambition_platformer2d_shared_tangle::lifecycle::{
    MenuPauseRoster, SessionScopedEntity, SimulationIsland, SimulationIslands, live_session_scope,
    session_root_for_scope, simulation_island,
};
use bevy::prelude::*;

pub(crate) fn publish_menu_pause_roster(world: &mut World) {
    let scope = live_session_scope(world);
    let mut roster = MenuPauseRoster {
        scope,
        members: Vec::new(),
    };
    if let Some(scope) = scope {
        let members = session_root_for_scope(world, scope)
            .and_then(|root| world.get::<SessionParticipants>(root))
            .map(|members| members.iter().collect::<Vec<_>>())
            .unwrap_or_default();
        let room_authority = world
            .get_resource::<SimulationIslands>()
            .is_some_and(|policy| (policy.0)(world));
        let bodies = if room_authority {
            let mut query = world.query::<(Entity, &DrivingParticipant, &SessionScopedEntity)>();
            query
                .iter(world)
                .filter(|(_, _, owner)| owner.0 == scope)
                .map(|(entity, driver, _)| (entity, driver.0))
                .collect()
        } else {
            Vec::new()
        };
        for slot in members {
            let island = if !room_authority {
                Some(SimulationIsland::Session(scope))
            } else {
                let mut islands = bodies
                    .iter()
                    .filter(|(_, driver)| *driver == slot)
                    .filter_map(|(entity, _)| simulation_island(world, *entity));
                let first = islands.next();
                // Missing body, transfer, or ambiguous authority: fail closed.
                first.filter(|first| {
                    matches!(first, SimulationIsland::Room(..))
                        && islands.all(|other| other == *first)
                })
            };
            roster.members.push((slot.0, island));
        }
    }
    let mut current = world.resource_mut::<MenuPauseRoster>();
    if *current != roster {
        *current = roster;
    }
}
