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
            // ⛔⛤ READ ONLY TO CLASSIFY THE DECLINE, never to mint from. Together
            // these two ARE `StrikeVictim`'s query — the pair, and nothing else,
            // is what makes a body a candidate victim — so they are how this
            // system tells "a body it cannot name" from "a body whose namelessness
            // is a determinism defect".
            Option<&ambition_combat::components::CenteredAabb>,
            Option<&ambition_combat::components::ActorFaction>,
        ),
        (
            bevy::ecs::query::With<ambition_platformer2d_shared_tangle::body::BodyKinematics>,
            bevy::ecs::query::Without<ambition_platformer2d_shared_tangle::sim_id::SimId>,
        ),
    >,
) {
    use ambition_platformer2d_shared_tangle::sim_id::SimId;
    for (entity, feature_id, primary, aabb, faction) in &unidentified {
        let id = match (feature_id, primary) {
            (Some(id), _) => SimId::placement(&id.0),
            // ⚠ **THE NET, NOT THE ROAD, SINCE 2026-09-17.** A player body built
            // through `PlayerIdentityBundle` mints `SimId::player_slot(slot)` at
            // construction, because this system runs at the head of the sim and a
            // body spawned after it carried no canonical identity for the rest of
            // its first frame — measured on the shipped route, where perception
            // reached the freshly spawned player and could not name it. What is
            // left for this arm is a body that BECOMES primary later, and it
            // answers `slot:0` for it, which is right only while one body at a
            // time is primary.
            (None, Some(_)) => SimId::player_slot(0),
            // Not identifiable from an authored fact. Its spawn site must mint it.
            //
            // ⛔⛔ AND A SILENT `continue` MADE THAT DEFERRAL THE REAL RULE. For
            // an ornament it is the right answer; for a body the STRIKE RESOLVER
            // will consider it is a determinism defect that shows up as a desync
            // and never as a message. `StrikeVictim::sim_id` is `Option` because
            // *"a body without one still gets hit, it just cannot win the tie"* —
            // and `victim_identity_key`'s own doc calls the case below what it is:
            // *"missing required target identity is a construction failure rather
            // than a sort fallback, so the answer lives where bodies are BUILT."*
            // Two such bodies at one position compare EQUAL on the final
            // tie-break, leaving Bevy query order to decide who is struck, and a
            // resimulation does not reproduce query order.
            //
            // ⚠ `debug_assert` RATHER THAN A PANIC, deliberately: a fail-closed
            // check here would pause a working game over a body that is very
            // likely harmless, and the population this fires on is exactly the
            // population a test can construct. The `error!` carries it in release.
            (None, None) => {
                if aabb.is_some() && faction.is_some() {
                    bevy::log::error!(
                        "{entity:?} is a candidate strike victim with no `SimId`,                          no `FeatureId` and no `PrimaryPlayer`, so nothing can                          name it: two of these at one position tie on every                          geometric key and the resolver's final tie-break compares                          them EQUAL. Its spawn site must mint one."
                    );
                    debug_assert!(
                        false,
                        "{entity:?}: a damageable body reached the sim with no identity and nothing to derive one from"
                    );
                }
                continue;
            }
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

/// Bodies the identity sweeper's `(None, None)` arm SKIPPED, and the population
/// it judged.
///
/// ⛔⛔ **A SKIPPED BODY WAS UNNAMEABLE, AND THAT IS THE WHOLE DEFECT.**
/// [`ensure_sim_id`] mints from two authored facts and `continue`s on everything
/// else, because the invariant is *"its spawn site must mint it"*. That
/// invariant lives in one comment and is enforced by nothing: a body with
/// `BodyKinematics`, no `SimId`, no `FeatureId` and no `PrimaryPlayer` is passed
/// over in silence, on every tick, forever.
///
/// ⚠ **THIS COUNTS AND NAMES; IT DOES NOT MINT.** Minting here would answer the
/// census and hide the spawn sites the invariant points at — the skip is the
/// finding, not the problem to paper over.
///
/// ⭐ **`observed` IS THE ANTI-VACUITY FLOOR AND IT IS NOT THE SAME NUMBER AS
/// `skipped`.** It counts every judged body, identified or not. A composition
/// that never installs the observer, or a run in which no body is ever built,
/// reports `skipped: 0` and reads exactly like a healthy one — so `observed` is
/// what tells a clean result from an absent measurement. ⇒ Read it first.
#[derive(bevy::prelude::Resource, Debug, Default, Clone, PartialEq, Eq)]
pub struct UnmintedBodyCensus {
    /// Body-OBSERVATIONS still on the `(None, None)` arm a tick after they
    /// became bodies. ⚠ THE SAME UNIT AS `observed`, NOT A HEADCOUNT: the loop
    /// below re-judges every body on every tick and adds to both fields, so ONE
    /// unnameable body standing for fifty ticks reports ~50. Measured: `209
    /// judged, 49 skipped` for a SINGLE body. Reading it as a population sends
    /// the next reader hunting 49 bodies that never existed.
    pub skipped: u64,
    /// Body-observations actually JUDGED. The denominator; zero means the
    /// observer saw nothing, not that the tree is clean.
    pub observed: u64,
    /// ⛔ WHO, AND BY WHICH ROAD — not just how many. A census that says "one"
    /// and cannot say WHICH sends the reader to re-derive the population by
    /// hand, and the sweeper's own `continue` names nobody at all.
    ///
    /// ⭐ EACH ENTRY CARRIES THE BODY'S [`SpawnOrigin`], which is A2's acceptance:
    /// *"the witness names the construction ROAD and the body rather than
    /// reporting a population count"*. An entity id and a `Name` say which body;
    /// only the origin says which construction road let it through, and that is
    /// the thing a repair edits.
    ///
    /// ⚠ **IT IS A SET, SO ITS LENGTH IS A HEADCOUNT WHERE [`Self::skipped`] IS
    /// AN OBSERVATION COUNT** — that is the whole reason it exists beside the
    /// number. ⛔ AND IT IS CAPPED AT [`SKIPPED_BODY_CAP`]: past the cap the
    /// length is a FLOOR, not a total, and [`Self::capped`] says which it is.
    /// An uncapped set in a 600-frame run is a memory leak in an instrument.
    pub skipped_bodies: std::collections::BTreeSet<String>,
    /// Whether [`Self::skipped_bodies`] hit its cap and stopped recording.
    /// ⚠ Read this before reading that set's length as a total.
    pub capped: bool,
}

/// How many distinct unnameable bodies [`UnmintedBodyCensus`] will name.
///
/// Small on purpose: the set is for a human reading a failure, and a witness
/// that names sixteen roads has already said everything a seventeenth would.
pub const SKIPPED_BODY_CAP: usize = 16;

/// Observe every body the sweeper has already had its turns on.
///
/// ⚠ **THE MOMENT IS THE WHOLE DESIGN, and `body_identity.rs` paid for this
/// lesson already.** `ensure_sim_id` runs at the head of the frame AND at the
/// tail, and a spawn site may mint within the same tick that built the body. A
/// body judged on the tick it appeared is being asked a question the engine has
/// not finished answering, and the primary player is guaranteed to be counted.
/// ⇒ `Added<BodyKinematics>` EXCLUDES this tick's arrivals; the rest are bodies
/// both sweeper passes and every in-tick spawner have already declined to name.
/// That is ordering-independent and needs no edge against `ensure_sim_id`.
pub fn observe_unminted_bodies(
    mut census: bevy::prelude::ResMut<UnmintedBodyCensus>,
    bodies: bevy::ecs::system::Query<
        (
            bevy::ecs::entity::Entity,
            Option<&ambition_platformer2d_shared_tangle::sim_id::SimId>,
            Option<&ambition_combat::components::FeatureId>,
            Option<&ambition_platformer2d_shared_tangle::markers::PrimaryPlayer>,
            Option<&bevy::prelude::Name>,
            // The construction road. A2's acceptance asks the witness to name it,
            // and it is the only field here that says where to go and edit.
            Option<&ambition_platformer2d_shared_tangle::construction::SpawnOrigin>,
        ),
        bevy::ecs::query::With<ambition_platformer2d_shared_tangle::body::BodyKinematics>,
    >,
    born_this_tick: bevy::ecs::system::Query<
        bevy::ecs::entity::Entity,
        bevy::ecs::query::Added<ambition_platformer2d_shared_tangle::body::BodyKinematics>,
    >,
) {
    for (entity, sim_id, feature_id, primary, name, origin) in &bodies {
        if born_this_tick.contains(entity) {
            continue;
        }
        census.observed += 1;
        if sim_id.is_none() && feature_id.is_none() && primary.is_none() {
            census.skipped += 1;
            if census.skipped_bodies.len() < SKIPPED_BODY_CAP {
                census.skipped_bodies.insert(format!(
                    "{entity} name={} origin={}",
                    name.map_or("<none>", |name| name.as_str()),
                    origin.map_or_else(
                        // ⛔ AND "NO ORIGIN" IS ITSELF THE ANSWER SOMETIMES. A
                        // body built by a road that states no `SpawnOrigin` is a
                        // road that recorded nothing about itself, which is a
                        // different repair from a road that named the wrong thing.
                        || "<no SpawnOrigin — the road recorded none>".to_string(),
                        |origin| format!("{origin:?}"),
                    ),
                ));
            } else {
                census.capped = true;
            }
        }
    }
}

#[cfg(test)]
mod unminted_body_census_tests {
    use super::*;
    use ambition_platformer2d_shared_tangle::body::BodyKinematics;
    use ambition_platformer2d_shared_tangle::construction::SpawnOrigin;
    use bevy::prelude::*;

    /// A world that has already had one tick, so `Added<BodyKinematics>` no
    /// longer excludes a body spawned before it.
    fn app_with(bodies: impl FnOnce(&mut World)) -> App {
        let mut app = App::new();
        app.init_resource::<UnmintedBodyCensus>();
        app.add_systems(Update, observe_unminted_bodies);
        bodies(app.world_mut());
        // ⛔ TWO UPDATES, NOT ONE. `Added<BodyKinematics>` deliberately excludes
        // this tick's arrivals, so a single update judges nothing and every arm
        // below would pass over an empty population.
        app.update();
        app.update();
        app
    }

    fn body() -> BodyKinematics {
        BodyKinematics {
            pos: bevy::math::Vec2::ZERO,
            vel: bevy::math::Vec2::ZERO,
            size: bevy::math::Vec2::splat(16.0),
            facing: 1.0,
        }
    }

    /// ⛔⛤ **A2's ACCEPTANCE: THE WITNESS NAMES THE CONSTRUCTION ROAD.**
    ///
    /// An entity id and a `Name` say WHICH BODY. Only [`SpawnOrigin`] says which
    /// road let it through, and the road is the thing a repair edits — the
    /// sweeper's own comment says the fix belongs at the spawn site, and a
    /// witness that cannot name the site sends the reader to find it by hand.
    #[test]
    fn a_skipped_body_is_named_with_the_road_that_built_it() {
        let app = app_with(|world| {
            world.spawn((
                body(),
                Name::new("unnameable thing"),
                SpawnOrigin::ProviderStaged {
                    provider: "test_provider".into(),
                    room: "test_room".into(),
                    instance: "occupant_7".into(),
                },
            ));
        });
        let census = app.world().resource::<UnmintedBodyCensus>();
        assert!(census.observed > 0, "the observer judged nothing");
        assert_eq!(census.skipped_bodies.len(), 1, "one unnameable body");
        let named = census.skipped_bodies.iter().next().unwrap();
        assert!(
            named.contains("unnameable thing"),
            "the witness does not name the body: {named}"
        );
        assert!(
            named.contains("test_provider") && named.contains("occupant_7"),
            "the witness does not name the construction ROAD, which is the thing \
             a repair edits: {named}"
        );
    }

    /// ⚠ **"NO ORIGIN" IS AN ANSWER, NOT A BLANK.** A road that states no
    /// `SpawnOrigin` recorded nothing about itself, and that is a DIFFERENT
    /// repair from a road that recorded the wrong thing. Formatting it as an
    /// empty string would make the two indistinguishable in the failure text.
    #[test]
    fn a_body_whose_road_recorded_nothing_says_so() {
        let app = app_with(|world| {
            world.spawn((body(), Name::new("origin-less")));
        });
        let census = app.world().resource::<UnmintedBodyCensus>();
        let named = census.skipped_bodies.iter().next().expect("one body");
        assert!(
            named.contains("no SpawnOrigin"),
            "an absent road reads as a blank instead of as a finding: {named}"
        );
    }

    /// ⛔ **THE CONTROL, AND IT IS THE LOAD-BEARING ARM.** A body the sweeper CAN
    /// name must not appear. Without this, every assertion above is satisfied by
    /// a census that records everything it sees.
    #[test]
    fn an_identified_body_is_not_named() {
        let app = app_with(|world| {
            world.spawn((
                body(),
                Name::new("named thing"),
                ambition_platformer2d_shared_tangle::sim_id::SimId::placement("known"),
            ));
        });
        let census = app.world().resource::<UnmintedBodyCensus>();
        assert!(
            census.observed > 0,
            "the observer judged nothing, so `skipped_bodies` being empty is a \
             reading about the observer"
        );
        assert!(
            census.skipped_bodies.is_empty(),
            "an identified body was named as unnameable: {:?}",
            census.skipped_bodies
        );
    }

    /// ⚠ **PAST THE CAP THE LENGTH IS A FLOOR, AND IT SAYS SO.** An uncapped set
    /// in a 600-frame run is a memory leak in an instrument; a capped one that
    /// does not admit it is a total that quietly stops counting.
    #[test]
    fn the_set_caps_and_admits_it() {
        let app = app_with(|world| {
            for i in 0..(SKIPPED_BODY_CAP + 5) {
                world.spawn((body(), Name::new(format!("body {i}"))));
            }
        });
        let census = app.world().resource::<UnmintedBodyCensus>();
        assert_eq!(census.skipped_bodies.len(), SKIPPED_BODY_CAP);
        assert!(census.capped, "the set hit its cap and did not say so");
        assert!(
            census.skipped > census.skipped_bodies.len() as u64,
            "the OBSERVATION count must keep counting past the cap — it is the \
             field that is not a floor"
        );
    }
}
