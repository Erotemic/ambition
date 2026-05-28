use ambition_engine as ae;

/// Full authored profile for a boss encounter.
///
/// This is the sandbox-side bridge from encounter progression to actual play.
/// `crate::boss_encounter::BossEncounterSpec` remains the engine-owned state-machine input, while
/// `BossProfile` owns the content-facing bundle: phase thresholds, movement,
/// hitboxes, damage tuning, music, and rewards.
#[derive(Clone, Debug, PartialEq)]
pub struct BossProfile {
    pub id: String,
    pub display_name: String,
    pub encounter: crate::boss_encounter::BossEncounterSpec,
    pub behavior: crate::features::BossBehaviorProfile,
    pub reward: BossRewardProfile,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BossRewardProfile {
    None,
    DropChest {
        pickup: crate::interaction::PickupKind,
        offset: ae::Vec2,
        size: ae::Vec2,
    },
}

impl BossProfile {
    pub fn clockwork_warden() -> Self {
        let encounter = crate::boss_encounter::BossEncounterSpec::clockwork_warden();
        Self {
            id: encounter.id.clone(),
            display_name: encounter.name.clone(),
            encounter,
            behavior: crate::features::BossBehaviorProfile::clockwork_warden(),
            reward: BossRewardProfile::None,
        }
    }

    pub fn mockingbird() -> Self {
        let encounter = crate::boss_encounter::BossEncounterSpec::mockingbird();
        Self {
            id: encounter.id.clone(),
            display_name: encounter.name.clone(),
            encounter,
            behavior: crate::features::BossBehaviorProfile::mockingbird(),
            reward: BossRewardProfile::DropChest {
                pickup: crate::interaction::PickupKind::Custom("pirate_hoard".to_string()),
                offset: ae::Vec2::new(0.0, 24.0),
                size: ae::Vec2::new(56.0, 56.0),
            },
        }
    }

    /// GNU-ton — the giant GNU wildebeest with a scholar on its shoulders.
    ///
    /// Phase 1: hands-only — player must dodge without dealing damage.
    /// Phase 2+: head descends periodically (SpikeHalo windows) — the
    /// player can deal damage during the head-down vulnerability period.
    /// The arena needs to be wide (~640px) to give space for the hands.
    pub fn gnu_ton() -> Self {
        let encounter = crate::boss_encounter::BossEncounterSpec::gnu_ton();
        Self {
            id: encounter.id.clone(),
            display_name: encounter.name.clone(),
            encounter,
            behavior: crate::features::BossBehaviorProfile::gnu_ton(),
            reward: BossRewardProfile::DropChest {
                pickup: crate::interaction::PickupKind::Custom("gnu_scroll".to_string()),
                offset: ae::Vec2::new(0.0, 30.0),
                size: ae::Vec2::new(52.0, 52.0),
            },
        }
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
        AUTHORED_BOSS_PROFILES
            .iter()
            .find(|(key, _)| *key == id.as_str())
            .map(|(_, ctor)| ctor())
            // Legacy alias: pre-rename gradient_sentinel ids in saves still
            // resolve to the renamed `clockwork_warden` profile.
            .or_else(|| match id.as_str() {
                "gradient_sentinel" => Some(Self::clockwork_warden()),
                _ => None,
            })
    }
}

/// Table of authored boss profiles. Adding a new boss is a single
/// row: write the `BossProfile::new_boss()` constructor, then append
/// `("new_boss", BossProfile::new_boss)` here. The two helpers below
/// (`for_encounter_id_or_name` + `default_boss_profiles`) walk this
/// slice instead of carrying their own id-string match arms.
const AUTHORED_BOSS_PROFILES: &[(&str, fn() -> BossProfile)] = &[
    ("clockwork_warden", BossProfile::clockwork_warden),
    ("mockingbird", BossProfile::mockingbird),
    ("gnu_ton", BossProfile::gnu_ton),
];

pub fn default_boss_profiles() -> Vec<BossProfile> {
    // Per ADR 0017 (Rust = behavior, RON = content): if a boss has
    // an `assets/data/boss_encounters/<id>.ron`, it overrides the
    // hardcoded `crate::boss_encounter::BossEncounterSpec::<id>()` constructor's numeric
    // fields. The Rust profile constructor still owns the behavior
    // wiring (`BossBehaviorProfile`, `BossRewardProfile`) — only the
    // encounter-spec numbers come from disk.
    let on_disk: std::collections::BTreeMap<String, crate::boss_encounter::BossEncounterSpec> =
        super::specs::load_boss_specs_from_disk()
            .into_iter()
            .map(|s| (s.id.clone(), s))
            .collect();
    AUTHORED_BOSS_PROFILES
        .iter()
        .map(|(_, ctor)| {
            let mut profile = ctor();
            if let Some(spec) = on_disk.get(&profile.id) {
                profile.encounter = spec.clone();
            }
            profile
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authored_profiles_have_unique_ids() {
        let profiles = default_boss_profiles();
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
        let profile = BossProfile::mockingbird();
        assert!(matches!(
            profile.reward,
            BossRewardProfile::DropChest { .. }
        ));
    }

    /// Smoke tests for the RON-overrides-hardcoded path: each boss
    /// has a `boss_encounters/<id>.ron` on disk, so
    /// `default_boss_profiles` must produce an encounter spec
    /// equivalent to the hardcoded constructor (the per-field diff
    /// is pinned in `specs::tests::load_boss_specs_from_disk_finds_*`).
    /// These tests catch regressions where the RON drifts from the
    /// constructor or where the override loop accidentally drops the
    /// spec for a particular id.
    #[track_caller]
    fn assert_profile_matches(id: &str, hardcoded: crate::boss_encounter::BossEncounterSpec) {
        let profile = default_boss_profiles()
            .into_iter()
            .find(|p| p.id == id)
            .unwrap_or_else(|| panic!("{id} profile is registered"));
        assert_eq!(profile.encounter, hardcoded);
    }

    #[test]
    fn gnu_ton_profile_encounter_matches_hardcoded_constructor() {
        assert_profile_matches("gnu_ton", crate::boss_encounter::BossEncounterSpec::gnu_ton());
    }

    #[test]
    fn mockingbird_profile_encounter_matches_hardcoded_constructor() {
        assert_profile_matches("mockingbird", crate::boss_encounter::BossEncounterSpec::mockingbird());
    }

    #[test]
    fn clockwork_warden_profile_encounter_matches_hardcoded_constructor() {
        assert_profile_matches(
            "clockwork_warden",
            crate::boss_encounter::BossEncounterSpec::clockwork_warden(),
        );
    }
}
