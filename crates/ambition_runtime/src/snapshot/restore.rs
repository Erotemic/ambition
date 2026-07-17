//! `restore` — reconciliation of a `SimSnapshot` back onto the live world (N3.2),
//! plus `validate_snapshot` (the mutation-free pre-check) and the room-respawn path.
//!
//! Split out of the former `snapshot.rs` for the D-B module-size gate; shares the
//! module core via `use super::*`.
use super::*;

/// Ask the active room to rebuild one authored entity, by the id its `SimId` already is.
///
/// Returns the rebuilt entity, carrying its `SimId` so `restore` can patch the blob over
/// it. `None` when the id names nothing the room authors — a dynamically-spawned entity —
/// or when the world has no room at all (a headless fixture).
///
/// This is decision (3)'s *"room-reset already proves the world can rebuild"*, honoured
/// rather than quoted: the rebuild goes through `respawn_authored_entity`, the same
/// lowering the room ran at load, not through a restore-only code path.
fn respawn_from_the_room(world: &mut World, sim_id: &str) -> Option<Entity> {
    let iid = sim_id.strip_prefix("placement:")?;
    let registry = world
        .get_resource::<ambition_actors::world::placements::PlacementLoweringRegistry>()?
        .clone();
    let character_catalog = world
        .get_resource::<ambition_characters::actor::character_catalog::CharacterCatalog>()?
        .clone();
    let character_roster = world
        .get_resource::<ambition_actors::features::CharacterRoster>()?
        .clone();
    let boss_catalog = world
        .get_resource::<ambition_actors::boss_encounter::BossCatalog>()?
        .clone();
    let room = {
        let rooms = ambition_platformer_primitives::lifecycle::session_world_component::<
            ambition_world::rooms::RoomSet,
        >(world)?;
        rooms.rooms.get(rooms.active)?.clone()
    };
    let session_scope =
        ambition_platformer_primitives::lifecycle::SessionSpawnScope::for_optional_active_session(
            world.get_resource::<ambition_platformer_primitives::lifecycle::ActiveSessionScope>(),
        )?;

    let built = {
        let mut commands = world.commands();
        ambition_actors::features::respawn_authored_entity(
            &mut commands,
            &character_catalog,
            &character_roster,
            &boss_catalog,
            &room,
            &registry,
            session_scope,
            iid,
        )
    };
    if !built {
        return None;
    }
    // The lowering used `Commands`; nothing exists until they run.
    world.flush();

    // Find what it built. An authored entity wears its `FeatureId`, which IS the iid —
    // that identity is exactly why a snapshot can key on it.
    let entity = {
        let mut q = world.try_query::<(Entity, &ambition_combat::components::FeatureId)>()?;
        let mut found = None;
        for (entity, feature) in q.iter(world) {
            if feature.0 == iid {
                found = Some(entity);
                break;
            }
        }
        found?
    };
    world.entity_mut(entity).insert((
        SimId::from_snapshot(sim_id.to_string()),
        SimIdCounter::default(),
    ));
    Some(entity)
}

/// Rebuild a room's content-staged actor batch through the canonical request applier.
///
/// Content stagers may define relationships between members of one batch (the duel
/// fighters' mutual grudges are the current example), so restoring one missing member
/// independently would produce a roster with the right ids but the wrong authored graph.
/// When any snapshot member of a staged batch is absent, retire the whole live batch and
/// replay the pure stager's requests together before snapshot rows are reconciled.
fn rebuild_content_staged_batch(
    world: &mut World,
    requests: &[ambition_actors::features::SpawnActorRequest],
) {
    use ambition_combat::components::FeatureId;

    let staged_ids = requests
        .iter()
        .map(|request| request.id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let existing = match world.try_query::<(Entity, &FeatureId)>() {
        Some(mut query) => query
            .iter(world)
            .filter_map(|(entity, feature)| {
                staged_ids.contains(feature.0.as_str()).then_some(entity)
            })
            .collect::<Vec<_>>(),
        None => Vec::new(),
    };
    for entity in existing {
        world.despawn(entity);
    }

    let mut messages = world
        .get_resource_mut::<bevy::ecs::message::Messages<
            ambition_actors::features::SpawnActorRequest,
        >>()
        .expect("content-staged reconstruction requires SpawnActorRequest messages");
    messages.clear();
    for request in requests {
        messages.write(request.clone());
    }
    drop(messages);

    let _ = bevy::ecs::system::RunSystemOnce::run_system_once(
        &mut *world,
        ambition_actors::features::apply_spawn_actor_requests,
    );
    world.flush();
    let _ = bevy::ecs::system::RunSystemOnce::run_system_once(&mut *world, ensure_sim_id);
    world.flush();
}

/// **Validate a snapshot's shape against the registry, before restore mutates anything**
/// (re-audit finding 2).
///
/// A same-process rollback cannot reach here in error — [`take`] produces a canonical,
/// registry-agreeing snapshot. This phase exists for the N3.3 wire: a deserialized snapshot is
/// bytes that were never `take`-validated, and restore's by-id lookups and `binary_search`es
/// assume a shape (sorted, unique, registry-matching) that corrupt input can violate. Rather
/// than reconcile against a lie, restore refuses with [`RestoreError::MalformedSnapshot`].
///
/// Mutation-free by construction — it reads only the snapshot and the registry — so it runs
/// ahead of the first despawn and leaves a rejected world untouched. It establishes, in order:
/// a canonical (sorted) and unique roster; every snapshot entry names a registered entry of the
/// matching KIND, exactly once; every component row is sorted, unique, and identifies a roster
/// member; and no non-diagnostic registered entry is missing.
fn validate_snapshot(
    snapshot: &SimSnapshot,
    registry: &SnapshotRegistry,
) -> Result<(), RestoreError> {
    let malformed = |reason: String| RestoreError::MalformedSnapshot { reason };

    // Roster: canonical order and uniqueness. Restore builds its existence set and the hash
    // its roster term from the stored order, so an unsorted or duplicated roster must be
    // refused, not silently accepted (`duplicate_ids` is order-robust, so it catches even a
    // non-adjacent collision).
    if snapshot.roster.windows(2).any(|w| w[0] > w[1]) {
        return Err(malformed(
            "roster is not in canonical (sorted) order".into(),
        ));
    }
    let dups = snapshot.duplicate_ids();
    if !dups.is_empty() {
        return Err(malformed(format!(
            "roster carries duplicate identities: {dups:?}"
        )));
    }
    let roster: std::collections::BTreeSet<&str> =
        snapshot.roster.iter().map(String::as_str).collect();

    // **Entry order must match the registry's non-diagnostic order EXACTLY** (third-pass
    // re-audit). `take` emits registry order; `restore` iterates `snapshot.entries` directly,
    // so a permuted deserialized snapshot is operationally significant — a resolved codec
    // inspects OTHER components on the entity, and a reorder could resolve one before a
    // registered dependency is restored, even though the same entries in registry order would
    // apply. Requiring the exact order (which also subsumes unknown / missing / duplicate
    // entries in one comparison) removes the untrusted snapshot's ability to choose it.
    let expected: Vec<&StateEntry> = registry
        .entries
        .iter()
        .filter(|e| !matches!(e.kind, EntryKind::Diagnostic { .. }))
        .collect();
    if snapshot.entries.len() != expected.len() {
        return Err(malformed(format!(
            "snapshot has {} entries; the registry has {} non-diagnostic entries",
            snapshot.entries.len(),
            expected.len()
        )));
    }
    for (i, ((name, blob), entry)) in snapshot.entries.iter().zip(&expected).enumerate() {
        if *name != entry.name {
            return Err(malformed(format!(
                "entry {i} is `{name}`, but the registry expects `{}` there \
                 (snapshot entry order must match registry order)",
                entry.name
            )));
        }
        // Kind agreement + per-component-row canonical form. A `Diagnostic` cannot appear in
        // `expected` (filtered out), so the only kind failure is a component blob under a
        // resource entry or the reverse.
        match (&entry.kind, blob) {
            (EntryKind::Component { .. }, EntryBlob::Component(rows)) => {
                let mut prev: Option<&str> = None;
                for (id, _) in rows {
                    let id = id.as_str();
                    match prev {
                        Some(p) if p > id => {
                            return Err(malformed(format!(
                                "entry `{name}` rows are not sorted by id"
                            )))
                        }
                        Some(p) if p == id => {
                            return Err(malformed(format!(
                                "entry `{name}` has a duplicate row id `{id}`"
                            )))
                        }
                        _ => {}
                    }
                    if !roster.contains(id) {
                        return Err(malformed(format!(
                            "entry `{name}` row id `{id}` is not in the roster"
                        )));
                    }
                    prev = Some(id);
                }
            }
            (
                EntryKind::Resource { .. } | EntryKind::ResourceCursor { .. },
                EntryBlob::Resource(_),
            ) => {}
            (_, _) => {
                return Err(malformed(format!(
                    "entry `{name}` blob kind does not match its registry kind"
                )))
            }
        }
    }
    Ok(())
}

/// **Restore the sim to a snapshot, reconciling by [`SimId`].**
///
/// Three cases, and each falls out of *"the snapshot is the truth"*:
///
/// - **In both** → the entity is *patched*: every registered component is
///   overwritten from its blob, and one the snapshot lacks is **removed** (the
///   entity did not have it then). Nothing else is touched.
/// - **In the snapshot only** → *respawned* from blobs. It comes back carrying only
///   its registered components, because a blob is all that survives of it.
/// - **In the world only** → *despawned*. It was spawned after the rewound tick.
///
/// ## Deviation from the sketch, and why
///
/// netcode.md's decision (3) is *"restore = despawn-registered + respawn from blobs
/// (no in-place patching — simpler, and room-reset already proves the world can
/// rebuild)"*. Despawn-everything **is** simpler, and it was what shipped first. It
/// is also wrong for the case a rollback is made of.
///
/// A sim body carries two kinds of component. **Authored config** — its brain, its
/// moveset, its action set, its faction — is immutable for the body's life and is
/// created by the room spawner from content. **Mutable state** — kinematics, meters,
/// timers, cooldowns — is what the sim advances. Rewinding needs to restore the
/// second and must not disturb the first. Despawn-and-respawn destroys *both*, and
/// then obliges the registry to carry authored config in every blob of every tick of
/// the rollback buffer so that respawn can put it back. That is not simpler; it is a
/// serialization of the entire content pipeline, sixty times a second.
///
/// Patching the survivors is strictly better and no more complex: the despawn and
/// respawn paths still exist, for exactly the entities whose EXISTENCE changed —
/// which is the case decision (3) was really reasoning about, and the one where
/// "room-reset proves the world can rebuild" actually applies. Measured on
/// `gap_run`, this is the difference between a restore that destroys 53 component
/// types and one that destroys none.
///
/// ## What it still cannot do, it reports
///
/// A patched entity keeps its **unregistered mutable** state — a timer nobody
/// registered still reads the tick we rewound from. [`RestoreReport::stale_components`]
/// names those, and [`RestoreReport::unidentified_survivors`] counts the bodies with
/// no identity at all. Both are gated in `ambition_app`'s ledger tests. Reporting the
/// gap is what keeps it from being discovered in a playtest.
pub fn restore(
    world: &mut World,
    snapshot: &SimSnapshot,
    registry: &SnapshotRegistry,
) -> Result<RestoreReport, RestoreError> {
    // **A snapshot never restores across sessions — the FIRST preflight.** The
    // snapshot is bound at capture to its owning `SessionScopeId`; a stale
    // snapshot from a retired session presented to a successor (same provider,
    // same room, different scope) must refuse before any other check even looks
    // at the world. This is what scopes a future rollback ring to one live
    // session by construction rather than by convention.
    let live_session = world
        .get_resource::<ambition_platformer_primitives::lifecycle::ActiveSessionScope>()
        .and_then(|scope| scope.current());
    if snapshot.session != live_session {
        return Err(RestoreError::SessionMismatch {
            snapshot: snapshot.session.map(|s| s.0),
            live: live_session.map(|s| s.0),
        });
    }

    let live_world_identity = SnapshotWorldIdentity::from_world(world);
    if snapshot.world != live_world_identity {
        return Err(RestoreError::WorldMismatch {
            snapshot: snapshot.world.clone(),
            live: live_world_identity,
        });
    }

    // **The active room is restored sim state (netcode.md N3.2b).** A snapshot taken
    // in another room is not refused: the snapshot's room is STAGED — through the same
    // canonical construction a room transition runs (`RoomStaging`: the scoped-entity
    // sweep, active-spec/geometry swap, moving-platform rebuild, and the App-installed
    // placement lowering) — and only then are the registered blobs reconciled, so
    // `respawn_from_the_room` consults the RIGHT `RoomSpec` and the room-scoped entity
    // set is the snapshot's.
    //
    // Transactionality: `RoomStaging::prepare` is mutation-free (it resolves the room
    // and clones every construction service, refusing with the world untouched), and
    // `apply` runs only after every currently available standalone preflight below
    // has passed. Cursor/resolved codecs are still applied after mutation; an
    // internal decode inconsistency there may return from a partially changed world
    // until the later transactional-codec work lands.
    //
    // The two `Option<String>` are compared WHOLE (re-audit finding 5): a snapshot with a
    // room restored into a world with none — or vice versa — is a session-shape mismatch
    // no staging can bridge, and remains the `CrossRoomBoundary` refusal. `None == None`
    // (a headless fixture with no `RoomSet`) needs no staging.
    let active_room = ambition_platformer_primitives::lifecycle::session_world_component::<
        ambition_world::rooms::RoomSet,
    >(world)
    .map(|rs| rs.active_spec().id.clone());
    let staging = match (&snapshot.active_room, &active_room) {
        (snapshot_room, live_room) if snapshot_room == live_room => None,
        (Some(snapshot_room), Some(_)) => {
            match ambition_actors::world::rooms::RoomStaging::prepare(world, snapshot_room) {
                Ok(staging) => Some(staging),
                // The room names nothing the live session can build (a different
                // prepared world), or a construction service is absent: refuse,
                // world untouched.
                Err(err) => {
                    return Err(RestoreError::RoomNotStageable {
                        room: snapshot_room.clone(),
                        reason: err.to_string(),
                    })
                }
            }
        }
        _ => {
            return Err(RestoreError::CrossRoomBoundary {
                snapshot_room: snapshot.active_room.clone(),
                active_room,
            })
        }
    };

    // **Identity invariant (audit H2), enforced BEFORE any lookup map is built.**
    // A `SimId` carried by two live entities, or two snapshot rows, makes every by-id
    // lookup pick one arbitrarily — the exact silent corruption N3.1 depends on not
    // happening. The old code indexed into a map where "later wins" and delegated the
    // bug upstream; it is a bug, so restore refuses it here rather than patch a coin-flip.
    let live_dups = duplicate_live_ids(world);
    assert!(
        live_dups.is_empty(),
        "restore: {} SimId(s) carried by more than one live entity — identity is not \
         unique and no lookup can be trusted. Fix the spawn site (a duplicated \
         `SimId::spawned`/placement id). Collisions (id, count): {live_dups:?}",
        live_dups.len(),
    );
    // **Snapshot well-formedness (re-audit finding 2), mutation-free, before the lookup map
    // is built.** The live-identity invariant above is a PANIC — a running world with a
    // duplicate `SimId` is a spawn-site bug. A malformed SNAPSHOT is different: it is corrupt
    // INPUT (the N3.3 wire), so it is a returned refusal, not a panic. `validate_snapshot`
    // establishes canonical order, registry/kind agreement, unique rows, and roster
    // membership — everything restore's `binary_search`es and by-id lookups below assume —
    // and touches nothing, so a rejected snapshot leaves the world exactly as it was.
    validate_snapshot(snapshot, registry)?;

    // The survivor prediction for the preflights below. Same-room: every live
    // identity survives into reconciliation. Staged: the sweep will despawn every
    // `RoomScopedEntity`, so only the identities OUTSIDE room scope survive it —
    // the player, the session-lifetime encounter authorities, and their kin.
    // Mutation-free: reads only queries.
    let mut survivors: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    if staging.is_some() {
        if let Some(mut q) =
            world.try_query_filtered::<&SimId, bevy::ecs::query::Without<
                ambition_platformer_primitives::lifecycle::RoomScopedEntity,
            >>()
        {
            for id in q.iter(world) {
                survivors.insert(id.as_str().to_string());
            }
        }
    } else if let Some(mut q) = world.try_query::<&SimId>() {
        for id in q.iter(world) {
            survivors.insert(id.as_str().to_string());
        }
    }

    let ids = snapshot.sim_ids();
    let mut report = RestoreReport::default();

    // **Predicted-roster preflight (re-audit finding 5 + GPT-5.6 closeout), BEFORE any
    // mutation.** Every snapshot identity must either survive into reconciliation or be
    // buildable by the world's own construction: the target room's authored lists
    // (what `respawn_from_the_room` reconstructs from) plus its registered content
    // staging — both predictable without mutating anything, because the staging seam's
    // stagers are pure functions of the `RoomSpec`. An id outside the prediction —
    // a `spawned(..)` child with no recipe (a projectile in flight at the snapshot
    // tick, dead now), or a `placement:` id this room cannot produce — refuses here,
    // with the world untouched, rather than bare-spawning a hollow identity for
    // registered rows to patch. Room-less headless fixtures skip this: with no
    // construction authority there is nothing to predict against, and the
    // bare-identity respawn below remains their honest fixture path.
    // Same-room content staging is prepared before mutation too. Its requests are pure
    // functions of the active RoomSpec, just like cross-room `RoomStaging::prepare`.
    // Keeping the owned requests lets reconciliation rebuild a coordinated batch when
    // the rollback window spans one member's death.
    let same_room_content_requests = if staging.is_none() {
        let active_spec = ambition_platformer_primitives::lifecycle::session_world_component::<
            ambition_world::rooms::RoomSet,
        >(world)
        .map(|rooms| rooms.active_spec().clone());
        match active_spec {
            Some(spec) => world
                .get_resource::<ambition_actors::features::RoomContentStagingRegistry>()
                .cloned()
                .unwrap_or_default()
                .try_requests_for(&spec)
                .map_err(|err| RestoreError::RoomNotStageable {
                    room: spec.id.clone(),
                    reason: err.to_string(),
                })?,
            None => Vec::new(),
        }
    } else {
        Vec::new()
    };

    let authored: std::collections::BTreeSet<String> = match &staging {
        Some(staging) => staging.predicted_authored_ids(),
        None => ambition_platformer_primitives::lifecycle::session_world_component::<
            ambition_world::rooms::RoomSet,
        >(world)
        .map(|rooms| {
            let spec = rooms.active_spec();
            spec.placements
                .iter()
                .map(|p| p.id.0.clone())
                .chain(spec.enemy_spawns.iter().map(|e| e.id.clone()))
                .chain(spec.boss_spawns.iter().map(|b| b.id.clone()))
                .chain(
                    same_room_content_requests
                        .iter()
                        .map(|request| request.id.clone()),
                )
                .collect()
        })
        .unwrap_or_default(),
    };
    let snapshot_ids = ids
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    let same_room_batch_needs_rebuild = same_room_content_requests.iter().any(|request| {
        let sim_id = SimId::placement(&request.id);
        snapshot_ids.contains(sim_id.as_str()) && !survivors.contains(sim_id.as_str())
    });
    if same_room_batch_needs_rebuild
        && world
            .get_resource::<bevy::ecs::message::Messages<
                ambition_actors::features::SpawnActorRequest,
            >>()
            .is_none()
    {
        return Err(RestoreError::RoomNotStageable {
            room: active_room.clone().unwrap_or_default(),
            reason: "content-staged reconstruction requires SpawnActorRequest messages"
                .to_string(),
        });
    }

    // The blob-rebuildable dynamic identities: ids carrying a row under a
    // registered DYNAMIC ANCHOR (`declare_dynamic_anchor`). Such a family's
    // whole component set is registered, so a dead member rebuilds from blobs
    // alone, exactly — the projectile family is the first.
    let blob_rebuildable: std::collections::BTreeSet<&str> = snapshot
        .entries
        .iter()
        .filter(|(name, _)| registry.dynamic_anchors.contains(name))
        .flat_map(|(_, blob)| match blob {
            EntryBlob::Component(rows) => rows.iter().map(|(id, _)| id.as_str()).collect(),
            EntryBlob::Resource(_) => Vec::new(),
        })
        .collect();
    for id in &ids {
        if survivors.contains(*id) {
            continue;
        }
        // A `spawned(..)` child (the vocabulary appends `/<seq>`) rebuilds from
        // blobs alone when a dynamic anchor claims it; otherwise it refuses in
        // ANY world — a dynamic entity outside an anchored family needs its
        // spawner's recipe to come back whole, and none is registered.
        if id.contains('/') {
            if blob_rebuildable.contains(*id) {
                continue;
            }
            return Err(RestoreError::UnsupportedReconstruction {
                sim_id: (*id).to_string(),
            });
        }
        // In a room-backed world, everything else must be buildable by the target
        // room's construction (authored lists ∪ registered content staging). A
        // room-less fixture skips this: it has no construction authority to
        // consult, and its bare-identity respawn path below is honest for it.
        if active_room.is_some() {
            let buildable = id
                .strip_prefix("placement:")
                .is_some_and(|iid| authored.contains(iid));
            if !buildable {
                return Err(RestoreError::UnsupportedReconstruction {
                    sim_id: (*id).to_string(),
                });
            }
        }
    }

    // **Codec decode preflight (re-audit finding 5), BEFORE any mutation.** Validate every
    // STANDALONE-decodable blob — plain components and plain resources — so an ordinary
    // codec failure refuses transactionally, with the world untouched, rather than after
    // the despawn/rebuild loop has half-reconciled it. Cursor and resolved codecs decode
    // into a live target and have no standalone probe; their failure is still caught, at
    // apply time (the named residual on `RestoreError::DecodeFailed`).
    for (name, blob) in &snapshot.entries {
        let Some(entry) = registry.entries.iter().find(|e| e.name == *name) else {
            continue;
        };
        match (&entry.kind, blob) {
            (
                EntryKind::Component {
                    probe: Some(probe), ..
                },
                EntryBlob::Component(rows),
            ) => {
                for (row_id, bytes) in rows {
                    if !probe(bytes) {
                        return Err(RestoreError::DecodeFailed {
                            entry: (*name).to_string(),
                            id: Some(row_id.clone()),
                        });
                    }
                }
            }
            (EntryKind::Resource { probe, .. }, EntryBlob::Resource(bytes)) => {
                if !probe(bytes) {
                    return Err(RestoreError::DecodeFailed {
                        entry: (*name).to_string(),
                        id: None,
                    });
                }
            }
            _ => {}
        }
    }

    // **Every mutation-free preflight has passed: stage the snapshot's room** (N3.2b).
    // From here the transaction no longer refuses transactionally — staging is
    // infallible after `prepare`, and the only remaining failure is the named
    // residual on `RestoreError::DecodeFailed` (a project-authored cursor/resolved
    // codec disagreement, which cannot be probed without a live target).
    report.staged_room = staging.map(|staging| {
        let room = staging.room_id().to_string();
        staging.apply(world);
        // The staged lowering spawns bodies that receive identity from
        // `ensure_sim_id` at the head of the next sim tick — but reconciliation
        // needs it NOW, or every staged body reads as absent and is rebuilt as a
        // duplicate. Run the SAME canonical identity pass, synchronously: no
        // restore-only code path (N3.1's rebuild rule).
        let _ = bevy::ecs::system::RunSystemOnce::run_system_once(&mut *world, ensure_sim_id);
        world.flush();
        room
    });

    if same_room_batch_needs_rebuild {
        rebuild_content_staged_batch(world, &same_room_content_requests);
    }

    // **The identity invariant holds for every reconstructed roster too** (GPT-5.6 closeout).
    // The pre-staging check above ran against the OLD room's entities; staging just
    // constructed a new roster (lowering + content staging + synchronous identity),
    // and a construction bug that minted one id twice — a content stager colliding
    // with an authored placement, a double-registered stager — would otherwise let
    // the live-map build below silently pick one of the two. Same panic, same
    // reason: a world with duplicate identity cannot be trusted, only fixed.
    if report.staged_room.is_some() || same_room_batch_needs_rebuild {
        let staged_dups = duplicate_live_ids(world);
        assert!(
            staged_dups.is_empty(),
            "restore: roster reconstruction produced {} duplicate SimId(s) — a construction bug \
             (a content stager colliding with an authored placement?). Collisions \
             (id, count): {staged_dups:?}",
            staged_dups.len(),
        );
    }

    // Now the map is unambiguous — every id appears once, so no insert overwrites —
    // and, for a staged restore, it reflects the STAGED room's entity set: the
    // survivors of the sweep plus everything the canonical lowering just built.
    let mut live: std::collections::BTreeMap<String, Entity> = std::collections::BTreeMap::new();
    if let Some(mut q) = world.try_query::<(Entity, &SimId)>() {
        for (entity, id) in q.iter(world) {
            live.insert(id.as_str().to_string(), entity);
        }
    }

    // Spawned after the snapshot — or staged just now by the room lowering but dead
    // by the snapshot tick: they never happened / had already died.
    for (id, entity) in &live {
        if ids.binary_search(&id.as_str()).is_err() {
            world.despawn(*entity);
            report.despawned += 1;
        }
    }

    for id in &ids {
        let entity = match live.get(*id) {
            Some(entity) => {
                report.patched += 1;
                *entity
            }
            // Gone since the snapshot, and a dynamic anchor claims it: the whole
            // family is registered, so the entity IS its rows — spawn the identity
            // and let the patch loop below put every component back, markers
            // included. (`ProjectileOwner`, the family's one Entity handle, is
            // derived: `heal_projectile_owners` re-resolves it from the id's
            // parent at the next identity pass.)
            None if blob_rebuildable.contains(*id) => {
                report.respawned += 1;
                world.spawn(SimId::from_snapshot((*id).to_string())).id()
            }
            // Gone since the snapshot. Ask the ROOM to build it again before falling
            // back to a bare `SimId` — the blob carries what the entity became, and only
            // the room carries what it was.
            None => match respawn_from_the_room(world, id) {
                Some(entity) => {
                    report.rebuilt += 1;
                    entity
                }
                // Gone, and the room could not build it. In a room-backed world this
                // is a construction disagreement with the predicted-roster preflight
                // (the id looked authored, the lowering declined) — refuse rather
                // than hand back a hollow identity. This one residual is
                // post-mutation: prediction reads the authored lists, construction
                // runs the interpreters, and only construction is authoritative.
                None if active_room.is_some() => {
                    return Err(RestoreError::UnsupportedReconstruction {
                        sim_id: (*id).to_string(),
                    });
                }
                // A `placement:`/`slot:` id in a room-less world is the
                // headless-fixture path: respawn bare, carrying only identity —
                // there is no construction authority to consult and the fixtures'
                // entities never had authored components to lose.
                None => {
                    report.respawned += 1;
                    world.spawn(SimId::from_snapshot((*id).to_string())).id()
                }
            },
        };

        for (name, blob) in &snapshot.entries {
            let EntryBlob::Component(rows) = blob else {
                continue;
            };
            let Some(entry) = registry.entries.iter().find(|e| e.name == *name) else {
                continue;
            };
            let EntryKind::Component { insert, remove, .. } = &entry.kind else {
                continue;
            };
            match rows.binary_search_by(|(k, _)| k.as_str().cmp(id)) {
                Ok(row) => {
                    let bytes = rows[row].1.clone();
                    let outcome = {
                        let mut e = world.entity_mut(entity);
                        insert(&mut e, &bytes)
                    };
                    match outcome {
                        ApplyOutcome::Applied => {}
                        // A registered row that could not be applied (a cursor with no live
                        // target, a resolve whose content is gone) did NOT come back. Count it
                        // so `lossless()` denies the restore rather than pass the old bare
                        // `true` as success (re-audit finding 3).
                        ApplyOutcome::Unapplied => report.unapplied_rows += 1,
                        ApplyOutcome::DecodeFailed => {
                            return Err(RestoreError::DecodeFailed {
                                entry: (*name).to_string(),
                                id: Some((*id).to_string()),
                            })
                        }
                    }
                }
                // The entity did not have this component at the snapshot tick.
                // Restoring exactly means taking it away now.
                Err(_) => {
                    let mut e = world.entity_mut(entity);
                    remove(&mut e);
                }
            }
        }
    }

    for (name, blob) in &snapshot.entries {
        let EntryBlob::Resource(bytes) = blob else {
            continue;
        };
        let Some(entry) = registry.entries.iter().find(|e| e.name == *name) else {
            continue;
        };
        match &entry.kind {
            EntryKind::Resource { load, .. } => {
                if !load(world, bytes) {
                    return Err(RestoreError::DecodeFailed {
                        entry: (*name).to_string(),
                        id: None,
                    });
                }
            }
            // A resource cursor now carries a presence tag and reports its own outcome
            // (re-audit finding 4): a snapshot-present resource restored into a world where it
            // is absent cannot be rebuilt from a cursor — `Unapplied`, counted so `lossless()`
            // denies it, where the old empty-blob heuristic swallowed it as success.
            EntryKind::ResourceCursor { apply, .. } => match apply(world, bytes) {
                ApplyOutcome::Applied => {}
                ApplyOutcome::Unapplied => report.resource_cursors_unresolved += 1,
                ApplyOutcome::DecodeFailed => {
                    return Err(RestoreError::DecodeFailed {
                        entry: (*name).to_string(),
                        id: None,
                    });
                }
            },
            _ => continue,
        }
    }

    // Messages from the future we are abandoning must not be read in the past we are
    // returning to. See `register_message_channel`.
    for channel in &registry.messages {
        (channel.clear)(world);
    }
    report.messages_cleared = registry.messages.len();

    // Reconcile catalog-backed NPC autonomous state to its restored `BrainBinding`
    // source. Two coordinated passes, in this order:
    //   1. CATALOG BRAIN (`reconcile_brain_bindings`): the `Brain` cursor is a
    //      no-op for peaceful NPC brains, so a rewind PAST a runtime brain switch
    //      would leave the live brain kind out of sync with the restored catalog
    //      selection. Rebuilds it where the kind diverged.
    //   2. AUTONOMOUS CONFIG (`reconcile_autonomous_actors`, ambition_actors): the
    //      archetype config (tuning / brain-spec / capabilities / action set) is a
    //      deterministic function of the source, reconstructed rather than
    //      serialized — a provoked source reruns its roster construction; a catalog
    //      source restores the peaceful config. Runs second so it can derive
    //      `config.brain` from the pass-1 live brain. Both leave the registered
    //      disposition / health / gravity blobs untouched.
    super::codecs::reconcile_brain_bindings(world);
    ambition_actors::features::reconcile_autonomous_actors(world);

    // **Stale state is measured AFTER reconciliation (audit H4), over the FINAL
    // restored roster.** Measuring it at the top — before the future-only entities are
    // despawned and the missing ones rebuilt — reported stale components on entities
    // that were about to vanish (false positives) and missed unregistered components on
    // entities just rebuilt (false negatives). The debt a rewind actually leaves behind
    // is the debt on the entities that survive the rewind, so it is counted here.
    report.stale_components = registry.unclaimed_components(world);
    report.unidentified_survivors = match world
        .try_query_filtered::<(), (With<BodyKinematics>, bevy::ecs::query::Without<SimId>)>()
    {
        Some(mut q) => q.iter(world).count(),
        None => 0,
    };
    // **Resource coverage, measured by restore itself (audit H3; re-audit finding 6),** so
    // `lossless()` reads its own field rather than trusting a caller-supplied count. The
    // census needs Bevy's debug resource names; where they are unavailable the count is a
    // meaningless 0, flagged so `lossless()` refuses rather than falsely succeed.
    report.resource_census_reliable = resource_names_available(world);
    report.unregistered_sim_resources = registry.unclaimed_sim_resources(world).len();
    Ok(report)
}

/// Whether Bevy's runtime resource NAMES are available in this build (re-audit finding 6).
///
/// They require `bevy_ecs/debug`; without it every `iter_resources` name is a fixed
/// placeholder (`"<Enable the debug feature to see the name>"`) that contains no `::`, so
/// the `ambition_*` resource census matches nothing and returns a spurious 0. A real Rust
/// type path always contains `::` (its module path), so one such name proves the census is
/// meaningful. Keyed on the SHAPE of a real path, not the exact placeholder string, so it
/// survives a Bevy version bump.
fn resource_names_available(world: &World) -> bool {
    world
        .iter_resources()
        .any(|(info, _)| info.name().to_string().contains("::"))
}
