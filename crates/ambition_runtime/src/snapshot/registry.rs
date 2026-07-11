//! `SnapshotRegistry` — the opt-in registry of sim state (N3.1 decision 1).
//!
//! Split out of the former `snapshot.rs` for the D-B module-size gate. The module
//! core (`SnapshotState`/`SnapshotCursor` traits, wire primitives, `EntryKind`/
//! `StateEntry`, `UnclaimedComponent`, `encode_one`/`decode_one`) lives in
//! `super` and is shared via the glob import.
use super::*;

/// The opt-in registry of sim state (N3.1 decision 1). Each sim crate's plugin
/// registers what it owns; nothing else is snapshot state, by definition.
///
/// **A `Resource`**, so a downstream crate can register the state it owns without any
/// crate above it knowing the type exists. `ambition_content`'s boss specials do
/// exactly that: they hold sim state, they live above `ambition_runtime`, and
/// [`register_engine_sim_state`] cannot name them. That is the *"each sim crate
/// registers its components' serialization"* shape netcode.md asks for, and it needed
/// a resource rather than a trait relocation.
#[derive(Default, Resource)]
pub struct SnapshotRegistry {
    /// `pub(super)` so `take` (in `mod.rs`) and `restore` (in `restore.rs`) can walk
    /// the registered entries in registration order.
    pub(super) entries: Vec<StateEntry>,
    /// Sim message channels. See [`SnapshotRegistry::register_message_channel`].
    /// `pub(super)` so `restore` can clear them on rewind.
    pub(super) messages: Vec<MessageChannel>,
    /// Component types declared **structurally derived** — rebuilt every tick by
    /// the same system that maintains them, per N3.1's "Excluded, structurally".
    /// Declaring one is a CLAIM, and [`SnapshotRegistry::unclaimed_components`]
    /// is what stops the claim list from being the whole world.
    derived: Vec<(std::any::TypeId, &'static str)>,
}

impl SnapshotRegistry {
    /// Register a component as sim state.
    ///
    /// The rows are keyed by [`SimId`]: an entity without one is not registered
    /// state, cannot be restored, and is counted by `unidentified_bodies`.
    pub fn register_component<C>(&mut self, name: &'static str)
    where
        C: Component + SnapshotState + Sized,
    {
        self.push(
            name,
            EntryKind::Component {
                type_id: std::any::TypeId::of::<C>(),
                rows: Box::new(|world: &World| {
                    let mut rows = Vec::new();
                    let Some(mut q) = world.try_query::<(&SimId, &C)>() else {
                        return rows;
                    };
                    for (id, value) in q.iter(world) {
                        rows.push((id.as_str().to_string(), encode_one(value)));
                    }
                    rows
                }),
                insert: Box::new(|entity, bytes| {
                    // A blob this registry wrote that it cannot read is a codec failure.
                    // Report it so restore fails loudly in EVERY build, rather than the old
                    // `debug_assert!(false)` that dropped the component silently in release
                    // and left stale state reading as restored. A plain component either
                    // decodes-and-applies or fails — it is never `Unapplied`.
                    if let Some(value) = decode_one::<C>(bytes) {
                        entity.insert(value);
                        ApplyOutcome::Applied
                    } else {
                        ApplyOutcome::DecodeFailed
                    }
                }),
                remove: Box::new(|entity| {
                    entity.remove::<C>();
                }),
                // Standalone: the blob decodes to a self-contained `C`, so restore can
                // validate it before mutating the world (finding 5).
                probe: Some(Box::new(|bytes| decode_one::<C>(bytes).is_some())),
            },
        );
    }

    /// Register a component whose snapshot is a **cursor applied in place** — see
    /// [`SnapshotCursor`]. It rewinds a survivor and cannot rebuild a respawn.
    pub fn register_cursor<C>(&mut self, name: &'static str)
    where
        C: Component<Mutability = bevy::ecs::component::Mutable> + SnapshotCursor + Sized,
    {
        self.push(
            name,
            EntryKind::Component {
                type_id: std::any::TypeId::of::<C>(),
                rows: Box::new(|world: &World| {
                    let mut rows = Vec::new();
                    let Some(mut q) = world.try_query::<(&SimId, &C)>() else {
                        return rows;
                    };
                    for (id, value) in q.iter(world) {
                        let mut bytes = Vec::new();
                        value.encode_cursor(&mut bytes);
                        rows.push((id.as_str().to_string(), bytes));
                    }
                    rows
                }),
                insert: Box::new(|entity, bytes| {
                    // No live target: the cursor has no authored half to rewind onto. On a
                    // respawned entity that is expected (`respawned` reports it separately);
                    // on a SURVIVOR that lost the component since the snapshot it is a genuine,
                    // un-rewindable loss. Either way the registered state did NOT come back —
                    // `Unapplied`, which `lossless()` denies. The old bare `true` here was the
                    // exact false-success the re-audit named (finding 3).
                    let Some(mut value) = entity.get_mut::<C>() else {
                        return ApplyOutcome::Unapplied;
                    };
                    let mut r = Reader::new(bytes);
                    if value.apply_cursor(&mut r).is_none() || r.finish().is_none() {
                        return ApplyOutcome::DecodeFailed; // codec disagreement -> restore reports it
                    }
                    ApplyOutcome::Applied
                }),
                remove: Box::new(|entity| {
                    entity.remove::<C>();
                }),
                // A cursor decodes INTO a live target (`apply_cursor(&mut self)`), so it
                // cannot be validated standalone — no probe. Its decode failure is caught at
                // apply time, after mutation has begun (the named residual, finding 5).
                probe: None,
            },
        );
    }

    /// Register a component that references authored content by id — see
    /// [`SnapshotResolve`]. Unlike a cursor, it restores the component's PRESENCE.
    pub fn register_resolved<C>(&mut self, name: &'static str)
    where
        C: Component + SnapshotResolve + Sized,
    {
        self.push(
            name,
            EntryKind::Component {
                type_id: std::any::TypeId::of::<C>(),
                rows: Box::new(|world: &World| {
                    let mut rows = Vec::new();
                    let Some(mut q) = world.try_query::<(&SimId, &C)>() else {
                        return rows;
                    };
                    for (id, value) in q.iter(world) {
                        let mut bytes = Vec::new();
                        value.encode_ref(&mut bytes);
                        rows.push((id.as_str().to_string(), bytes));
                    }
                    rows
                }),
                insert: Box::new(|entity, bytes| {
                    let mut r = Reader::new(bytes);
                    match C::resolve(entity, &mut r) {
                        Ok(Some(value)) => {
                            // `resolve` gets only `&mut Reader`, so it cannot assert it consumed
                            // the whole blob — a valid prefix followed by trailing garbage would
                            // resolve and apply. The insert closure DOES hold the reader after
                            // `resolve` returns, so it checks `finish` here: trailing bytes are a
                            // malformed blob, `DecodeFailed`, not a silent success (third-pass
                            // re-audit). A correct `encode_ref`/`resolve` pair consumes exactly.
                            if r.finish().is_none() {
                                return ApplyOutcome::DecodeFailed;
                            }
                            entity.insert(value);
                            ApplyOutcome::Applied
                        }
                        // The authored half the choice referenced is GONE (a respawn, a content
                        // change). This row did not come back: drop the component (honest for a
                        // save whose content changed) and deny `lossless()` (honest for a
                        // rollback). Deliberately NOT `DecodeFailed` — the bytes were fine, the
                        // world moved on.
                        Ok(None) => {
                            entity.remove::<C>();
                            ApplyOutcome::Unapplied
                        }
                        // The BLOB itself is malformed (truncated / non-canonical tag). Now
                        // distinguished from absence (the resolved-codec residual, closed): a
                        // corrupt wire input is `DecodeFailed`, which aborts the restore, rather
                        // than being silently laundered into a content-change `Unapplied`. The
                        // component is left untouched (as the trailing-bytes path does) — restore
                        // is aborting anyway.
                        Err(ResolveDecodeError) => ApplyOutcome::DecodeFailed,
                    }
                }),
                remove: Box::new(|entity| {
                    entity.remove::<C>();
                }),
                // A resolved codec decodes against the live entity's authored content
                // (`resolve(entity, ..)`) and cannot distinguish a decode failure from an
                // absent authored half anyway (see `insert`), so it has no standalone probe.
                probe: None,
            },
        );
    }

    /// Register a resource as sim state. `WorldTime`, `SimTick`, every seeded RNG.
    pub fn register_resource<R>(&mut self, name: &'static str)
    where
        R: Resource + SnapshotState + Sized,
    {
        self.push(
            name,
            EntryKind::Resource {
                type_id: std::any::TypeId::of::<R>(),
                bytes: Box::new(|world: &World| {
                    world
                        .get_resource::<R>()
                        .map(encode_one)
                        .unwrap_or_default()
                }),
                load: Box::new(|world: &mut World, bytes: &[u8]| {
                    if bytes.is_empty() {
                        world.remove_resource::<R>();
                        true
                    } else if let Some(value) = decode_one::<R>(bytes) {
                        world.insert_resource(value);
                        true
                    } else {
                        false // decode failure -> restore reports it (was: silent)
                    }
                }),
                // Standalone: an empty blob is a removal (always valid); a non-empty one
                // must decode to a self-contained `R`. Validated before mutation (finding 5).
                probe: Box::new(|bytes| bytes.is_empty() || decode_one::<R>(bytes).is_some()),
            },
        );
    }

    /// **Register a sim message channel.**
    ///
    /// A `Messages<M>` buffer is a `Resource`, so `unclaimed_components` never saw one,
    /// and its type name is `bevy_ecs::message::Messages<..>`, so the resource ledger's
    /// first filter missed one too. It is nonetheless sim state: at a tick boundary
    /// `Messages<ActorActionMessage>` is non-empty, and *a message written before a
    /// snapshot and read after a restore is an event that happens twice.*
    ///
    /// `restore` **clears** every registered channel. That is not a shortcut, it is the
    /// state: the messages standing in a buffer at the moment a snapshot is taken have
    /// already been read by every system that runs in that tick, so their CONTENT
    /// cannot affect the future — only the bookkeeping can, and Bevy's message cursors
    /// clamp themselves back to a cleared buffer. What must not survive is a message
    /// from the future we are abandoning.
    ///
    /// It is deliberately NOT hashed. The two sims of N0.4's canary run the same ticks
    /// and hold the same pending messages; a rewound sim holds none, and hashing that
    /// difference would make the exit oracle fail for the one thing it is trying to fix.
    pub fn register_message_channel<M>(&mut self, name: &'static str)
    where
        M: bevy::ecs::message::Message,
    {
        self.messages.push(MessageChannel {
            name,
            // A `Messages<M>` is a `Resource`, so the resource census would count it as
            // unregistered debt — yet `restore` deliberately clears every registered
            // channel, so it is handled, not debt. Record its TypeId so `unclaimed_resources`
            // treats it as CLAIMED (re-audit finding 6).
            type_id: std::any::TypeId::of::<bevy::ecs::message::Messages<M>>(),
            len: |world: &World| {
                world
                    .get_resource::<bevy::ecs::message::Messages<M>>()
                    .map_or(0, |m| m.len())
            },
            clear: |world: &mut World| {
                if let Some(mut m) = world.get_resource_mut::<bevy::ecs::message::Messages<M>>() {
                    m.clear();
                }
            },
        });
    }

    /// Pending messages per registered channel, for a report. Zero everywhere means the
    /// sim drained itself inside the tick, which is the shape a rollback wants.
    pub fn pending_messages(&self, world: &World) -> Vec<(&'static str, usize)> {
        self.messages
            .iter()
            .map(|c| (c.name, (c.len)(world)))
            .filter(|(_, n)| *n > 0)
            .collect()
    }

    /// Register a resource whose snapshot is a **cursor applied in place** — its
    /// authored half stays put. See [`SnapshotCursor`].
    pub fn register_resource_cursor<R>(&mut self, name: &'static str)
    where
        R: Resource + SnapshotCursor + Sized,
    {
        self.push(
            name,
            EntryKind::ResourceCursor {
                type_id: std::any::TypeId::of::<R>(),
                bytes: Box::new(|world: &World| {
                    // Presence tag (re-audit finding 4): a leading `bool` says whether the
                    // resource existed at snapshot time, so absence (`false`) and a
                    // present-but-empty cursor (`true` + no payload) are distinguishable —
                    // where both used to serialize to `[]` and restore treated the pair as a
                    // single no-op. The tag is part of the hashed bytes too, so the canary sees
                    // a resource that comes or goes.
                    let mut out = Vec::new();
                    match world.get_resource::<R>() {
                        Some(v) => {
                            put_bool(&mut out, true);
                            v.encode_cursor(&mut out);
                        }
                        None => put_bool(&mut out, false),
                    }
                    out
                }),
                apply: Box::new(|world: &mut World, bytes: &[u8]| {
                    let mut r = Reader::new(bytes);
                    let Some(present_at_snapshot) = r.bool() else {
                        return ApplyOutcome::DecodeFailed;
                    };
                    if !present_at_snapshot {
                        // The resource did not exist at the snapshot tick. Restoring exactly
                        // means it must not exist now: a resource created after the snapshot is
                        // a future birth, removed to match (where the old empty-blob no-op left
                        // it standing — the absence/empty conflation, re-audit finding 4). The
                        // absence blob is JUST the tag, so trailing bytes are corruption the
                        // remove path must still reject (third-pass re-audit): `finish` first.
                        if r.finish().is_none() {
                            return ApplyOutcome::DecodeFailed;
                        }
                        world.remove_resource::<R>();
                        return ApplyOutcome::Applied;
                    }
                    // Present at the snapshot: its cursor needs a live authored half to apply
                    // onto. Absent now → a cursor cannot rebuild a resource from nothing;
                    // report it incomplete rather than swallow it as success.
                    let Some(mut v) = world.get_resource_mut::<R>() else {
                        return ApplyOutcome::Unapplied;
                    };
                    if v.apply_cursor(&mut r).is_none() || r.finish().is_none() {
                        return ApplyOutcome::DecodeFailed; // decode/shape failure -> restore reports it
                    }
                    ApplyOutcome::Applied
                }),
            },
        );
    }

    /// Register a hash-only measurement. It is hashed and never restored — see
    /// [`EntryKind::Diagnostic`].
    pub fn register_diagnostic(&mut self, name: &'static str, hash: fn(&World, &mut StateHasher)) {
        self.push(name, EntryKind::Diagnostic { hash });
    }

    /// Declare a component **structurally derived**: rebuilt every tick by the
    /// system that maintains it, so `restore` is right to drop it.
    ///
    /// N3.1's rule, quoted: *"if restoring something requires a rebuild pass, the
    /// rebuild must be the SAME system that maintains it per-frame (no
    /// restore-only code paths)."* Declaring a component here asserts that.
    pub fn declare_derived<C: Component>(&mut self, why: &'static str) {
        let _ = why;
        self.derived
            .push((std::any::TypeId::of::<C>(), std::any::type_name::<C>()));
    }

    fn push(&mut self, name: &'static str, kind: EntryKind) {
        // Identity invariant (audit H2/M3), enforced in EVERY build, not just debug:
        // a duplicate registry name makes every by-name lookup in `take`/`restore`
        // (`entries.iter().find(|e| e.name == ..)`) silently pick the FIRST match, so
        // one of the two codecs never runs. Registration happens once at startup, so
        // an unconditional check is free.
        assert!(
            !self.entries.iter().any(|e| e.name == name),
            "sim-state entry `{name}` registered twice — a duplicate registry name \
             makes restore silently use only the first codec (audit H2)"
        );
        self.entries.push(StateEntry { name, kind });
    }

    /// Registration order, for a report.
    pub fn names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.entries.iter().map(|e| e.name)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn hash_entry(&self, entry: &StateEntry, world: &World, h: &mut StateHasher) {
        match &entry.kind {
            EntryKind::Component { rows, .. } => hash_entities_by_key(h, rows(world)),
            EntryKind::Resource { bytes, .. } | EntryKind::ResourceCursor { bytes, .. } => {
                h.write(&bytes(world))
            }
            EntryKind::Diagnostic { hash } => hash(world, h),
        }
    }

    /// The `hash_by_entry` name of the active-room pseudo-entry (re-audit finding 2). The
    /// NUL prefix keeps it from colliding with any registered entry name.
    /// `pub(super)` so the snapshot tests can assert the pseudo-entry ordering.
    pub(super) const ACTIVE_ROOM_ENTRY: &'static str = "\u{0}active_room";

    /// The `hash_by_entry` name of the identity-roster pseudo-entry (re-audit finding 1).
    /// `pub(super)` so the snapshot tests can assert the pseudo-entry ordering.
    pub(super) const ROSTER_ENTRY: &'static str = "\u{0}roster";

    /// **The identity roster, folded into the state hash.** The set of live `SimId`s is sim
    /// state in its own right: [`take`] captures it, [`restore`] reconstructs it exactly, and
    /// a `SimId` entity carrying NO registered component contributes to no component entry —
    /// so without this term the hash is blind to it, and two worlds differing only by such an
    /// entity hash equal. That is the very defect fixed for `active_room`, one level up: the
    /// snapshot carries state (the roster) the hash omitted (re-audit finding 1).
    ///
    /// Count + sorted ids, so it is stable against Bevy's archetype-dependent query order —
    /// the same discipline `hash_entities_by_key` uses. It does not perturb the N0.4 canary:
    /// two sims on one input stream spawn and despawn identically, so their rosters — and this
    /// term — are equal on both sides of every comparison, while a genuine identity desync (an
    /// entity that exists in one sim and not the other) now shows up instead of hiding.
    fn hash_roster(world: &World, h: &mut StateHasher) {
        let mut ids: Vec<String> = Vec::new();
        if let Some(mut q) = world.try_query::<&SimId>() {
            for id in q.iter(world) {
                ids.push(id.as_str().to_string());
            }
        }
        ids.sort();
        h.write_u64(ids.len() as u64);
        for id in &ids {
            h.write_str(id);
        }
    }

    /// **The active-room cursor, folded into the state hash.** The room a sim is in is sim
    /// state: [`take`] captures it and [`restore`] refuses a rollback window that crosses
    /// it. So it must be IN the hash, or two worlds that differ ONLY in which room is
    /// active would hash equal, and "the hash is the serialization" — the take/hash
    /// equivalence the N0.4 canary rests on — would be false (re-audit finding 2).
    ///
    /// Hashing it does not perturb the canary: two sims on the same input stream transit
    /// rooms identically, and `restore` never crosses a room boundary, so the term is
    /// equal on both sides of every comparison it takes part in — while a genuine
    /// room-transition desync now shows up instead of hiding.
    fn hash_active_room(world: &World, h: &mut StateHasher) {
        match world.get_resource::<ambition_world::rooms::RoomSet>() {
            Some(rs) => h.write_str(rs.active_spec().id.as_str()),
            // A headless world with no `RoomSet` hashes a sentinel distinct from a room
            // that is literally named "".
            None => h.write_str("\u{0}no-room"),
        }
    }

    /// **N0.4's per-tick hash of the whole registered sim state.**
    pub fn hash_world(&self, world: &World) -> u64 {
        let mut h = StateHasher::default();
        for entry in &self.entries {
            h.write_str(entry.name);
            self.hash_entry(entry, world, &mut h);
        }
        h.write_str(Self::ACTIVE_ROOM_ENTRY);
        Self::hash_active_room(world, &mut h);
        h.write_str(Self::ROSTER_ENTRY);
        Self::hash_roster(world, &mut h);
        h.finish()
    }

    /// The per-entry hashes, in registration order. A desync report wants this:
    /// "the worlds diverged, and it was `body_kinematics`" is a diagnosis; "the
    /// worlds diverged" is a fact.
    pub fn hash_by_entry(&self, world: &World) -> Vec<(&'static str, u64)> {
        let mut out: Vec<(&'static str, u64)> = self
            .entries
            .iter()
            .map(|entry| {
                let mut h = StateHasher::default();
                self.hash_entry(entry, world, &mut h);
                (entry.name, h.finish())
            })
            .collect();
        // The active-room cursor is part of `hash_world` (finding 2); surface it here too,
        // so a room-transition desync is a NAMED culprit rather than an aggregate hash
        // that moved with no entry to point at.
        let mut h = StateHasher::default();
        Self::hash_active_room(world, &mut h);
        out.push((Self::ACTIVE_ROOM_ENTRY, h.finish()));
        // Likewise the identity roster (finding 1): an entity that exists in one sim and not
        // the other is a desync this pseudo-entry names, where no component entry could.
        let mut h = StateHasher::default();
        Self::hash_roster(world, &mut h);
        out.push((Self::ROSTER_ENTRY, h.finish()));
        out
    }

    /// **The registration checklist, computed rather than written down.**
    ///
    /// Every component sitting on a `SimId` entity that this registry neither
    /// registers nor declares derived. `restore` destroys these, so the list is
    /// exactly what a rollback would silently lose today.
    ///
    /// Deduplicated and ordered by [`TypeId`](std::any::TypeId), which is always
    /// correct. The `name` is decoration: Bevy only stores component names when
    /// `bevy_ecs/debug` is on, and enabling it forks the bevy build cache between
    /// `cargo build` and `cargo test`. A gate that compares a COUNT does not care;
    /// a human reading the checklist can turn the feature on.
    pub fn unclaimed_components(&self, world: &World) -> Vec<UnclaimedComponent> {
        let claimed: Vec<std::any::TypeId> = self
            .entries
            .iter()
            .filter_map(|e| match &e.kind {
                EntryKind::Component { type_id, .. } => Some(*type_id),
                _ => None,
            })
            .chain(self.derived.iter().map(|(id, _)| *id))
            .collect();

        let Some(mut q) = world.try_query_filtered::<Entity, With<SimId>>() else {
            return Vec::new();
        };
        let mut out: Vec<UnclaimedComponent> = Vec::new();
        for entity in q.iter(world) {
            let Ok(infos) = world.inspect_entity(entity) else {
                continue;
            };
            for info in infos {
                // `SimId` itself is the key, not payload: `restore` writes it by
                // construction, so it is claimed by definition.
                match info.type_id() {
                    Some(id) if id == std::any::TypeId::of::<SimId>() => continue,
                    Some(id) if claimed.contains(&id) => continue,
                    // A component with no `TypeId` is dynamically registered — it
                    // cannot be claimed, so it is unclaimed, and it is reported.
                    type_id => out.push(UnclaimedComponent {
                        type_id,
                        name: info.name().to_string(),
                    }),
                }
            }
        }
        out.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
        out.dedup_by(|a, b| a.sort_key() == b.sort_key());
        out
    }
    /// **The ledger's other half: sim RESOURCES nobody registered.**
    ///
    /// `unclaimed_components` walks entities. A `Resource` sits on no entity, so
    /// resource-owned simulation state would otherwise be invisible to the ledger and
    /// untouched by restore. This complementary inventory keeps that class measurable.
    ///
    /// Filtered to types owned by this project, because Bevy's own resources (asset
    /// servers, schedules, render device state) are not sim state and never will be:
    /// the sim crates are the ones the determinism lints police. A name that does not
    /// start with `ambition_` is somebody else's problem; a name that does, and is not
    /// registered, is ours.
    ///
    /// Needs `bevy_ecs/debug` for the names, like `unclaimed_components`. Without it
    /// the list is empty rather than wrong, which is why the ledger test that reads it
    /// lives in `ambition_app`.
    pub fn unclaimed_resources(&self, world: &World) -> Vec<UnclaimedComponent> {
        let claimed: Vec<std::any::TypeId> =
            self.entries
                .iter()
                .filter_map(|e| match &e.kind {
                    EntryKind::Resource { type_id, .. }
                    | EntryKind::ResourceCursor { type_id, .. } => Some(*type_id),
                    _ => None,
                })
                .chain(self.derived.iter().map(|(id, _)| *id))
                // Registered `Messages<M>` channels are handled by restore (it clears them), so
                // their `Messages<M>` resource is claimed, not debt (finding 6). Without this,
                // the four channels restore deliberately clears counted against `lossless()`.
                .chain(self.messages.iter().map(|c| c.type_id))
                .collect();

        let mut out: Vec<UnclaimedComponent> = world
            .iter_resources()
            .map(|(info, _)| info)
            .filter(|info| {
                // `contains`, not `starts_with`. A `Messages<ambition_...::HitEvent>`
                // is named `bevy_ecs::message::Messages<..>` and is EVERY BIT the sim
                // state its payload is: a message written before a snapshot and read
                // after a restore is an event that happens twice. `starts_with` hid an
                // entire class of them behind Bevy's own module path.
                let name = info.name().to_string();
                name.contains("ambition_") && info.type_id().is_none_or(|id| !claimed.contains(&id))
            })
            .map(|info| UnclaimedComponent {
                type_id: info.type_id(),
                name: info.name().to_string(),
            })
            .collect();
        out.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
        out.dedup_by(|a, b| a.sort_key() == b.sort_key());
        out
    }

    /// **Genuine sim-state resource debt: [`unclaimed_resources`] MINUS the named
    /// exclusions.** (audit S2.8.)
    ///
    /// `unclaimed_resources` counts every unregistered `ambition_*` resource — but many
    /// are presentation, derived, or authored content that a rollback must NOT restore
    /// (`ActorRenderIndex` is rebuilt every frame; `SandboxLdtkProject` is immutable
    /// authored content; `CameraShakeState` is camera feel). Counting those as debt makes
    /// `lossless()` unachievable forever. The exclusions are named in
    /// [`SIM_RESOURCE_EXCLUSIONS`], each with a reason — an exclusion is a review event,
    /// not a silent gap. What remains is the sim state that genuinely must be registered.
    ///
    /// `lossless()` measures THIS, not the raw total.
    ///
    /// [`unclaimed_resources`]: SnapshotRegistry::unclaimed_resources
    pub fn unclaimed_sim_resources(&self, world: &World) -> Vec<UnclaimedComponent> {
        self.unclaimed_resources(world)
            .into_iter()
            .filter(|c| {
                !SIM_RESOURCE_EXCLUSIONS
                    .iter()
                    .any(|(needle, _)| c.name.contains(needle))
            })
            .collect()
    }
}

/// **Named exclusions from the sim-state resource universe** (audit S2.8).
///
/// A resource whose type name contains one of these is intentionally NOT sim state — it
/// is presentation, derived-per-frame, authored-immutable, or engine plumbing — so it
/// does not deny `lossless()`. Each entry carries the reason it is excluded; an exclusion
/// is a review event, not a silent gap. `contains` (not `starts_with`) is deliberate, so
/// a `Messages<ambition_sim_view::..>` channel is caught by its payload's namespace.
///
/// **Everything NOT matched here is genuine sim-state debt** and must be registered for a
/// room to become exactly restorable. Shrink this list only by proving a class is not
/// sim state; grow it only with a reason that survives review.
///
/// **Two forms, deliberately (re-audit finding 6):**
/// - A **namespace** needle (`crate::` or `crate::module::`) is permitted ONLY for a
///   subtree where EVERY resource is non-sim by construction — a presentation view, a menu,
///   camera feel, save I/O, engine plumbing. A sim resource added there is a layering error
///   the crate-boundary tests catch; the namespace sweep is therefore safe.
/// - An **exact type** needle (`::TypeName`) is required for a MIXED-purpose crate that
///   could gain a mutable sim resource. `ambition_ldtk_map` is the case the auditor named:
///   it holds authored-immutable + per-frame-derived resources today, but a namespace sweep
///   would auto-hide a NEW mutable one. Listing each type by name makes a new ldtk_map
///   resource count as debt — a review event — until it is explicitly classified here.
pub const SIM_RESOURCE_EXCLUSIONS: &[(&str, &str)] = &[
    (
        "ambition_sim_view::",
        "presentation view — derived from sim state every frame, never part of a state hash",
    ),
    (
        "ambition_platformer_primitives::camera_ease::",
        "camera feel (shake/ease) — presentation, runs on the feel clock",
    ),
    // ambition_ldtk_map — MIXED-purpose, so per-TYPE (see the two-forms note above). Every
    // current resource is authored-immutable content or a per-room-derived render/collision
    // index, restored by room-load, not by rollback. A new mutable one is NOT auto-excluded.
    (
        "::SandboxLdtkProject",
        "ldtk_map: loaded LDtk project — authored, immutable",
    ),
    (
        "::LdtkWorldAssets",
        "ldtk_map: asset handles — loaded content",
    ),
    (
        "::LdtkRuntimeIndex",
        "ldtk_map: per-room derived render index — rebuilt on room load",
    ),
    (
        "::LdtkRuntimeSpineStats",
        "ldtk_map: per-room derived spine stats — rebuilt on room load",
    ),
    (
        "::LdtkRuntimeSpineIndex",
        "ldtk_map: per-room derived spine index — rebuilt on room load",
    ),
    (
        "::LdtkRuntimeSolidIndex",
        "ldtk_map: per-room derived collision index — rebuilt on room load",
    ),
    (
        "::LdtkRuntimeOneWayIndex",
        "ldtk_map: per-room derived one-way index — rebuilt on room load",
    ),
    (
        "::LdtkRuntimeDamageIndex",
        "ldtk_map: per-room derived damage index — rebuilt on room load",
    ),
    (
        "::LdtkHotReloadState",
        "ldtk_map: editor hot-reload bookkeeping — out of sim scope",
    ),
    (
        "::LdtkRuntimeSpineParity",
        "ldtk_map: spine parity check — dev/derived",
    ),
    ("ambition_menu::", "menu / map UI state — presentation"),
    (
        "ambition_persistence::",
        "save / user-settings / quest-registry I/O — out of sim-tick scope",
    ),
    (
        "ambition_platformer_primitives::schedule::",
        "the sim schedule resource — engine plumbing, not sim state",
    ),
    (
        "ambition_platformer_primitives::physics::PhysicsSandboxSettings",
        "authored physics tuning — immutable for the room's life",
    ),
    (
        "ambition_portal::tuning::",
        "authored portal tuning — immutable",
    ),
    (
        "ambition_platformer_primitives::camera_ease::CameraEaseTuning",
        "authored camera tuning — immutable",
    ),
];
