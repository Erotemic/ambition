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

    /// Only canonical `0`/`1` decode; any other byte is corruption, not `true` (third-pass
    /// re-audit). `put_bool` writes exactly `0`/`1`, so a `2` in a `bool` slot is a malformed
    /// blob a decoder must reject, not silently accept as truthy.
    pub fn bool(&mut self) -> Option<bool> {
        match self.u8()? {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        }
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

/// **What applying one registered blob to its entity accomplished** (re-audit finding 3).
///
/// The old `insert` returned a bare `bool` — `false` for a decode failure, `true` for
/// "anything else, *including having applied nothing*." That conflation let a cursor with no
/// live target and a resolve whose content had vanished BOTH report success, so `lossless()`
/// could return `true` after registered state was silently not restored. Naming the third
/// outcome lets `restore` count it and `lossless()` deny it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ApplyOutcome {
    /// The blob's state is now on the entity.
    Applied,
    /// The blob did not decode — `restore` returns [`RestoreError::DecodeFailed`].
    DecodeFailed,
    /// The codec could not apply the row because the live target lacked what it needs — a
    /// cursor with no authored half to rewind, or a resolve whose authored content is gone.
    /// The registered state did NOT come back. It is not an error (a save legitimately drops
    /// a move whose content changed under it), but it is not lossless either: `lossless()`
    /// denies a report with any unapplied row.
    Unapplied,
}

/// What a registered entry *is*. The three kinds are not a taxonomy for its own
/// sake: they are the three answers to "what does `restore` do with this?"
enum EntryKind {
    /// Per-entity rows keyed by [`SimId`]. `restore` inserts each row onto the
    /// respawned entity that owns the key.
    Component {
        type_id: std::any::TypeId,
        rows: Box<dyn Fn(&World) -> Vec<(String, Vec<u8>)> + Send + Sync>,
        /// Applies one blob to its entity and reports the outcome (re-audit finding 3).
        /// [`ApplyOutcome::DecodeFailed`] becomes `RestoreError::DecodeFailed` rather than
        /// silently leaving stale state (audit M3/S2.5); [`ApplyOutcome::Unapplied`] — a
        /// cursor with no live target, a resolve whose content is gone — is counted and
        /// denies `lossless()` rather than passing as the old bare `true`.
        insert: Box<
            dyn Fn(&mut bevy::ecs::world::EntityWorldMut<'_>, &[u8]) -> ApplyOutcome + Send + Sync,
        >,
        /// A snapshot with no row for this entity means the entity did not HAVE the
        /// component then. Restoring exactly means taking it away now.
        remove: Box<dyn Fn(&mut bevy::ecs::world::EntityWorldMut<'_>) + Send + Sync>,
        /// **Standalone decode check for the transactional preflight** (re-audit finding
        /// 5). `Some` when the blob decodes to a self-contained value (`register_component`),
        /// so `restore` can validate it BEFORE mutating the world. `None` for cursor and
        /// resolved codecs, which decode into a live target and cannot be probed without
        /// one — their decode failure is caught at apply time, after mutation has begun (the
        /// named residual on `RestoreError::DecodeFailed`).
        probe: Option<Box<dyn Fn(&[u8]) -> bool + Send + Sync>>,
    },
    /// A single blob. `restore` puts it back.
    Resource {
        type_id: std::any::TypeId,
        bytes: Box<dyn Fn(&World) -> Vec<u8> + Send + Sync>,
        /// `false` on decode failure — see [`EntryKind::Component`]'s `insert`.
        load: Box<dyn Fn(&mut World, &[u8]) -> bool + Send + Sync>,
        /// Standalone decode check for the preflight — always `Some` for a plain resource
        /// (`register_resource` decodes to a self-contained value). See
        /// [`EntryKind::Component`]'s `probe`.
        probe: Box<dyn Fn(&[u8]) -> bool + Send + Sync>,
    },
    /// A resource that is half authored, half mutable — the [`SnapshotCursor`] shape,
    /// one level up. `CombatSlotsRes` is the archetype: authored slot geometry, live
    /// assignments.
    ResourceCursor {
        type_id: std::any::TypeId,
        /// A presence-tagged cursor blob (re-audit finding 4): a leading `bool` distinguishes
        /// "the resource existed at the snapshot tick" from "it did not", so an absent resource
        /// and a present-but-empty cursor no longer encode identically to `[]`.
        bytes: Box<dyn Fn(&World) -> Vec<u8> + Send + Sync>,
        /// Applies the tagged blob and reports the outcome (re-audit finding 4):
        /// [`ApplyOutcome::Applied`] (cursor applied, or an absent-tagged resource removed to
        /// match), [`ApplyOutcome::DecodeFailed`] (corrupt blob or a shape the cursor cannot
        /// faithfully apply), or [`ApplyOutcome::Unapplied`] (present at snapshot, absent now —
        /// a resource a cursor cannot rebuild, counted so `lossless()` denies it).
        apply: Box<dyn Fn(&mut World, &[u8]) -> ApplyOutcome + Send + Sync>,
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
///
/// **A `Resource`**, so a downstream crate can register the state it owns without any
/// crate above it knowing the type exists. `ambition_content`'s boss specials do
/// exactly that: they hold sim state, they live above `ambition_runtime`, and
/// [`register_engine_sim_state`] cannot name them. That is the *"each sim crate
/// registers its components' serialization"* shape netcode.md asks for, and it needed
/// a resource rather than a trait relocation.
#[derive(Default, Resource)]
pub struct SnapshotRegistry {
    entries: Vec<StateEntry>,
    /// Sim message channels. See [`SnapshotRegistry::register_message_channel`].
    messages: Vec<MessageChannel>,
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
                        Some(value) => {
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
                        // `resolve` returns `None` for BOTH a decode failure (e.g. a truncated
                        // blob) and a vanished authored half, and cannot distinguish them (the
                        // resolved-codec residual — making `resolve` return `Result` is the named
                        // fix, and it is now ONLY about that distinction, not trailing bytes).
                        // Either way this row did NOT come back, so it is `Unapplied`: the
                        // component is dropped (honest for a save whose content changed) and
                        // `lossless()` is denied (honest for a rollback). It is deliberately NOT
                        // `DecodeFailed` — that would refuse a legitimate content change as if the
                        // bytes were corrupt. The old bare `true` reported success for a row that
                        // never returned (re-audit finding 3).
                        None => {
                            entity.remove::<C>();
                            ApplyOutcome::Unapplied
                        }
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
    const ACTIVE_ROOM_ENTRY: &'static str = "\u{0}active_room";

    /// The `hash_by_entry` name of the identity-roster pseudo-entry (re-audit finding 1).
    const ROSTER_ENTRY: &'static str = "\u{0}roster";

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
    /// `unclaimed_components` walks entities. A `Resource` sits on no entity, so it was
    /// invisible to the ledger entirely — `EncounterState`, with its live phase and
    /// wave run, was never counted, and `restore` never touched it.
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

/// One `Messages<M>` buffer the rollback has to reckon with.
struct MessageChannel {
    name: &'static str,
    /// The `TypeId` of `Messages<M>`, so the resource census counts this registered
    /// (restore-cleared) channel as CLAIMED, not as unregistered debt (finding 6).
    type_id: std::any::TypeId,
    len: fn(&World) -> usize,
    clear: fn(&mut World),
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
    /// The room active when the snapshot was taken, by its `RoomSpec` id. The active
    /// room is sim state that `restore` does not yet restore, so a rollback window that
    /// spans a room transition would reconcile the snapshot's entities against the
    /// wrong `RoomSpec`. `restore` compares this against the world's current active room
    /// and REFUSES a mismatch (`RestoreError::CrossRoomBoundary`) rather than partially
    /// restore — a room transition also rebuilds room-scoped entities, platforms, and
    /// clocks, so a partial restore is more inconsistent than a refusal (netcode.md N3.2).
    /// `None` for a headless world with no `RoomSet` (the unit-test fixtures).
    pub active_room: Option<String>,
    /// **Every `SimId` carried by a live entity when the snapshot was taken** — sorted,
    /// with duplicates PRESERVED. The full identity roster, a superset of the
    /// component-row ids [`sim_ids`](Self::sim_ids) derives.
    ///
    /// A per-component-entry duplicate scan is blind to two entities that share one
    /// `SimId` but carry disjoint (or zero) registered components — each contributes at
    /// most one row per entry, so no single entry sees the collision (re-audit finding 3).
    /// The roster sees every identity regardless of components, so `duplicate_ids` catches
    /// it. `take` enforces uniqueness at capture; `restore` validates it independently,
    /// defending the deserialized-snapshot path (N3.3) that `take` never touched.
    roster: Vec<String>,
    entries: Vec<(&'static str, EntryBlob)>,
}

/// Values that appear more than once in a **sorted** slice, each named exactly once.
fn adjacent_dups(sorted: &[String]) -> Vec<String> {
    let mut dups = Vec::new();
    for pair in sorted.windows(2) {
        if pair[0] == pair[1] && dups.last() != Some(&pair[1]) {
            dups.push(pair[1].clone());
        }
    }
    dups
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum EntryBlob {
    /// Sorted by `SimId`, so two snapshots of equal worlds are `==`.
    Component(Vec<(String, Vec<u8>)>),
    Resource(Vec<u8>),
}

impl SimSnapshot {
    /// **The authoritative identity set the snapshot restores: the full [`roster`], sorted
    /// and deduped** (re-audit finding 1).
    ///
    /// Reads the captured `roster` — every live `SimId` at snapshot time — NOT the union of
    /// per-entry component-row ids. Those two differ by exactly the entities the old
    /// component-derived set was blind to: a `SimId` carrying no registered component appears
    /// in no row, so `restore` (which reconciles against THIS list) would never see it —
    /// despawning it if it survived, silently dropping it if it had died. Driving off the
    /// roster makes both cases correct with no other change, because the roster IS the set of
    /// entities that existed. `roster` is a superset of the component-row ids by construction,
    /// so no id that carried state is ever lost.
    pub fn sim_ids(&self) -> Vec<&str> {
        let mut out: Vec<&str> = self.roster.iter().map(String::as_str).collect();
        out.sort_unstable();
        out.dedup();
        out
    }

    /// `SimId`s carried by more than one live entity at capture — an ambiguous roster.
    /// `sim_ids()` silently `dedup`s these away; this surfaces them so `restore` can
    /// refuse a snapshot whose identity is not unique (audit H2).
    ///
    /// Reads the full [`roster`](Self::roster), not the per-entry component rows, so it
    /// catches a collision even between two entities that share a `SimId` but carry
    /// disjoint (or zero) registered components — the case the old per-entry scan missed
    /// (re-audit finding 3). A well-formed snapshot returns empty.
    ///
    /// **Sorts a clone before scanning** (re-audit finding 2): `take` stores the roster
    /// sorted, but a snapshot arriving over the N3.3 wire may not be, and an
    /// adjacent-only scan of `["dup", "other", "dup"]` would miss the collision. Detection
    /// must not depend on the caller having sorted first.
    pub fn duplicate_ids(&self) -> Vec<String> {
        let mut sorted = self.roster.clone();
        sorted.sort();
        adjacent_dups(&sorted)
    }

    /// Total bytes of state. A rollback window is a memory budget.
    pub fn size_bytes(&self) -> usize {
        let entries: usize = self
            .entries
            .iter()
            .map(|(_, blob)| match blob {
                EntryBlob::Component(rows) => {
                    rows.iter().map(|(k, v)| k.len() + v.len()).sum::<usize>()
                }
                EntryBlob::Resource(bytes) => bytes.len(),
            })
            .sum();
        // The active-room cursor and the identity roster are captured state too (findings
        // 2, 3), so they count against the window's memory budget.
        entries
            + self.active_room.as_ref().map_or(0, |s| s.len())
            + self.roster.iter().map(String::len).sum::<usize>()
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
    // The active room is captured so `restore` can refuse a window that spans a
    // transition. `respawn_from_the_room` already reaches `RoomSet`, so `take` may too.
    let active_room = world
        .get_resource::<ambition_world::rooms::RoomSet>()
        .map(|rs| rs.active_spec().id.clone());
    // The full identity roster: every live `SimId`, sorted, dups preserved. Captured
    // independently of which components an entity carries, so identity is validated even
    // for an entity with no registered state — the collision a per-component scan misses
    // (re-audit finding 3). Enforced unique HERE, at the source, rather than letting a
    // corrupt snapshot into the rollback buffer to be discovered at restore.
    let mut roster: Vec<String> = Vec::new();
    if let Some(mut q) = world.try_query::<&SimId>() {
        for id in q.iter(world) {
            roster.push(id.as_str().to_string());
        }
    }
    roster.sort();
    let dups = adjacent_dups(&roster);
    assert!(
        dups.is_empty(),
        "take: {} SimId(s) carried by more than one live entity — identity is not unique \
         and no snapshot of this world can be trusted. Fix the spawn site (a duplicated \
         `SimId::spawned`/placement id). Collisions: {dups:?}",
        dups.len(),
    );
    let mut entries = Vec::new();
    for entry in &registry.entries {
        match &entry.kind {
            EntryKind::Component { rows, .. } => {
                let mut rows = rows(world);
                rows.sort();
                entries.push((entry.name, EntryBlob::Component(rows)));
            }
            EntryKind::Resource { bytes, .. } | EntryKind::ResourceCursor { bytes, .. } => {
                entries.push((entry.name, EntryBlob::Resource(bytes(world))));
            }
            EntryKind::Diagnostic { .. } => {}
        }
    }
    SimSnapshot {
        tick,
        active_room,
        roster,
        entries,
    }
}

/// **Every `SimId` carried by more than one live entity, with its count, sorted.**
///
/// This is the single identity-roster check the audit (H2) asked for. Identity MUST be
/// unique: a duplicate means a spawn site minted the same id twice, and every by-id
/// lookup — `restore`, the ledgers — would otherwise pick one entity arbitrarily and
/// silently. `restore` calls this and refuses a world where it is non-empty; the SimId
/// ledger can call it to catch a duplicating spawner before a rewind ever runs. Empty
/// is the healthy case.
pub fn duplicate_live_ids(world: &mut World) -> Vec<(String, usize)> {
    let mut counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    if let Some(mut q) = world.try_query::<&SimId>() {
        for id in q.iter(world) {
            *counts.entry(id.as_str().to_string()).or_insert(0) += 1;
        }
    }
    counts.into_iter().filter(|(_, n)| *n > 1).collect()
}

/// What `restore` did, and what it could not rewind.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct RestoreReport {
    /// Entities present in BOTH the world and the snapshot. Their registered
    /// components were overwritten in place; everything else they carry survived.
    pub patched: usize,
    /// Entities in the snapshot that no longer existed and that the ROOM could build
    /// again, keyed by the authored id their `SimId` already is. They come back whole,
    /// and the blob patches their mutable half exactly as a survivor's.
    pub rebuilt: usize,
    /// Entities in the snapshot that no longer existed and that **no room authors** —
    /// dynamically-spawned ones (`SimId::spawned(..)`). Rebuilt from blobs **alone**, so
    /// they come back naked but for their registered components. A rollback window that
    /// spans such a birth is not exact, and this is the number that says so.
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
    /// Registered `Messages<M>` channels emptied, so no message from the abandoned
    /// future is read in the restored past.
    pub messages_cleared: usize,
    /// **Simulated bodies with no `SimId`, which `restore` could not touch.**
    ///
    /// They are not registered, so nothing identifies them, and they walk out of a
    /// rollback carrying whatever state the rewound tick gave them. A projectile in
    /// this set survives its own un-firing. `unidentified_bodies` is the same number
    /// seen from the canary's side.
    pub unidentified_survivors: usize,
    /// **Unregistered sim-state resources still standing after restore** (audit H3;
    /// re-audit finding 6). Measured BY `restore` from `unclaimed_sim_resources(world)`,
    /// not supplied by the caller — a caller-supplied count let `lossless(0)` be claimed
    /// against a world that had debt. `0` also when the census is unreliable (see
    /// [`resource_census_reliable`](Self::resource_census_reliable)); `lossless()` refuses
    /// regardless in that case.
    pub unregistered_sim_resources: usize,
    /// Whether the resource census that produced `unregistered_sim_resources` was even
    /// meaningful. Bevy only names resources under `bevy_ecs/debug`; without it every name
    /// is a placeholder, the `ambition_*` filter matches nothing, and the count is a
    /// spurious `0`. `lossless()` REQUIRES this true, so it cannot falsely succeed in a
    /// build where resource debt is invisible (re-audit finding 6).
    pub resource_census_reliable: bool,
    /// Registered resource CURSORS whose snapshot blob said the resource was present but whose
    /// target was absent at restore, so nothing was applied — a silent incompleteness the
    /// apply closure used to swallow by returning success (re-audit findings 4 + 6). Denied by
    /// `lossless()`.
    pub resource_cursors_unresolved: usize,
    /// **Registered COMPONENT rows that did not come back** (re-audit finding 3): a cursor with
    /// no live target to rewind, or a resolve whose authored content had vanished. The
    /// component is left off (honest), but the registered state the snapshot carried was not
    /// restored, so `lossless()` denies any report with an unapplied row. Distinct from
    /// `respawned` (a whole entity rebuilt from blobs) and `stale_components` (a survivor's
    /// UNregistered state): this is registered state that was asked for and could not be given.
    pub unapplied_rows: usize,
}

impl RestoreReport {
    /// **The positive completeness contract** (audit H3). A restore is lossless only if
    /// EVERY exactness condition holds — not merely the absence of the three defect
    /// classes the old method happened to check.
    ///
    /// Two conditions are guaranteed by this report *existing at all*, so they are not
    /// re-checked here:
    /// - **unique identity** — `restore` panics on a duplicate live/snapshot `SimId`
    ///   (S2.1), so a report is only produced for a world whose identity is unique;
    /// - **successful decode** — `restore` returns `Err(DecodeFailed)` on a codec failure
    ///   (S2.5), so a report means every registered blob decoded.
    ///
    /// The rest are checked here, from the report's OWN measured fields — the caller no
    /// longer supplies the resource count, so `lossless()` cannot be claimed against a
    /// world that had debt (re-audit finding 6):
    /// - **no unaccounted stale component** on a surviving entity (`stale_components`);
    /// - **every survivor carries an identity** (`unidentified_survivors == 0`);
    /// - **no naked reconstruction** — nothing came back from blobs alone, outside an
    ///   accepted policy (`respawned == 0`);
    /// - **complete mutable-RESOURCE coverage** (`unregistered_sim_resources == 0`),
    ///   measured by `restore` itself. This is the condition the old `lossless()` omitted,
    ///   and why H3 flagged it: a `Resource` sits on no entity, so `stale_components` never
    ///   saw one, and the method returned `true` while ~181 sim resources went unrestored;
    /// - **every resource cursor resolved** (`resource_cursors_unresolved == 0`) — a cursor
    ///   blob that said the resource was present, restored into a world where it is absent,
    ///   applied nothing (re-audit finding 4);
    /// - **every registered COMPONENT row applied** (`unapplied_rows == 0`) — a cursor with no
    ///   live target, or a resolve whose content vanished, left registered state unrestored
    ///   while the old bare-`true` insert reported success (re-audit finding 3);
    /// - **the resource census was meaningful** (`resource_census_reliable`). Without
    ///   `bevy_ecs/debug` the resource count is a spurious `0`; requiring this true stops a
    ///   build with invisible resource debt from reporting a false lossless.
    ///
    /// Message-channel coverage is not a separate condition: `restore` clears every
    /// REGISTERED channel (and their `Messages<M>` is now CLAIMED, so it is not counted as
    /// debt), and an UN-registered `Messages<ambition_..>` is a resource, so it lands in
    /// `unregistered_sim_resources` above.
    pub fn lossless(&self) -> bool {
        self.stale_components.is_empty()
            && self.unidentified_survivors == 0
            && self.respawned == 0
            && self.unregistered_sim_resources == 0
            && self.resource_cursors_unresolved == 0
            && self.unapplied_rows == 0
            && self.resource_census_reliable
    }
}

/// **Why a `restore` refused.**
///
/// Distinct from the identity-invariant PANICS (a duplicate `SimId` or registry name is
/// a bug that makes ALL rollback impossible — see `duplicate_live_ids`): a
/// `RestoreError` is a VALID world asking for a rollback that is not supported, so
/// restore returns rather than corrupts. The caller decides — a test `.expect()`s it, a
/// future netcode boundary logs it and refuses the rewind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RestoreError {
    /// The snapshot's active room and the world's do not MATCH — the rollback window
    /// spans a room transition. Reconciling would rebuild the snapshot's entities against
    /// the WRONG `RoomSpec`. The active room is not yet restored sim state, and a room
    /// transition rebuilds room-scoped entities, moving platforms, and clocks that a
    /// partial restore cannot reproduce, so restore refuses rather than produce a world
    /// more inconsistent than the one it started from (netcode.md N3.2: room transitions
    /// are rollback boundaries).
    ///
    /// Each side is `Option<String>` because *presence itself* is state: a snapshot taken
    /// with no room (`None`) restored into a world that now has one (`Some`) is as much a
    /// mismatch as two different room ids, so restore compares the full options, not just
    /// the both-`Some` case (re-audit finding 5). A headless fixture with no `RoomSet` on
    /// either side is `None == None` and does not refuse.
    CrossRoomBoundary {
        snapshot_room: Option<String>,
        active_room: Option<String>,
    },
    /// A registered codec failed to decode its blob during restore — the bytes are
    /// corrupt, or the encoder and decoder disagree. A SILENT continue (the old
    /// `debug_assert!(false)` + leave-it-alone, which fired only in debug builds) would
    /// leave stale state reading as restored. `entry` is the registry name; `id` is the
    /// `SimId` for a component row, `None` for a resource.
    ///
    /// **Transactionality (re-audit finding 5):** a STANDALONE codec — a plain component
    /// or plain resource — is decode-preflighted before any mutation, so this error leaves
    /// the world UNTOUCHED. A cursor/resolved codec decodes into a live target and has no
    /// standalone probe, so ITS decode failure can surface mid-reconciliation with the
    /// world PARTIALLY restored; that is the named residual, and the caller must discard
    /// the world (fetch a fresh snapshot). Only a project-authored cursor/resolved codec
    /// disagreement reaches that path — the common corrupt-blob case is transactional.
    DecodeFailed { entry: String, id: Option<String> },
    /// The snapshot holds a dynamically-spawned entity (a `SimId::spawned(..)` id — the
    /// vocabulary appends `/<seq>`) that **existed at the snapshot tick, is absent now,
    /// and cannot be reconstructed** because no spawn recipe exists. Rebuilding it from
    /// blobs ALONE is not exact: a dynamic entity needs its spawner's recipe to come back
    /// whole, and N3.2 does not yet register spawn recipes.
    ///
    /// This is precisely a **reconstruction** refusal — the entity died inside the window
    /// and restore is being asked to raise it — not a "birth inside the window" (an entity
    /// spawned AFTER the snapshot is future-only and simply despawned; re-audit finding 4).
    /// It establishes ONE reconstruction refusal, not a general bounded-window guarantee.
    /// Preflighted before any mutation (finding 5), so restore refuses cleanly rather than
    /// after partial work. The honest boundary until spawn recipes land.
    UnsupportedDynamicReconstruction { sim_id: String },
    /// **The snapshot is not well-formed against the registry restoring it** (re-audit
    /// finding 2) — caught by [`validate_snapshot`], a mutation-free phase that runs before
    /// restore touches a single entity. `take` cannot produce a malformed snapshot, so in a
    /// same-process rollback this never fires; it exists for the N3.3 wire, where a snapshot
    /// is deserialized bytes that were never take-validated and restore must not trust their
    /// shape. `reason` names the exact violation: a duplicate or unsorted roster; an entry
    /// naming no registered state; a component blob under a resource entry (or the reverse);
    /// rows out of order, duplicated, or carrying an id absent from the roster; or a
    /// registered entry the snapshot omitted. All of these would otherwise make restore's
    /// by-id lookups and `binary_search`es silently wrong rather than loudly refused.
    MalformedSnapshot { reason: String },
}

impl std::fmt::Display for RestoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RestoreError::CrossRoomBoundary {
                snapshot_room,
                active_room,
            } => {
                fn room(r: &Option<String>) -> String {
                    match r {
                        Some(id) => format!("`{id}`"),
                        None => "no room".to_string(),
                    }
                }
                write!(
                    f,
                    "cross-room rollback boundary: snapshot taken in {}, world is now in {} \
                     — a rollback window may not span a room transition",
                    room(snapshot_room),
                    room(active_room),
                )
            }
            RestoreError::UnsupportedDynamicReconstruction { sim_id } => write!(
                f,
                "unsupported dynamic reconstruction: `{sim_id}` is a dynamically-spawned \
                 entity that existed at the snapshot tick, is gone now, and no room authors \
                 it — rebuilding it from blobs alone is not exact (no spawn recipe yet). \
                 Restore refuses rather than raise a naked entity."
            ),
            RestoreError::MalformedSnapshot { reason } => write!(
                f,
                "malformed snapshot: {reason} — restore refuses a snapshot whose shape does \
                 not agree with the registry, rather than reconcile against it silently"
            ),
            RestoreError::DecodeFailed { entry, id } => write!(
                f,
                "codec `{entry}` failed to decode its blob{} — corrupt snapshot or an \
                 encoder/decoder disagreement; restore cannot honor it",
                match id {
                    Some(id) => format!(" for `{id}`"),
                    None => String::new(),
                }
            ),
        }
    }
}

impl std::error::Error for RestoreError {}

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
        .get_resource::<ambition_world::placements::PlacementLoweringRegistry>()?
        .clone();
    let room = {
        let rooms = world.get_resource::<ambition_world::rooms::RoomSet>()?;
        rooms.rooms.get(rooms.active)?.clone()
    };

    let built = {
        let mut commands = world.commands();
        ambition_actors::features::respawn_authored_entity(&mut commands, &room, &registry, iid)
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
    // **Room-transition boundary (audit item 2 / reviewer), checked BEFORE any entity
    // is touched.** The active room is not yet restored sim state, so a snapshot taken
    // in one room cannot be reconciled against another: `respawn_from_the_room` would
    // consult the wrong `RoomSpec`. Refuse rather than partially restore — restoring
    // only `RoomSet.active` + geometry, but not the room-scoped entities/platforms/
    // clocks a transition rebuilds, would leave a world more inconsistent than this
    // clean refusal. The full atomic room transaction is the bounded-window work.
    //
    // The two `Option<String>` are compared WHOLE (re-audit finding 5): a snapshot with a
    // room restored into a world with none — or vice versa — is a state mismatch as surely
    // as two different ids, and the old both-`Some` guard let it through. `None == None`
    // (a headless fixture with no `RoomSet`) is not a mismatch and does not refuse.
    let active_room = world
        .get_resource::<ambition_world::rooms::RoomSet>()
        .map(|rs| rs.active_spec().id.clone());
    if snapshot.active_room != active_room {
        return Err(RestoreError::CrossRoomBoundary {
            snapshot_room: snapshot.active_room.clone(),
            active_room,
        });
    }

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

    // Now the map is unambiguous: every id appears once, so no insert overwrites.
    let mut live: std::collections::BTreeMap<String, Entity> = std::collections::BTreeMap::new();
    if let Some(mut q) = world.try_query::<(Entity, &SimId)>() {
        for (entity, id) in q.iter(world) {
            live.insert(id.as_str().to_string(), entity);
        }
    }

    let ids = snapshot.sim_ids();
    let mut report = RestoreReport::default();

    // **Unsupported-dynamic-reconstruction preflight (re-audit finding 5), BEFORE any
    // mutation.** A `spawned(..)` id (the vocabulary appends `/<seq>`) that is in the
    // snapshot but gone from the live world would have to be rebuilt from blobs alone — no
    // room authors it and no spawn recipe exists, so the rebuild is not exact. Detecting it
    // here, from the id string and the live map alone (no world mutation), lets restore
    // refuse cleanly rather than after the despawn/rebuild loop has already half-reconciled
    // the world. `respawn_from_the_room` only ever handles `placement:` ids, so this is
    // exactly the set that used to reach the inline `None if contains('/')` branch — moved
    // ahead of the first despawn.
    for id in &ids {
        if !live.contains_key(*id) && id.contains('/') {
            return Err(RestoreError::UnsupportedDynamicReconstruction {
                sim_id: (*id).to_string(),
            });
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
            // Gone since the snapshot. Ask the ROOM to build it again before falling
            // back to a bare `SimId` — the blob carries what the entity became, and only
            // the room carries what it was.
            None => match respawn_from_the_room(world, id) {
                Some(entity) => {
                    report.rebuilt += 1;
                    entity
                }
                // Gone, and no room authors it. A `placement:`/`slot:` id with no room
                // record is the headless-fixture path: respawn bare. A `spawned(..)` id in
                // this position was already refused by the preflight above, so this arm
                // only ever sees static ids the fixtures use.
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
/// Install the [`SnapshotRegistry`] resource with the engine's own state registered.
///
/// Any plugin built AFTER this one may `resource_mut::<SnapshotRegistry>()` and add
/// its own entries. Registration order is part of the hash — deliberately, since a
/// canary comparing two builds with different registries is comparing two different
/// definitions of "the sim" — and it is a function of plugin build order, which is a
/// function of the binary. Two `SandboxSim`s of the same build agree.
pub struct SnapshotRegistryPlugin;

impl bevy::app::Plugin for SnapshotRegistryPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        // `init_resource` + register, never `insert_resource`: a plugin that built
        // BEFORE this one and added its own entries must not have them thrown away.
        // Registration is additive and order-independent; the resulting ORDER is a
        // function of plugin build order, hence of the binary, hence identical across
        // two sims of the same build — which is all the hash requires.
        app.init_resource::<SnapshotRegistry>();
        let mut registry = app.world_mut().resource_mut::<SnapshotRegistry>();
        register_engine_sim_state(&mut registry);
    }
}

pub fn register_engine_sim_state(registry: &mut SnapshotRegistry) {
    registry.register_resource::<ambition_time::SimTick>("sim_tick");
    registry.register_resource::<ambition_time::WorldTime>("world_time");
    registry.register_resource::<ambition_actors::features::GameplayElapsed>("gameplay_elapsed");
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
    registry.register_component::<bc::BodyLedgeState>("body_ledge_state");
    registry.register_component::<bc::BodyComboTrace>("body_combo_trace");
    registry.register_component::<ambition_characters::brain::ActorControl>("actor_control");
    registry.register_component::<ambition_time::ProperTimeScale>("proper_time_scale");
    registry.register_cursor::<ambition_actors::features::BossEncounter>("boss_encounter");

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

    registry
        .register_resource_cursor::<ambition_combat::slots::CombatSlotsRes>("combat_slot_board");

    // ── Sim message channels ─────────────────────────────────────────────────
    //
    // A message written before a snapshot and read after a restore is an event that
    // happens twice. `restore` clears these; see `register_message_channel`.
    registry
        .register_message_channel::<ambition_characters::brain::ActorActionMessage>("actor_action");
    registry.register_message_channel::<ambition_combat::events::HitEvent>("hit_event");
    registry
        .register_message_channel::<ambition_combat::on_hit::OnHitEffectMessage>("on_hit_effect");
    registry.register_message_channel::<ambition_combat::moveset::MoveEventMessage>("move_event");

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
#[macro_export]
macro_rules! snapshot_pod {
    ($ty:path { $($field:ident : $get:ident),+ $(,)? }) => {
        impl SnapshotState for $ty {
            fn encode(&self, out: &mut Vec<u8>) {
                $( $crate::snapshot::paste_put(out, self.$field); )+
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
pub trait PasteEncode: Copy {
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
pub fn paste_put<T: PasteEncode>(out: &mut Vec<u8>, v: T) {
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
#[macro_export]
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

snapshot_unit_enum!(ambition_engine_core::ledge_grab::LedgeGetupKind {
    Climb = 0,
    Roll = 1,
    Attack = 2,
});
snapshot_unit_enum!(ambition_engine_core::ledge_grab::LedgeGrabQuality {
    Precise = 0,
    Forgiving = 1,
});
snapshot_unit_enum!(ambition_engine_core::movement::MovementOp {
    Jump = 0,
    DoubleJump = 1,
    WallJump = 2,
    WallCling = 3,
    WallClimb = 4,
    LedgeGrab = 5,
    LedgeJump = 6,
    LedgeClimbStart = 7,
    LedgeClimbFinish = 8,
    LedgeDrop = 9,
    LedgeRoll = 10,
    LedgeGetupAttack = 11,
    SwimStroke = 12,
    Dash = 13,
    DoubleDash = 14,
    DodgeRoll = 15,
    FlyToggle = 16,
    Blink = 17,
    PrecisionBlink = 18,
    Pogo = 19,
    Rebound = 20,
    Slash = 21,
    Reset = 22,
    ShieldUp = 23,
});

/// A body hanging on a ledge. `grab: Option<LedgeGrabState>` is the whole state
/// machine: a rollback into a hang must land on the same anchor, with the same
/// carried momentum, or the getup goes somewhere else.
impl SnapshotState for bc::BodyLedgeState {
    fn encode(&self, out: &mut Vec<u8>) {
        match &self.grab {
            None => put_bool(out, false),
            Some(g) => {
                put_bool(out, true);
                put_f32(out, g.contact.wall_normal_x);
                put_vec2(out, g.contact.anchor);
                put_vec2(out, g.contact.climb_target);
                put_f32(out, g.elapsed);
                put_bool(out, g.climbing);
                g.getup_kind.encode(out);
                put_f32(out, g.climb_elapsed);
                put_vec2(out, g.momentum_at_grab);
                g.grab_quality.encode(out);
            }
        }
        put_f32(out, self.release_cooldown);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        use ambition_engine_core::ledge_grab::{
            LedgeContact, LedgeGetupKind, LedgeGrabQuality, LedgeGrabState,
        };
        let grab = if r.bool()? {
            Some(LedgeGrabState {
                contact: LedgeContact {
                    wall_normal_x: r.f32()?,
                    anchor: r.vec2()?,
                    climb_target: r.vec2()?,
                },
                elapsed: r.f32()?,
                climbing: r.bool()?,
                getup_kind: LedgeGetupKind::decode(r)?,
                climb_elapsed: r.f32()?,
                momentum_at_grab: r.vec2()?,
                grab_quality: LedgeGrabQuality::decode(r)?,
            })
        } else {
            None
        };
        Some(bc::BodyLedgeState {
            grab,
            release_cooldown: r.f32()?,
        })
    }
}

/// The recent-movement trace a combo/chain rule reads. A `Vec`, so its order IS its
/// meaning: the ops go out in the order they went in.
impl SnapshotState for bc::BodyComboTrace {
    fn encode(&self, out: &mut Vec<u8>) {
        put_u32(out, self.combo.len() as u32);
        for mark in &self.combo {
            mark.op.encode(out);
            put_f32(out, mark.age);
        }
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        use ambition_engine_core::movement::{ComboMark, MovementOp};
        let n = r.u32()?;
        let combo = (0..n)
            .map(|_| {
                Some(ComboMark {
                    op: MovementOp::decode(r)?,
                    age: r.f32()?,
                })
            })
            .collect::<Option<Vec<_>>>()?;
        Some(bc::BodyComboTrace { combo })
    }
}

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

/// **The boss's encounter phase**, and the `BossPhaseState` it is forwarded from.
///
/// A cursor, because the rest of `BossEncounter` is sprite metrics derived from the
/// sheet registry, and because `BossPhaseState.triggers` is authored data.
///
/// `encounter_phase` is the exposed MIRROR that `sync_boss_encounter_phase` copies out
/// of `encounter` every tick. Rewinding only the mirror is rewinding a thermometer:
/// `mockingbird_arena` telegraphed `wing_sweep` on the replay's tick 21 and stood still
/// on the original's, with every clock, seed, and cooldown identical, because the
/// replay's boss was already awake.
impl SnapshotCursor for ambition_actors::features::BossEncounter {
    fn encode_cursor(&self, out: &mut Vec<u8>) {
        self.encounter_phase.encode(out);
        match &self.encounter {
            None => put_bool(out, false),
            Some(e) => {
                put_bool(out, true);
                e.phase.encode(out);
                put_f32(out, e.phase_elapsed);
                put_f32(out, e.transition_lock);
                e.start_phase.encode(out);
            }
        }
    }
    fn apply_cursor(&mut self, r: &mut Reader<'_>) -> Option<()> {
        use ambition_characters::brain::boss_pattern::BossEncounterPhase;
        self.encounter_phase = BossEncounterPhase::decode(r)?;
        if r.bool()? {
            let phase = BossEncounterPhase::decode(r)?;
            let phase_elapsed = r.f32()?;
            let transition_lock = r.f32()?;
            let start_phase = BossEncounterPhase::decode(r)?;
            // The authored `triggers` stay where they are: a snapshot carries what the
            // fight has BECOME, never the rules it became it by.
            if let Some(e) = self.encounter.as_mut() {
                e.phase = phase;
                e.phase_elapsed = phase_elapsed;
                e.transition_lock = transition_lock;
                e.start_phase = start_phase;
            }
        } else {
            self.encounter = None;
        }
        Some(())
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

/// One beat of a **resolved** boss timeline.
///
/// `resolve_timeline` rolls every `Select` away before the first tick of the fight runs
/// — *"Select rolled away, Stance markers left in place as jumps"* — so a resolved
/// timeline holds only these four. A `Select` that survives into one is an invariant
/// violation, and this encodes it as a tag no decoder accepts: rejected, never silently
/// reinterpreted as a `Rest`.
///
/// The steps are *resolved instance state*, not authored content. The authored thing is
/// the `BossPattern`; the timeline is what one weighted roll made of it. Rewinding a
/// boss without rewinding the roll gives it a different fight.
impl SnapshotState for ambition_characters::brain::boss_pattern::BossPatternStep {
    fn encode(&self, out: &mut Vec<u8>) {
        use ambition_characters::brain::boss_pattern::BossPatternStep as S;
        match self {
            S::Telegraph {
                profile,
                duration,
                telegraph,
            } => {
                put_u8(out, 0);
                profile.encode(out);
                put_f32(out, *duration);
                match telegraph {
                    None => put_bool(out, false),
                    Some(spec) => {
                        put_bool(out, true);
                        put_opt_str(out, spec.pose.as_deref());
                        put_opt_str(out, spec.cue.as_deref());
                        put_opt_str(out, spec.vfx.as_deref());
                    }
                }
            }
            S::Strike { profile, duration } => {
                put_u8(out, 1);
                profile.encode(out);
                put_f32(out, *duration);
            }
            S::Rest { duration } => {
                put_u8(out, 2);
                put_f32(out, *duration);
            }
            S::Stance { id } => {
                put_u8(out, 3);
                put_str(out, id);
            }
            // Unreachable in a resolved timeline. Tag 4 decodes to `None`.
            S::Select { .. } => {
                debug_assert!(false, "a resolved timeline still holds a `Select`");
                put_u8(out, 4);
            }
        }
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        use ambition_characters::brain::boss_pattern::{
            BossAttackProfile, BossPatternStep as S, TelegraphSpec,
        };
        match r.u8()? {
            0 => {
                let profile = BossAttackProfile::decode(r)?;
                let duration = r.f32()?;
                let telegraph = if r.bool()? {
                    Some(TelegraphSpec {
                        pose: r.opt_str()?.map(str::to_string),
                        cue: r.opt_str()?.map(str::to_string),
                        vfx: r.opt_str()?.map(str::to_string),
                    })
                } else {
                    None
                };
                Some(S::Telegraph {
                    profile,
                    duration,
                    telegraph,
                })
            }
            1 => Some(S::Strike {
                profile: BossAttackProfile::decode(r)?,
                duration: r.f32()?,
            }),
            2 => Some(S::Rest { duration: r.f32()? }),
            3 => Some(S::Stance {
                id: r.str()?.to_string(),
            }),
            _ => None,
        }
    }
}

fn put_timeline(
    out: &mut Vec<u8>,
    steps: &[ambition_characters::brain::boss_pattern::BossPatternStep],
) {
    put_u32(out, steps.len() as u32);
    for s in steps {
        s.encode(out);
    }
}

fn read_timeline(
    r: &mut Reader<'_>,
) -> Option<Vec<ambition_characters::brain::boss_pattern::BossPatternStep>> {
    use ambition_characters::brain::boss_pattern::BossPatternStep;
    let n = r.u32()?;
    (0..n).map(|_| BossPatternStep::decode(r)).collect()
}

/// **The boss's mind, rewound.**
///
/// A `SnapshotCursor`, because `Brain` is half authored and half state: the brain's
/// KIND and its tuning came from content and survive the patch, and only
/// `BossPatternState`'s clocks, cursors, and **`rng_seed`** ride the blob. A seeded
/// RNG that is not snapshot state is a determinism bug the canary would eventually
/// catch, and netcode.md's checklist names it.
///
/// ## The `timeline` is instance state, not authored content
///
/// I first left `timeline` and `stance_stack` un-rewound, and called the resulting
/// hazard a *constraint*: "a rollback window must not span a pattern re-resolve."
/// `mockingbird_arena` then replayed exactly for twenty ticks and broke on the
/// twenty-first, which is what a re-resolve inside the window looks like.
///
/// The framing was wrong. The AUTHORED thing is the `BossPattern`; the timeline is what
/// **one weighted roll** made of it — *"the roll happens at RESOLUTION, not at the
/// cursor, so a fight's timeline is a concrete list of beats before the first tick of
/// it runs."* That is instance state by any definition, and rewinding a boss without
/// rewinding the roll gives it a different fight. It is encoded, and so is the
/// `stance_stack`, whose entries carry timelines of their own.
///
/// A resolved timeline holds only `Telegraph` / `Strike` / `Rest` / `Stance`: the
/// `Select`s are rolled away at resolution. So the beats are small, and the blob is a
/// handful of tags and floats — not the pattern, not the arms, not the weights.
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
        put_timeline(out, &s.timeline);
        put_opt_str(out, s.stance.as_deref());
        put_u32(out, s.stance_stack.len() as u32);
        for ret in &s.stance_stack {
            put_timeline(out, &ret.timeline);
            put_opt_str(out, ret.stance.as_deref());
            put_u32(out, ret.step_index as u32);
            put_f32(out, ret.step_elapsed);
        }
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
        let timeline = read_timeline(r)?;
        let stance = r.opt_str()?.map(str::to_string);
        let stance_stack = {
            use ambition_characters::brain::boss_pattern::StanceReturn;
            let n = r.u32()?;
            (0..n)
                .map(|_| {
                    Some(StanceReturn {
                        timeline: read_timeline(r)?,
                        stance: r.opt_str()?.map(str::to_string),
                        step_index: r.u32()? as usize,
                        step_elapsed: r.f32()?,
                    })
                })
                .collect::<Option<Vec<_>>>()?
        };
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
        s.timeline = timeline;
        s.stance = stance;
        s.stance_stack = stance_stack;
        s.interrupt_cooldowns = interrupt_cooldowns;
        s.interrupt_timers = interrupt_timers;
        s.last_hp = last_hp;
        Some(())
    }
}

snapshot_unit_enum!(ambition_engine_core::reference_frame::GameplayFramePolicy {
    ControlledBodyLocal = 0,
    AccelerationFrame = 1,
    WorldSpace = 2,
    ScreenSpace = 3,
});

/// **The brain's last-tick intent**, which the sim reads on the NEXT tick — the
/// `brain/README.md` calls it exactly that. So it is state, not a per-frame scratchpad,
/// and a rewind that leaves it stale hands the body an input it never chose.
///
/// Every field, in declaration order. There is no clever half of this component.
impl SnapshotState for ambition_characters::brain::ActorControl {
    fn encode(&self, out: &mut Vec<u8>) {
        let f = &self.0;
        put_vec2(out, f.locomotion);
        put_vec2(out, f.velocity_target);
        put_bool(out, f.drop_through);
        put_f32(out, f.facing);
        put_bool(out, f.melee_pressed);
        match &f.fire {
            None => put_bool(out, false),
            Some(fire) => {
                put_bool(out, true);
                put_vec2(out, fire.dir);
                fire.dir_policy.encode(out);
                put_f32(out, fire.speed);
            }
        }
        put_vec2(out, f.attack_axis);
        for b in [
            f.jump_pressed,
            f.jump_held,
            f.jump_released,
            f.dash_pressed,
            f.interact_pressed,
            f.body_contact_damage_enabled,
            f.shield_held,
            f.special_pressed,
            f.pogo_pressed,
            f.fast_fall_pressed,
            f.fly_toggle_pressed,
            f.projectile_pressed,
            f.projectile_held,
            f.projectile_released,
            f.blink_pressed,
            f.blink_held,
            f.blink_released,
        ] {
            put_bool(out, b);
        }
        put_vec2(out, f.blink_quick_dir);
        put_vec2(out, f.blink_aim_step);
        put_vec2(out, f.aim);
    }

    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        use ambition_characters::actor::control::{ActorControlFrame, ActorFireRequest};
        use ambition_engine_core::reference_frame::GameplayFramePolicy;
        let locomotion = r.vec2()?;
        let velocity_target = r.vec2()?;
        let drop_through = r.bool()?;
        let facing = r.f32()?;
        let melee_pressed = r.bool()?;
        let fire = if r.bool()? {
            Some(ActorFireRequest {
                dir: r.vec2()?,
                dir_policy: GameplayFramePolicy::decode(r)?,
                speed: r.f32()?,
            })
        } else {
            None
        };
        let attack_axis = r.vec2()?;
        let mut flags = [false; 17];
        for f in flags.iter_mut() {
            *f = r.bool()?;
        }
        Some(ambition_characters::brain::ActorControl(
            ActorControlFrame {
                locomotion,
                velocity_target,
                drop_through,
                facing,
                melee_pressed,
                fire,
                attack_axis,
                jump_pressed: flags[0],
                jump_held: flags[1],
                jump_released: flags[2],
                dash_pressed: flags[3],
                interact_pressed: flags[4],
                body_contact_damage_enabled: flags[5],
                shield_held: flags[6],
                special_pressed: flags[7],
                pogo_pressed: flags[8],
                fast_fall_pressed: flags[9],
                fly_toggle_pressed: flags[10],
                projectile_pressed: flags[11],
                projectile_held: flags[12],
                projectile_released: flags[13],
                blink_pressed: flags[14],
                blink_held: flags[15],
                blink_released: flags[16],
                blink_quick_dir: r.vec2()?,
                blink_aim_step: r.vec2()?,
                aim: r.vec2()?,
            },
        ))
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

/// **An accumulating sim clock**, and netcode.md's N3.1 checklist names it: *"`WorldTime`
/// + every sim clock"*. A brain stamps `RememberedActor.last_seen` with it, so a rewind
/// that leaves it running makes every memory look older than it is — which is exactly
/// how `gnu_ton_arena` diverged on `perception_memory` and nothing else.
impl SnapshotState for ambition_actors::features::GameplayElapsed {
    fn encode(&self, out: &mut Vec<u8>) {
        put_f32(out, self.0);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(ambition_actors::features::GameplayElapsed(r.f32()?))
    }
}

/// **The combat slot board**: which attacker holds which approach slot around the
/// target. The slot GEOMETRY is authored (`kind`, `offset`, `holding_offset`); the
/// `assigned_to: Option<String>` is live, and it is a stable id rather than an `Entity`,
/// so it rewinds cleanly. A boss holding a slot it never claimed attacks on a tick it
/// never earned.
impl SnapshotCursor for ambition_combat::slots::CombatSlotsRes {
    fn encode_cursor(&self, out: &mut Vec<u8>) {
        put_u32(out, self.0.slots.len() as u32);
        for slot in &self.0.slots {
            put_opt_str(out, slot.assigned_to.as_deref());
        }
    }
    fn apply_cursor(&mut self, r: &mut Reader<'_>) -> Option<()> {
        let n = r.u32()? as usize;
        // A board of a different SHAPE cannot be faithfully rewound by a cursor: the snapshot's
        // assignments would not line up with the live authored slots, and silently zipping the
        // shorter length leaves live slots untouched or drops snapshot assignments while
        // reporting success (re-audit finding 4). Within a supported window the shape is stable
        // — content does not change, and a cross-room rollback is already refused — so this
        // never fires there; if it ever did, refusing loudly (`None` → `DecodeFailed`) beats a
        // silent partial restore.
        if n != self.0.slots.len() {
            return None;
        }
        for slot in self.0.slots.iter_mut() {
            slot.assigned_to = r.opt_str()?.map(str::to_string);
        }
        Some(())
    }
}

/// **A body's proper-time dilation** (ADR 0011): hitstop, bullet-time, a boss's slow.
/// Every move clock and every brain timer advances on `world_time.entity_dt(scale)`, so
/// a stale scale makes a rewound body live in a differently-paced universe.
impl SnapshotState for ambition_time::ProperTimeScale {
    fn encode(&self, out: &mut Vec<u8>) {
        put_f32(out, self.0);
    }
    fn decode(r: &mut Reader<'_>) -> Option<Self> {
        Some(ambition_time::ProperTimeScale(r.f32()?))
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
        // Two registered diagnostics, plus the active-room (finding 2) and identity-roster
        // (finding 1) pseudo-entries.
        assert_eq!(by_entry.len(), 4);
        assert_eq!(by_entry[0].0, "a");
        assert_ne!(by_entry[0].1, by_entry[1].1);
        assert_eq!(by_entry[2].0, SnapshotRegistry::ACTIVE_ROOM_ENTRY);
        assert_eq!(by_entry[3].0, SnapshotRegistry::ROSTER_ENTRY);
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

    /// A resource that is half authored geometry, half mutable assignment — the
    /// `CombatSlotsRes` shape, in miniature. Its cursor carries only the mutable half.
    #[derive(bevy::ecs::resource::Resource, Debug, PartialEq)]
    struct TestBoard {
        /// Authored. Never in the cursor.
        slots: u32,
        /// Mutable. The only thing the cursor rewinds.
        assigned: u32,
    }

    impl SnapshotCursor for TestBoard {
        fn encode_cursor(&self, out: &mut Vec<u8>) {
            put_u32(out, self.assigned);
        }
        fn apply_cursor(&mut self, r: &mut Reader<'_>) -> Option<()> {
            self.assigned = r.u32()?;
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

        restore(&mut world, &snap, &reg).unwrap();
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
        let report = restore(&mut world, &snap, &reg).unwrap();
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

        let report = restore(&mut world, &snap, &reg).unwrap();
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

    /// **Identity is unique, and restore refuses a world where it is not** (audit H2).
    ///
    /// Two live entities carrying one `SimId` make every by-id lookup pick one at
    /// random — the silent corruption the old "later wins" map delegated upstream.
    /// `duplicate_live_ids` names the collision, and `restore` refuses (panics in every
    /// build) rather than patch an arbitrary one. Poison test for the identity
    /// invariant, in the same commit as the enforcement (poison-test atomicity rule).
    #[test]
    fn restore_refuses_a_world_with_two_entities_of_one_identity() {
        let reg = engine_registry();
        let mut world = sim_world();
        let snap = take(&world, &reg);

        // A SECOND entity claims an id that already exists.
        world.spawn((SimId::placement("boss-1"), kin(Vec2::ZERO, Vec2::ZERO)));

        // The detector names the collision precisely...
        assert_eq!(
            duplicate_live_ids(&mut world),
            vec![("placement:boss-1".to_string(), 2)],
            "the identity-roster check must name the duplicated id and its count"
        );

        // ...and restore refuses rather than corrupt. Suppress the backtrace: this
        // panic is expected, and an alarming trace on a passing test is noise.
        let prev = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let refused = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            restore(&mut world, &snap, &reg)
        }));
        std::panic::set_hook(prev);
        assert!(
            refused.is_err(),
            "restore must refuse a duplicated identity, not silently patch one of the two"
        );
    }

    /// The snapshot's own roster must be unambiguous: `duplicate_ids` surfaces the
    /// duplicates `sim_ids` dedups away, so restore can refuse a malformed snapshot — even
    /// a collision that shares NO registered component row, which a per-entry scan misses
    /// (re-audit finding 3).
    #[test]
    fn a_snapshot_roster_surfaces_its_duplicate_ids() {
        let reg = engine_registry();
        let world = sim_world();
        let good = take(&world, &reg);
        assert!(
            good.duplicate_ids().is_empty(),
            "a snapshot of a unique-identity world has no duplicate rows"
        );

        // Forge the collision a per-component scan is blind to: one id twice in the full
        // roster, sharing no component row. The roster sees identity regardless of which
        // (or how few) components an entity carried.
        let mut malformed = good.clone();
        malformed.roster.push("placement:ghost-dup".to_string());
        malformed.roster.push("placement:ghost-dup".to_string());
        malformed.roster.sort();
        assert_eq!(
            malformed.duplicate_ids(),
            vec!["placement:ghost-dup".to_string()],
            "duplicate_ids must catch a collision that carries no shared component row"
        );
    }

    /// **A zero-component `SimId` entity is snapshot state the roster makes authoritative**
    /// (re-audit finding 1).
    ///
    /// It appears in no component entry, so a restore driven by the old component-derived id
    /// set was blind to it: it despawned the entity if it survived, and dropped it if it had
    /// died. Driven by the roster, restore preserves it (a), reconstructs it (b), despawns a
    /// future one (c) — and the state hash now sees it at all.
    #[test]
    fn a_zero_component_sim_id_entity_is_covered_by_the_roster() {
        let reg = engine_registry();
        let mut world = sim_world();

        let before = reg.hash_world(&world);
        // An entity with an identity and NOTHING the registry knows — the case a
        // per-component id set cannot see.
        world.spawn(SimId::placement("ghost"));
        assert_ne!(
            reg.hash_world(&world),
            before,
            "the state hash is blind to a zero-component identity — finding 1's roster term \
             is missing"
        );

        // (a) It SURVIVES a restore that snapshotted it: not mistaken for a future birth.
        let with_ghost = take(&world, &reg);
        assert!(
            with_ghost.sim_ids().contains(&"placement:ghost"),
            "the snapshot's authoritative id set must include the zero-component entity"
        );
        restore(&mut world, &with_ghost, &reg).unwrap();
        assert!(
            live_ids(&mut world).contains_key("placement:ghost"),
            "restore despawned a zero-component survivor it had snapshotted"
        );

        // (b) It is RECONSTRUCTED when it died inside the window (bare — no room authors it).
        let ghost = *live_ids(&mut world).get("placement:ghost").unwrap();
        world.despawn(ghost);
        let report = restore(&mut world, &with_ghost, &reg).unwrap();
        assert!(
            live_ids(&mut world).contains_key("placement:ghost"),
            "restore did not reconstruct a zero-component entity that died inside the window"
        );
        assert_eq!(report.respawned, 1, "the reconstruction must be reported");

        // (c) A live one the snapshot never knew is despawned as a future birth.
        let without_ghost = take(&sim_world(), &reg);
        restore(&mut world, &without_ghost, &reg).unwrap();
        assert!(
            !live_ids(&mut world).contains_key("placement:ghost"),
            "restore kept a zero-component entity that was born after the snapshot"
        );
    }

    /// **Restore validates the snapshot roster independently of the live world** (re-audit
    /// findings 2 + 3). `take` enforces uniqueness at capture, but a snapshot arriving over
    /// the N3.3 wire was never take-validated. Restore refuses with a RETURNED
    /// `MalformedSnapshot` (corrupt input) — not a panic (reserved for a live-identity bug) —
    /// rather than pick one of the colliding rows, and its dup detection no longer trusts the
    /// caller to have sorted first.
    #[test]
    fn restore_refuses_a_snapshot_whose_roster_is_ambiguous() {
        let reg = engine_registry();
        let mut world = sim_world();

        // `duplicate_ids` no longer trusts the caller to have sorted: the audit's split
        // duplicate in an UNSORTED roster (`["dup", "other", "dup"]`) is still detected
        // (re-audit finding 2), where an adjacent-only scan of the stored order would miss it.
        let mut probe = take(&world, &reg);
        probe.roster = vec![
            "placement:ghost-dup".into(),
            "placement:other".into(),
            "placement:ghost-dup".into(),
        ];
        assert_eq!(
            probe.duplicate_ids(),
            vec!["placement:ghost-dup".to_string()],
            "duplicate_ids must detect a non-adjacent collision in an unsorted roster"
        );

        // The live world is clean; only the (deserialized) SNAPSHOT is corrupt. Restore
        // refuses it as a returned error, having touched nothing.
        let mut snap = take(&world, &reg);
        snap.roster.push("placement:ghost-dup".into());
        snap.roster.push("placement:ghost-dup".into());
        snap.roster.sort();
        match restore(&mut world, &snap, &reg) {
            Err(RestoreError::MalformedSnapshot { .. }) => {}
            other => panic!("restore accepted an ambiguous roster instead of refusing: {other:?}"),
        }
    }

    /// **A snapshot entry whose blob kind disagrees with the registry is refused**
    /// (re-audit finding 2). A component blob arriving under a resource entry — the sort of
    /// corruption a wire format admits — used to be silently SKIPPED by restore's
    /// `find(..).kind` match. `validate_snapshot` catches it before any mutation.
    #[test]
    fn restore_refuses_a_snapshot_with_a_kind_mismatched_entry() {
        let reg = engine_registry();
        let mut world = sim_world();
        let mut snap = take(&world, &reg);

        // Find a resource entry and corrupt its blob into a component blob.
        let (name, slot) = snap
            .entries
            .iter_mut()
            .find_map(|(n, b)| matches!(b, EntryBlob::Resource(_)).then_some((*n, b)))
            .expect("the engine registry has at least one resource entry");
        *slot = EntryBlob::Component(Vec::new());
        let _ = name;

        match restore(&mut world, &snap, &reg) {
            Err(RestoreError::MalformedSnapshot { .. }) => {}
            other => {
                panic!("restore accepted a kind-mismatched entry instead of refusing: {other:?}")
            }
        }
    }

    /// **A reordered snapshot is refused** (third-pass re-audit). `restore` iterates
    /// `snapshot.entries` directly, so a permuted deserialized snapshot is operationally
    /// significant (a resolved codec could resolve before a registered dependency is applied).
    /// `validate_snapshot` now requires the exact registry order.
    #[test]
    fn a_reordered_snapshot_is_rejected() {
        let reg = engine_registry();
        let mut world = sim_world();
        let mut snap = take(&world, &reg);
        assert!(snap.entries.len() >= 2, "need two entries to reorder");
        snap.entries.swap(0, 1);
        match restore(&mut world, &snap, &reg) {
            Err(RestoreError::MalformedSnapshot { .. }) => {}
            other => panic!("restore accepted a reordered snapshot instead of refusing: {other:?}"),
        }
    }

    /// **A resource-cursor absence blob rejects trailing bytes and a non-canonical tag**
    /// (third-pass re-audit). The `false` (absent) path used to remove the resource and report
    /// success WITHOUT exhausting the reader, and `Reader::bool` accepted any nonzero byte as
    /// `true`. Both are corruption a decoder must refuse.
    #[test]
    fn a_resource_cursor_absence_blob_rejects_trailing_bytes_and_a_bad_tag() {
        let mut reg = engine_registry();
        reg.register_resource_cursor::<TestBoard>("test_board");

        fn restore_with_board_blob(
            reg: &SnapshotRegistry,
            blob: Vec<u8>,
        ) -> Result<RestoreReport, RestoreError> {
            let mut world = sim_world(); // TestBoard absent -> tagged `false`
            let mut snap = take(&world, reg);
            for (name, b) in snap.entries.iter_mut() {
                if *name == "test_board" {
                    *b = EntryBlob::Resource(blob.clone());
                }
            }
            restore(&mut world, &snap, reg)
        }

        // Absence tag (0) + trailing bytes: corruption, not a clean removal.
        assert!(
            matches!(
                restore_with_board_blob(&reg, vec![0, 0xAB]),
                Err(RestoreError::DecodeFailed { .. })
            ),
            "trailing bytes after an absence tag were accepted"
        );
        // A non-canonical presence tag (2) is not `true`.
        assert!(
            matches!(
                restore_with_board_blob(&reg, vec![2]),
                Err(RestoreError::DecodeFailed { .. })
            ),
            "a non-canonical presence tag (2) was accepted as `true`"
        );
        // Sanity: the clean absence blob (just the tag) still applies.
        assert!(
            restore_with_board_blob(&reg, vec![0]).is_ok(),
            "a clean absence tag must still remove-and-apply"
        );
    }

    /// **A resolved blob with trailing garbage is refused** (third-pass re-audit). `resolve`
    /// reads only a prefix and cannot itself assert the whole blob was consumed (it holds only
    /// `&mut Reader`); the insert closure now checks `finish()` after a `Some`, so a valid
    /// prefix followed by bytes nobody wrote is `DecodeFailed`, not a silent success.
    #[test]
    fn a_resolved_blob_with_trailing_bytes_is_rejected() {
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
        let mut snap = take(&world, &reg);

        // Append a byte nobody encoded to the boss's `playing` row.
        for (name, blob) in snap.entries.iter_mut() {
            if *name == "playing" {
                if let EntryBlob::Component(rows) = blob {
                    for (id, bytes) in rows.iter_mut() {
                        if id == "placement:boss-1" {
                            bytes.push(0xFF);
                        }
                    }
                }
            }
        }
        match restore(&mut world, &snap, &reg) {
            Err(RestoreError::DecodeFailed { entry, .. }) => assert_eq!(entry, "playing"),
            other => panic!("a resolved blob with trailing garbage was accepted: {other:?}"),
        }
    }

    /// **Restore refuses to reconstruct a dead dynamic entity — cleanly** (re-audit
    /// findings 4 + 5).
    ///
    /// A `SimId::spawned(..)` entity that existed at the snapshot tick and is gone now has
    /// no room to rebuild it and no spawn recipe, so rebuilding it from blobs alone is not
    /// exact. This is a RECONSTRUCTION refusal — the entity DIED inside the window — not a
    /// "birth inside the window": an entity spawned AFTER the snapshot is future-only and
    /// simply despawned. And it is PREFLIGHTED: restore refuses before it mutates the
    /// world, so a would-be-despawned future entity is left standing. (A `placement:` id
    /// with no room record still respawns bare: the headless-fixture path, which
    /// `restore_forgets_the_future_and_remembers_the_dead` covers.)
    #[test]
    fn restore_refuses_to_reconstruct_a_dead_dynamic_entity_without_a_recipe() {
        let reg = engine_registry();
        let mut world = sim_world();

        // A dynamic child (its id contains `/`), present at snapshot time.
        let child = SimId::spawned(&SimId::placement("boss-1"), 3);
        assert!(
            child.as_str().contains('/'),
            "the spawned vocabulary uses `/`"
        );
        world.spawn((child.clone(), kin(Vec2::ZERO, Vec2::ZERO)));
        let snap = take(&world, &reg);

        // It dies inside the window...
        let entity = *live_ids(&mut world).get(child.as_str()).unwrap();
        world.despawn(entity);
        // ...and a fresh future-only entity appears that a MUTATING restore would despawn.
        let future = world
            .spawn((
                SimId::placement("future-canary"),
                kin(Vec2::ZERO, Vec2::ZERO),
            ))
            .id();

        match restore(&mut world, &snap, &reg) {
            Err(RestoreError::UnsupportedDynamicReconstruction { sim_id }) => {
                assert_eq!(sim_id, child.as_str());
            }
            other => {
                panic!("restore did not refuse to reconstruct a dead dynamic entity: {other:?}")
            }
        }

        // The refusal was PREFLIGHTED: the world was not touched, so the future-only
        // entity that a mutating restore would have despawned is still standing (finding 5).
        assert!(
            world.get::<SimId>(future).is_some(),
            "restore despawned a future entity before refusing — the unsupported-\
             reconstruction check is not the clean pre-mutation preflight finding 5 asks for"
        );
    }

    /// **Stale state is measured AFTER reconciliation, not before** (audit H4).
    ///
    /// A future-only entity — not in the snapshot, so `restore` despawns it — carries an
    /// UNREGISTERED component. Measured at the top (the old ordering), its component was
    /// counted as stale: a false positive on an entity about to cease to exist. Measured
    /// over the post-reconciliation roster, it does not appear, because the debt a rewind
    /// leaves behind is the debt on the entities that SURVIVE the rewind.
    #[test]
    fn stale_state_is_measured_after_reconciliation_not_before() {
        let reg = engine_registry();
        let mut world = sim_world();
        let snap = take(&world, &reg);

        // Future-only (a fresh id the snapshot never knew) with an unregistered component.
        world.spawn((
            SimId::placement("future-ghost"),
            kin(Vec2::ZERO, Vec2::ZERO),
            UnregisteredThing(7),
        ));

        let report = restore(&mut world, &snap, &reg).unwrap();
        assert_eq!(report.despawned, 1, "the future-only ghost was despawned");

        let probe = std::any::TypeId::of::<UnregisteredThing>();
        assert!(
            !report
                .stale_components
                .iter()
                .any(|c| c.type_id == Some(probe)),
            "an unregistered component on a DESPAWNED entity leaked into stale_components — \
             stale state was measured before reconciliation (audit H4): {:?}",
            report.stale_components
        );
    }

    /// **A corrupted STANDALONE blob makes restore refuse LOUDLY and transactionally**
    /// (audit M3/S2.5; re-audit finding 5).
    ///
    /// A registered codec that cannot read a blob it was handed is a codec failure. The
    /// old `debug_assert!(false)` dropped the component silently in release builds,
    /// leaving stale state reading as restored. Now restore returns `DecodeFailed`, names
    /// the entry, and — for a standalone codec (`body_kinematics` is a plain component) —
    /// decode-preflights it BEFORE any mutation, so the refusal leaves the world untouched.
    /// A future-only entity a mutating restore would despawn proves it. Poison test,
    /// co-located with the enforcement (atomicity rule).
    #[test]
    fn restore_refuses_a_corrupted_blob_rather_than_leaving_stale_state() {
        let reg = engine_registry();
        let mut world = sim_world();
        let mut snap = take(&world, &reg);

        // Corrupt a PLAIN-component row (`body_kinematics`, a standalone-decodable codec):
        // one byte short, so `decode_one` returns `None` (a truncated blob is rejected, not
        // guessed). Targeting the standalone codec is what exercises the pre-mutation
        // decode preflight rather than the apply-time cursor/resolved path.
        let corrupted = "body_kinematics";
        let mut hit = false;
        for (name, blob) in snap.entries.iter_mut() {
            if *name == corrupted {
                if let EntryBlob::Component(rows) = blob {
                    let (_, b) = rows
                        .iter_mut()
                        .find(|(_, b)| !b.is_empty())
                        .expect("a non-empty body_kinematics row");
                    b.pop();
                    hit = true;
                }
            }
        }
        assert!(hit, "expected a non-empty `{corrupted}` row to corrupt");

        // A future-only entity a MUTATING restore would despawn in its first pass.
        let future = world
            .spawn((
                SimId::placement("future-canary"),
                kin(Vec2::ZERO, Vec2::ZERO),
            ))
            .id();

        match restore(&mut world, &snap, &reg) {
            Err(RestoreError::DecodeFailed { entry, .. }) => assert_eq!(entry, corrupted),
            other => {
                panic!("restore accepted a corrupted blob instead of refusing: {other:?}")
            }
        }

        // Transactional: the standalone decode preflight refused before mutating, so the
        // future entity is still standing (finding 5).
        assert!(
            world.get::<SimId>(future).is_some(),
            "restore despawned a future entity before refusing a corrupted standalone blob \
             — the decode preflight is not transactional (finding 5)"
        );
    }

    /// Taking a snapshot of a restored world yields the identical snapshot. Restore
    /// is idempotent, which is what a rollback window replays across.
    #[test]
    fn take_after_restore_is_the_snapshot_you_restored() {
        let reg = engine_registry();
        let mut world = sim_world();
        let snap = take(&world, &reg);
        restore(&mut world, &snap, &reg).unwrap();
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

        let report = restore(&mut world, &snap, &reg).unwrap();
        assert_eq!(report.patched, 2, "both entities survived and were patched");
        assert_eq!(report.respawned, 0);
        // Not lossless: an unregistered component survives, stale. `restore` now measures
        // the resource term itself; in this crate's tests bevy's `debug` names are off, so
        // the census is unreliable and `lossless()` refuses on that ground alone — which is
        // the point of the census flag (finding 6): it does not falsely succeed blind.
        assert!(!report.lossless());
        assert!(!report.resource_census_reliable);
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

        restore(&mut world, &snap, &reg).unwrap();
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

        let report = restore(&mut world, &snap, &reg).unwrap();
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
        // The authored waypoints (12 bytes) must NOT enter the blob — only the 4-byte segment
        // cursor does. Measure the DELTA the cursor adds (its key + segment), a proxy immune to
        // the identity-roster (finding 1) and resource-cursor presence-tag (finding 4) overhead
        // that the old absolute `< 200` threshold folded in and that now trips it.
        world.entity_mut(boss).remove::<Patrol>();
        let without = take(&world, &reg).size_bytes();
        world.entity_mut(boss).insert(Patrol {
            waypoints: vec![0.0, 10.0, 20.0],
            segment: 1,
        });
        let with = take(&world, &reg).size_bytes();
        assert!(
            with - without < 30,
            "the authored waypoints leaked into the blob: the cursor added {} bytes",
            with - without
        );

        world.entity_mut(boss).get_mut::<Patrol>().unwrap().segment = 2;
        restore(&mut world, &snap, &reg).unwrap();

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
        let report = restore(&mut world, &snap, &reg).unwrap();
        assert_eq!(report.respawned, 1);

        let back = *live_ids(&mut world).get("placement:boss-1").unwrap();
        assert!(
            world.entity(back).get::<Patrol>().is_none(),
            "a cursor has nothing to apply itself to on a naked respawn — it must not \
             invent a path"
        );
    }

    /// **A resource cursor tags its presence and reports an incomplete restore** (re-audit
    /// finding 4).
    ///
    /// An absent resource and a present-but-empty cursor used to both encode to `[]`, so
    /// restore could not tell "it did not exist at the snapshot" from "its cursor was empty",
    /// and a snapshot-present resource that was gone at restore silently applied nothing while
    /// reporting success. The presence tag closes both.
    #[test]
    fn a_resource_cursor_tags_presence_and_reports_an_absent_target() {
        let mut reg = engine_registry();
        reg.register_resource_cursor::<TestBoard>("test_board");

        // (a) Present at snapshot, GONE at restore: a cursor cannot rebuild a resource from
        // nothing, so the restore is incomplete — reported, not swallowed as success.
        let mut world = sim_world();
        world.insert_resource(TestBoard {
            slots: 4,
            assigned: 2,
        });
        let snap = take(&world, &reg);
        world.remove_resource::<TestBoard>();
        let report = restore(&mut world, &snap, &reg).unwrap();
        assert_eq!(
            report.resource_cursors_unresolved, 1,
            "a snapshot-present resource absent at restore must be reported"
        );
        // The link to losslessness, isolated (a headless lib build has no reliable census, so
        // `report.lossless()` is false for that reason alone — pin the term itself instead).
        let probe = RestoreReport {
            resource_cursors_unresolved: 1,
            resource_census_reliable: true,
            ..RestoreReport::default()
        };
        assert!(
            !probe.lossless(),
            "an unresolved resource cursor must deny losslessness on its own"
        );

        // (b) ABSENT at snapshot, PRESENT at restore: the tag lets restore remove a resource
        // born after the snapshot, where the old empty-blob no-op left it standing.
        let mut world = sim_world();
        let absent = take(&world, &reg); // TestBoard never inserted -> tagged absent
        world.insert_resource(TestBoard {
            slots: 4,
            assigned: 9,
        });
        restore(&mut world, &absent, &reg).unwrap();
        assert!(
            world.get_resource::<TestBoard>().is_none(),
            "restore did not remove a resource that did not exist at the snapshot tick"
        );

        // (c) Present on BOTH sides: the cursor rewinds the mutable half exactly, nothing
        // reported unresolved.
        let mut world = sim_world();
        world.insert_resource(TestBoard {
            slots: 4,
            assigned: 2,
        });
        let snap = take(&world, &reg);
        world.get_resource_mut::<TestBoard>().unwrap().assigned = 7;
        let report = restore(&mut world, &snap, &reg).unwrap();
        assert_eq!(
            world.get_resource::<TestBoard>().unwrap().assigned,
            2,
            "the cursor rewound the mutable half"
        );
        assert_eq!(report.resource_cursors_unresolved, 0);
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

        // The resolved move must encode only a REFERENCE (its id + progress), never the
        // catalog it resolves `power` out of. Measure the DELTA the move adds to the
        // snapshot — a proxy immune to identity-roster and active-room overhead, which is
        // identical with and without the move (re-audit finding 3 added the roster, so an
        // absolute threshold no longer isolates the blob).
        world.entity_mut(boss).remove::<Playing>();
        let without_move = take(&world, &reg).size_bytes();
        world.entity_mut(boss).insert(Playing {
            power: 20.0,
            id: "smash".into(),
            t: 0.25,
        });
        let snap = take(&world, &reg);
        assert!(
            snap.size_bytes() - without_move < 40,
            "the catalog leaked into the blob: the resolved move added {} bytes",
            snap.size_bytes() - without_move
        );

        // The move ends. The component goes away.
        world.entity_mut(boss).remove::<Playing>();
        restore(&mut world, &snap, &reg).unwrap();
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
        restore(&mut world, &empty, &reg).unwrap();
        assert!(world.entity(boss).get::<Playing>().is_some());
    }

    /// A name the content no longer knows leaves the component OFF, rather than
    /// resolving to a plausible neighbour. Impossible in a rollback; loud in a save.
    ///
    /// **And it is not lossless** (re-audit finding 3): the registered `Playing` row the
    /// snapshot carried did not come back, so restore reports one unapplied row and
    /// `lossless()` is false — where the old bare-`true` resolved insert reported success for
    /// a row that never returned. Dropping the component stays correct (a save whose content
    /// changed should not guess); claiming the restore was complete does not.
    #[test]
    fn a_resolved_component_that_names_missing_content_is_dropped_and_denies_lossless() {
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
        let report = restore(&mut world, &snap, &reg).unwrap();
        assert!(
            world.entity(boss).get::<Playing>().is_none(),
            "a name the content forgot must not resolve to a neighbour"
        );
        assert_eq!(
            report.unapplied_rows, 1,
            "the dropped resolved row must be counted, not swallowed as success"
        );
        assert!(
            !report.lossless(),
            "a restore that silently dropped registered state is not lossless (finding 3)"
        );
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

        restore(&mut world, &snap, &reg).unwrap();
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
        restore(&mut world, &snap, &reg).unwrap();
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
