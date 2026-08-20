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
//! derived from the game's own facts*, and the constructors below are the only
//! facts there are.
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
/// `ambition_platformer2d_runtime::rollback` checksum projections).
///
/// ## An identity ALWAYS carries the stream its descendants are minted from
///
/// [`SimIdCounter`] is not a separate opt-in fact: [`SimId::spawned`] is the only
/// way to name a dynamically-spawned entity, and it needs a counter *on the
/// spawner*. So "identified" and "able to be descended from" are the same
/// condition, and the pairing is structural rather than remembered at each mint
/// site.
///
/// ⛔ **it was remembered at two of the six sites that mint an id.**
/// `ensure_sim_id` and Sanic's scattered rings inserted the pair; the
/// construction executor — which is how every authored actor, including every
/// boss, reaches the world — inserted the `SimId` alone. Because `ensure_sim_id`
/// is filtered `Without<SimId>` it then skipped those bodies entirely, so they
/// were never backfilled. `apply_summon_effects` requires both, so **the gradient
/// sentinel's Minima Trap warned and summoned nothing** — a shipped boss with a
/// dead special. Measured 2026-08-08 in the real app on `sandbox:basement_boss`:
/// `sim=placement:BossSpawn-0158 counter=None`, no minion, no "Puppy Slug".
///
/// `#[require]` rather than an insert in the executor: the executor is one site
/// of six, and repairing it alone leaves the same hole at the rest — the
/// split-offspring path and the strike-volume path each mint a bare `SimId` too.
/// A required component makes the invariant a property of the TYPE, so a future
/// mint site cannot omit it.
///
/// ⚠ **it never overwrites.** A required component is supplied only when absent,
/// so a snapshot restore that puts back `SimIdCounter(7)` keeps 7, and nothing
/// double-mints on rollback. `Default` is `0`, which is what a freshly built body
/// has anyway.
#[derive(Component, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[require(SimIdCounter)]
pub struct SimId(String);

/// Make one authored segment safe to concatenate.
///
/// `/` is the structural separator between segments and `:` between a namespace
/// and its body, so an authored id containing either would produce a string that
/// parses — to a reader, or to a future tool — as a different identity. Without
/// this, `placement("giant/0")` and `spawned(placement("giant"), 0)` are the SAME
/// STRING (GPT 5.6, 2026-07-27), and a collision there merges two distinct
/// entities on restore, misattributes a reference probe, or despawns the wrong
/// body.
///
/// Percent-escaping rather than rejection, because an authored id with a slash in
/// it is not a mistake the engine gets to veto — LDtk hands over whatever the
/// designer typed. `%` is escaped first so the encoding stays reversible, which is
/// what makes it injective: distinct inputs cannot produce the same output.
///
/// The common case costs nothing. An id with no reserved character passes through
/// unchanged, so the ids in a desync report still read as sentences.
fn escape_segment(segment: &str) -> std::borrow::Cow<'_, str> {
    if !segment.contains(['%', '/', ':']) {
        return std::borrow::Cow::Borrowed(segment);
    }
    let mut escaped = String::with_capacity(segment.len() + 8);
    for ch in segment.chars() {
        match ch {
            '%' => escaped.push_str("%25"),
            '/' => escaped.push_str("%2F"),
            ':' => escaped.push_str("%3A"),
            other => escaped.push(other),
        }
    }
    std::borrow::Cow::Owned(escaped)
}

impl SimId {
    /// An authored placement: an LDtk iid, a `FeatureId`, an actor's config id.
    /// The identity the MAP gave it, which is the identity a save file already
    /// uses.
    pub fn placement(id: &str) -> Self {
        Self(format!("placement:{}", escape_segment(id)))
    }

    /// A player body, by its slot. Not by which entity happens to hold the brain:
    /// possession transfers `DrivingParticipant(slot)` between bodies, and the body's
    /// identity does not travel with it.
    pub fn player_slot(slot: u8) -> Self {
        Self(format!("slot:{slot}"))
    }

    /// An encounter AUTHORITY entity (E11), by its encounter id (the LDtk
    /// trigger id for a wave arena, the boss placement id for a wrap). Its own
    /// namespace on purpose: a boss wrap's id IS the boss's placement id, and
    /// the boss BODY already owns `placement:{id}` — orchestration and body
    /// are two rows, not one.
    pub fn encounter(id: &str) -> Self {
        Self(format!("encounter:{}", escape_segment(id)))
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

    /// A strike volume: the transient hitbox a move's active window opens.
    ///
    /// DERIVED, not sequenced — which is why it is not [`Self::spawned`]. A
    /// counter would be wrong here: the box is opened and closed by window
    /// membership, so the same volume of the same window of the same move
    /// re-opens many times over a match and each re-open would mint a new number.
    /// `(owner, move, window, volume)` determines it completely, and at most one
    /// box per tuple is ever live, so the derived form is both stable and unique.
    ///
    /// Why it needs an id at all: these carry rollback state (`Hitbox`,
    /// `StrikeVolume`, `HitboxHits`) whose probes project through the CARRIER's
    /// identity. With no id every anonymous carrier folded to the same constant,
    /// so two simultaneous hitboxes with SWAPPED owners hashed identically — the
    /// exact permutation the pair projection was added to catch.
    pub fn strike_volume(owner: &SimId, move_id: &str, window: usize, volume: usize) -> Self {
        Self(format!(
            "{}/strike/{}/w{window}/v{volume}",
            owner.0,
            escape_segment(move_id)
        ))
    }

    /// Rebuild an id from a snapshot blob's key.
    ///
    /// The ONLY way to make a `SimId` from a raw string, and it is named for its
    /// GGRS entity recreation and deterministic construction. Everything else must
    /// go through [`SimId::placement`] / [`SimId::player_slot`] /
    /// [`SimId::spawned`] / [`SimId::encounter`], because those ARE the
    /// vocabulary — another way to mint one is another namespace to collide in.
    pub fn from_snapshot(raw: String) -> Self {
        Self(raw)
    }

    /// The raw string: sorted, compared, and printed.
    ///
    /// **Not parsed.** The spelling is a legibility convenience — it exists so a
    /// desync report reads as a sentence — and nothing may recover a fact from
    /// it. Provenance in particular is
    /// [`SpawnOrigin`](crate::construction::SpawnOrigin), a component the entity
    /// carries, precisely so that changing this format cannot silently change
    /// what reconstruction believes. (This doc used to make that claim while
    /// `heal_projectile_owners` split the string on `/`; the claim is true now.)
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
/// **Required by [`SimId`]** — every identified entity is a potential spawner, and
/// an id whose descendants have nowhere to draw a sequence from is only half an
/// identity. No mint site inserts this by hand; see `SimId`'s docs for the shipped
/// boss whose summon died of exactly that omission.
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
    fn the_constructors_never_collide() {
        assert_ne!(SimId::placement("0"), SimId::player_slot(0));
        assert_ne!(SimId::placement("slot:0"), SimId::player_slot(0));
        // A boss WRAP and the boss BODY share the raw id string but live in
        // different namespaces (orchestration vs body).
        assert_ne!(SimId::encounter("boss_1"), SimId::placement("boss_1"));
    }

    /// **The encoding is INJECTIVE: distinct constructions, distinct strings.**
    ///
    /// `the_constructors_never_collide` above checks three hand-picked pairs, and
    /// that is what let the real collision through: the constructors concatenated
    /// unescaped segments, so `placement("giant/0")` and
    /// `spawned(placement("giant"), 0)` produced the SAME STRING (GPT 5.6,
    /// 2026-07-27). Two distinct entities with one identity merge on restore,
    /// misattribute a reference probe, and can despawn each other.
    ///
    /// So this enumerates a cross-product of adversarial segments — every one
    /// containing a separator this format uses — and asserts the whole set maps to
    /// distinct strings. Adding a constructor without extending this list leaves
    /// the new one unchecked, which is the honest limit of the approach; adding a
    /// RESERVED CHARACTER without escaping it fails here immediately.
    #[test]
    fn the_id_encoding_is_injective_over_adversarial_segments() {
        let segments = [
            "giant", "giant/0", "0", "a/b", "a%2Fb", "%", "::", "slot:0", "strike", "w0",
        ];
        let mut minted: Vec<(String, String)> = Vec::new();
        for segment in segments {
            minted.push((
                format!("placement({segment:?})"),
                SimId::placement(segment).0,
            ));
            minted.push((
                format!("encounter({segment:?})"),
                SimId::encounter(segment).0,
            ));
            for sequence in 0..3u64 {
                let parent = SimId::placement(segment);
                minted.push((
                    format!("spawned(placement({segment:?}), {sequence})"),
                    SimId::spawned(&parent, sequence).0,
                ));
            }
            for other in segments {
                let owner = SimId::placement(other);
                minted.push((
                    format!("strike_volume(placement({other:?}), {segment:?}, 0, 0)"),
                    SimId::strike_volume(&owner, segment, 0, 0).0,
                ));
            }
        }
        for slot in 0..4u8 {
            minted.push((format!("player_slot({slot})"), SimId::player_slot(slot).0));
        }

        let mut seen: std::collections::BTreeMap<String, String> =
            std::collections::BTreeMap::new();
        for (how, id) in minted {
            if let Some(first) = seen.insert(id.clone(), how.clone()) {
                panic!(
                    "two DIFFERENT constructions produced the same identity `{id}`:\n  \
                     {first}\n  {how}\nTwo entities sharing one SimId merge on restore."
                );
            }
        }
    }

    /// The common case still reads as a sentence. Escaping that fired on every id
    /// would trade a real bug for an unreadable desync report, which is a bad
    /// trade — the format exists to be read.
    #[test]
    fn an_ordinary_id_is_untouched_by_escaping() {
        assert_eq!(
            SimId::placement("BossSpawn-4308").as_str(),
            "placement:BossSpawn-4308"
        );
        assert_eq!(
            SimId::spawned(&SimId::placement("BossSpawn-4308"), 3).as_str(),
            "placement:BossSpawn-4308/3"
        );
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
