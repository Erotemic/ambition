//! **`SimId` — the one identity vocabulary for snapshot, replay, and netcode.**
//!
//! `docs/planning/engine/netcode.md` N3.1, *Identity & scope* (pinned 2026-07-06):
//!
//! > *"One identity vocabulary, shared with SimView. Every snapshot-registered
//! > entity carries a `SimId` — the EXISTING stable ids, not a new system: actors
//! > use `ActorConfig.id` (== LDtk iid; placement identity), player bodies use
//! > their slot, dynamically-spawned sim entities (projectiles, dropped items,
//! > spawned adds) get a deterministic sequence id minted at spawn (`(spawner
//! > SimId, per-spawner counter)` — deterministic because the sim is;
//! > wall-clock/Entity-index ids are forbidden). … `Entity` values never appear in
//! > a blob."*
//!
//! This module is that vocabulary. It is deliberately NOT a new id scheme: every
//! constructor wraps an identity the sim already has.
//!
//! ## Why an entity index is not an identity
//!
//! `Entity` is a slot in an allocator. Two sims fed the same inputs can hand the
//! same body different indices — spawn order across archetypes is not part of the
//! game's state. A snapshot keyed on `Entity` restores into a different world; a
//! desync hash keyed on `Entity` cries wolf every run. So a `SimId` is a *string
//! derived from the game's own facts*, and the three constructors below are the
//! only three facts there are.
//!
//! ## Why a `String` and not a `u64` hash
//!
//! Because a desync report has to be readable. `feature:BossSpawn-4308/3` names a
//! projectile fired by a boss; `9f3ac21e` names nothing. The ids are compared and
//! sorted, never hashed for lookup, so the cost is a `strcmp` on a path that runs
//! once per snapshot, not once per frame.

use bevy::prelude::Component;

/// A stable, deterministic identity for one simulated entity.
///
/// Ordered, so a snapshot's entity rows sort into a canonical sequence regardless
/// of the archetype layout Bevy's `Query` happened to walk (see
/// `ambition_runtime::snapshot::hash_entities_by_key`).
#[derive(Component, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SimId(String);

impl SimId {
    /// An authored placement: an LDtk iid, a `FeatureId`, an actor's config id.
    /// The identity the MAP gave it, which is the identity a save file already
    /// uses.
    pub fn placement(id: &str) -> Self {
        Self(format!("placement:{id}"))
    }

    /// A player body, by its slot. Not by which entity happens to hold the brain:
    /// possession transfers `Brain::Player(slot)` between bodies, and the body's
    /// identity does not travel with it.
    pub fn player_slot(slot: u8) -> Self {
        Self(format!("slot:{slot}"))
    }

    /// A dynamically-spawned sim entity: a projectile, a dropped item, a summoned
    /// add. `(spawner SimId, per-spawner counter)` — deterministic because the sim
    /// is, and legible because the parent is right there in the string.
    ///
    /// The counter must come from a [`SimIdCounter`] on the SPAWNER, never from a
    /// global one: a global counter couples two unrelated spawners, so a
    /// projectile fired on tick 5 would get a different id depending on whether a
    /// boss summoned an add on tick 4.
    pub fn spawned(spawner: &SimId, sequence: u64) -> Self {
        Self(format!("{}/{sequence}", spawner.0))
    }

    /// The raw string. Sorted and compared; never parsed.
    /// Rebuild an id from a snapshot blob's key.
    ///
    /// The ONLY way to make a `SimId` from a raw string, and it is named for its
    /// one caller (`ambition_runtime::snapshot::restore`). Everything else must
    /// go through [`SimId::placement`] / [`SimId::player_slot`] /
    /// [`SimId::spawned`], because those three ARE the vocabulary — a fourth way
    /// to mint one is a fourth namespace to collide in.
    pub fn from_snapshot(raw: String) -> Self {
        Self(raw)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SimId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A spawner's per-spawner sequence counter. Lives on the spawner ENTITY, so it is
/// snapshot state like everything else, and so two spawners never share a stream.
///
/// Wrapping is not handled and does not need to be: at 60 Hz, a single body would
/// have to emit one entity per tick for nine billion years.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SimIdCounter(pub u64);

impl SimIdCounter {
    /// Take the next sequence number. `&mut self`, because minting an id is a
    /// state change the snapshot has to see — two sims that minted a different
    /// number of ids are not in the same state, even if nothing else differs.
    pub fn next(&mut self) -> u64 {
        let n = self.0;
        self.0 += 1;
        n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_three_constructors_never_collide() {
        assert_ne!(SimId::placement("0"), SimId::player_slot(0));
        assert_ne!(SimId::placement("slot:0"), SimId::player_slot(0));
    }

    /// A spawned id names its parent, so a desync report reads as a sentence.
    #[test]
    fn a_spawned_id_carries_its_spawners_identity() {
        let boss = SimId::placement("BossSpawn-4308");
        let shot = SimId::spawned(&boss, 3);
        assert_eq!(shot.as_str(), "placement:BossSpawn-4308/3");
        assert!(shot.as_str().starts_with(boss.as_str()));
    }

    /// Nested spawns keep nesting. A minion's projectile is legible as such.
    #[test]
    fn spawned_ids_nest() {
        let boss = SimId::placement("b");
        let minion = SimId::spawned(&boss, 0);
        let shot = SimId::spawned(&minion, 7);
        assert_eq!(shot.as_str(), "placement:b/0/7");
    }

    /// **Per-spawner, never global.** A global counter couples unrelated spawners:
    /// a projectile fired on tick 5 would take a different id depending on whether
    /// some boss summoned an add on tick 4. Two counters, two streams.
    #[test]
    fn two_spawners_mint_independent_sequences() {
        let (a, b) = (SimId::placement("a"), SimId::placement("b"));
        let mut ca = SimIdCounter::default();
        let mut cb = SimIdCounter::default();

        assert_eq!(SimId::spawned(&a, ca.next()).as_str(), "placement:a/0");
        assert_eq!(SimId::spawned(&b, cb.next()).as_str(), "placement:b/0");
        assert_eq!(SimId::spawned(&a, ca.next()).as_str(), "placement:a/1");
        assert_eq!(cb.0, 1, "b's stream did not advance when a fired");
    }

    /// Ids sort, so a snapshot's rows have a canonical order that does not depend
    /// on the archetype layout a `Query` happened to walk.
    #[test]
    fn sim_ids_order_canonically() {
        let mut ids = vec![
            SimId::player_slot(1),
            SimId::placement("z"),
            SimId::placement("a"),
            SimId::player_slot(0),
        ];
        ids.sort();
        let seen: Vec<&str> = ids.iter().map(|i| i.as_str()).collect();
        assert_eq!(seen, ["placement:a", "placement:z", "slot:0", "slot:1"]);
    }
}
