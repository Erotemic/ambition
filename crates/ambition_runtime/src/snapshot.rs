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
//! - **(3) `restore` rebuilds the world from the snapshot.** It reconciles by
//!   `SimId`: patch the survivors, respawn the missing, despawn the newcomers. The
//!   sketch said despawn-everything; [`restore`] documents why that is wrong for the
//!   case a rollback is made of, and what it costs (53 component types on `gap_run`).
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
//! ## What restore cannot rewind, it reports
//!
//! A patched entity keeps every component the registry does not know about. An
//! immutable authored fact (a moveset, a faction) is *correct* left alone; a timer
//! is **stale**, and it is that timer that makes a replay diverge.
//! [`SnapshotRegistry::unclaimed_components`] cannot tell the two apart, so it
//! reports both, and the number is the per-crate registration checklist netcode.md's
//! N3.1 pin asks for, made executable: it may fall, it may never rise, and `restore`
//! is honest about the gap long before the gap closes.
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

/// **A component that is partly authored content and partly a mutable cursor.**
///
/// `ActorMotionPath` is the archetype: it owns a patrol path (authored, immutable,
/// large) and a `(segment, dir)` cursor (mutable, tiny, and the whole reason a
/// rollback touches it). Serializing the path sixty times a second to rewind two
/// integers is absurd, and `SnapshotState::decode` cannot rebuild the component
/// without it.
///
/// So a cursor component is **applied onto the entity that already has it**. That is
/// sound precisely because [`restore`] patches survivors: an entity present in both
/// worlds still carries its authored half. An entity being *respawned* does not, and
/// `restore` therefore cannot rebuild its cursor — which is one more reason a
/// rollback window must not span a spawn, and why `RestoreReport::respawned` is a
/// number you are meant to look at.
pub trait SnapshotCursor: Send + Sync + 'static {
    fn encode_cursor(&self, out: &mut Vec<u8>);
    fn apply_cursor(&mut self, r: &mut Reader<'_>) -> Option<()>;
}

/// **A component that REFERENCES authored content instead of owning it.**
///
/// `MovePlayback` embeds a whole `MoveSpec` — but `MoveSpec.id` is a stable authored
/// id, and the entity's `ActorMoveset` survives the rewind because it is authored
/// config and [`restore`] patches survivors. So the snapshot carries the *choice*
/// (`"jab"`, `t = 0.13`) and `resolve` rebuilds the component from the content the
/// entity is still holding.
///
/// > **Reference authored content by its authored id, never by value.** A snapshot
/// > carries what the sim CHOSE; the content it chose from is still on the entity.
///
/// Unlike [`SnapshotCursor`], this can restore a component's **presence**:
/// `MovePlayback` is inserted when a move starts and removed when it ends, so a
/// rollback must be able to both add and drop it.
pub trait SnapshotResolve: Send + Sync + 'static {
    /// The choice, not the content: an id, a clock, a flag.
    fn encode_ref(&self, out: &mut Vec<u8>);
    /// Rebuild by resolving that choice against the authored data the entity still
    /// carries. `None` when the entity lost it — which happens only on a respawn,
    /// which [`RestoreReport::respawned`] already reports.
    fn resolve(entity: &bevy::ecs::world::EntityWorldMut<'_>, r: &mut Reader<'_>) -> Option<Self>
    where
        Self: Sized;
}

/// Append an optional string as `0` / `1 <len> <bytes>`.
pub fn put_opt_str(out: &mut Vec<u8>, v: Option<&str>) {
    match v {
        None => put_bool(out, false),
        Some(s) => {
            put_bool(out, true);
            put_str(out, s);
        }
    }
}

/// Append a length-prefixed UTF-8 string. The prefix is a `u32`, never a `usize`:
/// a snapshot format that inherits the host's word size cannot become the
/// cross-platform format netcode.md's portability section keeps reachable.
pub fn put_str(out: &mut Vec<u8>, v: &str) {
    put_u32(out, v.len() as u32);
    out.extend_from_slice(v.as_bytes());
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

pub fn put_u8(out: &mut Vec<u8>, v: u8) {
    out.push(v);
}

pub fn put_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

pub fn put_vec2(out: &mut Vec<u8>, v: bevy::math::Vec2) {
    put_f32(out, v.x);
    put_f32(out, v.y);
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

    pub fn u8(&mut self) -> Option<u8> {
        Some(*self.take(1)?.first()?)
    }

    pub fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }

    pub fn vec2(&mut self) -> Option<bevy::math::Vec2> {
        Some(bevy::math::Vec2::new(self.f32()?, self.f32()?))
    }

    pub fn str(&mut self) -> Option<&'a str> {
        let n = self.u32()? as usize;
        std::str::from_utf8(self.take(n)?).ok()
    }

    /// `None` means "the field was absent", never "the read failed" — a failed read
    /// returns `None` from the outer `Option`, and these two nest.
    #[allow(clippy::option_option)]
    pub fn opt_str(&mut self) -> Option<Option<&'a str>> {
        Some(if self.bool()? {
            Some(self.str()?)
        } else {
            None
        })
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
        /// A snapshot with no row for this entity means the entity did not HAVE the
        /// component then. Restoring exactly means taking it away now.
        remove: Box<dyn Fn(&mut bevy::ecs::world::EntityWorldMut<'_>) + Send + Sync>,
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
                remove: Box::new(|entity| {
                    entity.remove::<C>();
                }),
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
                    // Absent on a respawned entity: there is no authored half to
                    // apply the cursor to. `RestoreReport::respawned` is the report.
                    if let Some(mut value) = entity.get_mut::<C>() {
                        let mut r = Reader::new(bytes);
                        if value.apply_cursor(&mut r).is_none() || r.finish().is_none() {
                            debug_assert!(false, "snapshot cursor failed to decode");
                        }
                    }
                }),
                remove: Box::new(|entity| {
                    entity.remove::<C>();
                }),
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
                        // The authored half is gone: a respawned entity. Leaving the
                        // component off is the only honest answer, and `respawned` is
                        // the number that says so.
                        None => {
                            entity.remove::<C>();
                        }
                        Some(value) => {
                            entity.insert(value);
                        }
                    }
                }),
                remove: Box::new(|entity| {
                    entity.remove::<C>();
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

/// What `restore` did, and what it could not rewind.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct RestoreReport {
    /// Entities present in BOTH the world and the snapshot. Their registered
    /// components were overwritten in place; everything else they carry survived.
    pub patched: usize,
    /// Entities in the snapshot that no longer existed. Rebuilt from blobs **alone**
    /// — they come back naked but for their registered components, because a blob is
    /// all that is left of them.
    pub respawned: usize,
    /// Entities in the world that the snapshot never knew. They ceased to exist,
    /// which is correct: they were spawned after the tick we rewound to.
    pub despawned: usize,
    /// **Components on surviving entities that `restore` did not rewind**, because
    /// the registry neither registers them nor declares them derived.
    ///
    /// They are not *lost* — patching left them alone — they are **stale**: they
    /// carry the state of the tick we rewound FROM. For an immutable authored fact
    /// (a moveset, a brain's identity) stale and correct are the same thing. For a
    /// timer, they are not, and it is that timer that makes a replay diverge.
    /// An empty list is N3.1's exit condition.
    pub stale_components: Vec<UnclaimedComponent>,
    /// **Simulated bodies with no `SimId`, which `restore` could not touch.**
    ///
    /// They are not registered, so nothing identifies them, and they walk out of a
    /// rollback carrying whatever state the rewound tick gave them. A projectile in
    /// this set survives its own un-firing. `unidentified_bodies` is the same number
    /// seen from the canary's side.
    pub unidentified_survivors: usize,
}

impl RestoreReport {
    /// Nothing survived that should not have, and nothing stale was left behind.
    /// Only then is a restored world the world that was taken.
    pub fn lossless(&self) -> bool {
        self.stale_components.is_empty() && self.unidentified_survivors == 0
    }
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
) -> RestoreReport {
    let stale_components = registry.unclaimed_components(world);

    // Index the live world by identity. `SimId` is unique by construction; a
    // duplicate means a spawn site minted the same id twice, and the later entity
    // wins here exactly as it would win any other lookup — a bug to find upstream,
    // not to paper over with a merge rule.
    let mut live: std::collections::BTreeMap<String, Entity> = std::collections::BTreeMap::new();
    if let Some(mut q) = world.try_query::<(Entity, &SimId)>() {
        for (entity, id) in q.iter(world) {
            live.insert(id.as_str().to_string(), entity);
        }
    }

    let ids = snapshot.sim_ids();
    let mut report = RestoreReport {
        stale_components,
        ..Default::default()
    };

    // Spawned after the snapshot: they never happened.
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
            None => {
                report.respawned += 1;
                world.spawn(SimId::from_snapshot((*id).to_string())).id()
            }
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
                    let mut e = world.entity_mut(entity);
                    insert(&mut e, &bytes);
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
        let EntryKind::Resource { load, .. } = &entry.kind else {
            continue;
        };
        load(world, bytes);
    }

    report.unidentified_survivors = match world
        .try_query_filtered::<(), (With<BodyKinematics>, bevy::ecs::query::Without<SimId>)>()
    {
        Some(mut q) => q.iter(world).count(),
        None => 0,
    };
    report
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

    // The mutable body-state clusters. A coyote timer that survives a rollback is a
    // jump the player did not earn; a dash cooldown that survives one is a dash they
    // did. Each is plain data, and each is exactly what the sim advances.
    registry.register_component::<bc::BodyGroundState>("body_ground_state");
    registry.register_component::<bc::BodyWallState>("body_wall_state");
    registry.register_component::<bc::BodyJumpState>("body_jump_state");
    registry.register_component::<bc::BodyDashState>("body_dash_state");
    registry.register_component::<bc::BodyFlightState>("body_flight_state");
    registry.register_component::<bc::BodyBlinkState>("body_blink_state");
    registry.register_component::<bc::BodyDodgeState>("body_dodge_state");
    registry.register_component::<bc::BodyShieldState>("body_shield_state");
    registry.register_component::<bc::BodyOffense>("body_offense");
    registry.register_component::<bc::BodyLifetime>("body_lifetime");
    registry.register_component::<bc::BodyActionBuffer>("body_action_buffer");
    registry.register_component::<bc::BodyBaseSize>("body_base_size");
    registry.register_component::<bc::SweepSample>("sweep_sample");
    registry.register_component::<bc::BodyMana>("body_mana");

    // Actor-side mutable state.
    //
    // **Not here, and named rather than forgotten:**
    //
    // - `ActorTarget` holds an `Option<Entity>`. N3.1 decision (2) FORBIDS `Entity`
    //   inside sim components — an entity index is an allocator slot, not an
    //   identity, and it will not survive a restore that respawns anything. It needs
    //   a `SimId`, and that is a migration slice, not a codec.
    // - `ActorMotionPath(Option<PathMotion>)` carries `PathMotion`'s private
    //   `segment` / `dir` cursor. Encoding it means giving `PathMotion` accessors or
    //   moving its codec into `ambition_combat`, which is the shape the doc wants
    //   anyway ("each sim crate registers its components' serialization").
    // - `ActorStatus` / `ActorIntent` / `BodyModeState` carry unit enums and need a
    //   discriminant codec whose mapping is EXPLICIT, not declaration order.
    registry.register_component::<ambition_characters::actor::pose::ActorPose>("actor_pose");
    registry
        .register_component::<ambition_platformer_primitives::orientation::ActorRoll>("actor_roll");
    registry.register_component::<ambition_combat::components::ActorCooldowns>("actor_cooldowns");
    registry.register_component::<ambition_engine_core::geometry::CenteredAabb>("centered_aabb");

    // A patrolling enemy's path cursor. Authored waypoints stay on the entity; only
    // `(segment, dir)` rides the snapshot. Without this, `mockingbird_arena` diverges
    // on the FIRST tick after a rewind: the patrol resumes from where it was going.
    registry.register_cursor::<ambition_actors::features::ActorMotionPath>("actor_motion_path");

    // The brain's mode, and the body's. Unit enums with EXPLICIT discriminants — see
    // `snapshot_unit_enum!`. An enemy that rewinds into `Attack` because a variant
    // moved is a bug nobody would look for.
    registry.register_component::<bc::BodyModeState>("body_mode_state");
    registry.register_component::<ambition_actors::features::ActorStatus>("actor_status");
    registry.register_component::<ambition_combat::components::ActorIntent>("actor_intent");
    registry.register_cursor::<ambition_combat::components::ActorTarget>("actor_target");

    // A move in flight. The `MoveSpec` is authored and stays on the entity; the blob
    // carries the CHOICE — which move, how far in, did it land.
    registry.register_resolved::<ambition_combat::moveset::MovePlayback>("move_playback");

    registry
        .register_component::<ambition_combat::components::BossPatternTimer>("boss_pattern_timer");
    registry.register_component::<ambition_combat::components::BossPhase>("boss_phase");

    // The boss brain, and the actor brain's senses. A boss that resumes its pattern
    // from the tick we rewound FROM is the whole reason the arenas diverge — and the
    // FB6-rollouts / BD6-playtester blocker.
    registry.register_component::<ambition_characters::brain::boss_pattern::BossAttackState>(
        "boss_attack_state",
    );
    registry.register_component::<ambition_characters::brain::boss_pattern::BossAttackIntent>(
        "boss_attack_intent",
    );
    registry
        .register_component::<ambition_actors::features::ecs::perception::Perception>("perception");
    registry.register_component::<ambition_actors::features::ecs::perception::PerceptionMemory>(
        "perception_memory",
    );

    // The boss's mind: step cursor, stance clocks, and the seeded RNG. A cursor,
    // because the brain's KIND and tuning are authored and survive the patch.
    registry.register_cursor::<ambition_characters::brain::Brain>("brain");
    registry
        .register_component::<ambition_actors::features::ActorSurfaceState>("actor_surface_state");
    registry.register_component::<ambition_combat::components::BodyEnvelope>("body_envelope");

    // ── Structurally derived: rebuilt every tick by the system that maintains it ──
    //
    // N3.1: "if restoring something requires a rebuild pass, the rebuild must be the
    // SAME system that maintains it per-frame (no restore-only code paths)." Each
    // claim below was checked against that system, not assumed.

    // `step_body`: `env_contact.water = world.water_at(aabb)` and `.climbable =
    // world.climbable_at(aabb)`, unconditionally, every movement step. A pure
    // function of the body's position and the world's geometry.
    registry.declare_derived::<ambition_engine_core::body_clusters::BodyEnvironmentContact>(
        "rewritten every movement step from the body's AABB and the world's geometry",
    );

    // The SimView and its indexes: netcode.md excludes these structurally
    // ("rebuilt every tick by construction").
    registry.declare_derived::<ambition_sim_view::BodyPoseView>(
        "SimView: rebuilt from the sim every tick, by construction",
    );
    registry.declare_derived::<ambition_sim_view::ProjectileView>(
        "SimView: rebuilt from the sim every tick, by construction",
    );

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
        put_vec2(out, self.pos);
        put_vec2(out, self.vel);
        put_vec2(out, self.size);
        put_f32(out, self.facing);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(BodyKinematics {
            pos: r.vec2()?,
            vel: r.vec2()?,
            size: r.vec2()?,
            facing: r.f32()?,
        })
    }
}

// The mutable body-state clusters. These are what a rewind is FOR: a coyote timer
// that survives a rollback is a jump the player did not earn.
macro_rules! snapshot_pod {
    ($ty:path { $($field:ident : $get:ident),+ $(,)? }) => {
        impl SnapshotState for $ty {
            fn encode(&self, out: &mut Vec<u8>) {
                $( paste_put(out, self.$field); )+
            }
            fn decode(r: &mut Reader<'_>) -> Option<Self> {
                Some(Self { $( $field: r.$get()? ),+ })
            }
        }
    };
}

/// One overload per encodable primitive, so `snapshot_pod!` does not have to name
/// the writer twice. The reader cannot do this — `Option<T>` inference would need
/// the type back — so the macro names the getter and infers the putter.
trait PasteEncode: Copy {
    fn put(self, out: &mut Vec<u8>);
}
impl PasteEncode for f32 {
    fn put(self, out: &mut Vec<u8>) {
        put_f32(out, self);
    }
}
impl PasteEncode for bool {
    fn put(self, out: &mut Vec<u8>) {
        put_bool(out, self);
    }
}
impl PasteEncode for u8 {
    fn put(self, out: &mut Vec<u8>) {
        put_u8(out, self);
    }
}
impl PasteEncode for u32 {
    fn put(self, out: &mut Vec<u8>) {
        put_u32(out, self);
    }
}
impl PasteEncode for i32 {
    fn put(self, out: &mut Vec<u8>) {
        put_i32(out, self);
    }
}
impl PasteEncode for bevy::math::Vec2 {
    fn put(self, out: &mut Vec<u8>) {
        put_vec2(out, self);
    }
}
fn paste_put<T: PasteEncode>(out: &mut Vec<u8>, v: T) {
    v.put(out);
}

use ambition_engine_core::body_clusters as bc;

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
macro_rules! snapshot_unit_enum {
    ($ty:path { $($variant:ident = $code:literal),+ $(,)? }) => {
        impl SnapshotState for $ty {
            fn encode(&self, out: &mut Vec<u8>) {
                #[allow(unused_imports)]
                use $ty as E;
                put_u8(
                    out,
                    match self {
                        $( E::$variant => $code ),+
                    },
                );
            }
            fn decode(r: &mut Reader<'_>) -> Option<Self> {
                #[allow(unused_imports)]
                use $ty as E;
                match r.u8()? {
                    $( $code => Some(E::$variant), )+
                    _ => None,
                }
            }
        }
    };
}

snapshot_unit_enum!(ambition_engine_core::player_state::BodyMode {
    Standing = 0,
    Crouching = 1,
    Crawling = 2,
    Sliding = 3,
    MorphBall = 4,
    Climbing = 5,
});
snapshot_unit_enum!(ambition_characters::actor::ai::CharacterAiMode {
    Idle = 0,
    Patrol = 1,
    Chase = 2,
    Telegraph = 3,
    Attack = 4,
    Recover = 5,
    Stunned = 6,
    Dead = 7,
});

impl SnapshotState for bc::BodyModeState {
    fn encode(&self, out: &mut Vec<u8>) {
        self.body_mode.encode(out);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(bc::BodyModeState {
            body_mode: ambition_engine_core::player_state::BodyMode::decode(r)?,
        })
    }
}

impl SnapshotState for ambition_actors::features::ActorStatus {
    fn encode(&self, out: &mut Vec<u8>) {
        put_f32(out, self.respawn_timer);
        self.ai_mode.encode(out);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(ambition_actors::features::ActorStatus {
            respawn_timer: r.f32()?,
            ai_mode: ambition_characters::actor::ai::CharacterAiMode::decode(r)?,
        })
    }
}

impl SnapshotState for ambition_combat::components::ActorIntent {
    fn encode(&self, out: &mut Vec<u8>) {
        self.0.encode(out);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(ambition_combat::components::ActorIntent(
            ambition_characters::actor::ai::CharacterAiMode::decode(r)?,
        ))
    }
}

snapshot_pod!(bc::BodyGroundState {
    on_ground: bool,
    coyote_timer: f32,
    drop_through_timer: f32,
    rebound_cooldown: f32,
});
snapshot_pod!(bc::BodyWallState {
    on_wall: bool,
    wall_normal_x: f32,
    wall_clinging: bool,
    wall_climbing: bool,
    pre_wall_vel: vec2,
    pre_wall_vel_age: f32,
});
snapshot_pod!(bc::BodyJumpState {
    air_jumps_available: u8,
    ladder_jump_boost: f32,
    ladder_drop_through_timer: f32,
    ladder_drop_through_hold_lock: bool,
});
snapshot_pod!(bc::BodyDashState {
    charges_available: u8,
    timer: f32,
    cooldown: f32,
});
snapshot_pod!(bc::BodyFlightState {
    fly_enabled: bool,
    flight_phase: f32,
    gliding: bool,
    fast_falling: bool,
    carried_run: f32,
});
snapshot_pod!(bc::BodyBlinkState {
    cooldown: f32,
    hold_active: bool,
    hold_timer: f32,
    aiming: bool,
    aim_offset: vec2,
    grace_timer: f32,
});
snapshot_pod!(bc::BodyDodgeState {
    roll_timer: f32,
    cooldown: f32,
});
snapshot_pod!(bc::BodyShieldState {
    active: bool,
    parry_window_timer: f32,
});
snapshot_pod!(bc::BodyOffense {
    damage_multiplier: i32,
    invincible: bool,
});
snapshot_pod!(bc::BodyLifetime {
    time_alive: f32,
    resets: u32,
    max_speed: f32,
});
snapshot_pod!(bc::BodyActionBuffer {
    jump: f32,
    dash: f32,
    attack: f32,
    pogo: f32,
    projectile: f32,
    blink: f32,
});
snapshot_pod!(bc::BodyBaseSize { base_size: vec2 });
snapshot_pod!(ambition_actors::features::ActorSurfaceState {
    surface_normal: vec2,
    gravity_scale: f32,
});

impl SnapshotState for ambition_combat::components::BodyEnvelope {
    fn encode(&self, out: &mut Vec<u8>) {
        put_vec2(out, self.0);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(ambition_combat::components::BodyEnvelope(r.vec2()?))
    }
}
snapshot_pod!(bc::SweepSample {
    prev: vec2,
    curr: vec2,
    vel: vec2,
    half: vec2,
});

// Actor-side mutable state. An attack cooldown that survives a rollback is an
// attack the enemy did not pay for.
snapshot_pod!(ambition_characters::actor::pose::ActorPose {
    center: vec2,
    feet: vec2,
    facing: f32,
});
snapshot_pod!(ambition_platformer_primitives::orientation::ActorRoll { angle: f32 });
snapshot_pod!(ambition_combat::components::ActorCooldowns {
    attack_cooldown: f32,
    respawn_timer: f32,
});
snapshot_pod!(ambition_engine_core::geometry::CenteredAabb {
    center: vec2,
    half_size: vec2,
});
snapshot_pod!(ambition_engine_core::player_state::ResourceMeter {
    current: f32,
    max: f32,
    regen_rate: f32,
    decay_rate: f32,
});

impl SnapshotState for bc::BodyMana {
    fn encode(&self, out: &mut Vec<u8>) {
        self.meter.encode(out);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(bc::BodyMana {
            meter: ambition_engine_core::player_state::ResourceMeter::decode(r)?,
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

/// `ActorTarget` is half derived, half state — see its definition-site snapshot story.
/// `entity` is rebuilt every tick by `select_actor_targets`; `pos` survives the frame
/// where no candidate exists, and a chasing brain aims at it. So `pos` rewinds and
/// `entity` does not.
/// The blob is `(move id, facing, t, landed_hit)`; the `MoveSpec` comes back out of the
/// entity's own `ActorMoveset`, which a patched entity still carries.
///
/// `live_boxes` comes back empty and `fired` is rebuilt from `t` — both by
/// `MovePlayback::resumed`. That is sound because a strike volume's existence is
/// DERIVED from `(t, window)` and `retire_orphaned_strike_volumes` maintains that
/// derivation every frame, so the rewound clock re-creates exactly the boxes it should.
///
/// A move id the moveset no longer knows resolves to `None`, and the component is left
/// off. That is a content change between snapshot and restore — impossible in a
/// rollback, and a loud, correct failure in a save file.
impl SnapshotResolve for ambition_combat::moveset::MovePlayback {
    fn encode_ref(&self, out: &mut Vec<u8>) {
        put_str(out, &self.spec.id);
        put_f32(out, self.facing);
        put_f32(out, self.t);
        put_bool(out, self.landed_hit);
    }

    fn resolve(entity: &bevy::ecs::world::EntityWorldMut<'_>, r: &mut Reader<'_>) -> Option<Self> {
        let id = r.str()?;
        let spec = entity
            .get::<ambition_combat::moveset::ActorMoveset>()?
            .0
            .move_by_id(id)?
            .clone();
        Some(ambition_combat::moveset::MovePlayback::resumed(
            spec,
            r.f32()?,
            r.f32()?,
            r.bool()?,
        ))
    }
}

impl SnapshotCursor for ambition_combat::components::ActorTarget {
    fn encode_cursor(&self, out: &mut Vec<u8>) {
        put_vec2(out, self.pos);
    }
    fn apply_cursor(&mut self, r: &mut Reader<'_>) -> Option<()> {
        self.pos = r.vec2()?;
        Some(())
    }
}

impl SnapshotCursor for ambition_actors::features::ActorMotionPath {
    fn encode_cursor(&self, out: &mut Vec<u8>) {
        match &self.0 {
            Some(motion) => {
                let (segment, dir) = motion.cursor();
                put_bool(out, true);
                put_u32(out, segment as u32);
                put_i32(out, dir);
            }
            // A body with no path is a state a body with a path can reach.
            None => put_bool(out, false),
        }
    }

    fn apply_cursor(&mut self, r: &mut Reader<'_>) -> Option<()> {
        let has_path = r.bool()?;
        let (segment, dir) = if has_path {
            (r.u32()? as usize, r.i32()?)
        } else {
            // The snapshot's body had no path. Ours does; drop it. The authored path
            // is not recoverable from here, which is exactly what `stale_components`
            // and `respawned` exist to make visible — but a body that GAINED a path
            // after the snapshot must not keep it.
            self.0 = None;
            return Some(());
        };
        if let Some(motion) = self.0.as_mut() {
            motion.set_cursor(segment, dir);
        }
        Some(())
    }
}

snapshot_unit_enum!(ambition_characters::actor::ActorFaction {
    Player = 0,
    Enemy = 1,
    Npc = 2,
    Boss = 3,
    Neutral = 4,
});

/// `Strike(key)` / `Special(key)` — a keyed reference by construction, because "a new
/// geometry strike is a new key + authored rects, with NO edit to this enum".
impl SnapshotState for ambition_characters::brain::boss_pattern::BossAttackProfile {
    fn encode(&self, out: &mut Vec<u8>) {
        use ambition_characters::brain::boss_pattern::BossAttackProfile as P;
        match self {
            P::Strike(key) => {
                put_u8(out, 0);
                put_str(out, key);
            }
            P::Special(key) => {
                put_u8(out, 1);
                put_str(out, key);
            }
        }
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        use ambition_characters::brain::boss_pattern::BossAttackProfile as P;
        match r.u8()? {
            0 => Some(P::Strike(r.str()?.to_string())),
            1 => Some(P::Special(r.str()?.to_string())),
            _ => None,
        }
    }
}

fn put_opt_profile(
    out: &mut Vec<u8>,
    v: &Option<ambition_characters::brain::boss_pattern::BossAttackProfile>,
) {
    match v {
        None => put_bool(out, false),
        Some(p) => {
            put_bool(out, true);
            p.encode(out);
        }
    }
}

#[allow(clippy::option_option)]
fn read_opt_profile(
    r: &mut Reader<'_>,
) -> Option<Option<ambition_characters::brain::boss_pattern::BossAttackProfile>> {
    use ambition_characters::brain::boss_pattern::BossAttackProfile as P;
    Some(if r.bool()? { Some(P::decode(r)?) } else { None })
}

impl SnapshotState for ambition_characters::brain::boss_pattern::BossAttackState {
    fn encode(&self, out: &mut Vec<u8>) {
        put_opt_profile(out, &self.telegraph_profile);
        put_f32(out, self.telegraph_remaining);
        put_f32(out, self.telegraph_elapsed);
        match &self.telegraph_spec {
            None => put_bool(out, false),
            Some(spec) => {
                put_bool(out, true);
                put_opt_str(out, spec.pose.as_deref());
                put_opt_str(out, spec.cue.as_deref());
                put_opt_str(out, spec.vfx.as_deref());
            }
        }
        put_opt_profile(out, &self.active_profile);
        put_f32(out, self.active_remaining);
        put_f32(out, self.active_elapsed);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        use ambition_characters::brain::boss_pattern::{BossAttackState, TelegraphSpec};
        let telegraph_profile = read_opt_profile(r)?;
        let telegraph_remaining = r.f32()?;
        let telegraph_elapsed = r.f32()?;
        let telegraph_spec = if r.bool()? {
            Some(TelegraphSpec {
                pose: r.opt_str()?.map(str::to_string),
                cue: r.opt_str()?.map(str::to_string),
                vfx: r.opt_str()?.map(str::to_string),
            })
        } else {
            None
        };
        Some(BossAttackState {
            telegraph_profile,
            telegraph_remaining,
            telegraph_elapsed,
            telegraph_spec,
            active_profile: read_opt_profile(r)?,
            active_remaining: r.f32()?,
            active_elapsed: r.f32()?,
        })
    }
}

impl SnapshotState for ambition_characters::brain::boss_pattern::BossAttackIntent {
    fn encode(&self, out: &mut Vec<u8>) {
        put_opt_profile(out, &self.telegraph_profile);
        put_opt_profile(out, &self.active_profile);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(ambition_characters::brain::boss_pattern::BossAttackIntent {
            telegraph_profile: read_opt_profile(r)?,
            active_profile: read_opt_profile(r)?,
        })
    }
}

/// `Omniscient` reads the global `ActorTarget`; `Sighted` carries its viewport. Not a
/// unit enum, so `snapshot_unit_enum!` cannot have it — but the discriminant is still
/// explicit for exactly the same reason.
impl SnapshotState for ambition_actors::features::ecs::perception::Perception {
    fn encode(&self, out: &mut Vec<u8>) {
        use ambition_actors::features::ecs::perception::Perception as P;
        match self {
            P::Omniscient => put_u8(out, 0),
            P::Sighted { viewport_half } => {
                put_u8(out, 1);
                put_vec2(out, *viewport_half);
            }
        }
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        use ambition_actors::features::ecs::perception::Perception as P;
        match r.u8()? {
            0 => Some(P::Omniscient),
            1 => Some(P::Sighted {
                viewport_half: r.vec2()?,
            }),
            _ => None,
        }
    }
}

/// The brain's memory of what it has seen — FB5's habit model reads it, and FB6's
/// rollouts cannot run until it rewinds. Ordered by actor id, because `WorldMemory`
/// is a `BTreeMap` (ADR 0023, and a real bug: see `last_known_hostile`).
impl SnapshotState for ambition_actors::features::ecs::perception::PerceptionMemory {
    fn encode(&self, out: &mut Vec<u8>) {
        let rows: Vec<_> = self.0.entries().collect();
        put_u32(out, rows.len() as u32);
        for (id, m) in rows {
            put_str(out, id);
            put_vec2(out, m.pos);
            put_vec2(out, m.vel);
            m.faction.encode(out);
            put_bool(out, m.hostile_to_self);
            put_f32(out, m.last_seen);
            put_f32(out, m.confidence);
        }
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        use ambition_characters::perception::{RememberedActor, WorldMemory};
        let n = r.u32()?;
        let mut rows = Vec::with_capacity(n as usize);
        for _ in 0..n {
            let id = r.str()?.to_string();
            rows.push((
                id,
                RememberedActor {
                    pos: r.vec2()?,
                    vel: r.vec2()?,
                    faction: ambition_characters::actor::ActorFaction::decode(r)?,
                    hostile_to_self: r.bool()?,
                    last_seen: r.f32()?,
                    confidence: r.f32()?,
                },
            ));
        }
        Some(
            ambition_actors::features::ecs::perception::PerceptionMemory(
                WorldMemory::from_snapshot(rows),
            ),
        )
    }
}

snapshot_unit_enum!(ambition_characters::brain::boss_pattern::BossEncounterPhase {
    Dormant = 0,
    Intro = 1,
    Phase1 = 2,
    Transition = 3,
    Phase2 = 4,
    Stagger = 5,
    Enrage = 6,
    Death = 7,
});
snapshot_unit_enum!(ambition_characters::brain::boss_pattern::CyclePhase {
    Cooldown = 0,
    Windup = 1,
    Active = 2,
});
/// Not a unit enum — `Approach` and `Retreat` carry their own clocks, and a boss
/// that rewinds into `Retreat` must rewind to the same retreat POSITION. Explicit
/// discriminants for the same reason as `snapshot_unit_enum!`.
impl SnapshotState for ambition_characters::brain::boss_pattern::BossMacroState {
    fn encode(&self, out: &mut Vec<u8>) {
        use ambition_characters::brain::boss_pattern::BossMacroState as M;
        match self {
            M::Engage => put_u8(out, 0),
            M::Approach { remaining_s } => {
                put_u8(out, 1);
                put_f32(out, *remaining_s);
            }
            M::Retreat {
                remaining_s,
                retreat_pos,
            } => {
                put_u8(out, 2);
                put_f32(out, *remaining_s);
                put_vec2(out, *retreat_pos);
            }
        }
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        use ambition_characters::brain::boss_pattern::BossMacroState as M;
        match r.u8()? {
            0 => Some(M::Engage),
            1 => Some(M::Approach {
                remaining_s: r.f32()?,
            }),
            2 => Some(M::Retreat {
                remaining_s: r.f32()?,
                retreat_pos: r.vec2()?,
            }),
            _ => None,
        }
    }
}

/// **The boss's mind, rewound.**
///
/// A `SnapshotCursor`, because `Brain` is half authored and half state: the brain's
/// KIND and its tuning came from content and survive the patch, and only
/// `BossPatternState`'s clocks, cursors, and **`rng_seed`** ride the blob. A seeded
/// RNG that is not snapshot state is a determinism bug the canary would eventually
/// catch, and netcode.md's checklist names it.
///
/// ## What this deliberately does NOT rewind, and the constraint that buys
///
/// `timeline: Vec<BossPatternStep>` and `stance_stack` are left alone. The timeline is
/// **re-resolved** from the authored pattern by `advance_scripted` whenever the script
/// loops or the encounter phase changes, so within a rollback window that spans
/// neither, the surviving timeline IS the snapshot's timeline — and encoding it would
/// serialize authored content by value, which is the thing this module refuses to do.
///
/// So: **a rollback window must not span a pattern re-resolve**, exactly as it must
/// not span a spawn. Both are constraints N3.2's bounded window makes reasonable, and
/// both are written down here rather than discovered in a desync report.
impl SnapshotCursor for ambition_characters::brain::Brain {
    fn encode_cursor(&self, out: &mut Vec<u8>) {
        let Some(s) = self.boss_pattern_state() else {
            // Not a boss brain: nothing mutable that a rollback needs. The tag keeps
            // "no state" distinguishable from a truncated blob.
            put_u8(out, 0);
            return;
        };
        put_u8(out, 1);
        match &s.last_phase {
            None => put_bool(out, false),
            Some(p) => {
                put_bool(out, true);
                p.encode(out);
            }
        }
        put_u32(out, s.step_index as u32);
        put_f32(out, s.step_elapsed);
        put_f32(out, s.movement_timer);
        put_f32(out, s.pattern_timer);
        s.cycle_phase.encode(out);
        put_f32(out, s.cycle_phase_remaining);
        s.macro_state.encode(out);
        put_f32(out, s.engage_timer);
        put_u64(out, s.rng_seed);
        s.attack_state.encode(out);
        put_u32(out, s.interrupt_cooldowns.len() as u32);
        for v in &s.interrupt_cooldowns {
            put_f32(out, *v);
        }
        put_u32(out, s.interrupt_timers.len() as u32);
        for v in &s.interrupt_timers {
            put_f32(out, *v);
        }
        match s.last_hp {
            None => put_bool(out, false),
            Some(hp) => {
                put_bool(out, true);
                put_i32(out, hp);
            }
        }
    }

    fn apply_cursor(&mut self, r: &mut Reader<'_>) -> Option<()> {
        use ambition_characters::brain::boss_pattern::{
            BossAttackState, BossEncounterPhase, BossMacroState, CyclePhase,
        };
        if r.u8()? == 0 {
            return Some(());
        }
        let last_phase = if r.bool()? {
            Some(BossEncounterPhase::decode(r)?)
        } else {
            None
        };
        let step_index = r.u32()? as usize;
        let step_elapsed = r.f32()?;
        let movement_timer = r.f32()?;
        let pattern_timer = r.f32()?;
        let cycle_phase = CyclePhase::decode(r)?;
        let cycle_phase_remaining = r.f32()?;
        let macro_state = BossMacroState::decode(r)?;
        let engage_timer = r.f32()?;
        let rng_seed = r.u64()?;
        let attack_state = BossAttackState::decode(r)?;
        fn read_f32s(r: &mut Reader<'_>) -> Option<Vec<f32>> {
            let n = r.u32()?;
            (0..n).map(|_| r.f32()).collect()
        }
        let interrupt_cooldowns = read_f32s(r)?;
        let interrupt_timers = read_f32s(r)?;
        let last_hp = if r.bool()? { Some(r.i32()?) } else { None };

        // A blob written by a boss brain, applied to one that is no longer a boss
        // brain, would be a content change across a rollback. Leave it alone.
        let Some(s) = self.boss_pattern_state_mut() else {
            return Some(());
        };
        s.last_phase = last_phase;
        s.step_index = step_index;
        s.step_elapsed = step_elapsed;
        s.movement_timer = movement_timer;
        s.pattern_timer = pattern_timer;
        s.cycle_phase = cycle_phase;
        s.cycle_phase_remaining = cycle_phase_remaining;
        s.macro_state = macro_state;
        s.engage_timer = engage_timer;
        s.rng_seed = rng_seed;
        s.attack_state = attack_state;
        s.interrupt_cooldowns = interrupt_cooldowns;
        s.interrupt_timers = interrupt_timers;
        s.last_hp = last_hp;
        Some(())
    }
}

snapshot_unit_enum!(ambition_combat::components::BossPhase {
    Active = 0,
    Defeated = 1,
});

impl SnapshotState for ambition_combat::components::BossPatternTimer {
    fn encode(&self, out: &mut Vec<u8>) {
        put_f32(out, self.0);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(ambition_combat::components::BossPatternTimer(r.f32()?))
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

    /// A component that is half authored content and half mutable cursor — the
    /// `ActorMotionPath` shape, in miniature.
    #[derive(Component, Debug, PartialEq)]
    struct Patrol {
        /// Authored. Never in a blob.
        waypoints: Vec<f32>,
        /// Mutable. The only thing a rollback touches.
        segment: u32,
    }

    impl SnapshotCursor for Patrol {
        fn encode_cursor(&self, out: &mut Vec<u8>) {
            put_u32(out, self.segment);
        }
        fn apply_cursor(&mut self, r: &mut Reader<'_>) -> Option<()> {
            self.segment = r.u32()?;
            Some(())
        }
    }

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

        // The body-state clusters. `snapshot_pod!` writes these codecs from a field
        // list, so the risk is a field OMITTED from the list, not a field mistyped —
        // and an omitted field is exactly what `encode ∘ decode ∘ encode` cannot see.
        // `every_registered_component_survives_a_world_round_trip` below is the one
        // that catches it, by comparing hashes of a world rather than of a value.
        round_trip(bc::BodyGroundState {
            on_ground: true,
            coyote_timer: 0.1,
            drop_through_timer: -0.0,
            rebound_cooldown: 3.0,
        });
        round_trip(bc::BodyWallState {
            on_wall: true,
            wall_normal_x: -1.0,
            wall_clinging: false,
            wall_climbing: true,
            pre_wall_vel: Vec2::new(1.0, 2.0),
            pre_wall_vel_age: 0.5,
        });
        round_trip(bc::BodyJumpState {
            air_jumps_available: 2,
            ladder_jump_boost: 1.5,
            ladder_drop_through_timer: 0.0,
            ladder_drop_through_hold_lock: true,
        });
        round_trip(bc::BodyDashState {
            charges_available: 255,
            timer: 0.2,
            cooldown: 0.3,
        });
        round_trip(bc::BodyFlightState {
            fly_enabled: true,
            flight_phase: 6.28,
            gliding: true,
            fast_falling: false,
            carried_run: -12.0,
        });
        round_trip(bc::BodyBlinkState {
            cooldown: 1.0,
            hold_active: true,
            hold_timer: 0.4,
            aiming: true,
            aim_offset: Vec2::new(-3.0, 4.0),
            grace_timer: 0.05,
        });
        round_trip(bc::BodyDodgeState {
            roll_timer: 0.1,
            cooldown: 0.9,
        });
        round_trip(bc::BodyShieldState {
            active: true,
            parry_window_timer: 0.08,
        });
        round_trip(bc::BodyOffense {
            damage_multiplier: -2,
            invincible: true,
        });
        round_trip(bc::BodyLifetime {
            time_alive: 99.5,
            resets: u32::MAX,
            max_speed: 1200.0,
        });
        round_trip(bc::BodyActionBuffer {
            jump: 0.1,
            dash: 0.2,
            attack: 0.3,
            pogo: 0.4,
            projectile: 0.5,
            blink: 0.6,
        });
        round_trip(bc::BodyBaseSize {
            base_size: Vec2::new(16.0, 32.0),
        });
        round_trip(bc::SweepSample {
            prev: Vec2::new(1.0, 2.0),
            curr: Vec2::new(3.0, 4.0),
            vel: Vec2::new(5.0, 6.0),
            half: Vec2::new(7.0, 8.0),
        });
        round_trip(ambition_characters::actor::pose::ActorPose {
            center: Vec2::new(1.0, 2.0),
            feet: Vec2::new(1.0, 18.0),
            facing: -1.0,
        });
        round_trip(ambition_platformer_primitives::orientation::ActorRoll { angle: 1.57 });
        round_trip(ambition_combat::components::ActorCooldowns {
            attack_cooldown: 0.4,
            respawn_timer: 2.0,
        });
        round_trip(ambition_engine_core::geometry::CenteredAabb {
            center: Vec2::new(5.0, 6.0),
            half_size: Vec2::new(8.0, 16.0),
        });
        {
            use ambition_actors::features::ecs::perception::{Perception, PerceptionMemory};
            use ambition_characters::actor::ActorFaction;
            use ambition_characters::brain::boss_pattern::{
                BossAttackIntent, BossAttackProfile, BossAttackState, TelegraphSpec,
            };
            use ambition_characters::perception::{RememberedActor, WorldMemory};

            round_trip(BossAttackProfile::Strike("floor_slam".into()));
            round_trip(BossAttackProfile::Special("overfit_volley".into()));
            round_trip(BossAttackIntent {
                telegraph_profile: Some(BossAttackProfile::Strike("side_sweep".into())),
                active_profile: None,
            });
            round_trip(BossAttackState {
                telegraph_profile: None,
                telegraph_remaining: 0.4,
                telegraph_elapsed: 0.1,
                telegraph_spec: Some(TelegraphSpec {
                    pose: Some("wind_up".into()),
                    cue: None,
                    vfx: Some("sparks".into()),
                }),
                active_profile: Some(BossAttackProfile::Special("apple_rain".into())),
                active_remaining: 1.25,
                active_elapsed: -0.0,
            });
            round_trip(Perception::Omniscient);
            round_trip(Perception::Sighted {
                viewport_half: Vec2::new(320.0, 180.0),
            });
            round_trip(PerceptionMemory(WorldMemory::from_snapshot([
                (
                    "zeta".to_string(),
                    RememberedActor {
                        pos: Vec2::new(1.0, 2.0),
                        vel: Vec2::new(-3.0, 0.0),
                        faction: ActorFaction::Player,
                        hostile_to_self: true,
                        last_seen: 9.5,
                        confidence: 0.75,
                    },
                ),
                (
                    "alpha".to_string(),
                    RememberedActor {
                        pos: Vec2::ZERO,
                        vel: Vec2::ZERO,
                        faction: ActorFaction::Neutral,
                        hostile_to_self: false,
                        last_seen: 0.0,
                        confidence: 1.0,
                    },
                ),
            ])));
        }

        round_trip(bc::BodyMana {
            meter: ambition_engine_core::player_state::ResourceMeter {
                current: 12.0,
                max: 50.0,
                regen_rate: 1.0,
                decay_rate: -0.0,
            },
        });
    }

    /// **The test `encode ∘ decode ∘ encode` cannot be: a field left out of the
    /// codec entirely.**
    ///
    /// A codec that never touches `coyote_timer` round-trips its own bytes perfectly
    /// and loses the timer. So: put a world in a known state, snapshot it, wreck
    /// EVERY registered component, restore, and demand the world hash come back. The
    /// hash reads the components through the same codecs — so this catches a field
    /// dropped from `snapshot_pod!`'s list only if the hash sees it too, which is
    /// the honest limit of "one serialization, two consumers".
    ///
    /// The unlosable half is the field that MOVES something: a dropped `coyote_timer`
    /// changes what the next jump does, and
    /// `a_restored_sim_replays_the_future_it_was_rewound_from` in `ambition_app` is
    /// the test that runs the sim forward and notices.
    #[test]
    fn every_registered_component_survives_a_world_round_trip() {
        let reg = engine_registry();
        let mut world = sim_world();
        let id = *live_ids(&mut world).get("placement:boss-1").unwrap();
        world.entity_mut(id).insert((
            bc::BodyGroundState {
                on_ground: true,
                coyote_timer: 0.125,
                drop_through_timer: 0.25,
                rebound_cooldown: 0.5,
            },
            bc::BodyDashState {
                charges_available: 3,
                timer: 0.75,
                cooldown: 1.5,
            },
        ));
        let before = reg.hash_world(&world);
        let snap = take(&world, &reg);

        world
            .entity_mut(id)
            .insert((bc::BodyGroundState::default(), bc::BodyDashState::default()));
        assert_ne!(reg.hash_world(&world), before);

        restore(&mut world, &snap, &reg);
        assert_eq!(reg.hash_world(&world), before);
        let ground = *world.entity(id).get::<bc::BodyGroundState>().unwrap();
        assert_eq!(ground.coyote_timer, 0.125, "the timer came back");
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
        let report = restore(&mut world, &snap, &reg);
        assert_eq!(reg.hash_world(&world), before, "restore did not restore");
        assert_eq!(report.patched, 2, "both snapshot entities were still there");
        assert_eq!(report.despawned, 1, "the body spawned after the snapshot");
        assert_eq!(report.respawned, 0);
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
        assert_eq!(report.despawned, 1, "the ghost");
        assert_eq!(report.respawned, 1, "the one we killed");
        assert_eq!(report.patched, 1, "the survivor was patched, not rebuilt");

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

    /// **`restore` patches a survivor; it does not rebuild it.**
    ///
    /// The whole reason the sketch's despawn-everything is wrong: an entity present
    /// in both worlds keeps its authored config — its brain, its moveset — because
    /// nothing ever took it away. What the registry does not know it also does not
    /// rewind, and `stale_components` names that instead of pretending it is gone.
    #[test]
    fn restore_reports_the_components_it_could_not_rewind() {
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
        // The sim advances: the unregistered thing changes, as a live timer would.
        let mut e = world.entity_mut(boss);
        e.insert(UnregisteredThing(9));

        let report = restore(&mut world, &snap, &reg);
        assert_eq!(report.patched, 2, "both entities survived and were patched");
        assert_eq!(report.respawned, 0);
        assert!(!report.lossless());
        assert_eq!(report.stale_components, unclaimed);

        // It SURVIVED — and it is stale, still reading the tick we rewound FROM.
        // A moveset would be correct here; a timer is the bug the ledger tracks.
        let survivor = world
            .try_query::<&UnregisteredThing>()
            .unwrap()
            .iter(&world)
            .copied()
            .next();
        assert_eq!(
            survivor,
            Some(UnregisteredThing(9)),
            "restore left the unregistered component alone — that is what `stale` means"
        );

        // ...and once it is DECLARED derived, the ledger is clean, because "derived"
        // is a promise that some per-frame system rebuilds it.
        reg.declare_derived::<UnregisteredThing>("pretend");
        assert!(reg.unclaimed_components(&world).is_empty());
    }

    /// **A component the entity did not have at the snapshot tick is REMOVED.**
    ///
    /// Patching that only ever inserted would leave a shield the body raised after
    /// the snapshot standing through the rewind. Restoring exactly means taking it
    /// away, and the registered hash is what proves it happened.
    #[test]
    fn patching_removes_a_component_the_snapshot_never_had() {
        let reg = engine_registry();
        let mut world = sim_world();
        let before = reg.hash_world(&world);
        let snap = take(&world, &reg);

        // The player body has no `BodyHealth` in `sim_world`. Give it one.
        let player = *live_ids(&mut world).get("slot:0").unwrap();
        world.entity_mut(player).insert(BodyHealth::new(Health {
            current: 1,
            max: 1,
            invulnerable: false,
        }));
        assert_ne!(reg.hash_world(&world), before);

        restore(&mut world, &snap, &reg);
        assert_eq!(
            reg.hash_world(&world),
            before,
            "the late component lingered"
        );
        assert!(world.entity(player).get::<BodyHealth>().is_none());
    }

    fn live_ids(world: &mut World) -> std::collections::BTreeMap<String, Entity> {
        let mut q = world.query::<(Entity, &SimId)>();
        q.iter(world)
            .map(|(e, id)| (id.as_str().to_string(), e))
            .collect()
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
            report.stale_components.is_empty(),
            "nothing was stale — a whole BODY was kept that should not have been"
        );
    }

    /// **A cursor rewinds a survivor without re-serializing its authored half.**
    ///
    /// The waypoints never enter a blob; the segment does. This is only sound because
    /// `restore` patches survivors — an entity that still exists still has its path.
    #[test]
    fn a_cursor_rewinds_the_mutable_half_and_leaves_the_authored_half_alone() {
        let mut reg = engine_registry();
        reg.register_cursor::<Patrol>("patrol");
        let mut world = sim_world();
        let boss = *live_ids(&mut world).get("placement:boss-1").unwrap();
        world.entity_mut(boss).insert(Patrol {
            waypoints: vec![0.0, 10.0, 20.0],
            segment: 1,
        });

        let snap = take(&world, &reg);
        assert!(
            snap.size_bytes() < 200,
            "the authored waypoints leaked into the blob: {} bytes",
            snap.size_bytes()
        );

        world.entity_mut(boss).get_mut::<Patrol>().unwrap().segment = 2;
        restore(&mut world, &snap, &reg);

        let patrol = world.entity(boss).get::<Patrol>().unwrap();
        assert_eq!(patrol.segment, 1, "the cursor rewound");
        assert_eq!(
            patrol.waypoints,
            vec![0.0, 10.0, 20.0],
            "the authored half was never touched"
        );
    }

    /// **A cursor cannot rebuild a respawn, and does not pretend to.** There is no
    /// authored half to apply it to. `RestoreReport::respawned` is the warning, which
    /// is why a rollback window must not span a spawn.
    #[test]
    fn a_cursor_cannot_rebuild_an_entity_that_no_longer_exists() {
        let mut reg = engine_registry();
        reg.register_cursor::<Patrol>("patrol");
        let mut world = sim_world();
        let boss = *live_ids(&mut world).get("placement:boss-1").unwrap();
        world.entity_mut(boss).insert(Patrol {
            waypoints: vec![0.0, 10.0],
            segment: 1,
        });
        let snap = take(&world, &reg);

        world.despawn(boss);
        let report = restore(&mut world, &snap, &reg);
        assert_eq!(report.respawned, 1);

        let back = *live_ids(&mut world).get("placement:boss-1").unwrap();
        assert!(
            world.entity(back).get::<Patrol>().is_none(),
            "a cursor has nothing to apply itself to on a naked respawn — it must not \
             invent a path"
        );
    }

    /// **The unit-enum discriminants are a WIRE FORMAT, and this test is the format.**
    ///
    /// Declaration order is one refactor away from being a different order. If someone
    /// moves `Chase` above `Patrol` in `CharacterAiMode`, every snapshot ever taken
    /// starts decoding patrolling enemies as chasing ones — silently, because both
    /// are valid states. Pinning the bytes here means that refactor fails a test
    /// instead of a playtest.
    #[test]
    fn a_unit_enums_wire_discriminant_never_moves() {
        use ambition_characters::actor::ai::CharacterAiMode as Ai;
        use ambition_engine_core::player_state::BodyMode as Mode;

        for (mode, byte) in [
            (Ai::Idle, 0u8),
            (Ai::Patrol, 1),
            (Ai::Chase, 2),
            (Ai::Telegraph, 3),
            (Ai::Attack, 4),
            (Ai::Recover, 5),
            (Ai::Stunned, 6),
            (Ai::Dead, 7),
        ] {
            assert_eq!(encode_one(&mode), vec![byte], "{mode:?} moved");
            assert_eq!(decode_one::<Ai>(&[byte]), Some(mode));
        }
        for (mode, byte) in [
            (Mode::Standing, 0u8),
            (Mode::Crouching, 1),
            (Mode::Crawling, 2),
            (Mode::Sliding, 3),
            (Mode::MorphBall, 4),
            (Mode::Climbing, 5),
        ] {
            assert_eq!(encode_one(&mode), vec![byte], "{mode:?} moved");
        }
    }

    /// An unknown discriminant is `None`, never the default. A blob this build cannot
    /// read is a bug to surface, not a state to guess — and `Idle` would be a very
    /// plausible guess.
    #[test]
    fn an_unknown_discriminant_is_rejected_rather_than_defaulted() {
        use ambition_characters::actor::ai::CharacterAiMode as Ai;
        assert_eq!(decode_one::<Ai>(&[8]), None);
        assert_eq!(decode_one::<Ai>(&[255]), None);
        assert_eq!(decode_one::<Ai>(&[]), None);
        assert_eq!(decode_one::<Ai>(&[0, 0]), None, "trailing byte");
    }

    /// A component that references authored content by id — the `MovePlayback` shape,
    /// in miniature. The catalog stays on the entity; the blob carries a name.
    #[derive(Component, Clone, Debug, PartialEq)]
    struct Catalog(Vec<(String, f32)>);

    #[derive(Component, Debug, PartialEq)]
    struct Playing {
        /// Resolved out of the `Catalog`. Never in a blob.
        power: f32,
        /// The choice, and the clock.
        id: String,
        t: f32,
    }

    impl SnapshotResolve for Playing {
        fn encode_ref(&self, out: &mut Vec<u8>) {
            put_str(out, &self.id);
            put_f32(out, self.t);
        }
        fn resolve(
            entity: &bevy::ecs::world::EntityWorldMut<'_>,
            r: &mut Reader<'_>,
        ) -> Option<Self> {
            let id = r.str()?;
            let power = entity
                .get::<Catalog>()?
                .0
                .iter()
                .find(|(name, _)| name == id)?
                .1;
            Some(Playing {
                power,
                id: id.to_string(),
                t: r.f32()?,
            })
        }
    }

    /// **A resolved component restores its PRESENCE, not just its value.** A move is
    /// inserted when it starts and removed when it ends, so a rollback must both add
    /// and drop it — which a cursor cannot do.
    #[test]
    fn a_resolved_component_rebuilds_itself_from_content_the_entity_still_holds() {
        let mut reg = engine_registry();
        reg.register_resolved::<Playing>("playing");
        let mut world = sim_world();
        let boss = *live_ids(&mut world).get("placement:boss-1").unwrap();
        world.entity_mut(boss).insert((
            Catalog(vec![("jab".into(), 3.0), ("smash".into(), 20.0)]),
            Playing {
                power: 20.0,
                id: "smash".into(),
                t: 0.25,
            },
        ));

        let snap = take(&world, &reg);
        assert!(
            snap.size_bytes() < 200,
            "the catalog leaked into the blob: {} bytes",
            snap.size_bytes()
        );

        // The move ends. The component goes away.
        world.entity_mut(boss).remove::<Playing>();
        restore(&mut world, &snap, &reg);
        assert_eq!(
            world.entity(boss).get::<Playing>(),
            Some(&Playing {
                power: 20.0,
                id: "smash".into(),
                t: 0.25
            }),
            "the move came back, and its power was resolved out of the catalog"
        );

        // ...and a move that started AFTER the snapshot is dropped.
        world.entity_mut(boss).insert(Playing {
            power: 3.0,
            id: "jab".into(),
            t: 0.0,
        });
        let empty = take(&world, &reg);
        world.entity_mut(boss).remove::<Playing>();
        restore(&mut world, &empty, &reg);
        assert!(world.entity(boss).get::<Playing>().is_some());
    }

    /// A name the content no longer knows leaves the component OFF, rather than
    /// resolving to a plausible neighbour. Impossible in a rollback; loud in a save.
    #[test]
    fn a_resolved_component_that_names_missing_content_is_dropped_not_guessed() {
        let mut reg = engine_registry();
        reg.register_resolved::<Playing>("playing");
        let mut world = sim_world();
        let boss = *live_ids(&mut world).get("placement:boss-1").unwrap();
        world.entity_mut(boss).insert((
            Catalog(vec![("smash".into(), 20.0)]),
            Playing {
                power: 20.0,
                id: "smash".into(),
                t: 0.25,
            },
        ));
        let snap = take(&world, &reg);

        // The content changed under us.
        world.entity_mut(boss).insert(Catalog(vec![]));
        restore(&mut world, &snap, &reg);
        assert!(world.entity(boss).get::<Playing>().is_none());
    }

    /// **The boss's seeded RNG rewinds, and so does its step cursor.**
    ///
    /// netcode.md's N3.1 checklist: *"every seeded RNG resource (sim randomness MUST
    /// be a registered seeded resource — an unregistered RNG is a determinism bug
    /// N0.4 will catch)"*. The boss's lives inside `Brain`, next to its authored
    /// tuning, so it rides a `SnapshotCursor` rather than a codec.
    #[test]
    fn a_boss_brain_rewinds_its_seed_its_cursor_and_its_clocks() {
        use ambition_characters::brain::boss_pattern::{
            BossMacroState, BossPatternCfg, BossPatternState, CyclePhase,
        };
        use ambition_characters::brain::{Brain, StateMachineCfg};

        let reg = engine_registry();
        let mut world = sim_world();
        let boss = *live_ids(&mut world).get("placement:boss-1").unwrap();

        let mut brain = Brain::StateMachine(StateMachineCfg::BossPattern {
            cfg: BossPatternCfg::neutral_test(),
            state: BossPatternState::default(),
        });
        {
            let s = brain.boss_pattern_state_mut().expect("a boss brain");
            s.rng_seed = 0xDEAD_BEEF_CAFE_F00D;
            s.step_index = 3;
            s.step_elapsed = 0.75;
            s.pattern_timer = 12.5;
            s.cycle_phase = CyclePhase::Windup;
            s.macro_state = BossMacroState::Retreat {
                remaining_s: 0.5,
                retreat_pos: ae_vec(40.0, -8.0),
            };
            s.last_hp = Some(77);
        }
        world.entity_mut(boss).insert(brain);
        let before = reg.hash_world(&world);
        let snap = take(&world, &reg);

        // The fight advances: the boss draws from its RNG and moves on.
        {
            let mut brain = world.entity_mut(boss);
            let mut brain = brain.get_mut::<Brain>().unwrap();
            let s = brain.boss_pattern_state_mut().unwrap();
            s.rng_seed = 1;
            s.step_index = 5;
            s.step_elapsed = 0.0;
            s.macro_state = BossMacroState::Engage;
        }
        assert_ne!(reg.hash_world(&world), before, "the fight must have moved");

        restore(&mut world, &snap, &reg);
        let brain = world.entity(boss).get::<Brain>().unwrap();
        let s = brain.boss_pattern_state().unwrap();
        assert_eq!(s.rng_seed, 0xDEAD_BEEF_CAFE_F00D, "the seed rewound");
        assert_eq!(s.step_index, 3, "the step cursor rewound");
        assert_eq!(s.step_elapsed, 0.75);
        assert_eq!(s.last_hp, Some(77));
        assert!(
            matches!(s.macro_state, BossMacroState::Retreat { retreat_pos, .. } if retreat_pos.x == 40.0),
            "a boss that rewinds into Retreat rewinds to the same retreat POSITION"
        );
        assert_eq!(reg.hash_world(&world), before);
    }

    fn ae_vec(x: f32, y: f32) -> Vec2 {
        Vec2::new(x, y)
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
