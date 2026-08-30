//! Backend-neutral stable simulation identity maintenance.
//!
//! These systems run in every simulation host.

/// Give every body the sim can identify a [`SimId`], once.
///
/// Two facts exist today, and this system reads exactly those two: an authored
/// placement's `FeatureId` (the LDtk iid a save file already keys on) and the
/// primary player's slot. Dynamically-spawned entities are NOT covered —
/// N3.1's pin says they get `(spawner SimId, per-spawner counter)`, which the
/// spawn sites must mint at spawn (they know their spawner; this system does not).
///
/// ⛔ THE MIGRATION'S NUMBER COMES FROM THE TAKE RECORDER (`moveset_takes`), the
/// first consumer that CANNOT work without identity: it refuses to write a
/// recording containing a body with no `SimId`, and names it, because its
/// ordering and its bundle join are built on one. ⚠ Do not name a counter here
/// that nothing computes — a doc naming one is why nobody goes looking.
///
/// Runs at the head of the sim, before anything reads identity.
pub fn ensure_sim_id(
    mut commands: bevy::ecs::system::Commands,
    unidentified: bevy::ecs::system::Query<
        (
            bevy::ecs::entity::Entity,
            Option<&ambition_combat::components::FeatureId>,
            Option<&ambition_platformer2d_shared_tangle::markers::PrimaryPlayer>,
        ),
        (
            bevy::ecs::query::With<ambition_platformer2d_shared_tangle::body::BodyKinematics>,
            bevy::ecs::query::Without<ambition_platformer2d_shared_tangle::sim_id::SimId>,
        ),
    >,
) {
    use ambition_platformer2d_shared_tangle::sim_id::SimId;
    for (entity, feature_id, primary) in &unidentified {
        let id = match (feature_id, primary) {
            (Some(id), _) => SimId::placement(&id.0),
            (None, Some(_)) => SimId::player_slot(0),
            // Not identifiable from an authored fact. Its spawn site must mint it.
            (None, None) => continue,
        };
        // The `SimIdCounter` rides along: every identified body is a potential spawner (a boss
        // summons, a player fires), so `SimId` REQUIRES it.
        commands.entity(entity).insert(id);
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
            bevy::ecs::query::Without<ambition_platformer2d_shared_tangle::sim_id::SimId>,
        ),
    >,
    mut owners: bevy::ecs::system::Query<(
        &ambition_platformer2d_shared_tangle::sim_id::SimId,
        &mut ambition_platformer2d_shared_tangle::sim_id::SimIdCounter,
    )>,
) {
    use ambition_platformer2d_shared_tangle::sim_id::SimId;

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
            ambition_platformer2d_shared_tangle::sim_id::SimIdCounter::default(),
            ambition_platformer2d_shared_tangle::construction::SpawnOrigin::Dynamic {
                parent: owner_id.clone(),
                sequence,
            },
        ));
    }
}

// ─── The projectile family: the first blob-rebuildable dynamic family ────────
//
// The projectile domain declares its authoritative rollback state through the
// backend-neutral registrar. `ProjectileOwner` — the one `Entity` handle — is
// derived and healed per identity pass from the spawned occurrence's provenance.
// That lets a dead projectile in a snapshot rebuild from authoritative blobs and
// then recover its live owner handle without serializing a Bevy `Entity`.

/// Re-resolve [`ProjectileOwner`](ambition_projectiles::ProjectileOwner) — the
/// projectile family's one `Entity` handle — from the projectile's declared
/// provenance.
///
/// N3.1 decision (2) forbids `Entity` in blobs, so the owner handle is DERIVED
/// state. The durable fact behind it is
/// [`SpawnOrigin::Dynamic`](ambition_platformer2d_shared_tangle::construction::SpawnOrigin)'s
/// `parent`, stamped at minting and carried through snapshots, and this system
/// re-resolves the handle wherever it is missing or stale — a blob-rebuilt
/// projectile after a restore, or a shot whose firer was itself rebuilt.
/// Scheduled with the identity pair (head and tail of the sim tick), so an
/// owner is healed before anything reads it.
///
/// Provenance is data now; the spelling is free to change.
pub fn heal_projectile_owners(
    mut commands: bevy::ecs::system::Commands,
    projectiles: bevy::ecs::system::Query<
        (
            bevy::ecs::entity::Entity,
            &ambition_platformer2d_shared_tangle::construction::SpawnOrigin,
            Option<&ambition_projectiles::ProjectileOwner>,
        ),
        bevy::ecs::query::With<ambition_projectiles::LiveProjectile>,
    >,
    identities: bevy::ecs::system::Query<(
        bevy::ecs::entity::Entity,
        &ambition_platformer2d_shared_tangle::sim_id::SimId,
    )>,
) {
    let mut orphans: Vec<(
        bevy::ecs::entity::Entity,
        &ambition_platformer2d_shared_tangle::sim_id::SimId,
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
        &ambition_platformer2d_shared_tangle::sim_id::SimId,
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
