//! Assembled per-boss profile: the content-facing bundle.
//!
//! `BossProfile` stitches the two data registries into one struct —
//! encounter numbers (`BossEncounterSpec` from `boss_encounters/<id>.ron`) plus
//! behavior + reward (`BossBehaviorProfile` from `boss_profiles.ron`).
//! `BossProfile::from_id` / `for_encounter_id_or_name` resolve one by id (with
//! the legacy `gradient_sentinel` -> `clockwork_warden` save alias);
//! `default_boss_profiles` builds the full installed list. Consumed by
//! `registry`/`systems` to register encounters.

/// Full authored profile for a boss encounter.
///
/// This is the sandbox-side bridge from encounter progression to actual play.
/// `crate::boss_encounter::BossEncounterSpec` remains the engine-owned state-machine input, while
/// `BossProfile` owns the content-facing bundle: phase thresholds, movement,
/// hitboxes, damage tuning, music, and rewards.
///
/// `BossProfile` is authored as DATA, never via named Rust constructors:
/// every named boss instance lives on disk. The encounter numbers come from
/// `assets/data/boss_encounters/<id>.ron` and the behavior + reward
/// come from `assets/data/boss_profiles.ron`; `BossProfile::from_id`
/// stitches the two registries together. The general type + the
/// encounter system stay in core (they're the reusable pattern); only
/// the per-boss instance data lives on disk.
#[derive(Clone, Debug, PartialEq)]
pub struct BossProfile {
    pub id: String,
    pub display_name: String,
    pub encounter: crate::boss_encounter::BossEncounterSpec,
    pub behavior: crate::features::BossBehaviorProfile,
    pub reward: BossRewardProfile,
}

use super::behavior;
use crate::boss_encounter::BossSpecRoster;
/// `BossRewardProfile` is authored in `boss_profiles.ron` and parsed
/// into `BossBehaviorProfile::reward`. Re-exported from its definition
/// site (`content::features::bosses`) so existing
/// `crate::boss_encounter::BossRewardProfile` call sites keep compiling.
pub use behavior::BossRewardProfile;

impl BossProfile {
    /// Assemble a boss profile from its canonical id by combining the
    /// two content-installed data registries:
    /// * encounter numbers from `boss_encounters/<id>.ron`, and
    /// * behavior + reward from `boss_profiles.ron`.
    ///
    /// Returns `None` if the id has no authored encounter spec.
    pub fn from_id(id: &str) -> Option<Self> {
        let encounter = default_boss_specs_by_id().get(id)?.clone();
        let behavior = crate::features::BossBehaviorProfile::from_data(id);
        Some(Self {
            id: encounter.id.clone(),
            display_name: encounter.name.clone(),
            reward: behavior.reward.clone(),
            behavior,
            encounter,
        })
    }

    pub fn generic(id: impl Into<String>, display_name: impl Into<String>, max_hp: i32) -> Self {
        let id = id.into();
        let display_name = display_name.into();
        let mut encounter = crate::boss_encounter::BossEncounterSpec::gradient_sentinel();
        encounter.id = id.clone();
        encounter.name = display_name.clone();
        encounter.max_hp = max_hp.max(1);
        Self {
            id: id.clone(),
            display_name,
            encounter,
            behavior: crate::features::BossBehaviorProfile::generic(id),
            reward: BossRewardProfile::None,
        }
    }

    pub fn for_encounter_id_or_name(id_or_name: &str) -> Option<Self> {
        let id = super::encounter_id_from_name(id_or_name);
        Self::from_id(&id)
            // Legacy alias: pre-rename gradient_sentinel ids in saves still
            // resolve to the renamed `clockwork_warden` profile.
            .or_else(|| match id.as_str() {
                "gradient_sentinel" => Self::from_id("clockwork_warden"),
                _ => None,
            })
    }
}

/// Installed encounter specs keyed by id. Per ADR 0017, named boss encounter
/// numbers live in `ambition_content/assets/data/boss_encounters/<id>.ron`;
/// gameplay-core only holds the generic schema and the installed roster.
fn default_boss_specs_by_id(
) -> std::collections::BTreeMap<String, crate::boss_encounter::BossEncounterSpec> {
    let mut specs: std::collections::BTreeMap<String, crate::boss_encounter::BossEncounterSpec> =
        std::collections::BTreeMap::new();
    for spec in super::specs::boss_encounter_specs() {
        specs.insert(spec.id.clone(), spec);
    }
    specs
}

/// Every authored boss profile, derived from the content-installed encounter
/// specs (`boss_encounters/<id>.ron`). The engine hardcodes no boss list —
/// adding a boss is purely content data (an encounter RON + a `boss_profiles.ron`
/// row), with no lib edit. Iterates the installed specs in install order so
/// registration/spawn order stays stable (and replay-deterministic).
pub fn default_boss_profiles() -> Vec<BossProfile> {
    super::specs::boss_encounter_specs()
        .iter()
        .filter_map(|spec| BossProfile::from_id(&spec.id))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authored_profiles_have_unique_ids() {
        let profiles = default_boss_profiles();
        assert_eq!(
            profiles.len(),
            super::super::specs::boss_encounter_specs().len(),
            "every installed boss encounter spec must resolve to a profile",
        );
        let mut ids = std::collections::BTreeSet::new();
        for profile in profiles {
            assert!(
                ids.insert(profile.id.clone()),
                "duplicate boss profile id {}",
                profile.id
            );
            assert_eq!(profile.encounter.id, profile.id);
            assert_eq!(profile.behavior.id, profile.id);
        }
    }

    #[test]
    fn mockingbird_profile_declares_reward_chest() {
        let profile = BossProfile::from_id("mockingbird").expect("mockingbird is authored");
        assert!(matches!(
            profile.reward,
            BossRewardProfile::DropChest { .. }
        ));
    }

    #[test]
    fn flying_spaghetti_monster_boss_profile_declares_reward_chest() {
        let profile = BossProfile::from_id("flying_spaghetti_monster_boss")
            .expect("flying_spaghetti_monster_boss is authored");
        assert!(matches!(
            profile.reward,
            BossRewardProfile::DropChest { .. }
        ));
    }

    /// The bosses that carried a `BossRewardProfile::None` in the old
    /// constructors must still resolve to `None` from the RON (i.e. the
    /// `reward:` field is absent / authored as `None`), and the ones
    /// that dropped a chest must still drop a chest. Pins the reward
    /// migration so the RON can't silently drop a chest.
    #[test]
    fn reward_kinds_match_legacy_constructors() {
        for id in ["clockwork_warden", "smirking_behemoth_boss"] {
            let profile = BossProfile::from_id(id).unwrap();
            assert!(
                matches!(profile.reward, BossRewardProfile::None),
                "{id} should have no reward chest",
            );
        }
        for id in [
            "mockingbird",
            "gnu_ton",
            "flying_spaghetti_monster_boss",
            "trex_boss",
            "mode_collapse_boss",
            "exploding_gradient_boss",
            "overflow_boss",
        ] {
            let profile = BossProfile::from_id(id).unwrap();
            assert!(
                matches!(profile.reward, BossRewardProfile::DropChest { .. }),
                "{id} should drop a reward chest",
            );
        }
    }

    /// Legacy gradient_sentinel ids in saves resolve to the renamed
    /// clockwork_warden profile.
    #[test]
    fn gradient_sentinel_id_aliases_to_clockwork_warden() {
        let profile = BossProfile::for_encounter_id_or_name("gradient_sentinel")
            .expect("gradient_sentinel aliases to clockwork_warden");
        assert_eq!(profile.id, "clockwork_warden");
    }
}
