//! Stable deterministic identity for snapshot, replay, and netcode.
//!
//! Ids come from gameplay identity, never Bevy `Entity` allocation or wall time.
//! Dynamic descendants use `(spawner SimId, per-spawner counter)`, and readable
//! strings keep desync reports diagnosable.

use bevy::prelude::Component;

/// Stable ordered identity for one simulated entity.
///
/// Ordering gives snapshots a canonical entity sequence. Every `SimId` also
/// requires a [`SimIdCounter`], so any identified entity can mint deterministic
/// descendants. Required components do not overwrite restored counter values.
#[derive(Component, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[require(SimIdCounter)]
pub struct SimId(String);

/// Percent-escape structural separators in one authored id segment.
///
/// Escaping `%`, `/`, and `:` keeps concatenated identities injective while
/// preserving ordinary authored ids verbatim for readable diagnostics.
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

    /// A DEATH DROP: the object a defeated body left behind, by which drop it is.
    ///
    /// ⭐ DERIVED, not sequenced, and the drop road's own provenance already
    /// argues why: a body dies once and leaves at most one drop of each kind, so
    /// `(parent, kind)` determines it completely and a counter would number a
    /// thing that cannot repeat. A counter is rollback state and a derivation is
    /// not, so this stays stable across a rewind for free — which matters here
    /// more than for [`Self::strike_volume`], because the drop falls out of a
    /// death the rollback host may re-simulate.
    ///
    /// ⛔ ITS OWN `drop` SEGMENT, and that is a collision and not a preference:
    /// a body mints its projectiles and summons through [`Self::spawned`] as
    /// `{parent}/{n}` from its own [`SimIdCounter`], so a bare sequence here
    /// would eventually spell the same string as one of them.
    ///
    /// ⚠ ONLY A DROP THAT BECOMES A CARRIABLE OBJECT TAKES ONE. A drop that
    /// grants a quantity — a coin, a heart, an ability pickup — is recorded by
    /// `OwnedItems` and restored wholesale from it, so an identity there would be
    /// a second authority over a fact the bag already settles.
    pub fn death_drop(parent: &SimId, kind: &str) -> Self {
        Self(format!("{}/drop/{}", parent.0, escape_segment(kind)))
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

    /// An object the MATCH itself put into the world — a spawned item.
    ///
    /// ⭐ DERIVED, not sequenced, for the same reason [`Self::strike_volume`]
    /// is: `(match, tick)` determines it completely and at most one spawn per
    /// tick exists, so a counter would be a second authority on a fact the tick
    /// already settles. That matters more here than convenience — the pickup
    /// road mints under the THROWER and takes a `SimIdCounter` from it, and a
    /// match-level spawner has no thrower to take one from. Deriving is not a
    /// workaround for that; it is the reason the problem does not arise.
    ///
    /// ⛔ THE TICK IS NOT DECORATION. Two items spawned on different ticks are
    /// different objects and a save must be able to tell them apart; two spawned
    /// on the SAME tick would be the same object, which is why the caller must
    /// not draw twice in one tick without the schedule saying so.
    pub fn match_spawn(activation: u64, tick: u64) -> Self {
        Self(format!("match:{activation}/spawn/{tick}"))
    }

    /// An entity of which the world holds AT MOST ONE per `key` — a placed
    /// portal per channel, a gameplay session's world root per activation.
    ///
    /// DERIVED like [`Self::strike_volume`], and for the same reason: the key
    /// determines it completely, and re-opening it (a portal re-placed on the
    /// same wall) is the same logical object, not a new one. `kind` is its own
    /// namespace so `portal:blue` can never collide with a placement iid.
    ///
    /// Why anchored singletons need an id at all: they carry rollback state,
    /// and a census of "every anchored entity has one stable identity" is only
    /// worth running if it admits no waivers.
    pub fn singleton(kind: &str, key: &str) -> Self {
        Self(format!("{}:{}", escape_segment(kind), escape_segment(key)))
    }

    /// A piece of durable ROOM GEOMETRY, by its [`GeoId`].
    ///
    /// ⭐ ITS OWN NAMESPACE BECAUSE GEOMETRY IS NOT A PLACEMENT. A `GeoId` is a
    /// source plus an ordinal — one LDtk placement can emit several blocks, and a
    /// tile layer emits blocks that have no placement iid at all — so folding it
    /// into [`Self::placement`] would let block 1 of placement `p` and the actor
    /// placed at `p` claim the same identity.
    ///
    /// ⚠ [`GeoSource::Anon`] IS FIXTURE GEOMETRY AND IS NOT DURABLY NAMED. The
    /// authoring pipeline never emits it, so it appears only in tests — but two
    /// anon blocks spell the SAME id here, which is why a population that sorts
    /// by identity should also assert
    /// [`no_two_candidates_share_an_identity`](crate::sim_selection::no_two_candidates_share_an_identity)
    /// rather than merely that everyone has one.
    pub fn geometry(geo: &ambition_platformer2d_core::GeoId) -> Self {
        use ambition_platformer2d_core::GeoSource;
        let source = match &geo.source {
            GeoSource::Placement(id) => format!("placement/{}", escape_segment(id.as_str())),
            GeoSource::TileLayer { layer } => format!("tile/{}", escape_segment(layer)),
            GeoSource::Generator(id) => format!("generator/{}", escape_segment(id.as_str())),
            GeoSource::Delta { op_index } => format!("delta/{op_index}"),
            GeoSource::Anon => "anon".to_string(),
        };
        Self(format!("geo:{source}/{}", geo.index))
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
    /// Not parsed. The spelling is a legibility convenience — it exists so a desync report
    /// reads as a sentence — and nothing may recover a fact from it. Provenance in particular is
    /// [`SpawnOrigin`](crate::construction::SpawnOrigin), a component the entity carries, precisely
    /// so that changing this format cannot silently change what reconstruction believes.
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
/// Required by [`SimId`] — every identified entity is a potential spawner, and
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

    /// ⭐⭐ A RESIMULATED TICK RE-MINTS THE SAME ID — the half of that claim that
    /// can be proved without a rollback session, proved.
    ///
    /// ✔✔ **AND LEG (b) IS NOW MEASURED TOO, by somebody else's poison** — a
    /// process-global drift term added to `next()` reds the populated timeline
    /// with `GGRS sync-test checksum mismatch at frames [2..7]`. ⇒ So `SimId` is
    /// genuinely IN the session checksum and an unstable mint is genuinely SEEN,
    /// which `rollback_component_canonical` alone could not have established —
    /// this repo already records that **REGISTERED ≠ CHECKSUMMED**, and that a
    /// real desync once read clean.
    ///
    /// ⭐ Worth knowing about that poison: it works only because the fixture's
    /// subject fires bolts every ninth frame INSIDE the rolled-back window. The
    /// five ids minted before the frames are never re-run, so without those bolts
    /// it would have proved nothing about resimulation while appearing to.
    /// **A poison at the wrong layer passes for the right one.**
    ///
    /// ⛔ **THE CLAIM HAS TWO LEGS AND THIS TEST IS ONE OF THEM.** When
    /// `PortalFireIntent` gained a minted `SimId` (2026-09-04) the argument was:
    /// the counter lives on the firer and is rollback state, so a rewind restores
    /// it, so the re-simulation mints the same number. ⇒ Leg (a) is *minting is a
    /// pure function of the counter's value* — that is this test. Leg (b) is *the
    /// rewind actually restores the counter* — that is
    /// `rollback_component_canonical::<SimIdCounter>("body.sim_id_counter")` in
    /// `rollback_registration.rs`, and it is asserted by the schema baseline
    /// rather than here.
    ///
    /// ⚠ Written because the author of that change flagged the whole claim as
    /// ARGUED and not measured, and half of it is measurable in four lines. The
    /// remaining half is now a named registration rather than a piece of
    /// reasoning. ⇒ And the two legs want DIFFERENT INSTRUMENTS, which is why
    /// neither substitutes for the other: purity is a unit test, *is it in the
    /// checksum* is a poisoned session, and the registration line is neither.
    /// ⭐ Two independent instruments reached the same verdict here, and this
    /// test would have caught that poison on its own — a global drift makes two
    /// mints from `SimIdCounter(7)` differ, which is exactly what it asserts.
    #[test]
    fn a_resimulated_tick_re_mints_the_same_id() {
        let firer = SimId::player_slot(0);

        // The tick as it first ran.
        let mut counter = SimIdCounter(7);
        let first = SimId::spawned(&firer, counter.next());

        // The rewind: the snapshot held the counter's value from BEFORE the
        // tick, which is exactly what makes a re-simulation a re-simulation.
        let mut restored = SimIdCounter(7);
        let again = SimId::spawned(&firer, restored.next());
        assert_eq!(
            first, again,
            "a re-simulated tick minted a DIFFERENT id from the same counter \
             value — the shot would rewind into a different identity and the \
             rollback would diverge"
        );

        // ⛔ ANTI-VACUITY. Without the restore the id must differ, or the
        // assertion above is satisfied by `spawned` ignoring the sequence and
        // would pass for a function that returns a constant.
        let not_restored = SimId::spawned(&firer, counter.next());
        assert_ne!(
            first, not_restored,
            "two mints from an ADVANCING counter produced the same id, so the \
             sequence number is not reaching the id at all"
        );

        // ⭐ And the spawner is part of it: two bodies at the same sequence must
        // not collide, which is the reason the counter is per-firer rather than
        // global.
        let other = SimId::player_slot(1);
        assert_ne!(
            first,
            SimId::spawned(&other, SimIdCounter(7).next()),
            "two different firers at the same sequence minted one id"
        );
    }

    #[test]
    fn the_constructors_never_collide() {
        assert_ne!(SimId::placement("0"), SimId::player_slot(0));
        assert_ne!(SimId::placement("slot:0"), SimId::player_slot(0));
        // A boss WRAP and the boss BODY share the raw id string but live in
        // different namespaces (orchestration vs body).
        assert_ne!(SimId::encounter("boss_1"), SimId::placement("boss_1"));
    }

    /// Geometry gets its OWN namespace, and the reason is a collision that would
    /// otherwise be silent: one placement can emit several blocks, so block 1 of
    /// placement `p` and the actor placed at `p` must not spell the same id.
    #[test]
    fn geometry_ids_do_not_collide_with_placements_or_with_each_other() {
        use ambition_platformer2d_core::{GeoId, PlacementId};
        let block0 = SimId::geometry(&GeoId::placement(PlacementId::new("p"), 0));
        let block1 = SimId::geometry(&GeoId::placement(PlacementId::new("p"), 1));
        let actor = SimId::placement("p");
        assert_ne!(block0, block1, "the ordinal is part of the identity");
        assert_ne!(block0, actor, "a block is not the actor placed at its iid");
        // A tile-layer block has no placement iid at all; a generator's ordinal is
        // its emission index. All three sources stay distinct.
        assert_ne!(
            SimId::geometry(&GeoId::tile_layer("Collision", 0)),
            SimId::geometry(&GeoId::placement(PlacementId::new("Collision"), 0)),
        );
        // ⚠ And the documented hole: fixture geometry is not durably named, which
        // is why a population sorting by identity owes a DISTINCTNESS check too.
        assert_eq!(
            SimId::geometry(&GeoId::anon()),
            SimId::geometry(&GeoId::anon())
        );
    }

    /// The encoding is INJECTIVE: distinct constructions, distinct strings.
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
            for other in segments {
                let parent = SimId::placement(other);
                minted.push((
                    format!("death_drop(placement({other:?}), {segment:?})"),
                    SimId::death_drop(&parent, segment).0,
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

    /// The common case still reads as a sentence.
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

    /// Per-spawner, never global. A global counter couples unrelated spawners:
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
