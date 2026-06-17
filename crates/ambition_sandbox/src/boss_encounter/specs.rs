//! Installed boss-encounter spec holder.
//!
//! Holds the content-installed `BossEncounterSpec`s (the numeric encounter
//! schema from `boss_encounters/<id>.ron`, per ADR 0017). `ambition_content`
//! calls `install_boss_encounter_specs`; production REQUIRES it, while
//! `cfg(test)` falls back to reading the content RON dir directly so lib tests
//! resolve standalone. `boss_encounter_specs` / `default_boss_specs` read the
//! installed set; the lib embeds no boss data itself.

use super::profile::default_boss_profiles;

/// Default boss specs shipped with the sandbox. Populated lazily so
/// hot reloads of LDtk content don't double-register.
pub fn default_boss_specs() -> Vec<crate::boss_encounter::BossEncounterSpec> {
    default_boss_profiles()
        .into_iter()
        .map(|profile| profile.encounter)
        .collect()
}

use crate::boss_encounter::BossEncounterSpec;

/// The installed per-boss encounter specs (`boss_encounters/<id>.ron`). The
/// named encounter DATA is content, owned + installed by `ambition_content`
/// (`install_boss_roster`); the lib holds only this generic holder + the
/// `BossEncounterSpec` schema. Per ADR 0017 the RON owns the encounter numbers
/// (HP / phase thresholds / timings / music ids); the `BossBehaviorProfile`
/// (boss_profiles.ron) owns movement/attacks/rewards.
static BOSS_ENCOUNTER_SPEC_OVERRIDE: std::sync::OnceLock<Vec<BossEncounterSpec>> =
    std::sync::OnceLock::new();

/// Install the authored per-boss encounter specs — `ambition_content` calls
/// this (alongside `install_boss_profiles`) with the parsed, embedded
/// `boss_encounters/*.ron`.
pub fn install_boss_encounter_specs(specs: Vec<BossEncounterSpec>) {
    let _ = BOSS_ENCOUNTER_SPEC_OVERRIDE.set(specs);
}

/// The installed encounter specs. Production REQUIRES the content install (no
/// embedded boss data in the lib binary); lib unit tests read content's
/// authored `boss_encounters/` at compile-relative path so they resolve
/// standalone.
pub(super) fn boss_encounter_specs() -> Vec<BossEncounterSpec> {
    if let Some(specs) = BOSS_ENCOUNTER_SPEC_OVERRIDE.get() {
        return specs.clone();
    }
    boss_encounter_specs_fallback()
}

#[cfg(test)]
fn boss_encounter_specs_fallback() -> Vec<BossEncounterSpec> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../ambition_content/assets/data/boss_encounters");
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("ron") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        if let Ok(spec) = ron::from_str::<BossEncounterSpec>(&text) {
            out.push(spec);
        }
    }
    out
}

#[cfg(not(test))]
fn boss_encounter_specs_fallback() -> Vec<BossEncounterSpec> {
    panic!(
        "boss encounter specs not installed — AmbitionContent must call \
         install_boss_encounter_specs() (via init_sandbox_resources) before any \
         boss resolves"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every RON file under `boss_encounters/` must correspond to an
    /// authored profile in `AUTHORED_BOSS_PROFILES`. A stray RON
    /// (typo'd filename, leftover from a renamed boss) would be
    /// silently ignored by the loader override loop; this test trips
    /// instead. The reverse (profile without RON) is fine — the in-lib
    /// generic `gradient_sentinel` base covers an unauthored boss.
    #[test]
    fn every_on_disk_ron_matches_an_authored_profile() {
        let profile_ids: std::collections::BTreeSet<String> =
            default_boss_profiles().into_iter().map(|p| p.id).collect();
        let orphans: Vec<String> = boss_encounter_specs()
            .into_iter()
            .map(|s| s.id)
            .filter(|id| !profile_ids.contains(id))
            .collect();
        assert!(
            orphans.is_empty(),
            "boss_encounters/<id>.ron files have no matching authored profile: {orphans:?}"
        );
    }

    /// The loader produces no duplicate ids — a misnamed `.ron` file
    /// (e.g. `gnu_ton_old.ron` with `id: \"gnu_ton\"`) would land
    /// duplicate specs in the override map; the profile loop would
    /// then nondeterministically pick whichever the BTreeMap collected
    /// last. This test trips on that case.
    #[test]
    fn load_boss_specs_from_disk_has_no_duplicate_ids() {
        let specs = boss_encounter_specs();
        let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        let dupes: Vec<String> = specs
            .iter()
            .filter_map(|s| {
                if seen.insert(s.id.clone()) {
                    None
                } else {
                    Some(s.id.clone())
                }
            })
            .collect();
        assert!(
            dupes.is_empty(),
            "duplicate boss spec ids on disk: {dupes:?}"
        );
    }
}
