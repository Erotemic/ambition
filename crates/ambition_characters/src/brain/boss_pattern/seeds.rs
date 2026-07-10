//! **The boss seed library** — attack archetypes as documented, validated data.
//!
//! `docs/planning/engine/boss-design.md` §2: *"A content-side catalog of
//! parameterized building blocks, each a `MoveSpec`/pattern prefab with named
//! params and a written design intent … The library starts from the existing
//! bosses' moves (extract → generalize → document), and grows by accretion."*
//! This module is the vocabulary; the catalog itself is content
//! (`game/ambition_content/assets/data/boss_seeds.ron`).
//!
//! ## Why a seed is data and not a doc
//!
//! Three consumers, and only the first is human:
//!
//! 1. **An authoring agent** reads [`MoveSeed::intent`] and [`MoveSeed::recipes`]
//!    to compose a fight from 4–7 seeds plus one bespoke move.
//! 2. **BD5's fight validator** reads [`MoveSeed::fair_counters`] (rule 2:
//!    *"across the fight every core movement verb must appear in some attack's
//!    counter set"*) and [`MoveSeed::threat`] (rule 1: telegraph proportionality).
//!    Those rules cannot run against prose.
//! 3. **BD6's playtester** buckets `damage_sources` by seed to check that no
//!    archetype supplies more than half a fight's damage.
//!
//! ## The bands are MEASURED, not invented
//!
//! [`MoveSeed::telegraph`] and [`MoveSeed::active`] are the observed envelope of
//! every instance in the shipped roster, widened to nothing. The content test
//! `boss_seeds_bands_are_the_measured_envelope` re-derives them from
//! `boss_profiles.ron` and fails if a seed's band drifts off its own instances.
//! A band that cannot be violated by the data it describes is a comment; this one
//! is a fixture.
//!
//! ## What a seed does NOT carry
//!
//! **Recovery.** §3's commitment rule wants a punish window per attack, and there
//! is no per-attack recovery in `BossPatternStep` today — the punish window is the
//! `Rest` beat that FOLLOWS a `Strike`, which is a property of the occurrence, not
//! of the move. BD5 must measure it per beat. Recording that here rather than
//! inventing a `recovery` field that nothing would fill.

use std::collections::BTreeMap;

/// The nine archetypes the shipped roster actually contains. Seven were named in
/// boss-design.md §2; two — [`BodyNova`](SeedArchetype::BodyNova) and
/// [`SpreadVolley`](SeedArchetype::SpreadVolley) — came out of the extraction and
/// are new to the list. §2's `counter_stance`, `enrage_repeat`, and
/// `grab_command` have no instance in the roster and are therefore NOT here: an
/// archetype with no example teaches nothing. They arrive with the fight that
/// first needs them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Deserialize)]
pub enum SeedArchetype {
    /// Horizontal denial at the boss's own height. Answered by leaving the ground.
    Sweep,
    /// Vertical punish onto the floor. Answered by not being under it.
    Slam,
    /// A short-range burst centered on the boss's body. Answered by distance.
    BodyNova,
    /// A hazard volume that persists after the strike ends. Answered by reading
    /// the arena, not the boss.
    ZoneDenial,
    /// Projectiles that arrive from elsewhere — falling, tracking, flooding.
    /// The positioning test.
    ProjectileRain,
    /// One instant burst of projectiles outward from the boss: a ring, or a cone
    /// aimed at the player. Answered by finding the gap.
    SpreadVolley,
    /// A directed line of denial. Answered by leaving the line.
    Beam,
    /// The boss crosses the arena through the player. The cross-up.
    DashThrough,
    /// Adds. Answered by splitting attention, or by refusing to.
    Summon,
}

/// Threat class, which sets the telegraph floor (§3 rule 1). The calibration
/// bands live in a per-game RON, not here — this enum only names the tier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Deserialize)]
pub enum ThreatClass {
    /// Chip-level. §3 rule 3 exempts it from the punish-window floor, capped at
    /// ≤ 10% victim HP per touch.
    Pressure,
    /// ≤ 8 damage, one volume.
    Light,
    Medium,
    /// One-shot threat or arena-wide.
    Heavy,
}

/// A core movement verb a player answers an attack with. §3 rule 2 requires every
/// one of these to appear in some attack's counter set across a fight, so a fight
/// exercises the kit rather than testing one button.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Deserialize)]
pub enum MovementVerb {
    Jump,
    Dash,
    /// Walk out of the threatened region. The verb a fight forgets to demand.
    WalkOut,
    /// Drop below / fall through. Present because a `Beam` at head height is
    /// answered downward, not sideways.
    Descend,
    Shield,
    Parry,
    Blink,
}

/// An inclusive duration envelope, seconds.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
pub struct DurationBand {
    pub min_s: f32,
    pub max_s: f32,
}

impl DurationBand {
    pub fn contains(&self, s: f32) -> bool {
        // Authored RON carries two decimals; compare with a tick of slack so a
        // 0.30 in the file never loses to a 0.3000001 float.
        const EPS: f32 = 1e-4;
        s >= self.min_s - EPS && s <= self.max_s + EPS
    }
}

/// A named parameterization of a seed — the "2–3 param recipes" §2 asks for.
/// A recipe is a STARTING POINT an agent tunes, not a constant.
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct SeedRecipe {
    /// e.g. `"readable_opener"`, `"enrage_tight"`.
    pub name: String,
    pub telegraph_s: f32,
    pub active_s: f32,
    /// Why this recipe exists and where it belongs in a fight.
    pub notes: String,
}

/// One archetype, extracted from the shipped roster, generalized, documented.
#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct MoveSeed {
    pub archetype: SeedArchetype,
    /// The design intent, in prose, for the agent composing a fight.
    pub intent: String,
    /// The player skill this attack tests. One sentence.
    pub skill_tested: String,
    /// The movement verbs that ANSWER it. Never empty (BD5 rule 2 errors on it),
    /// and pinned by `every_seed_declares_a_fair_counter_and_a_written_intent`.
    pub fair_counters: Vec<MovementVerb>,
    pub threat: ThreatClass,
    /// The observed telegraph envelope across every instance in the roster.
    pub telegraph: DurationBand,
    /// The observed active-window envelope. A `ZoneDenial` seed's upper bound is
    /// long by nature — the hazard is supposed to outlive the swing.
    pub active: DurationBand,
    /// The `BossAttackProfile` keys in the shipped roster that ARE this seed —
    /// `Strike("side_sweep")`, `Special("overfit_volley")`, and so on. Every key
    /// the roster uses belongs to exactly one seed, and the content test
    /// `every_shipped_boss_attack_key_belongs_to_exactly_one_seed` is the oracle.
    pub instances: Vec<String>,
    pub recipes: Vec<SeedRecipe>,
}

/// The catalog, keyed by seed id (`"sweep"`, `"slam"`, …). `BTreeMap` so
/// iteration is ordered: a validator's error list must not depend on hash seed
/// (ADR 0023).
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize)]
#[serde(transparent)]
pub struct SeedLibrary {
    by_id: BTreeMap<String, MoveSeed>,
}

impl SeedLibrary {
    /// Parse a seed-library RON document (`{ "<id>": MoveSeed }`).
    pub fn from_ron(ron: &str) -> Result<Self, ron::error::SpannedError> {
        ron::from_str(ron)
    }

    pub fn get(&self, id: &str) -> Option<&MoveSeed> {
        self.by_id.get(id)
    }

    /// Seeds in id order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &MoveSeed)> {
        self.by_id.iter().map(|(k, v)| (k.as_str(), v))
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    /// Which seed owns this `BossAttackProfile` move key, if any. The lookup
    /// BD5's validator and BD6's `damage_sources` bucketing both need.
    pub fn seed_for_move(&self, move_key: &str) -> Option<(&str, &MoveSeed)> {
        self.iter()
            .find(|(_, seed)| seed.instances.iter().any(|i| i == move_key))
    }

    /// Every movement verb some seed in `move_keys` is answered by — §3 rule 2's
    /// coverage set for one fight. Sorted and deduped.
    pub fn counter_coverage<'a>(
        &self,
        move_keys: impl IntoIterator<Item = &'a str>,
    ) -> Vec<MovementVerb> {
        let mut verbs: Vec<MovementVerb> = move_keys
            .into_iter()
            .filter_map(|k| self.seed_for_move(k))
            .flat_map(|(_, seed)| seed.fair_counters.iter().copied())
            .collect();
        verbs.sort();
        verbs.dedup();
        verbs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TINY: &str = r#"{
        "sweep": (
            archetype: Sweep,
            intent: "denies the ground",
            skill_tested: "leave the floor on the tell",
            fair_counters: [Jump, Blink],
            threat: Medium,
            telegraph: (min_s: 0.5, max_s: 1.4),
            active: (min_s: 0.32, max_s: 0.7),
            instances: ["side_sweep", "hand_sweep"],
            recipes: [(name: "opener", telegraph_s: 0.9, active_s: 0.4, notes: "readable")],
        ),
        "slam": (
            archetype: Slam,
            intent: "punishes standing still",
            skill_tested: "read the shadow",
            fair_counters: [WalkOut, Dash],
            threat: Heavy,
            telegraph: (min_s: 0.6, max_s: 1.8),
            active: (min_s: 0.4, max_s: 1.4),
            instances: ["floor_slam"],
            recipes: [],
        ),
    }"#;

    #[test]
    fn a_library_round_trips_from_ron_and_iterates_in_id_order() {
        let lib = SeedLibrary::from_ron(TINY).expect("the fixture parses");
        assert_eq!(lib.len(), 2);
        let ids: Vec<&str> = lib.iter().map(|(id, _)| id).collect();
        assert_eq!(ids, ["slam", "sweep"], "BTreeMap order, not hash order");
    }

    #[test]
    fn a_move_key_resolves_to_its_seed() {
        let lib = SeedLibrary::from_ron(TINY).unwrap();
        assert_eq!(
            lib.seed_for_move("hand_sweep").map(|(id, _)| id),
            Some("sweep")
        );
        assert_eq!(
            lib.seed_for_move("floor_slam").map(|(id, _)| id),
            Some("slam")
        );
        assert!(lib.seed_for_move("nothing_authored").is_none());
    }

    /// §3 rule 2: a fight must exercise the kit. The coverage set is what the
    /// validator compares against the game's core-verb list.
    #[test]
    fn counter_coverage_unions_the_fights_seeds() {
        let lib = SeedLibrary::from_ron(TINY).unwrap();
        assert_eq!(
            lib.counter_coverage(["side_sweep"]),
            vec![MovementVerb::Jump, MovementVerb::Blink],
            "declaration order of MovementVerb is the sort order"
        );
        assert_eq!(
            lib.counter_coverage(["side_sweep", "floor_slam"]),
            vec![
                MovementVerb::Jump,
                MovementVerb::Dash,
                MovementVerb::WalkOut,
                MovementVerb::Blink
            ],
            "a fight of two seeds demands four verbs — and still no Shield/Parry"
        );
        // An unknown key contributes nothing rather than panicking: the validator
        // reports it as an uncatalogued move, which is a different error.
        assert!(lib.counter_coverage(["nope"]).is_empty());
    }

    #[test]
    fn a_band_is_inclusive_and_tolerates_authored_decimals() {
        let b = DurationBand {
            min_s: 0.30,
            max_s: 1.40,
        };
        assert!(b.contains(0.30));
        assert!(b.contains(1.40));
        assert!(b.contains(0.9));
        assert!(!b.contains(0.29));
        assert!(!b.contains(1.41));
    }
}
