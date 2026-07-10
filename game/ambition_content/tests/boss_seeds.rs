//! **BD4's oracle: the seed library describes the roster it claims to describe.**
//!
//! `docs/planning/engine/boss-design.md` §2 asks for a catalog that "starts from
//! the existing bosses' moves (extract → generalize → document), and grows by
//! accretion." A catalog that merely *sits beside* the roster rots the first time
//! a boss is retuned, and an authoring agent that trusts a rotted catalog authors
//! a rotted fight. These tests are what make the library load-bearing:
//!
//! 1. **Coverage** — every `BossAttackProfile` key the roster uses belongs to
//!    exactly one seed. A new boss move fails the build until it is classified.
//! 2. **The bands are the measurement.** Every occurrence lies inside its seed's
//!    band, AND every band's endpoints are achieved by some occurrence. Together
//!    those two say `band == observed envelope`, with no padding. Retuning a boss
//!    therefore updates `boss_seeds.ron` — which is the accretion discipline,
//!    enforced.
//! 3. **BD5's preconditions** — no seed ships with an empty `fair_counters` (rule
//!    2 would error on it at install time) and the union across a fight is
//!    inspectable.

use std::collections::{BTreeMap, BTreeSet};

use ambition_actors::features::BossBehaviorProfile;
use ambition_characters::brain::boss_pattern::{
    seeds::SeedLibrary, BossAttackPattern, BossAttackProfile, BossPattern, BossPatternStep,
};
use ambition_content::bosses::{seed_library, BOSS_PROFILES_RON};

/// One authored appearance of an attack: its telegraph and active durations.
#[derive(Clone, Copy, Debug)]
struct Occurrence {
    telegraph_s: f32,
    active_s: f32,
}

/// Re-derive, from the same bytes the game loads, every authored occurrence of
/// every attack key.
///
/// Two shapes to walk. A `Scripted` boss carries per-step `Telegraph`/`Strike`
/// durations across its five phases. A `Cycle` boss has none — it rotates its
/// `attacks` on the profile's flat `attack_windup` / `attack_active`, which is
/// why the mockingbird's four moves all read 0.44s / 0.28s. Both are real
/// occurrences; a band that ignored the `Cycle` bosses would be a lie about half
/// the roster.
fn occurrences_by_move_key() -> BTreeMap<String, Vec<Occurrence>> {
    let profiles: BTreeMap<String, BossBehaviorProfile> = ron::from_str(BOSS_PROFILES_RON)
        .expect("boss_profiles.ron parses as the shipped BossBehaviorProfile schema");

    let mut out: BTreeMap<String, Vec<Occurrence>> = BTreeMap::new();
    for profile in profiles.values() {
        match &profile.attack_pattern {
            BossAttackPattern::Cycle => {
                for attack in &profile.attacks {
                    out.entry(move_key(attack).to_string())
                        .or_default()
                        .push(Occurrence {
                            telegraph_s: profile.attack_windup,
                            active_s: profile.attack_active,
                        });
                }
            }
            BossAttackPattern::Scripted {
                intro,
                phase1,
                transition,
                phase2,
                enrage,
            } => {
                for pattern in [intro, phase1, transition, phase2, enrage] {
                    collect_scripted(&pattern, &mut out);
                }
            }
        }
    }
    out
}

/// A `Scripted` phase pairs each `Telegraph` with the `Strike` of the same
/// profile that follows it. Authored beats always come in that order (a `Strike`
/// with no `Telegraph` would be a §3 rule-1 error, and none exist), so pairing by
/// "the pending telegraph for this key" is exact rather than heuristic.
fn collect_scripted(pattern: &BossPattern, out: &mut BTreeMap<String, Vec<Occurrence>>) {
    let mut pending: BTreeMap<String, f32> = BTreeMap::new();
    for step in &pattern.steps {
        match step {
            BossPatternStep::Telegraph { profile, duration } => {
                pending.insert(move_key(profile).to_string(), *duration);
            }
            BossPatternStep::Strike { profile, duration } => {
                let key = move_key(profile).to_string();
                let telegraph_s = pending.remove(&key).unwrap_or_else(|| {
                    panic!(
                        "`{key}` strikes with no preceding Telegraph beat in this phase — \
                         that is a §3 rule-1 error (missing telegraph), not a test bug"
                    )
                });
                out.entry(key).or_default().push(Occurrence {
                    telegraph_s,
                    active_s: *duration,
                });
            }
            BossPatternStep::Rest { .. } => {}
        }
    }
}

fn move_key(profile: &BossAttackProfile) -> &str {
    match profile {
        BossAttackProfile::Strike(k) | BossAttackProfile::Special(k) => k.as_str(),
    }
}

fn library() -> &'static SeedLibrary {
    seed_library()
}

/// (1) Coverage, both directions. A move the roster uses that no seed claims is an
/// unclassified attack; a seed instance no boss uses is a stale catalog entry.
#[test]
fn every_shipped_boss_attack_key_belongs_to_exactly_one_seed() {
    let lib = library();
    let observed: BTreeSet<String> = occurrences_by_move_key().keys().cloned().collect();

    let mut claimed: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (seed_id, seed) in lib.iter() {
        for instance in &seed.instances {
            claimed.entry(instance.as_str()).or_default().push(seed_id);
        }
    }

    let unclassified: Vec<&String> = observed
        .iter()
        .filter(|k| !claimed.contains_key(k.as_str()))
        .collect();
    assert!(
        unclassified.is_empty(),
        "these boss attacks are in the roster but in no seed: {unclassified:?}. \
         Classify each into an existing archetype in `boss_seeds.ron`, or — if it \
         genuinely answers a new way — add a seed and say so in its `intent`."
    );

    let stale: Vec<&&str> = claimed.keys().filter(|k| !observed.contains(**k)).collect();
    assert!(
        stale.is_empty(),
        "these seed instances name attacks no boss uses any more: {stale:?}. \
         The catalog is describing a roster that does not exist."
    );

    let ambiguous: Vec<(&&str, &Vec<&str>)> = claimed.iter().filter(|(_, v)| v.len() > 1).collect();
    assert!(
        ambiguous.is_empty(),
        "an attack belongs to exactly one archetype; these are claimed twice: \
         {ambiguous:?}"
    );
}

/// (2) **The bands ARE the measurement.** Not "roughly cover", not "were true once".
#[test]
fn boss_seeds_bands_are_the_measured_envelope() {
    let lib = library();
    let occurrences = occurrences_by_move_key();

    for (seed_id, seed) in lib.iter() {
        let all: Vec<Occurrence> = seed
            .instances
            .iter()
            .flat_map(|k| occurrences.get(k).cloned().unwrap_or_default())
            .collect();
        assert!(
            !all.is_empty(),
            "seed `{seed_id}` has no authored occurrence — coverage should have \
             caught this first"
        );

        // (a) containment: nothing authored escapes its own seed's band.
        for occ in &all {
            assert!(
                seed.telegraph.contains(occ.telegraph_s),
                "seed `{seed_id}`: a shipped beat telegraphs for {:.2}s, outside the \
                 authored band {:?}. Widen the band in `boss_seeds.ron` (and ask \
                 whether the retune was intended).",
                occ.telegraph_s,
                seed.telegraph
            );
            assert!(
                seed.active.contains(occ.active_s),
                "seed `{seed_id}`: a shipped beat is active for {:.2}s, outside the \
                 authored band {:?}.",
                occ.active_s,
                seed.active
            );
        }

        // (b) tightness: the band's endpoints are ACHIEVED. A band wider than its
        //     own instances is a guess wearing a measurement's clothes.
        let tmin = all.iter().map(|o| o.telegraph_s).fold(f32::MAX, f32::min);
        let tmax = all.iter().map(|o| o.telegraph_s).fold(f32::MIN, f32::max);
        let amin = all.iter().map(|o| o.active_s).fold(f32::MAX, f32::min);
        let amax = all.iter().map(|o| o.active_s).fold(f32::MIN, f32::max);
        const EPS: f32 = 1e-3;
        assert!(
            (seed.telegraph.min_s - tmin).abs() < EPS && (seed.telegraph.max_s - tmax).abs() < EPS,
            "seed `{seed_id}`: authored telegraph band {:?} is not the observed \
             envelope ({tmin:.2}..{tmax:.2}). Bands are measured, never padded.",
            seed.telegraph
        );
        assert!(
            (seed.active.min_s - amin).abs() < EPS && (seed.active.max_s - amax).abs() < EPS,
            "seed `{seed_id}`: authored active band {:?} is not the observed envelope \
             ({amin:.2}..{amax:.2}).",
            seed.active
        );
    }
}

/// (3) BD5 rule 2's precondition. An attack with no fair counter is a fight the
/// player cannot answer; the validator errors on it, so the library must never
/// hand it one.
#[test]
fn every_seed_declares_a_fair_counter_and_a_written_intent() {
    for (seed_id, seed) in library().iter() {
        assert!(
            !seed.fair_counters.is_empty(),
            "seed `{seed_id}` has no fair counter — BD5 rule 2 makes this a hard error"
        );
        assert!(
            seed.intent.len() > 80,
            "seed `{seed_id}`'s intent is {} chars. The intent is what an authoring \
             agent reads INSTEAD of guessing; one clause is not enough.",
            seed.intent.len()
        );
        assert!(
            !seed.skill_tested.is_empty(),
            "seed `{seed_id}` names no player skill"
        );
        assert!(
            !seed.recipes.is_empty(),
            "seed `{seed_id}` has no param recipe — §2 asks for 2–3 per entry"
        );
    }
}

/// Every recipe is a point inside its own seed's bands. A recipe that recommends a
/// telegraph no shipped boss ever used is a suggestion; one outside the band is a
/// contradiction.
#[test]
fn every_recipe_lies_inside_its_seeds_bands() {
    for (seed_id, seed) in library().iter() {
        for recipe in &seed.recipes {
            assert!(
                seed.telegraph.contains(recipe.telegraph_s),
                "seed `{seed_id}` recipe `{}`: telegraph {:.2}s is outside the seed's \
                 own band {:?}",
                recipe.name,
                recipe.telegraph_s,
                seed.telegraph
            );
            assert!(
                seed.active.contains(recipe.active_s),
                "seed `{seed_id}` recipe `{}`: active {:.2}s is outside {:?}",
                recipe.name,
                recipe.active_s,
                seed.active
            );
        }
    }
}

/// The whole roster's counter coverage, as BD5 will compute it per fight. Today it
/// exercises five of the seven verbs — **`Parry` appears nowhere**, and `Shield`
/// only through `spread_volley`. That is a finding about the shipped bosses, not
/// about this test, and it is exactly the "forced-movement variety" gap §3 rule 2
/// exists to name. Pinned so the day a fight adds a parry-answerable attack, this
/// assertion tells us.
#[test]
fn the_shipped_roster_does_not_yet_demand_a_parry() {
    use ambition_characters::brain::boss_pattern::seeds::MovementVerb;
    let lib = library();
    let all_keys: Vec<&str> = lib
        .iter()
        .flat_map(|(_, s)| s.instances.iter().map(|i| i.as_str()))
        .collect();
    let coverage = lib.counter_coverage(all_keys);

    for demanded in [
        MovementVerb::Jump,
        MovementVerb::Dash,
        MovementVerb::WalkOut,
        MovementVerb::Descend,
        MovementVerb::Shield,
        MovementVerb::Blink,
    ] {
        assert!(
            coverage.contains(&demanded),
            "no shipped boss attack is answered by {demanded:?}"
        );
    }
    assert!(
        !coverage.contains(&MovementVerb::Parry),
        "a boss attack now declares Parry as a fair counter. Good — delete this \
         assertion and add Parry to the demanded list above."
    );
}
