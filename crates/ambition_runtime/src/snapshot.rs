//! **N3.1's registration seam, and N0.4's desync canary.**
//!
//! `docs/planning/engine/netcode.md` N0.4: *"Two sims, same input stream,
//! state-hash per tick (hash = the snapshot serialization of N3.1 — build them
//! together), first-divergence report. This is the tool that keeps N0 true
//! forever."*
//!
//! So they are built together, and this module is the half that exists today.
//!
//! ## What is here, and what is deliberately not
//!
//! N3.1's sketch pins four decisions. This module implements the two that do not
//! depend on the `SimId` migration:
//!
//! - **(1) Registration is OPT-IN per plugin.** Un-registered state is by
//!   definition presentation or derived, and *"the desync canary hashes exactly
//!   the registered set, which keeps the two features honest against each other."*
//! - **A registered entry must name its own stable ordering.** Bevy's `Query`
//!   iteration order is not stable, so an entry that walks entities hands back a
//!   `(stable_key, bytes)` pair per entity and this module sorts. A hash that
//!   depends on archetype layout is a hash that reports a desync on every run.
//!
//! - **(3) `restore` = despawn-registered + respawn from blobs.** No in-place
//!   patching. See [`restore`].
//!
//! ## One serialization, two consumers
//!
//! N0.4's line is *"state-hash per tick (hash = the snapshot serialization of N3.1
//! — build them together)"*. That is taken **literally**: a registered component
//! implements [`SnapshotState`] once, and the bytes it produces are BOTH the bytes
//! the canary hashes and the bytes [`take`] stores. There is no second encoder to
//! drift out of agreement with the first, and a component whose `decode` loses a
//! field is caught by a hash that no longer round-trips.
//!
//! ## What restore cannot rebuild, it reports
//!
//! Decision (3) despawns registered entities and respawns them from blobs. Every
//! component on such an entity that is neither **registered** nor explicitly
//! **declared derived** is therefore *lost* — silently, unless someone counts it.
//! [`SnapshotRegistry::unclaimed_components`] counts it. That number is the
//! per-crate registration checklist netcode.md's N3.1 pin asks for, made
//! executable: it starts high, it may never rise, and `restore` is honest about
//! the gap long before the gap closes.
//!
//! ## The hash
//!
//! FNV-1a over `(entry name, sorted (key, bytes) pairs)`, in registration order.
//! Deliberately not `std::hash::DefaultHasher`: `RandomState` is seeded per
//! process, so two runs of the same binary would disagree — which is exactly the
//! bug class ADR 0023 exists to prevent, and the last thing a desync canary should
//! be built on.

use ambition_platformer_primitives::body::BodyKinematics;
use ambition_platformer_primitives::sim_id::{SimId, SimIdCounter};
use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::query::With;
use bevy::ecs::resource::Resource;
use bevy::ecs::world::World;

/// A deterministic, process-stable hash. FNV-1a, 64-bit.
///
/// Not `std::collections::hash_map::DefaultHasher`, whose `RandomState` is seeded
/// per process. A canary that changes its mind between runs is noise.
#[derive(Clone, Copy, Debug)]
pub struct StateHasher(u64);

impl Default for StateHasher {
    fn default() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }
}

impl StateHasher {
    pub fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 ^= *b as u64;
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    /// Hash an `f32` by its BITS, canonicalizing every NaN to one pattern.
    ///
    /// Two sims that both produced a NaN agree; `-0.0` and `0.0` do not (they are
    /// different bit patterns and, in a physics sim, genuinely different states —
    /// a body resting on a surface at `-0.0` velocity has been pushed).
    pub fn write_f32(&mut self, v: f32) {
        let bits = if v.is_nan() {
            f32::NAN.to_bits()
        } else {
            v.to_bits()
        };
        self.write(&bits.to_le_bytes());
    }

    pub fn write_u64(&mut self, v: u64) {
        self.write(&v.to_le_bytes());
    }

    pub fn write_i32(&mut self, v: i32) {
        self.write(&v.to_le_bytes());
    }

    pub fn write_str(&mut self, v: &str) {
        self.write(v.as_bytes());
        self.write(&[0]); // length-delimit, so "ab"+"c" ≠ "a"+"bc"
    }

    pub fn finish(self) -> u64 {
        self.0
    }
}

/// **A component or resource that is sim state.**
///
/// The doc's `SnapshotState`. Implement it once; the canary hashes what it
/// produces and [`take`] stores the same bytes.
///
/// ## The encoding rules, which are not style
///
/// Explicit field order, fixed-width little-endian, no `usize`, no padding. A
/// snapshot format that inherits the host's word size cannot become the
/// cross-platform format netcode.md §"Portability" keeps reachable. Floats go
/// through [`put_f32`], which canonicalizes NaN — two sims that both produced a
/// NaN agree.
///
/// `decode` returns `None` on any byte string it did not write. A registry that
/// silently accepts a truncated blob restores a lie.
pub trait SnapshotState: Send + Sync + 'static {
    fn encode(&self, out: &mut Vec<u8>);
    fn decode(r: &mut Reader<'_>) -> Option<Self>
    where
        Self: Sized;
}

/// Append a canonical `f32`. NaN collapses to one bit pattern; `-0.0` does not
/// collapse to `0.0` (in a physics sim a body resting at `-0.0` has been pushed).
pub fn put_f32(out: &mut Vec<u8>, v: f32) {
    out.extend_from_slice(&canonical_f32_bits(v).to_le_bytes());
}

pub fn put_i32(out: &mut Vec<u8>, v: i32) {
    out.extend_from_slice(&v.to_le_bytes());
}

pub fn put_u64(out: &mut Vec<u8>, v: u64) {
    out.extend_from_slice(&v.to_le_bytes());
}

pub fn put_bool(out: &mut Vec<u8>, v: bool) {
    out.push(v as u8);
}

/// A cursor over a blob. Every getter returns `None` past the end, so a decoder
/// that reads more than its encoder wrote fails rather than guesses.
pub struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }

    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let end = self.at.checked_add(n)?;
        let out = self.bytes.get(self.at..end)?;
        self.at = end;
        Some(out)
    }

    pub fn f32(&mut self) -> Option<f32> {
        Some(f32::from_bits(u32::from_le_bytes(
            self.take(4)?.try_into().ok()?,
        )))
    }

    pub fn i32(&mut self) -> Option<i32> {
        Some(i32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }

    pub fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.take(8)?.try_into().ok()?))
    }

    pub fn bool(&mut self) -> Option<bool> {
        Some(*self.take(1)?.first()? != 0)
    }

    /// Every byte was consumed. A decoder that leaves bytes on the floor has
    /// disagreed with its encoder about the shape of the state.
    pub fn finish(self) -> Option<()> {
        (self.at == self.bytes.len()).then_some(())
    }
}

fn encode_one<T: SnapshotState>(v: &T) -> Vec<u8> {
    let mut out = Vec::new();
    v.encode(&mut out);
    out
}

fn decode_one<T: SnapshotState>(bytes: &[u8]) -> Option<T> {
    let mut r = Reader::new(bytes);
    let v = T::decode(&mut r)?;
    r.finish()?;
    Some(v)
}

/// What a registered entry *is*. The three kinds are not a taxonomy for its own
/// sake: they are the three answers to "what does `restore` do with this?"
enum EntryKind {
    /// Per-entity rows keyed by [`SimId`]. `restore` inserts each row onto the
    /// respawned entity that owns the key.
    Component {
        type_id: std::any::TypeId,
        rows: Box<dyn Fn(&World) -> Vec<(String, Vec<u8>)> + Send + Sync>,
        insert: Box<dyn Fn(&mut bevy::ecs::world::EntityWorldMut<'_>, &[u8]) + Send + Sync>,
    },
    /// A single blob. `restore` puts it back.
    Resource {
        bytes: Box<dyn Fn(&World) -> Vec<u8> + Send + Sync>,
        load: Box<dyn Fn(&mut World, &[u8]) + Send + Sync>,
    },
    /// Hashed, never restored: a MEASUREMENT of the world rather than a part of
    /// it. `unidentified_bodies` is the archetype — you cannot restore a count.
    Diagnostic { hash: fn(&World, &mut StateHasher) },
}

/// One registered piece of sim state.
struct StateEntry {
    name: &'static str,
    kind: EntryKind,
}

/// The opt-in registry of sim state (N3.1 decision 1). Each sim crate's plugin
/// registers what it owns; nothing else is snapshot state, by definition.
#[derive(Default)]
pub struct SnapshotRegistry {
    entries: Vec<StateEntry>,
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
                    if let Some(value) = decode_one::<C>(bytes) {
                        entity.insert(value);
                    } else {
                        // A blob this registry wrote that this registry cannot read
                        // is a codec bug, not a data condition. Fail loudly in
                        // tests; in a shipped rollback, drop the component rather
                        // than the frame.
                        debug_assert!(false, "snapshot blob failed to decode");
                    }
                }),
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
                bytes: Box::new(|world: &World| {
                    world
                        .get_resource::<R>()
                        .map(encode_one)
                        .unwrap_or_default()
                }),
                load: Box::new(|world: &mut World, bytes: &[u8]| {
                    if bytes.is_empty() {
                        world.remove_resource::<R>();
                    } else if let Some(value) = decode_one::<R>(bytes) {
                        world.insert_resource(value);
                    }
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
        debug_assert!(
            !self.entries.iter().any(|e| e.name == name),
            "sim-state entry `{name}` registered twice"
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
            EntryKind::Resource { bytes, .. } => h.write(&bytes(world)),
            EntryKind::Diagnostic { hash } => hash(world, h),
        }
    }

    /// **N0.4's per-tick hash of the whole registered sim state.**
    pub fn hash_world(&self, world: &World) -> u64 {
        let mut h = StateHasher::default();
        for entry in &self.entries {
            h.write_str(entry.name);
            self.hash_entry(entry, world, &mut h);
        }
        h.finish()
    }

    /// The per-entry hashes, in registration order. A desync report wants this:
    /// "the worlds diverged, and it was `body_kinematics`" is a diagnosis; "the
    /// worlds diverged" is a fact.
    pub fn hash_by_entry(&self, world: &World) -> Vec<(&'static str, u64)> {
        self.entries
            .iter()
            .map(|entry| {
                let mut h = StateHasher::default();
                self.hash_entry(entry, world, &mut h);
                (entry.name, h.finish())
            })
            .collect()
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
}

/// A component `restore` would destroy and cannot rebuild. One row of N3.1's
/// per-crate registration checklist.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnclaimedComponent {
    /// `None` for a dynamically-registered component, which can never be claimed.
    pub type_id: Option<std::any::TypeId>,
    /// Bevy only stores real component names under `bevy_ecs/debug`; otherwise this
    /// is a placeholder, identical for every component. Never dedup on it.
    pub name: String,
}

impl UnclaimedComponent {
    fn sort_key(&self) -> (Option<std::any::TypeId>, &str) {
        (self.type_id, self.name.as_str())
    }
}

impl std::fmt::Display for UnclaimedComponent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// **A snapshot of the registered sim state at one tick.**
///
/// ## Deviation from the sketch, stated rather than drifted
///
/// netcode.md sketches `SimSnapshot { tick: u64, blobs: Vec<(StateTypeId,
/// Box<[u8]>)> }` — one flat byte string per entry. This keeps the entity ROWS
/// structured (`Vec<(SimId, Vec<u8>)>`) instead of concatenating them into a blob
/// a reader has to re-split. The reason is decision (3): `restore` must group rows
/// by `SimId` across entries to respawn one entity carrying all of its components.
/// A flat blob would be parsed back into exactly this shape on the first line of
/// `restore`, and the parse could fail. This cannot.
///
/// The wire format — where `Box<[u8]>` and a version tag earn their keep — is
/// N3.3's, and it serializes THIS, which is why the per-entry bytes are already
/// canonical and word-size-free.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SimSnapshot {
    /// The `SimTick` this was taken at. Not derived from the entries: a caller
    /// comparing two snapshots wants the tick before it wants the bytes.
    pub tick: u64,
    entries: Vec<(&'static str, EntryBlob)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum EntryBlob {
    /// Sorted by `SimId`, so two snapshots of equal worlds are `==`.
    Component(Vec<(String, Vec<u8>)>),
    Resource(Vec<u8>),
}

impl SimSnapshot {
    /// Every `SimId` the snapshot knows about, sorted. These are exactly the
    /// entities `restore` will respawn.
    pub fn sim_ids(&self) -> Vec<&str> {
        let mut out: Vec<&str> = self
            .entries
            .iter()
            .flat_map(|(_, blob)| match blob {
                EntryBlob::Component(rows) => rows.iter().map(|(k, _)| k.as_str()).collect(),
                EntryBlob::Resource(_) => Vec::new(),
            })
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Total bytes of state. A rollback window is a memory budget.
    pub fn size_bytes(&self) -> usize {
        self.entries
            .iter()
            .map(|(_, blob)| match blob {
                EntryBlob::Component(rows) => {
                    rows.iter().map(|(k, v)| k.len() + v.len()).sum::<usize>()
                }
                EntryBlob::Resource(bytes) => bytes.len(),
            })
            .sum()
    }
}

/// **Take a snapshot of the registered sim state.**
///
/// Diagnostics are skipped: you cannot restore a count. Everything else is copied
/// out through the same [`SnapshotState::encode`] the canary hashes, which is what
/// makes "the hash IS the serialization" a fact rather than a comment.
pub fn take(world: &World, registry: &SnapshotRegistry) -> SimSnapshot {
    let tick = world
        .get_resource::<ambition_time::SimTick>()
        .map_or(0, |t| t.0);
    let mut entries = Vec::new();
    for entry in &registry.entries {
        match &entry.kind {
            EntryKind::Component { rows, .. } => {
                let mut rows = rows(world);
                rows.sort();
                entries.push((entry.name, EntryBlob::Component(rows)));
            }
            EntryKind::Resource { bytes, .. } => {
                entries.push((entry.name, EntryBlob::Resource(bytes(world))));
            }
            EntryKind::Diagnostic { .. } => {}
        }
    }
    SimSnapshot { tick, entries }
}

/// What `restore` did, and what it could not do.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct RestoreReport {
    /// Entities carrying a `SimId` that were destroyed.
    pub despawned: usize,
    /// Entities recreated from the snapshot. Differs from `despawned` whenever the
    /// snapshot is from a tick with a different population — which is the point.
    pub spawned: usize,
    /// Component types that existed on the despawned entities and that the registry
    /// neither registers nor declares derived. **These are gone.** An empty list is
    /// half of N3.1's real exit condition.
    pub lost_components: Vec<UnclaimedComponent>,
    /// **Simulated bodies with no `SimId`, which `restore` could not touch.**
    ///
    /// They are not registered, so decision (3) does not despawn them — and so they
    /// walk out of a rollback carrying whatever state the rewound tick gave them.
    /// A projectile in this set survives its own un-firing. Zero is the other half
    /// of N3.1's exit condition, and `unidentified_bodies` is the same number seen
    /// from the canary's side.
    pub unidentified_survivors: usize,
}

impl RestoreReport {
    /// The registry covered everything the restore touched, and there was nothing
    /// it could not touch. Only then is a restored world the world that was taken.
    pub fn lossless(&self) -> bool {
        self.lost_components.is_empty() && self.unidentified_survivors == 0
    }
}

/// **Restore the sim to a snapshot: despawn every registered entity, respawn from
/// blobs** (N3.1 decision 3 — no in-place patching).
///
/// An entity spawned after the snapshot simply ceases to exist; one despawned since
/// is recreated. Both are correct, and both fall out of "the snapshot is the truth"
/// rather than out of a diff.
///
/// ## The honest part
///
/// A respawned entity carries exactly its registered components. Everything else it
/// had — brains, sprites, colliders, timers not yet registered — is **lost**, and
/// the returned [`RestoreReport::lost_components`] names it.
///
/// And what is not registered is not despawned: a simulated body with no `SimId`
/// **survives the rollback**, because decision (3) despawns the *registered* set and
/// it is not in it. [`RestoreReport::unidentified_survivors`] counts those. A
/// projectile in that set outlives its own un-firing, which is exactly the kind of
/// bug a rollback is supposed to make impossible — so the number is reported at
/// every call and gated at zero in `ambition_app`'s ledger test.
///
/// Callers that need a whole body back must finish the registration checklist;
/// callers that need only registered state (N0.4's canary, FB6's rollouts on a
/// scratch world) are already served. Reporting the gap is what keeps it from being
/// discovered in a playtest.
pub fn restore(
    world: &mut World,
    snapshot: &SimSnapshot,
    registry: &SnapshotRegistry,
) -> RestoreReport {
    let lost_components = registry.unclaimed_components(world);

    let doomed: Vec<Entity> = match world.try_query_filtered::<Entity, With<SimId>>() {
        Some(mut q) => q.iter(world).collect(),
        None => Vec::new(),
    };
    let despawned = doomed.len();
    for entity in doomed {
        world.despawn(entity);
    }

    // Respawn in SimId order. Entity ALLOCATION order is not sim state, but a
    // deterministic order costs nothing and makes two restores of one snapshot
    // produce identical worlds down to the entity indices — which a debugger reads.
    let ids = snapshot.sim_ids();
    let mut spawned = 0usize;
    for id in &ids {
        let entity = world.spawn(SimId::from_snapshot((*id).to_string())).id();
        spawned += 1;
        for (name, blob) in &snapshot.entries {
            let EntryBlob::Component(rows) = blob else {
                continue;
            };
            let Ok(row) = rows.binary_search_by(|(k, _)| k.as_str().cmp(id)) else {
                continue;
            };
            let Some(entry) = registry.entries.iter().find(|e| e.name == *name) else {
                continue;
            };
            let EntryKind::Component { insert, .. } = &entry.kind else {
                continue;
            };
            let bytes = rows[row].1.clone();
            let mut e = world.entity_mut(entity);
            insert(&mut e, &bytes);
        }
    }

    for (name, blob) in &snapshot.entries {
        let EntryBlob::Resource(bytes) = blob else {
            continue;
        };
        let Some(entry) = registry.entries.iter().find(|e| e.name == *name) else {
            continue;
        };
        let EntryKind::Resource { load, .. } = &entry.kind else {
            continue;
        };
        load(world, bytes);
    }

    let unidentified_survivors = match world
        .try_query_filtered::<(), (With<BodyKinematics>, bevy::ecs::query::Without<SimId>)>()
    {
        Some(mut q) => q.iter(world).count(),
        None => 0,
    };

    RestoreReport {
        despawned,
        spawned,
        lost_components,
        unidentified_survivors,
    }
}

/// Hash a set of `(stable_key, payload)` pairs, sorted by key.
///
/// **Bevy's `Query` iteration order is not stable** — it follows archetype layout,
/// which depends on spawn order and component insertion history. Two sims fed the
/// same inputs can walk the same entities in different orders. Sorting by a stable
/// key is what makes a hash a statement about the SIM rather than about the
/// allocator.
///
/// Duplicate keys are a bug in the caller's identity scheme, and the hash folds
/// them in sorted-by-payload order so it at least stays deterministic while
/// someone fixes it.
pub fn hash_entities_by_key(h: &mut StateHasher, mut rows: Vec<(String, Vec<u8>)>) {
    rows.sort();
    h.write_u64(rows.len() as u64);
    for (key, payload) in rows {
        h.write_str(&key);
        h.write(&payload);
    }
}

/// Per-tick hashes of two runs, and where they first disagreed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesyncReport {
    /// The first tick whose hashes differ. `None` when the runs agree throughout.
    pub first_divergence_tick: Option<u64>,
    /// How many ticks were compared.
    pub ticks_compared: u64,
}

impl DesyncReport {
    pub fn in_sync(&self) -> bool {
        self.first_divergence_tick.is_none()
    }
}

/// Compare two per-tick hash streams. The canary's whole judgement, as a pure
/// function — so its own correctness is a unit test rather than a two-sim run.
///
/// Streams of different length diverge at the shorter one's end: a sim that
/// stopped early did not agree, it stopped.
pub fn compare_hash_streams(a: &[u64], b: &[u64]) -> DesyncReport {
    let n = a.len().min(b.len());
    for tick in 0..n {
        if a[tick] != b[tick] {
            return DesyncReport {
                first_divergence_tick: Some(tick as u64),
                ticks_compared: n as u64,
            };
        }
    }
    if a.len() != b.len() {
        return DesyncReport {
            first_divergence_tick: Some(n as u64),
            ticks_compared: n as u64,
        };
    }
    DesyncReport {
        first_divergence_tick: None,
        ticks_compared: n as u64,
    }
}

// ── The engine's own registrations ───────────────────────────────────────────

/// Register the sim state `ambition_runtime` and its immediate neighbours own.
///
/// This is deliberately NOT the checklist in netcode.md's N3.1 pin — that list
/// (move playbacks, brain memory, portal transit, falling-sand grids, every seeded
/// RNG) needs `SimId` on the entities that carry it. What is here is what has a
/// stable key TODAY:
///
/// - the sim clock (`SimTick`), which N0.2's stream and N0.4's hash both key on;
/// - `WorldTime`'s scaled dt, because a clock that drifts desyncs everything;
/// - every body with a `FeatureId` — actors, bosses, spawned features — keyed by
///   that id, which IS the LDtk placement identity N3.1 names;
/// - the primary player's body, keyed by its slot.
///
/// Anything else is unregistered, and by N3.1 decision 1 that is a CLAIM: it is
/// presentation, derived, or it is missing. `netcode.md`'s N3.1 section carries
/// the migration list; this function is where each row lands as it gets an id.
pub fn register_engine_sim_state(registry: &mut SnapshotRegistry) {
    registry.register_resource::<ambition_time::SimTick>("sim_tick");
    registry.register_resource::<ambition_time::WorldTime>("world_time");
    registry.register_component::<BodyKinematics>("body_kinematics");
    registry.register_component::<ambition_characters::actor::BodyHealth>("body_health");

    // Each spawner's minted-id count. Two sims that minted a different NUMBER of
    // projectiles are not in the same state even if every surviving body agrees —
    // the divergence may be a shot that has already despawned.
    registry.register_component::<SimIdCounter>("sim_id_counters");

    // **The blind spot, made loud.** Simulated bodies with no `SimId` cannot be
    // snapshotted, restored, or defended by the canary. Hashing the COUNT means a
    // sim that spawned a different number of un-identified bodies still diverges —
    // the canary reports "I cannot see what changed", which beats reporting green.
    // `unidentified_bodies` goes to zero as the SimId migration finishes.
    //
    // A DIAGNOSTIC, not state: you cannot restore a count.
    registry.register_diagnostic("unidentified_bodies", |world, h| {
        let Some(mut q) = world
            .try_query_filtered::<(), (With<BodyKinematics>, bevy::ecs::query::Without<SimId>)>()
        else {
            return;
        };
        h.write_u64(q.iter(world).count() as u64);
    });
}

// ── The engine's codecs ──────────────────────────────────────────────────────
//
// Explicit field order, fixed-width LE, every field present. A codec that skips a
// field the sim reads is a restore that silently rewinds to a different world; the
// round-trip oracle in this module's tests is what catches one.

impl SnapshotState for ambition_time::SimTick {
    fn encode(&self, out: &mut Vec<u8>) {
        put_u64(out, self.0);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(ambition_time::SimTick(r.u64()?))
    }
}

impl SnapshotState for ambition_time::WorldTime {
    fn encode(&self, out: &mut Vec<u8>) {
        put_f32(out, self.raw_dt);
        put_f32(out, self.scaled_dt);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(ambition_time::WorldTime {
            raw_dt: r.f32()?,
            scaled_dt: r.f32()?,
        })
    }
}

impl SnapshotState for BodyKinematics {
    fn encode(&self, out: &mut Vec<u8>) {
        for v in [
            self.pos.x,
            self.pos.y,
            self.vel.x,
            self.vel.y,
            self.size.x,
            self.size.y,
            self.facing,
        ] {
            put_f32(out, v);
        }
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(BodyKinematics {
            pos: bevy::math::Vec2::new(r.f32()?, r.f32()?),
            vel: bevy::math::Vec2::new(r.f32()?, r.f32()?),
            size: bevy::math::Vec2::new(r.f32()?, r.f32()?),
            facing: r.f32()?,
        })
    }
}

impl SnapshotState for ambition_characters::actor::BodyHealth {
    fn encode(&self, out: &mut Vec<u8>) {
        put_i32(out, self.health.current);
        put_i32(out, self.health.max);
        put_bool(out, self.health.invulnerable);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(ambition_characters::actor::BodyHealth::new(
            ambition_characters::actor::Health {
                current: r.i32()?,
                max: r.i32()?,
                invulnerable: r.bool()?,
            },
        ))
    }
}

impl SnapshotState for SimIdCounter {
    fn encode(&self, out: &mut Vec<u8>) {
        put_u64(out, self.0);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(SimIdCounter(r.u64()?))
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
        let id = SimId::spawned(owner_id, counter.next());
        // A projectile can itself spawn (a splitting shot), so it gets a counter.
        commands.entity(entity).insert((
            id,
            ambition_platformer_primitives::sim_id::SimIdCounter::default(),
        ));
    }
}

fn canonical_f32_bits(v: f32) -> u32 {
    if v.is_nan() {
        f32::NAN.to_bits()
    } else {
        v.to_bits()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hasher_is_process_stable_and_order_sensitive() {
        let mut a = StateHasher::default();
        a.write_str("x");
        a.write_u64(7);
        let mut b = StateHasher::default();
        b.write_u64(7);
        b.write_str("x");
        assert_ne!(a.finish(), b.finish(), "order matters");

        // The literal FNV-1a offset basis, so a refactor of the constants is loud.
        assert_eq!(StateHasher::default().finish(), 0xcbf2_9ce4_8422_2325);
    }

    /// Strings are length-delimited, so `"ab" + "c"` and `"a" + "bc"` differ. A
    /// hash that confused two entity ids would report sync across a real desync.
    #[test]
    fn string_writes_are_length_delimited() {
        let mut a = StateHasher::default();
        a.write_str("ab");
        a.write_str("c");
        let mut b = StateHasher::default();
        b.write_str("a");
        b.write_str("bc");
        assert_ne!(a.finish(), b.finish());
    }

    /// Every NaN hashes alike (two sims that both blew up agree), but `-0.0` and
    /// `0.0` do not: in a physics sim a body resting at `-0.0` velocity has been
    /// pushed, and that is a state difference worth catching.
    #[test]
    fn nan_is_canonical_but_negative_zero_is_not() {
        let hash = |v: f32| {
            let mut h = StateHasher::default();
            h.write_f32(v);
            h.finish()
        };
        assert_eq!(hash(f32::NAN), hash(-f32::NAN));
        assert_eq!(hash(f32::NAN), hash(0.0 / 0.0));
        assert_ne!(hash(0.0), hash(-0.0));
    }

    /// **The reason `hash_entities_by_key` exists.** Bevy's query order follows
    /// archetype layout; two sims can walk the same entities in different orders.
    /// A hash that noticed would cry desync on every run.
    #[test]
    fn entity_rows_hash_the_same_however_the_query_walked_them() {
        let rows = |order: [usize; 3]| {
            let all = [
                ("b".to_string(), vec![2u8]),
                ("a".to_string(), vec![1u8]),
                ("c".to_string(), vec![3u8]),
            ];
            let mut h = StateHasher::default();
            hash_entities_by_key(
                h_mut(&mut h),
                order.iter().map(|i| all[*i].clone()).collect(),
            );
            h.finish()
        };
        fn h_mut(h: &mut StateHasher) -> &mut StateHasher {
            h
        }
        assert_eq!(rows([0, 1, 2]), rows([2, 1, 0]));
        assert_eq!(rows([0, 1, 2]), rows([1, 2, 0]));
    }

    /// ...but the row COUNT is hashed, so an entity that failed to spawn in one
    /// sim is a divergence rather than a shrug.
    #[test]
    fn a_missing_entity_changes_the_hash() {
        let mut a = StateHasher::default();
        hash_entities_by_key(&mut a, vec![("x".into(), vec![1]), ("y".into(), vec![2])]);
        let mut b = StateHasher::default();
        hash_entities_by_key(&mut b, vec![("x".into(), vec![1])]);
        assert_ne!(a.finish(), b.finish());
    }

    #[test]
    fn identical_streams_are_in_sync() {
        let r = compare_hash_streams(&[1, 2, 3], &[1, 2, 3]);
        assert!(r.in_sync());
        assert_eq!(r.ticks_compared, 3);
    }

    #[test]
    fn the_report_names_the_first_divergent_tick_and_not_the_last() {
        let r = compare_hash_streams(&[1, 2, 9, 9], &[1, 2, 3, 4]);
        assert_eq!(r.first_divergence_tick, Some(2));
    }

    /// A sim that stopped early did not agree; it stopped.
    #[test]
    fn a_short_stream_diverges_at_its_own_end() {
        let r = compare_hash_streams(&[1, 2], &[1, 2, 3]);
        assert_eq!(r.first_divergence_tick, Some(2));
        assert!(!r.in_sync());
    }

    #[test]
    fn a_registry_hashes_its_entry_names_so_two_registries_never_agree_by_luck() {
        let world = World::new();
        let mut a = SnapshotRegistry::default();
        a.register_diagnostic("alpha", |_, h| h.write_u64(1));
        let mut b = SnapshotRegistry::default();
        b.register_diagnostic("beta", |_, h| h.write_u64(1));
        assert_ne!(a.hash_world(&world), b.hash_world(&world));
        assert_eq!(a.len(), 1);
        assert_eq!(a.names().collect::<Vec<_>>(), ["alpha"]);
    }

    #[test]
    fn per_entry_hashes_localize_a_divergence() {
        let world = World::new();
        let mut reg = SnapshotRegistry::default();
        reg.register_diagnostic("a", |_, h| h.write_u64(1));
        reg.register_diagnostic("b", |_, h| h.write_u64(2));
        let by_entry = reg.hash_by_entry(&world);
        assert_eq!(by_entry.len(), 2);
        assert_eq!(by_entry[0].0, "a");
        assert_ne!(by_entry[0].1, by_entry[1].1);
    }

    // ── N3.1: take / restore ─────────────────────────────────────────────────

    use ambition_characters::actor::{BodyHealth, Health};
    use bevy::math::Vec2;

    /// A component nothing registers and nothing declares derived. It stands in for
    /// every un-migrated piece of sim state: a brain, a cooldown, a portal's transit
    /// latch. `restore` destroys it, and the report must SAY so.
    #[derive(Component, Clone, Copy, PartialEq, Debug)]
    struct UnregisteredThing(u32);

    /// A component `restore` is allowed to destroy, because the system that
    /// maintains it rebuilds it every tick.
    #[derive(Component)]
    struct DerivedThing;

    fn kin(pos: Vec2, vel: Vec2) -> BodyKinematics {
        BodyKinematics {
            pos,
            vel,
            size: Vec2::new(16.0, 32.0),
            facing: 1.0,
        }
    }

    fn engine_registry() -> SnapshotRegistry {
        let mut reg = SnapshotRegistry::default();
        register_engine_sim_state(&mut reg);
        reg
    }

    fn sim_world() -> World {
        let mut world = World::new();
        world.insert_resource(ambition_time::SimTick(11));
        world.insert_resource(ambition_time::WorldTime {
            raw_dt: 1.0 / 60.0,
            scaled_dt: 1.0 / 60.0,
        });
        world.spawn((
            SimId::placement("boss-1"),
            SimIdCounter(3),
            kin(Vec2::new(10.0, -4.5), Vec2::new(0.0, 120.0)),
            BodyHealth::new(Health {
                current: 40,
                max: 100,
                invulnerable: false,
            }),
        ));
        world.spawn((
            SimId::player_slot(0),
            SimIdCounter(0),
            kin(Vec2::new(-3.0, 0.0), Vec2::ZERO),
        ));
        world
    }

    /// **Every codec round-trips.** A field an encoder writes and a decoder skips is
    /// a restore that rewinds to a *different* world — quietly, and only in the
    /// field nobody encoded. `Reader::finish` refuses leftover bytes, so this also
    /// catches a decoder that reads too little.
    ///
    /// The property is `encode ∘ decode ∘ encode == encode`, asserted on BYTES
    /// rather than on values: a decoder that drops a field re-encodes a default in
    /// its place, and not every sim type wants a `PartialEq` it does not otherwise
    /// need just so a test can look at it.
    #[test]
    fn every_engine_codec_round_trips_exactly() {
        fn round_trip<T: SnapshotState>(v: T) {
            let bytes = encode_one(&v);
            let back = decode_one::<T>(&bytes).expect("decodes");
            assert_eq!(
                encode_one(&back),
                bytes,
                "{} lost a field",
                std::any::type_name::<T>()
            );
        }
        round_trip(ambition_time::SimTick(9_000_000_001));
        round_trip(ambition_time::WorldTime {
            raw_dt: 0.016,
            scaled_dt: -0.0,
        });
        round_trip(kin(Vec2::new(1.5, -2.25), Vec2::new(-0.0, 7.0)));
        round_trip(BodyHealth::new(Health {
            current: -3,
            max: 250,
            invulnerable: true,
        }));
        round_trip(SimIdCounter(u64::MAX));
    }

    /// A truncated blob decodes to `None` rather than to a plausible lie.
    #[test]
    fn a_short_blob_is_rejected_rather_than_guessed() {
        let bytes = encode_one(&kin(Vec2::ONE, Vec2::ZERO));
        assert!(decode_one::<BodyKinematics>(&bytes[..bytes.len() - 1]).is_none());
        let mut too_long = bytes.clone();
        too_long.push(0);
        assert!(
            decode_one::<BodyKinematics>(&too_long).is_none(),
            "leftover bytes mean the decoder disagreed with the encoder"
        );
    }

    /// **The oracle for the whole slice.** Take, wreck the world, restore, and the
    /// registered state hashes to what it hashed before. This is the property N0.4
    /// and FB6 both actually need, and it is one assertion.
    #[test]
    fn a_restored_world_hashes_exactly_as_the_taken_one_did() {
        let reg = engine_registry();
        let mut world = sim_world();
        let before = reg.hash_world(&world);
        let snap = take(&world, &reg);

        // Advance the sim, badly: move a body, hurt it, kill the other, spawn a
        // third, wind the clock.
        let boss = world
            .try_query_filtered::<Entity, With<SimId>>()
            .unwrap()
            .iter(&world)
            .next()
            .unwrap();
        world
            .entity_mut(boss)
            .insert(kin(Vec2::splat(999.0), Vec2::ZERO));
        world.insert_resource(ambition_time::SimTick(50));
        world.spawn((
            SimId::spawned(&SimId::player_slot(0), 1),
            kin(Vec2::ZERO, Vec2::ZERO),
        ));

        assert_ne!(reg.hash_world(&world), before, "the wreck must be visible");
        restore(&mut world, &snap, &reg);
        assert_eq!(reg.hash_world(&world), before, "restore did not restore");
    }

    /// An entity spawned after the snapshot ceases to exist; one despawned since is
    /// recreated. Both fall out of "the snapshot is the truth", not out of a diff.
    #[test]
    fn restore_forgets_the_future_and_remembers_the_dead() {
        let reg = engine_registry();
        let mut world = sim_world();
        let snap = take(&world, &reg);
        assert_eq!(snap.sim_ids(), ["placement:boss-1", "slot:0"]);

        let doomed: Vec<Entity> = world
            .try_query_filtered::<Entity, With<SimId>>()
            .unwrap()
            .iter(&world)
            .collect();
        world.despawn(doomed[0]);
        world.spawn((SimId::placement("ghost"), kin(Vec2::ZERO, Vec2::ZERO)));

        let report = restore(&mut world, &snap, &reg);
        assert_eq!(report.despawned, 2, "one survivor + one ghost");
        assert_eq!(report.spawned, 2);

        let ids: Vec<String> = world
            .try_query::<&SimId>()
            .unwrap()
            .iter(&world)
            .map(|id| id.as_str().to_string())
            .collect();
        assert!(
            ids.contains(&"placement:boss-1".to_string()),
            "the dead came back"
        );
        assert!(
            !ids.contains(&"ghost".to_string()),
            "the future was forgotten"
        );
    }

    /// Taking a snapshot of a restored world yields the identical snapshot. Restore
    /// is idempotent, which is what a rollback window replays across.
    #[test]
    fn take_after_restore_is_the_snapshot_you_restored() {
        let reg = engine_registry();
        let mut world = sim_world();
        let snap = take(&world, &reg);
        restore(&mut world, &snap, &reg);
        assert_eq!(take(&world, &reg), snap);
    }

    /// **`restore` names what it destroyed.** The whole point of the coverage
    /// ledger: an unregistered component is lost, and the loss is reported rather
    /// than discovered. A declared-derived one is lost silently, on purpose.
    #[test]
    fn restore_reports_the_components_it_could_not_rebuild() {
        let mut reg = engine_registry();
        reg.declare_derived::<DerivedThing>("rebuilt every tick by the same system");
        let mut world = sim_world();
        let boss = world
            .try_query_filtered::<Entity, With<SimId>>()
            .unwrap()
            .iter(&world)
            .next()
            .unwrap();
        world
            .entity_mut(boss)
            .insert((UnregisteredThing(7), DerivedThing));

        let unclaimed = reg.unclaimed_components(&world);
        assert_eq!(unclaimed.len(), 1, "got {unclaimed:?}");
        assert_eq!(
            unclaimed[0].type_id,
            Some(std::any::TypeId::of::<UnregisteredThing>()),
            "the ledger keys on TypeId, because component NAMES need bevy's \
             `debug` feature and would all dedup to one placeholder without it"
        );

        let snap = take(&world, &reg);
        let report = restore(&mut world, &snap, &reg);
        assert!(!report.lossless());
        assert_eq!(report.lost_components, unclaimed);
        assert_eq!(
            world
                .try_query::<&UnregisteredThing>()
                .unwrap()
                .iter(&world)
                .count(),
            0,
            "restore really did destroy it — the report is not a warning, it is a fact"
        );
        // ...and once it is DECLARED derived, the ledger is clean, because "derived"
        // is a promise that some per-frame system rebuilds it.
        reg.declare_derived::<UnregisteredThing>("pretend");
        assert!(reg.unclaimed_components(&world).is_empty());
    }

    /// **A body with no `SimId` walks out of a rollback.** `restore` despawns the
    /// REGISTERED set, and an unidentified body is not in it. This is the bug class
    /// N3.1's identity pin exists to close, and until it is closed the count is
    /// reported at every restore rather than left to a playtest.
    #[test]
    fn an_unidentified_body_survives_the_restore_and_is_counted() {
        let reg = engine_registry();
        let mut world = sim_world();
        let snap = take(&world, &reg);
        world.spawn(kin(Vec2::splat(5.0), Vec2::ZERO)); // no SimId: a ghost

        let report = restore(&mut world, &snap, &reg);
        assert_eq!(report.unidentified_survivors, 1);
        assert!(
            !report.lossless(),
            "a restore that leaves a body standing did not restore the world"
        );
        assert!(
            report.lost_components.is_empty(),
            "nothing was lost — something was KEPT that should not have been"
        );
    }

    /// Diagnostics are hashed and never snapshotted: you cannot restore a count.
    ///
    /// And so `unidentified_bodies` measures something `restore` cannot fix — which
    /// is precisely why it is hashed. The canary sees the stray body that the
    /// rollback left standing, and cries desync, which is the correct verdict.
    #[test]
    fn a_diagnostic_is_hashed_but_never_snapshotted() {
        let reg = engine_registry();
        let mut world = sim_world();
        let clean = reg.hash_world(&world);
        let snap = take(&world, &reg);
        assert!(
            !snap
                .entries
                .iter()
                .any(|(n, _)| *n == "unidentified_bodies"),
            "a count has no blob"
        );

        world.spawn(kin(Vec2::ZERO, Vec2::ZERO)); // no SimId
        restore(&mut world, &snap, &reg);
        assert_ne!(
            reg.hash_world(&world),
            clean,
            "the stray body outlived the restore, and the canary must say so"
        );
    }

    /// The snapshot's rows are sorted, so two equal worlds produce `==` snapshots
    /// whatever order their archetypes happened to be walked in.
    #[test]
    fn two_equal_worlds_take_equal_snapshots() {
        let reg = engine_registry();
        let a = take(&sim_world(), &reg);
        let b = take(&sim_world(), &reg);
        assert_eq!(a, b);
        assert_eq!(a.tick, 11);
        assert!(a.size_bytes() > 0);
    }
}
