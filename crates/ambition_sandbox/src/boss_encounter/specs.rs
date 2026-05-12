use ambition_engine as ae;

use super::profile::default_boss_profiles;

/// Default boss specs shipped with the sandbox. Populated lazily so
/// hot reloads of LDtk content don't double-register.
pub fn default_boss_specs() -> Vec<ae::BossEncounterSpec> {
    default_boss_profiles()
        .into_iter()
        .map(|profile| profile.encounter)
        .collect()
}
