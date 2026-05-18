use ambition_engine as ae;

/// Full authored profile for a boss encounter.
///
/// This is the sandbox-side bridge from encounter progression to actual play.
/// `ae::BossEncounterSpec` remains the engine-owned state-machine input, while
/// `BossProfile` owns the content-facing bundle: phase thresholds, movement,
/// hitboxes, damage tuning, music, and rewards.
#[derive(Clone, Debug, PartialEq)]
pub struct BossProfile {
    pub id: String,
    pub display_name: String,
    pub encounter: ae::BossEncounterSpec,
    pub behavior: crate::features::BossBehaviorProfile,
    pub reward: BossRewardProfile,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BossRewardProfile {
    None,
    DropChest {
        pickup: ae::PickupKind,
        offset: ae::Vec2,
        size: ae::Vec2,
    },
}

impl BossProfile {
    pub fn clockwork_warden() -> Self {
        let mut encounter = ae::BossEncounterSpec::gradient_sentinel();
        encounter.id = "clockwork_warden".into();
        encounter.name = "Clockwork Warden".into();
        encounter.max_hp = 36;
        Self {
            id: encounter.id.clone(),
            display_name: encounter.name.clone(),
            encounter,
            behavior: crate::features::BossBehaviorProfile::clockwork_warden(),
            reward: BossRewardProfile::None,
        }
    }

    pub fn mockingbird() -> Self {
        let encounter = ae::BossEncounterSpec::mockingbird();
        Self {
            id: encounter.id.clone(),
            display_name: encounter.name.clone(),
            encounter,
            behavior: crate::features::BossBehaviorProfile::mockingbird(),
            reward: BossRewardProfile::DropChest {
                pickup: ae::PickupKind::Custom("pirate_hoard".to_string()),
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
        let encounter = ae::BossEncounterSpec::gnu_ton();
        Self {
            id: encounter.id.clone(),
            display_name: encounter.name.clone(),
            encounter,
            behavior: crate::features::BossBehaviorProfile::gnu_ton(),
            reward: BossRewardProfile::DropChest {
                pickup: ae::PickupKind::Custom("gnu_scroll".to_string()),
                offset: ae::Vec2::new(0.0, 30.0),
                size: ae::Vec2::new(52.0, 52.0),
            },
        }
    }

    pub fn generic(id: impl Into<String>, display_name: impl Into<String>, max_hp: i32) -> Self {
        let id = id.into();
        let display_name = display_name.into();
        let mut encounter = ae::BossEncounterSpec::gradient_sentinel();
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
        match id.as_str() {
            "clockwork_warden" | "gradient_sentinel" => Some(Self::clockwork_warden()),
            "mockingbird" => Some(Self::mockingbird()),
            "gnu_ton" => Some(Self::gnu_ton()),
            _ => None,
        }
    }
}

pub fn default_boss_profiles() -> Vec<BossProfile> {
    vec![
        BossProfile::clockwork_warden(),
        BossProfile::mockingbird(),
        BossProfile::gnu_ton(),
    ]
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
}
