//! Plugins that connect the LDtk runtime spine to the Bevy app.
//!
//! `AmbitionLdtkRegistrationPlugin` registers the entity bundle and markers so
//! bevy_ecs_ldtk spawns Ambition entities. `LdtkRuntimeSpinePlugin` adds the
//! index-rebuild systems. `sync_plugin_spawned_ambition_entities` attaches
//! gameplay semantics and names to new plugin entities. Components are in
//! sibling `components`, rebuild systems in `systems`.

use bevy::prelude::{
    Added, App, Commands, Entity, IntoScheduleConfigs, Name, Plugin, Query, ResMut,
};
use bevy_ecs_ldtk::prelude::{EntityInstance as PluginEntityInstance, LdtkEntityAppExt};

use super::components::{
    AmbitionLdtkEntity, AmbitionLdtkMarkerBundle, LdtkDamageVolume, LdtkOneWayPlatform, LdtkSolid,
};
use super::indices::LdtkRuntimeSpineStats;
use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;

pub struct AmbitionLdtkRegistrationPlugin;

impl Plugin for AmbitionLdtkRegistrationPlugin {
    fn build(&self, app: &mut App) {
        // Derived from the vocabulary, not listed: every identifier the engine can
        // convert gets a `bevy_ecs_ldtk` marker registration, except the pair below.
        let vocabulary = crate::conversion::LdtkVocabulary::engine();
        for identifier in vocabulary.identifiers() {
            if MARKERLESS_IDENTIFIERS.contains(&identifier) {
                continue;
            }
            app.register_ldtk_entity::<AmbitionLdtkMarkerBundle>(identifier);
        }
    }
}

/// Bevy plugin for the LDtk runtime-spine indexes.
///
/// Owns the chain that walks plugin-spawned Ambition entities
/// (`sync_plugin_spawned_ambition_entities`), rebuilds the per-active-area
/// solid / one-way / hazard runtime indexes, and checks parity with the JSON
/// adapter.
///
/// Runs in [`Platformer2dSimulationPhaseMonolith::LdtkRuntimeSpine`]
/// (configured by `actor_monolith/src/schedule/schedule.rs`). Every system in
/// the chain is in `ldtk_world::bevy_runtime`, so this crate owns the schedule
/// registration.
pub struct LdtkRuntimeSpinePlugin;

impl Plugin for LdtkRuntimeSpinePlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        // The spine's own index/stat resources (the owner initializes them). The
        // rebuild chain fills them from any LDtk entities (none in a RON-only demo).
        app.init_resource::<super::indices::LdtkRuntimeSpineStats>();
        app.init_resource::<super::indices::LdtkRuntimeSpineIndex>();
        app.init_resource::<super::indices::LdtkRuntimeSolidIndex>();
        app.init_resource::<super::indices::LdtkRuntimeOneWayIndex>();
        app.init_resource::<super::indices::LdtkRuntimeDamageIndex>();
        app.init_resource::<super::parity::LdtkRuntimeSpineParity>();
        app.add_systems(
            sim,
            (
                sync_plugin_spawned_ambition_entities,
                super::systems::rebuild_ldtk_runtime_spine_index,
                super::systems::rebuild_ldtk_runtime_solid_index,
                super::systems::rebuild_ldtk_runtime_one_way_index,
                super::systems::rebuild_ldtk_runtime_damage_index,
                super::parity::check_ldtk_runtime_spine_parity,
            )
                .chain()
                // Run only when an LDtk world is installed.
                .run_if(super::asset::ldtk_world_installed),
        );
    }
}

pub fn sync_plugin_spawned_ambition_entities(
    mut commands: Commands,
    mut stats: ResMut<LdtkRuntimeSpineStats>,
    query: Query<(Entity, &PluginEntityInstance), Added<PluginEntityInstance>>,
) {
    for (entity, instance) in &query {
        stats.spawned_entities = stats.spawned_entities.saturating_add(1);
        stats.revision = stats.revision.saturating_add(1);
        let ambition_entity = AmbitionLdtkEntity {
            iid: instance.iid.clone(),
            identifier: instance.identifier.clone(),
            px: [instance.px.x, instance.px.y],
            size: [instance.width, instance.height],
            world: instance.world_x.zip(instance.world_y).map(|(x, y)| [x, y]),
        };
        stats.last_entity = format!("{} {}", ambition_entity.identifier, ambition_entity.iid);
        stats.sample_entity = ambition_entity.summary();

        // Attach typed Ambition components for collision-heavy LDtk categories. The
        // generic `AmbitionLdtkEntity` is always added; typed siblings let systems
        // query without matching identifier strings.
        let mut entity_commands = commands.entity(entity);
        entity_commands.insert((
            Name::new(format!(
                "LDtk {} {}",
                ambition_entity.identifier, ambition_entity.iid
            )),
            ambition_entity.clone(),
        ));
        // Plugin-spawned `Solid` entities get `LdtkSolid`, so the
        // `LdtkRuntimeSolidIndex` collision authority finds them without parsing
        // identifiers.
        match ambition_entity.identifier.as_str() {
            "Solid" => {
                entity_commands.insert(LdtkSolid {
                    level_px: ambition_entity.px,
                    size: ambition_entity.size,
                });
            }
            "OneWayPlatform" => {
                entity_commands.insert(LdtkOneWayPlatform {
                    level_px: ambition_entity.px,
                    size: ambition_entity.size,
                });
            }
            "DamageVolume" | "HazardBlock" => {
                entity_commands.insert(LdtkDamageVolume {
                    level_px: ambition_entity.px,
                    size: ambition_entity.size,
                    // `damage` is not yet part of the LDtk schema; default
                    // to the JSON adapter's hazard amount (1).
                    damage: 1,
                });
            }
            _ => {}
        }
    }
}

/// The engine identifiers that are converted but get no `bevy_ecs_ldtk`
/// marker registration.
///
/// A new engine entity is registered by default. To leave one out, add it here
/// with a reason.
///
/// This pair is kept as it was, not endorsed. Registering them changes what
/// the `bevy_ecs_ldtk` path spawns, which is awaiting-maintainer-decision #64.
const MARKERLESS_IDENTIFIERS: &[&str] = &["SurfaceLoop", "SurfaceRamp"];

#[cfg(test)]
mod marker_registration_tests {
    use super::MARKERLESS_IDENTIFIERS;
    use crate::conversion::LdtkVocabulary;

    /// An exclusion must name a real converter. If a converter is renamed or
    /// deleted and still listed here, the loop registers everything while this file
    /// still seems to hold two entities back.
    #[test]
    fn every_markerless_identifier_is_one_the_engine_can_convert() {
        let vocabulary = LdtkVocabulary::engine();
        let known: Vec<&str> = vocabulary.identifiers().collect();
        for identifier in MARKERLESS_IDENTIFIERS {
            assert!(
                known.contains(identifier),
                "MARKERLESS_IDENTIFIERS names `{identifier}`, which the engine \
                 vocabulary does not contain -- the exclusion is stale and is \
                 holding nothing back"
            );
        }
    }

    /// What the plugin registers, read back from `bevy_ecs_ldtk`.
    ///
    /// The other two tests reason about the vocabulary and the exclusion list; they
    /// do not observe any `register_ldtk_entity` call. This test reads
    /// `LdtkEntityMap` from a built `App`, so it fails if the loop stops iterating
    /// the vocabulary, if the exclusion widens, or if the registration call is
    /// removed.
    #[test]
    fn the_plugin_registers_exactly_the_vocabulary_it_derives_from() {
        use bevy::prelude::App;

        let mut app = App::new();
        app.add_plugins(super::AmbitionLdtkRegistrationPlugin);

        let map = app
            .world_mut()
            .get_non_send::<bevy_ecs_ldtk::app::LdtkEntityMap>()
            .expect("the plugin must have registered at least one entity");
        let registered: std::collections::BTreeSet<String> = map
            .keys()
            .filter_map(|(_layer, identifier)| identifier.clone())
            .collect();

        let vocabulary = LdtkVocabulary::engine();
        let expected: std::collections::BTreeSet<String> = vocabulary
            .identifiers()
            .filter(|identifier| !MARKERLESS_IDENTIFIERS.contains(identifier))
            .map(str::to_string)
            .collect();

        assert_eq!(
            registered, expected,
            "the markers `bevy_ecs_ldtk` actually holds are not the engine \
             vocabulary minus the excluded pair"
        );
    }

    /// The registration set is the vocabulary minus the excluded pair.
    ///
    /// This checks the count, not a list of names: a name list would be a second
    /// copy of the vocabulary. A new engine converter raises both sides together.
    ///
    /// It is not redundant with the test above. If an exclusion is replaced with a
    /// duplicate of the other (`["SurfaceLoop", "SurfaceLoop"]`), the test above
    /// passes (both names are real) and this one fails. In that state the
    /// displaced identifier is registered while the list seems to hold two back.
    #[test]
    fn the_registered_set_is_the_vocabulary_minus_the_excluded_pair() {
        let vocabulary = LdtkVocabulary::engine();
        let registered = vocabulary
            .identifiers()
            .filter(|identifier| !MARKERLESS_IDENTIFIERS.contains(identifier))
            .count();
        assert_eq!(
            registered,
            vocabulary.identifiers().count() - MARKERLESS_IDENTIFIERS.len(),
            "an excluded identifier appeared twice in the vocabulary, or an \
             exclusion matched nothing"
        );
    }
}
