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
//! **`take` / `restore` are NOT here.** N3.1's decision (3) is *"restore =
//! despawn-registered + respawn from blobs"*, which requires decision's identity
//! pin: *"every snapshot-registered entity carries a `SimId`."* No `SimId` exists
//! in the tree yet. Shipping a `restore` that keyed on `Entity` would violate the
//! doc's own rule ("`Entity` values never appear in a blob") and would have to be
//! torn out. So restore waits on the identity migration, and the canary — which
//! needs only the hash — does not.
//!
//! ## The hash
//!
//! FNV-1a over `(entry name, sorted (key, bytes) pairs)`, in registration order.
//! Deliberately not `std::hash::DefaultHasher`: `RandomState` is seeded per
//! process, so two runs of the same binary would disagree — which is exactly the
//! bug class ADR 0023 exists to prevent, and the last thing a desync canary should
//! be built on.

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

/// One registered piece of sim state.
///
/// `hash` walks the world and feeds the hasher. It MUST be order-independent of
/// archetype layout: an entry that iterates entities sorts by a stable key first
/// (see [`hash_entities_by_key`]).
struct StateEntry {
    name: &'static str,
    hash: fn(&World, &mut StateHasher),
}

/// The opt-in registry of sim state (N3.1 decision 1). Each sim crate's plugin
/// registers what it owns; nothing else is snapshot state, by definition.
#[derive(Default)]
pub struct SnapshotRegistry {
    entries: Vec<StateEntry>,
}

impl SnapshotRegistry {
    /// Register one piece of sim state under a stable name.
    ///
    /// The NAME is hashed too, so adding an entry changes the hash — which is
    /// correct: a canary comparing two builds with different registries is
    /// comparing two different definitions of "the sim".
    pub fn register(&mut self, name: &'static str, hash: fn(&World, &mut StateHasher)) {
        debug_assert!(
            !self.entries.iter().any(|e| e.name == name),
            "sim-state entry `{name}` registered twice"
        );
        self.entries.push(StateEntry { name, hash });
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

    /// **N0.4's per-tick hash of the whole registered sim state.**
    pub fn hash_world(&self, world: &World) -> u64 {
        let mut h = StateHasher::default();
        for entry in &self.entries {
            h.write_str(entry.name);
            (entry.hash)(world, &mut h);
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
                (entry.hash)(world, &mut h);
                (entry.name, h.finish())
            })
            .collect()
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
    registry.register("sim_tick", |world, h| {
        let tick = world
            .get_resource::<ambition_time::SimTick>()
            .map_or(0, |t| t.0);
        h.write_u64(tick);
    });

    registry.register("world_time", |world, h| {
        if let Some(t) = world.get_resource::<ambition_time::WorldTime>() {
            h.write_f32(t.scaled_dt);
        }
    });

    registry.register("body_kinematics", |world, h| {
        let mut rows: Vec<(String, Vec<u8>)> = Vec::new();
        let mut q = world.try_query::<(
            &ambition_platformer_primitives::body::BodyKinematics,
            Option<&ambition_combat::components::FeatureId>,
            Option<&ambition_platformer_primitives::markers::PrimaryPlayer>,
        )>();
        let Some(mut q) = q.take() else {
            return;
        };
        for (kin, feature_id, primary) in q.iter(world) {
            // The two stable identities that exist today. A body with neither is
            // NOT hashed, and that silence is the SimId migration, named in the
            // module docs rather than papered over with an `Entity` index.
            let key = match (feature_id, primary) {
                (Some(id), _) => format!("feature:{}", id.0),
                (None, Some(_)) => "player:slot0".to_string(),
                (None, None) => continue,
            };
            let mut bytes = Vec::with_capacity(20);
            for v in [kin.pos.x, kin.pos.y, kin.vel.x, kin.vel.y, kin.facing] {
                bytes.extend_from_slice(&canonical_f32_bits(v).to_le_bytes());
            }
            rows.push((key, bytes));
        }
        hash_entities_by_key(h, rows);
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
        a.register("alpha", |_, h| h.write_u64(1));
        let mut b = SnapshotRegistry::default();
        b.register("beta", |_, h| h.write_u64(1));
        assert_ne!(a.hash_world(&world), b.hash_world(&world));
        assert_eq!(a.len(), 1);
        assert_eq!(a.names().collect::<Vec<_>>(), ["alpha"]);
    }

    #[test]
    fn per_entry_hashes_localize_a_divergence() {
        let world = World::new();
        let mut reg = SnapshotRegistry::default();
        reg.register("a", |_, h| h.write_u64(1));
        reg.register("b", |_, h| h.write_u64(2));
        let by_entry = reg.hash_by_entry(&world);
        assert_eq!(by_entry.len(), 2);
        assert_eq!(by_entry[0].0, "a");
        assert_ne!(by_entry[0].1, by_entry[1].1);
    }
}
