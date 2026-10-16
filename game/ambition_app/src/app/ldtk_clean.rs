//! `bevy_ecs_ldtk`'s respawn clean, with its query state kept between runs.
//!
//! `bevy_ecs_ldtk::systems::clean_respawn_entities` makes a new `SystemState`
//! (four query states) each time it runs, every frame. A new query state
//! matches itself against every archetype of the world; the hub has about 2000.
//! That was 1.04% of all CPU samples of the phone with Mary-O in the hub
//! (`android-all-maryo-1-1-morning3`), with nothing to clean.
//!
//! [`clean_respawn_entities`] is the same system with the same order and
//! timing. It replaces the upstream one in `ProcessLdtkApi`, after
//! `apply_level_set`: the upstream `ApplyDeferred` before the clean stays, and
//! the ordering edge gives the same sync point, so the level-selection commands
//! are applied before the clean, as before. Only the state is kept: Bevy gives
//! an exclusive system a `&mut SystemState` that lives as long as the system,
//! and it then matches only the archetypes made since its last run.
//!
//! Remove this module when `bevy_ecs_ldtk` keeps the state itself.

use bevy::ecs::schedule::ScheduleCleanupPolicy;
use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use bevy_ecs_ldtk::prelude::{LdtkProjectHandle, LevelEvent, LevelIid, ProcessLdtkApi, Respawn, Worldly};

type CleanParams<'w, 's> = (
    Query<'w, 's, &'static Children, (With<LdtkProjectHandle>, With<Respawn>)>,
    Query<'w, 's, (Entity, &'static LevelIid), With<Respawn>>,
    Query<'w, 's, &'static LevelIid, Without<Respawn>>,
    Query<'w, 's, Entity, With<Worldly>>,
    MessageWriter<'w, LevelEvent>,
);

/// Put [`clean_respawn_entities`] in place of the upstream system. Call it
/// after `LdtkPlugin` is added.
pub fn install(app: &mut App) {
    let removed = app.remove_systems_in_set(
        ProcessLdtkApi,
        bevy_ecs_ldtk::systems::clean_respawn_entities,
        ScheduleCleanupPolicy::RemoveSystemsOnly,
    );
    assert_eq!(
        removed.ok(),
        Some(1),
        "bevy_ecs_ldtk's clean_respawn_entities is not in ProcessLdtkApi; \
         check the bevy_ecs_ldtk version",
    );
    app.add_systems(
        ProcessLdtkApi,
        clean_respawn_entities.after(bevy_ecs_ldtk::systems::apply_level_set),
    );
}

/// `bevy_ecs_ldtk::systems::clean_respawn_entities` (0.15.0), with `state`
/// kept by the system in place of a new one each run.
pub fn clean_respawn_entities(world: &mut World, state: &mut SystemState<CleanParams>) {
    let mut entities_to_despawn_recursively = Vec::new();
    let mut entities_to_despawn_descendants = Vec::new();

    {
        let (ldtk_worlds_to_clean, ldtk_levels_to_clean, other_ldtk_levels, worldly_entities, mut level_events) =
            state.get_mut(world).unwrap();

        for world_children in ldtk_worlds_to_clean.iter() {
            for child in world_children
                .iter()
                .filter(|l| other_ldtk_levels.contains(*l) || worldly_entities.contains(*l))
            {
                entities_to_despawn_recursively.push(child);

                if let Ok(level_iid) = other_ldtk_levels.get(child) {
                    level_events.write(LevelEvent::Despawned(level_iid.clone()));
                }
            }
        }

        for (level_entity, level_iid) in ldtk_levels_to_clean.iter() {
            entities_to_despawn_descendants.push(level_entity);

            level_events.write(LevelEvent::Despawned(level_iid.clone()));
        }
    }

    for entity in entities_to_despawn_recursively {
        world.entity_mut(entity).despawn();
    }

    for entity in entities_to_despawn_descendants {
        world.entity_mut(entity).despawn_related::<Children>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::message::Messages;
    use bevy::ecs::system::RunSystemOnce;

    #[test]
    fn a_level_marked_respawn_loses_its_descendants_and_reports_it() {
        let mut world = World::new();
        world.init_resource::<Messages<LevelEvent>>();
        let level = world.spawn((LevelIid::new("level-a"), Respawn)).id();
        let child = world.spawn(ChildOf(level)).id();

        world.run_system_once(clean_respawn_entities).unwrap();

        assert!(world.get_entity(level).is_ok(), "the level itself stays");
        assert!(world.get_entity(child).is_err(), "its descendants go");
        let events: Vec<_> = world.resource_mut::<Messages<LevelEvent>>().drain().collect();
        assert!(matches!(events.as_slice(), [LevelEvent::Despawned(iid)] if iid == &LevelIid::new("level-a")));
    }

    #[test]
    fn with_nothing_marked_nothing_changes() {
        let mut world = World::new();
        world.init_resource::<Messages<LevelEvent>>();
        let level = world.spawn(LevelIid::new("level-a")).id();
        let child = world.spawn(ChildOf(level)).id();

        world.run_system_once(clean_respawn_entities).unwrap();

        assert!(world.get_entity(child).is_ok());
        assert_eq!(world.resource_mut::<Messages<LevelEvent>>().drain().count(), 0);
    }
}
