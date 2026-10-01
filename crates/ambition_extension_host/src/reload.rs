//! Replacing a loaded module while the game runs.
//!
//! A developer rebuilds a module file; the composition stages its new code
//! here and publishes it through the engine's mechanical-edit protocol, so
//! the rollback timeline's owner decides when new code may become
//! authoritative (`MechanicalEditSet::{Propose, Admit, Publish}`, `Q120`).
//! This module only knows how to make a candidate and how to switch to it.
//!
//! ⭐ LAST-GOOD BY CONSTRUCTION. Staging re-admits the WHOLE composition with
//! the new modules. A candidate that admission refuses is never staged, so
//! the running code stays the code that runs.
//!
//! ⛔ A STATE SHAPE MAY NOT CHANGE UNDER LIVE RECORDS. The records the bodies
//! carry were written under the old schema. A reload whose schema of the same
//! key has a different shape is refused; reconstructing the room (or a
//! restart) is the road for that.

use std::sync::Arc;

use ambition_extension_sdk::ModuleDescriptor;
use bevy::prelude::*;

use crate::admission::{admit, Admitted, DeclaredModule, ModuleBackend, ModuleCode};
use crate::{AdmittedExtensions, ExtensionComposition, API_VERSION};

/// The candidate waiting for the mechanical-edit admission.
#[derive(Resource, Default)]
pub struct StagedModuleReplacement {
    candidate: Option<(Arc<Admitted>, Vec<DeclaredModule>)>,
    published: u32,
}

impl StagedModuleReplacement {
    pub fn is_staged(&self) -> bool {
        self.candidate.is_some()
    }

    /// How many reloads this App has published. A diagnostic.
    pub fn published(&self) -> u32 {
        self.published
    }
}

/// Stage the modules of a reloaded file as replacements. `Err` explains a
/// refusal; the running code is unchanged either way until
/// [`publish_staged_replacement`].
pub fn stage_loaded_replacement(
    world: &mut World,
    backend: Arc<dyn ModuleBackend>,
    modules: Vec<ModuleDescriptor>,
) -> Result<(), String> {
    let composition = world
        .get_resource::<ExtensionComposition>()
        .ok_or("the extension host has not admitted a composition yet")?;
    let current = world
        .get_resource::<AdmittedExtensions>()
        .ok_or("the extension host has not admitted a composition yet")?
        .0
        .clone();
    let reloaded_keys: Vec<_> = modules.iter().map(|m| m.key.clone()).collect();
    // The previous loaded build of each reloaded module leaves; a linked
    // module of the same key stays, replaced again.
    let mut declared: Vec<DeclaredModule> = composition
        .declared
        .iter()
        .filter(|m| {
            !(m.replaces
                && matches!(m.code, ModuleCode::Loaded { .. })
                && reloaded_keys.contains(&m.descriptor.key))
        })
        .cloned()
        .collect();
    for (index, descriptor) in modules.into_iter().enumerate() {
        declared.push(DeclaredModule {
            descriptor,
            code: ModuleCode::Loaded {
                backend: backend.clone(),
                module: index as u32,
            },
            replaces: true,
        });
    }
    let candidate = admit(API_VERSION, &composition.offers, &declared).map_err(|refusals| {
        let lines: Vec<String> = refusals.iter().map(|r| format!("  - {r}")).collect();
        format!("the reloaded composition is refused:\n{}", lines.join("\n"))
    })?;
    // A changed field list is migrated at publication (`migrate_records`).
    // A record that would change STORE (its attachment) or save road cannot
    // be carried over.
    for (key, schema) in &candidate.schemas {
        if let Some(live) = current.schemas.get(key) {
            if live.schema.attachment != schema.schema.attachment || live.schema.save != schema.schema.save {
                return Err(format!(
                    "schema {key} changed its attachment or its save policy under live records; \
                     reconstruct the room or restart to take it"
                ));
            }
        }
    }
    world.resource_mut::<StagedModuleReplacement>().candidate = Some((Arc::new(candidate), declared));
    Ok(())
}

/// Switch to the staged candidate. Call it only where the mechanical-edit
/// admission said `Publish`. Returns whether there was a candidate.
pub fn publish_staged_replacement(world: &mut World) -> bool {
    let Some((admitted, declared)) = world
        .get_resource_mut::<StagedModuleReplacement>()
        .and_then(|mut staged| {
            let candidate = staged.candidate.take();
            if candidate.is_some() {
                staged.published += 1;
            }
            candidate
        })
    else {
        return false;
    };
    info!(
        "extension modules reloaded: admitted digest {:016x}; replaced {:?}",
        admitted.digest, admitted.replaced
    );
    let before = world.resource::<AdmittedExtensions>().0.clone();
    migrate_records(world, &before, &admitted);
    world.insert_resource(crate::ExtensionGeneration::of(&declared));
    world.resource_mut::<ExtensionComposition>().declared = declared;
    world.insert_resource(AdmittedExtensions(admitted));
    true
}

/// Carry every live record whose schema changed its fields over to the new
/// shape, by field tag (`StateSchema::migrate`). The publication is a
/// mechanical edit the timeline's owner admitted, so no rewind crosses it.
fn migrate_records(world: &mut World, before: &crate::Admitted, after: &crate::Admitted) {
    let changed: Vec<(&ambition_extension_sdk::SchemaKey, &crate::admission::AdmittedSchema, &crate::admission::AdmittedSchema)> = after
        .schemas
        .iter()
        .filter_map(|(key, new)| {
            let old = before.schemas.get(key)?;
            (old.shape != new.shape).then_some((key, old, new))
        })
        .collect();
    if changed.is_empty() {
        return;
    }
    let migrate = |records: &mut crate::RecordSet| {
        for (key, old, new) in &changed {
            if let Some(record) = records.get(key) {
                let migrated = new.schema.migrate(&old.schema, record);
                records.put((*key).clone(), new.shape, migrated);
            }
        }
    };
    let mut bodies = world.query::<&mut crate::BodyRecords>();
    for mut records in bodies.iter_mut(world) {
        migrate(&mut records.0);
    }
    let mut sessions = world.query::<&mut crate::SessionRecords>();
    for mut records in sessions.iter_mut(world) {
        migrate(&mut records.0);
    }
}
