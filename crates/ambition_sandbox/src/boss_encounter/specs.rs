
use super::profile::default_boss_profiles;

/// Default boss specs shipped with the sandbox. Populated lazily so
/// hot reloads of LDtk content don't double-register.
pub fn default_boss_specs() -> Vec<crate::boss_encounter::BossEncounterSpec> {
    default_boss_profiles()
        .into_iter()
        .map(|profile| profile.encounter)
        .collect()
}

/// Per-boss spec read from `assets/data/boss_encounters/<id>.ron`.
///
/// Per ADR 0017 (Rust = behavior, RON = content). Called by
/// [`super::profile::default_boss_profiles`]: any RON file whose
/// `id` matches an authored profile overrides the hardcoded
/// `crate::boss_encounter::BossEncounterSpec::<id>()` constructor's numeric fields.
/// The Rust profile constructor still owns the behavior wiring
/// (`BossBehaviorProfile`, `BossRewardProfile`); only the encounter-
/// spec numbers come from disk.
///
/// Returns an empty `Vec` when the directory is missing or unreadable
/// so the build runs cleanly on a fresh clone before any RON has
/// been authored.
pub fn load_boss_specs_from_disk() -> Vec<crate::boss_encounter::BossEncounterSpec> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/data/boss_encounters");
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
        match ron::from_str::<crate::boss_encounter::BossEncounterSpec>(&text) {
            Ok(spec) => out.push(spec),
            Err(err) => {
                bevy::log::warn!(
                    target: "ambition::boss_encounter",
                    "boss_encounters: failed to parse {}: {err}",
                    path.display(),
                );
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Asserts the on-disk RON for the named boss is field-by-field
    /// equivalent to the supplied hardcoded constructor's output.
    /// Used by every per-boss RON pin test so a drift in any single
    /// field (HP, timing, music id) trips a focused diff.
    #[track_caller]
    fn assert_spec_matches_disk(id: &str, hardcoded: crate::boss_encounter::BossEncounterSpec) {
        let specs = load_boss_specs_from_disk();
        let on_disk = specs
            .iter()
            .find(|s| s.id == id)
            .unwrap_or_else(|| panic!("{id}.ron should load"));
        assert_eq!(
            *on_disk, hardcoded,
            "boss_encounters/{id}.ron drifted from constructor"
        );
    }

    #[test]
    fn load_boss_specs_from_disk_finds_gnu_ton() {
        assert_spec_matches_disk("gnu_ton", crate::boss_encounter::BossEncounterSpec::gnu_ton());
    }

    #[test]
    fn load_boss_specs_from_disk_finds_mockingbird() {
        assert_spec_matches_disk("mockingbird", crate::boss_encounter::BossEncounterSpec::mockingbird());
    }

    #[test]
    fn load_boss_specs_from_disk_finds_clockwork_warden() {
        assert_spec_matches_disk(
            "clockwork_warden",
            crate::boss_encounter::BossEncounterSpec::clockwork_warden(),
        );
    }

    /// Every RON file under `boss_encounters/` must correspond to an
    /// authored profile in `AUTHORED_BOSS_PROFILES`. A stray RON
    /// (typo'd filename, leftover from a renamed boss) would be
    /// silently ignored by the loader override loop; this test trips
    /// instead. The reverse (profile without RON) is fine — the
    /// hardcoded constructor stays the fallback.
    #[test]
    fn every_on_disk_ron_matches_an_authored_profile() {
        let profile_ids: std::collections::BTreeSet<String> =
            default_boss_profiles().into_iter().map(|p| p.id).collect();
        let orphans: Vec<String> = load_boss_specs_from_disk()
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
        let specs = load_boss_specs_from_disk();
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
