/// Full authored profile for a boss encounter.
///
/// This is the sandbox-side bridge from encounter progression to actual play.
/// `crate::boss_encounter::BossEncounterSpec` remains the engine-owned state-machine input, while
/// `BossProfile` owns the content-facing bundle: phase thresholds, movement,
/// hitboxes, damage tuning, music, and rewards.
///
/// `BossProfile` is no longer authored via named Rust constructors —
/// every named boss instance is DATA. The encounter numbers come from
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
    /// two data registries:
    /// * encounter numbers from `boss_encounters/<id>.ron`
    ///   (falling back to the in-memory default specs), and
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

/// Canonical boss ids that have an authored encounter + behavior.
/// Adding a new boss is two data edits — a `boss_encounters/<id>.ron`
/// file and a `boss_profiles.ron` row — plus appending its id here.
pub const AUTHORED_BOSS_IDS: &[&str] = &[
    "clockwork_warden",
    "mockingbird",
    "gnu_ton",
    "smirking_behemoth_boss",
    "flying_spaghetti_monster_boss",
    "trex_boss",
    "mode_collapse_boss",
    "exploding_gradient_boss",
    "overflow_boss",
];

/// Default encounter specs keyed by id. Reads from disk
/// (`boss_encounters/<id>.ron`) per ADR 0017; the on-disk RON is the
/// authoritative numeric source. Authored ids without a RON file fall
/// back to the in-memory default spec for that id so a fresh clone
/// still boots before any RON has been written.
fn default_boss_specs_by_id(
) -> std::collections::BTreeMap<String, crate::boss_encounter::BossEncounterSpec> {
    let mut specs: std::collections::BTreeMap<String, crate::boss_encounter::BossEncounterSpec> =
        std::collections::BTreeMap::new();
    for spec in super::specs::load_boss_specs_from_disk() {
        specs.insert(spec.id.clone(), spec);
    }
    specs
}

pub fn default_boss_profiles() -> Vec<BossProfile> {
    AUTHORED_BOSS_IDS
        .iter()
        .filter_map(|id| BossProfile::from_id(id))
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
            AUTHORED_BOSS_IDS.len(),
            "every authored boss id must resolve to a profile",
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
