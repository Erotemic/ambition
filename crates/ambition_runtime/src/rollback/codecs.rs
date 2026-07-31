//! Engine-owned canonical GGRS strategies/checksum projections plus `SimId`
//! minting and relationship repair.
//!
//! These explicit encoders are domain adapters for `bevy_ggrs`; they do not own
//! frame history, restore dispatch, entity reconciliation, or rollback control.
use super::*;

// ── The engine's codecs ──────────────────────────────────────────────────────
//
// Explicit field order, fixed-width LE, every field present. A codec that skips a
// field the sim reads is a restore that silently rewinds to a different world; the
// round-trip oracle in this module's tests is what catches one.

/// **A unit enum's wire discriminant, written down.**
///
/// The mapping is EXPLICIT and the numbers are load-bearing: reordering a variant in
/// its `enum` must never silently reinterpret a snapshot. Declaration order is a
/// refactor away from being a different order, and `#[derive(Default)]` on a variant
/// makes it look reorderable. Adding a variant means adding a number; changing one
/// means breaking every stored blob, which is what a version tag would be for.
///
/// An unknown discriminant decodes to `None`, not to the default: a blob this build
/// cannot read is a bug to surface, not a state to guess.

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
/// - **Temporary control is untouchable.** A body under player possession
///   (`Brain::Player`) or mount control (`Mounted`) is skipped — its live brain is
///   control, not its autonomous selection; reconciling would clobber it.
/// - **Externally-owned brains are left to their authority.** A binding whose
///   selection is `External` (provoke/challenge installed a non-catalog hostile
///   brain) has no `active_preset()` — reconcile skips it, so the disposition/provoke
///   authority owns that brain across the rewind, never the catalog default.
///
/// Skips gracefully when the world has no `CharacterCatalog` (headless fixtures).
pub fn reconcile_brain_bindings(world: &mut bevy::ecs::world::World) {
    use ambition_characters::actor::ActorPose;
    use ambition_characters::actor::character_catalog::{
        AuthoredBrainContext, BrainBinding, BrainBuildContext,
    };
    use ambition_characters::brain::Brain;

    struct Job {
        entity: bevy::ecs::entity::Entity,
        preset: String,
        ctx: BrainBuildContext,
        live: Brain,
    }

    // 1. Collect each AUTONOMOUS catalog-backed NPC's active preset, authored build
    //    context, and a clone of its live brain (an immutable pass). Player /
    //    mounted / external actors are filtered out here (see the doc note).
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
            bevy::ecs::query::Has<ambition_actors::features::Mounted>,
        )>();
        q.iter(world)
            .filter_map(|(entity, binding, authored, pose, brain, mounted)| {
                if brain.is_player() || mounted {
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

/// Give every body the sim can identify a [`SimId`], once.
///
/// Two facts exist today, and this system reads exactly those two: an authored
/// placement's `FeatureId` (the LDtk iid a save file already keys on) and the
/// primary player's slot. **Dynamically-spawned entities are NOT covered** —
/// N3.1's pin says they get `(spawner SimId, per-spawner counter)`, which the
/// spawn sites must mint at spawn (they know their spawner; this system does not).
/// `unidentified_bodies` counts what is left, so the migration has a number.
///
/// Runs at the head of the sim, before anything reads identity.
pub fn ensure_sim_id(
    mut commands: bevy::ecs::system::Commands,
    unidentified: bevy::ecs::system::Query<
        (
            bevy::ecs::entity::Entity,
            Option<&ambition_combat::components::FeatureId>,
            Option<&ambition_platformer_primitives::markers::PrimaryPlayer>,
        ),
        (
            bevy::ecs::query::With<ambition_platformer_primitives::body::BodyKinematics>,
            bevy::ecs::query::Without<ambition_platformer_primitives::sim_id::SimId>,
        ),
    >,
) {
    use ambition_platformer_primitives::sim_id::{SimId, SimIdCounter};
    for (entity, feature_id, primary) in &unidentified {
        let id = match (feature_id, primary) {
            (Some(id), _) => SimId::placement(&id.0),
            (None, Some(_)) => SimId::player_slot(0),
            // Not identifiable from an authored fact. Its spawn site must mint it.
            (None, None) => continue,
        };
        // Every identified body is a potential spawner (a boss summons, a player
        // fires), and its counter is snapshot state.
        commands
            .entity(entity)
            .insert((id, SimIdCounter::default()));
    }
}

/// Mint `SimId::spawned(spawner, counter.next())` for every in-flight projectile
/// that has none — N3.1's rule for dynamically-spawned sim entities.
///
/// ## Why this is one system rather than an edit at every spawn site
///
/// A projectile already carries the fact this needs: `ProjectileOwner`. Threading
/// a `SimIdCounter` through a dozen fire paths would put the same lookup in a
/// dozen places and leave the thirteenth out.
///
/// ## Why the order is deterministic
///
/// A `Query` walks archetypes, not spawn order, so two sims could mint a pair of
/// same-tick projectiles' ids in opposite order. Sorting by
/// `(owner SimId, ProjectileSeq)` fixes that: `ProjectileSeq` is the existing
/// monotonic spawn-sequence the step system already sorts by to keep iteration
/// deterministic. Its counter is global — which N3.1 forbids for *identity*,
/// because it couples unrelated spawners — but a global counter is a perfectly
/// good *total order*, which is all this uses it for. The identity itself comes
/// from the owner's own `SimIdCounter`, one stream per spawner.
pub fn mint_spawned_sim_ids(
    mut commands: bevy::ecs::system::Commands,
    newborns: bevy::ecs::system::Query<
        (
            bevy::ecs::entity::Entity,
            &ambition_projectiles::ProjectileOwner,
            &ambition_projectiles::ProjectileSeq,
        ),
        (
            bevy::ecs::query::With<ambition_projectiles::LiveProjectile>,
            bevy::ecs::query::Without<ambition_platformer_primitives::sim_id::SimId>,
        ),
    >,
    mut owners: bevy::ecs::system::Query<(
        &ambition_platformer_primitives::sim_id::SimId,
        &mut ambition_platformer_primitives::sim_id::SimIdCounter,
    )>,
) {
    use ambition_platformer_primitives::sim_id::SimId;

    let mut rows: Vec<(
        String,
        u64,
        bevy::ecs::entity::Entity,
        bevy::ecs::entity::Entity,
    )> = Vec::new();
    for (entity, owner, seq) in &newborns {
        // An owner with no identity cannot lend one. Its own migration comes first.
        let Ok((owner_id, _)) = owners.get(owner.0) else {
            continue;
        };
        rows.push((owner_id.as_str().to_string(), seq.0, entity, owner.0));
    }
    rows.sort();

    for (_, _, entity, owner_entity) in rows {
        let Ok((owner_id, mut counter)) = owners.get_mut(owner_entity) else {
            continue;
        };
        let sequence = counter.next();
        let id = SimId::spawned(owner_id, sequence);
        // A projectile can itself spawn (a splitting shot), so it gets a counter.
        //
        // It also gets its PROVENANCE, stated rather than spelled: the owner it
        // descends from is right here, so recording it costs nothing and saves
        // `heal_projectile_owners` from having to recover it by splitting the id
        // string back apart.
        commands.entity(entity).insert((
            id,
            ambition_platformer_primitives::sim_id::SimIdCounter::default(),
            ambition_platformer_primitives::construction::SpawnOrigin::Dynamic {
                parent: owner_id.clone(),
                sequence,
            },
        ));
    }
}

// ─── The projectile family: the first blob-rebuildable dynamic family ────────
//
// Every component an in-flight projectile carries is registered below (the ZST
// markers included), and `ProjectileOwner` — the one `Entity` handle — is
// declared derived and healed per identity pass from the spawned id's parent.
// That is what lets `register_engine_sim_state` declare `projectile_gameplay` a
// DYNAMIC ANCHOR: a dead projectile in a snapshot rebuilds from blobs alone,
// exactly, so a rollback window may span a projectile's whole life.

/// Re-resolve [`ProjectileOwner`](ambition_projectiles::ProjectileOwner) — the
/// projectile family's one `Entity` handle — from the projectile's declared
/// provenance.
///
/// N3.1 decision (2) forbids `Entity` in blobs, so the owner handle is DERIVED
/// state. The durable fact behind it is
/// [`SpawnOrigin::Dynamic`](ambition_platformer_primitives::construction::SpawnOrigin)'s
/// `parent`, stamped at minting and carried through snapshots, and this system
/// re-resolves the handle wherever it is missing or stale — a blob-rebuilt
/// projectile after a restore, or a shot whose firer was itself rebuilt.
/// Scheduled with the identity pair (head and tail of the sim tick), so an
/// owner is healed before anything reads it.
///
/// **This used to read the parent out of the id string** (`rsplit_once('/')` on
/// `placement:duel_pca/0`). That worked only for as long as every dynamic
/// entity's id was spelled by `SimId::spawned` — it silently produced no parent
/// for any dynamic body whose id came from somewhere else, and it welded
/// reconstruction to an id grammar that is supposed to be a human-readable
/// convenience. Provenance is data now; the spelling is free to change.
pub fn heal_projectile_owners(
    mut commands: bevy::ecs::system::Commands,
    projectiles: bevy::ecs::system::Query<
        (
            bevy::ecs::entity::Entity,
            &ambition_platformer_primitives::construction::SpawnOrigin,
            Option<&ambition_projectiles::ProjectileOwner>,
        ),
        bevy::ecs::query::With<ambition_projectiles::LiveProjectile>,
    >,
    identities: bevy::ecs::system::Query<(
        bevy::ecs::entity::Entity,
        &ambition_platformer_primitives::sim_id::SimId,
    )>,
) {
    let mut orphans: Vec<(
        bevy::ecs::entity::Entity,
        &ambition_platformer_primitives::sim_id::SimId,
    )> = Vec::new();
    for (entity, origin, owner) in &projectiles {
        // A live, resolvable handle needs no healing.
        if owner.is_some_and(|owner| identities.get(owner.0).is_ok()) {
            continue;
        }
        if let Some(parent) = origin.parent() {
            orphans.push((entity, parent));
        }
    }
    if orphans.is_empty() {
        return;
    }
    let by_id: std::collections::BTreeMap<
        &ambition_platformer_primitives::sim_id::SimId,
        bevy::ecs::entity::Entity,
    > = identities.iter().map(|(entity, id)| (id, entity)).collect();
    for (entity, parent) in orphans {
        if let Some(owner) = by_id.get(parent) {
            commands
                .entity(entity)
                .insert(ambition_projectiles::ProjectileOwner(*owner));
        }
    }
}

// ─── Encounter authority (E11) ───────────────────────────────────────────────
//
// The generic encounter entity (`Encounter` + `SimId::encounter(id)`) carries
// three snapshot-relevant components. `EncounterLifecycle` and
// `EncounterParticipants` are plain state; `EncounterWaves` is a RESOLVED
// codec — its authored `EncounterSpec` is content the surviving entity still
// carries, so the blob stores only the live run (the choice, not the content).
// Participant `entity` handles are NEVER serialized (N3.1 decision 2): the
// durable identity is the id string, and the adapters re-resolve the live
// entity every tick (wave liveness refresh / boss progress update).

// ── State that the former inventory identified but the old restore engine left
// stale. GGRS now owns the storage; these explicit projections make the mutable
// values participate in sync-test/desync checks as well.

// ── GGRS resource/state additions ───────────────────────────────────────────
