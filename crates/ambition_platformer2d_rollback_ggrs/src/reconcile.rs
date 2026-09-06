//! GGRS post-load repair for authored brain bindings.
//!
//! Raw entity/component restoration is owned by GGRS; this repair therefore
//! belongs beside the backend load schedule rather than in the generic runtime.

/// Reconcile autonomous catalog-backed brains after GGRS restores their bindings.
///
/// Rebuild only when the restored preset differs by authored configuration, not
/// merely by label. Rebuild from restored [`AuthoredBrainContext`], preserving the
/// authored home. Mounted bodies and externally-owned bindings are excluded; their
/// current brain belongs to another authority. A missing catalog is valid for
/// headless fixtures and leaves the world unchanged.
pub fn reconcile_brain_bindings(world: &mut bevy::ecs::world::World) {
    use ambition_characters::actor::character_catalog::{
        AuthoredBrainContext, BrainBinding, BrainBuildContext,
    };
    use ambition_characters::actor::ActorPose;
    use ambition_characters::brain::Brain;

    struct Job {
        entity: bevy::ecs::entity::Entity,
        preset: String,
        ctx: BrainBuildContext,
        live: Brain,
    }

    // 1. Collect each AUTONOMOUS catalog-backed NPC's active preset, authored build
    //    context, and a clone of its live brain (an immutable pass). Mounted /
    //    external actors are filtered out here (see the doc note).
    //    `query` (not `try_query`) so the optional `AuthoredBrainContext` / `Mounted`
    //    component types are initialized even in a world that never spawned one — a
    //    `try_query` returns `None` there and would silently skip reconciliation.
    let jobs: Vec<Job> = {
        let mut q = world.query::<(
            bevy::ecs::entity::Entity,
            &BrainBinding,
            Option<&AuthoredBrainContext>,
            &ActorPose,
            &Brain,
            bevy::ecs::query::Has<ambition_mount::Mounted>,
        )>();
        q.iter(world)
            .filter_map(|(entity, binding, authored, pose, brain, mounted)| {
                if mounted {
                    return None;
                }
                // `None` => External => an authority other than the catalog owns it.
                let preset = binding.active_preset()?;
                let ctx = authored
                    .map(AuthoredBrainContext::build_context)
                    .unwrap_or_else(|| BrainBuildContext::at(pose.origin().x));
                Some(Job {
                    entity,
                    preset: preset.0.clone(),
                    ctx,
                    live: brain.clone(),
                })
            })
            .collect()
    };
    if jobs.is_empty() {
        return;
    }

    // 2. Rebuild only where the live brain's authored configuration differs from
    //    the brain the restored selection resolves to, via the same catalog seam as
    //    spawn.
    let rebuilt: Vec<(bevy::ecs::entity::Entity, Brain)> = {
        let Some(catalog) =
            world.get_resource::<ambition_characters::actor::character_catalog::CharacterCatalog>()
        else {
            return;
        };
        jobs.iter()
            .filter_map(|job| {
                let candidate = catalog.build_brain_from_preset(&job.preset, &job.ctx)?;
                (!job.live.same_authored_configuration(&candidate))
                    .then_some((job.entity, candidate))
            })
            .collect()
    };

    // 3. Write the reconciled brains back.
    for (entity, brain) in rebuilt {
        if let Ok(mut entity_mut) = world.get_entity_mut(entity) {
            entity_mut.insert(brain);
        }
    }
}
