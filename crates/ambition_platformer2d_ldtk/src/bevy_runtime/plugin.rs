//! Plugins that connect the LDtk runtime spine to the Bevy app.
//!
//! `LdtkRuntimePlugin` installs the LDtk runtime of a game painted in LDtk.
//! `AmbitionLdtkRegistrationPlugin` registers the entity bundle and markers so
//! bevy_ecs_ldtk spawns Ambition entities. `LdtkRuntimeSpinePlugin` adds the
//! index-rebuild system. `sync_plugin_spawned_ambition_entities` attaches
//! identity and names to new plugin entities. Components are in sibling
//! `components`, the rebuild system in `systems`.

use bevy::prelude::{
    Added, App, Commands, Entity, IntoScheduleConfigs, Name, Plugin, Query, ResMut, Startup, Update,
};
use bevy_ecs_ldtk::prelude::{EntityInstance as PluginEntityInstance, LdtkEntityAppExt};

use super::components::{AmbitionLdtkEntity, AmbitionLdtkMarkerBundle};
use super::indices::LdtkRuntimeSpineStats;
use ambition_platformer2d_shared_tangle::schedule::SimScheduleExt;

/// The LDtk runtime of a game whose rooms are painted in LDtk: `LdtkPlugin`,
/// the entity registrations, a handle per world file of the manifest, and
/// each live room's painted levels at its own origin.
///
/// Needs a `RenderApp`: `bevy_ecs_tilemap`, inside `LdtkPlugin`, panics
/// without one. A game's provider adds it where the host has one.
///
/// Nothing in the simulation reads the entities it spawns: collision and
/// placements come from the world IR the LDtk backend converts into
/// (ADR 0021), which a host with no render app builds the same way.
pub struct LdtkRuntimePlugin;

impl Plugin for LdtkRuntimePlugin {
    fn build(&self, app: &mut App) {
        use bevy_ecs_ldtk::prelude::{IntGridRendering, LdtkSettings, LevelBackground};
        app.add_plugins(bevy_ecs_ldtk::prelude::LdtkPlugin);
        super::clean::install(app);
        app.insert_resource(LdtkSettings {
            // The engine renders the rooms; bevy_ecs_ldtk owns the level and
            // entity lifecycle without also drawing a background rectangle
            // behind every level.
            level_background: LevelBackground::Nonexistent,
            // bevy_ecs_ldtk's default `IntGridRendering::Colorful` spawns a
            // colored tile sprite per non-zero IntGrid cell when no tileset
            // is configured (1004 sprites for central_hub_main alone). Those
            // tiles render in raw LDtk world-pixel coordinates from
            // `LdtkWorldBundle`'s default transform, while the compose path
            // (`int_grid_value_to_block` → `spawn_block`) renders in the
            // centered Bevy frame via `world_to_bevy`. The two frames disagree
            // by ~half-room-width on x, so the plugin's IntGrid output appeared
            // as a duplicated, horizontally-shifted copy of the room. Collision
            // comes from the world IR, not from these cells.
            int_grid_rendering: IntGridRendering::Invisible,
            ..Default::default()
        });
        app.add_plugins(AmbitionLdtkRegistrationPlugin)
            .add_systems(Startup, super::asset::load_ldtk_asset_handle)
            .add_systems(
                Update,
                super::asset::present_ldtk_levels_per_live_room
                    .run_if(ambition_platformer2d_shared_tangle::lifecycle::session_world_exists),
            );
    }
}

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
/// (`sync_plugin_spawned_ambition_entities`) and rebuilds the per-active-area
/// spine index.
///
/// Runs in [`Platformer2dSimulationPhase::LdtkRuntimeSpine`]
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
        app.add_systems(
            sim,
            (
                sync_plugin_spawned_ambition_entities,
                super::systems::rebuild_ldtk_runtime_spine_index,
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

        commands.entity(entity).insert((
            Name::new(format!(
                "LDtk {} {}",
                ambition_entity.identifier, ambition_entity.iid
            )),
            ambition_entity,
        ));
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
