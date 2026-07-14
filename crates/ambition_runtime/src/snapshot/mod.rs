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

// `body_clusters as bc` is used by `register_engine_sim_state` (below) and,
// separately, by the codec impls in `codecs.rs`, which re-declare it.
use ambition_engine_core::body_clusters as bc;

// D-B split: the registry, restore/reconciliation, and per-type codecs live in
// sibling files. Their public items are re-exported so `snapshot::<Item>` paths
// (and `tests.rs`'s `use super::*`) are unchanged by the relocation.
mod codecs;
mod motion_codec;
mod registry;
mod restore;

pub use codecs::*;
pub use registry::*;
pub use restore::*;

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
/// `resolve` could not DECODE its blob — it was truncated or non-canonical. This
/// is distinct from the authored content being absent (`Ok(None)`): a decode
/// failure is a corrupt/incompatible wire input (→ `ApplyOutcome::DecodeFailed`,
/// which aborts the restore), while absent content is a legitimate change (→
/// `ApplyOutcome::Unapplied`, which drops the component and denies `lossless()`).
/// A named error rather than `()` so the `Result` reads at the call site and
/// clippy's `result_unit_err` stays quiet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolveDecodeError;

pub trait SnapshotResolve: Send + Sync + 'static {
    /// The choice, not the content: an id, a clock, a flag.
    fn encode_ref(&self, out: &mut Vec<u8>);
    /// Rebuild by resolving that choice against the authored data the entity still
    /// carries. Three outcomes, kept DISTINCT (the resolved-codec residual, now
    /// closed):
    /// - `Ok(Some(value))` — the choice resolved against present content.
    /// - `Ok(None)` — the authored content the choice referenced is GONE (a respawn
    ///   / content change). Honest; [`RestoreReport::respawned`] and `Unapplied`
    ///   report it, and it denies `lossless()`. NOT a decode failure.
    /// - `Err(ResolveDecodeError)` — the BLOB itself is malformed (a `Reader`
    ///   primitive returned `None`: truncated, or a non-canonical bool tag). A
    ///   corrupt wire input, mapped to `ApplyOutcome::DecodeFailed`.
    ///
    /// Decode the blob's leading reference FIRST, then look up the content, so a
    /// truncated blob is `Err` regardless of whether the content is present.
    fn resolve(
        entity: &bevy::ecs::world::EntityWorldMut<'_>,
        r: &mut Reader<'_>,
    ) -> Result<Option<Self>, ResolveDecodeError>
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

/// One `Messages<M>` buffer the rollback has to reckon with.
///
/// Lives in `mod.rs` (not `registry.rs`, which owns `SnapshotRegistry`) because
/// `restore` in `restore.rs` reads `clear` on rewind: a parent-module type is
/// visible — private fields and all — to every submodule, the same way `StateEntry`
/// and `EntryKind` are.
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
    let active_room = ambition_platformer_primitives::lifecycle::session_world_component::<ambition_world::rooms::RoomSet>(world)
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
    // A worn `HostCode` persona derives its effective ActionSet from this
    // source. It is mutable progression/dev state, not authored configuration;
    // rewinding identity without it reconstructs the wrong kit.
    registry.register_component::<bc::BodyAbilities>("body_abilities");
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
    // The canonical playable-persona identity. A restore patches it onto the
    // survivor; the identity/ability Changed<> derive re-applies gameplay and
    // the presentation binder re-applies the visual the following tick.
    registry.register_component::<ambition_characters::actor::WornCharacter>("worn_character");
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
    // The body's explicit movement policy (identity + authored params +
    // policy-private state). The current environmental frame is NOT model
    // state: restore re-resolves it from the live environment.
    registry.register_component::<ambition_engine_core::MotionModel>("motion_model");
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

    // ADR 0024 frame law: the per-body resolved frame is transient environment
    // output, recomputed by the frame resolution phase every tick from the
    // restored world — snapshotting it would freeze an environmental fact.
    registry.declare_derived::<ambition_platformer_primitives::frame_env::ResolvedMotionFrame>(
        "published by the frame resolution phase every tick from the live environment",
    );

    // ADR 0024 O4: the semantic maneuver projection is rewritten from the
    // body's (snapshotted) `MotionModel` after every movement step.
    registry.declare_derived::<ambition_engine_core::BodyMotionFacts>(
        "republished from the body's movement policy after every movement step",
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

fn canonical_f32_bits(v: f32) -> u32 {
    if v.is_nan() {
        f32::NAN.to_bits()
    } else {
        v.to_bits()
    }
}

#[cfg(test)]
mod tests;
