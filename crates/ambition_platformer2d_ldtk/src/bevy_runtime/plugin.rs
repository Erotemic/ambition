//! Plugins wiring the LDtk runtime spine into the Bevy app.
//!
//! `AmbitionLdtkRegistrationPlugin` registers the entity bundle/markers so
//! bevy_ecs_ldtk spawns Ambition entities; `LdtkRuntimeSpinePlugin` adds the
//! index-rebuild systems. `sync_plugin_spawned_ambition_entities` attaches
//! gameplay semantics + names to freshly spawned plugin entities. Components
//! live in sibling `components`, rebuild systems in `systems`.

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
        // ⭐ DERIVED, NOT LISTED. Every identifier the engine can CONVERT gets a
        // `bevy_ecs_ldtk` marker registration, minus the pair below. The hand-kept
        // 32-name list this replaced was a second spelling of a vocabulary that
        // already has one owner, and it had already drifted from it.
        let vocabulary = crate::conversion::LdtkVocabulary::engine();
        for identifier in vocabulary.identifiers() {
            if MARKERLESS_IDENTIFIERS.contains(&identifier) {
                continue;
            }
            app.register_ldtk_entity::<AmbitionLdtkMarkerBundle>(identifier);
        }
    }
}

/// Module-local Bevy plugin for the LDtk runtime-spine indexes.
///
/// Owns the chain that walks plugin-spawned Ambition entities
/// (`sync_plugin_spawned_ambition_entities`), rebuilds the per-active-
/// area solid / one-way / hazard runtime indexes, and pins parity with
/// the JSON adapter via the spine parity check.
///
/// Runs in [`Platformer2dSimulationPhaseMonolith::LdtkRuntimeSpine`] (configured by
/// `app/schedule.rs`). Carved out of `app/plugins.rs::register_ldtk_runtime_spine_systems`
/// per OVERNIGHT-TODO #6 — every system in this chain lives under
/// `ldtk_world::bevy_runtime`, so it's the right domain to own the
/// schedule registration.
pub struct LdtkRuntimeSpinePlugin;

impl Plugin for LdtkRuntimeSpinePlugin {
    fn build(&self, app: &mut App) {
        let sim = app.sim_schedule();
        // The spine's own index/stat resources (anti-god rule 5: the owner
        // initializes). Empty defaults; the rebuild chain below fills them
        // from whatever LDtk entities exist (none, in a RON-only demo).
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
                // The index being optional is what finally makes that statable.
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

        // Attach typed Ambition components for promoted collision-heavy LDtk
        // categories. The generic `AmbitionLdtkEntity` always lands; typed
        // sibling components let downstream systems query specifically without
        // identifier-string matching.
        let mut entity_commands = commands.entity(entity);
        entity_commands.insert((
            Name::new(format!(
                "LDtk {} {}",
                ambition_entity.identifier, ambition_entity.iid
            )),
            ambition_entity.clone(),
        ));
        // Plugin-spawned `Solid` LDtk entities get the typed `LdtkSolid`
        // component so the `LdtkRuntimeSolidIndex` collision authority can
        // pick them up without reparsing identifiers.
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

/// The engine identifiers that are CONVERTED but deliberately get no
/// `bevy_ecs_ldtk` marker registration.
///
/// ⛔⛔ THIS USED TO BE THE WHOLE LIST -- 32 names typed out beside a converter
/// table of 34, with no test pinning the two. It had drifted, and the drift is
/// these two entries: they were authorable, convertible, and invisible to the
/// marker path, and nothing said so. Inverting the list is what makes that
/// impossible: a new engine entity is now registered BY DEFAULT, and leaving one
/// out costs a line here with a reason attached.
///
/// ⚠ THE PAIR IS NOT ENDORSED, it is PRESERVED. Registering them would change
/// what the `bevy_ecs_ldtk` path spawns, which is a behaviour decision filed as
/// awaiting-maintainer-decision #64; this rewrite is deliberately
/// behaviour-identical so that decision stays open and separate. MEASURED
/// 2026-09-05: `sandbox.ldtk` authors one `SurfaceLoop` and no world defines or
/// instances a `SurfaceRamp`, so the pair costs nothing while it waits.
const MARKERLESS_IDENTIFIERS: &[&str] = &["SurfaceLoop", "SurfaceRamp"];

#[cfg(test)]
mod marker_registration_tests {
    use super::MARKERLESS_IDENTIFIERS;
    use crate::conversion::LdtkVocabulary;

    /// ⛔ AN EXCLUSION THAT EXCLUDES NOTHING IS A LIE THAT COSTS NOTHING TO TELL.
    /// Rename or delete a converter and `MARKERLESS_IDENTIFIERS` keeps naming it;
    /// the registration loop then quietly registers everything and this file still
    /// reads as though two entities were held back.
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

    /// The registration set is the vocabulary MINUS the excluded pair.
    ///
    /// ⚠ This pins the ARITHMETIC, not a list of names: a name list here would
    /// be the same hand-kept second spelling the derivation removed. Adding an
    /// engine converter should raise both sides together and keep this green.
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
