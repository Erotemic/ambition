//! GGRS post-load repair for authored brain bindings.
//!
//! Raw entity/component restoration is owned by GGRS; this repair therefore
//! belongs beside the backend load schedule rather than in the generic runtime.

/// Post-restore reconcile: rebuild an AUTONOMOUS catalog-backed NPC's live `Brain`
/// from its restored [`BrainBinding`] **only when its authored configuration
/// diverged** — i.e. a rewind crossed a runtime brain switch, so the live brain no
/// longer matches the restored selection.
///
/// The `Brain` cursor is a no-op for the peaceful/patrol NPC brains (their kind was
/// authored-immutable before runtime switching existed), so it cannot restore a
/// switched kind. Left unreconciled, the next re-simulated tick would drive the
/// wrong brain — a desync.
///
/// Correctness details:
/// - **Configuration equality, not the label.** We compare via
///   [`Brain::same_authored_configuration`], not `label()`: two presets in the same
///   family (`wanderer_slow` / `wanderer_fast`) share a label but differ here, so a
///   rewind across such a switch is caught. Same config → leave the live brain
///   untouched, preserving the state the `Brain` cursor already restored (this is
///   also the RESTORE ORDER guarantee: the cursor runs first, and reconcile only
///   overwrites when the preset genuinely differs — in which case the cursor state
///   was for the wrong brain anyway).
/// - **Authored home.** A rebuild uses the actor's restored [`AuthoredBrainContext`]
///   (its spawn anchor + patrol radius), not its current pose, so a restored patrol
///   brain recenters where it was authored.
/// - **A DISPLACED brain is untouchable.** A body under mount control
///   (`Mounted`) is skipped — its live brain is the controller's, not its
///   autonomous selection, and reconciling would clobber it.
///   **possession is no longer on that list.** A possessed body keeps its own
///   policy the whole time (the seat moved, the brain did not), so its live brain
///   IS its autonomous selection and reconciling it is exactly right.
/// - **Externally-owned brains are left to their authority.** A binding whose
///   selection is `External` (provoke/challenge installed a non-catalog hostile
///   brain) has no `active_preset()` — reconcile skips it, so the disposition/provoke
///   authority owns that brain across the rewind, never the catalog default.
///
/// Skips gracefully when the world has no `CharacterCatalog` (headless fixtures).
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
            bevy::ecs::query::Has<ambition_platformer2d_actor_monolith::features::Mounted>,
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
