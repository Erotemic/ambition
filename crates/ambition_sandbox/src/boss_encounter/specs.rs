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

/// Per-boss spec read from `assets/data/boss_encounters/<id>.ron`.
///
/// Proof-of-concept (2026-05-24) for the deferred follow-up in
/// `TODO-character-catalog-and-hall.md` — the catalog's
/// `BossPattern` brain preset references encounter ids by string,
/// and per ADR 0017 the per-encounter schedule belongs in RON.
/// Today only `gnu_ton.ron` exists; this loader silently returns an
/// empty `Vec` when the directory is missing so hot reload + the
/// hardcoded `default_boss_specs()` path still work.
///
/// Future work: replace the hardcoded
/// `ae::BossEncounterSpec::{gradient_sentinel, mockingbird, gnu_ton}`
/// constructors with calls to this loader. Until then, the RON
/// file is the *authoritative* source for any boss whose RON exists
/// and `default_boss_specs()` is the fallback for the rest.
#[allow(
    dead_code,
    reason = "Proof-of-concept loader for the boss_encounters RON migration; wired by a future PR that flips BossEncounterRegistry from hardcoded specs to RON-loaded."
)]
pub fn load_boss_specs_from_disk() -> Vec<ae::BossEncounterSpec> {
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
        match ron::from_str::<ae::BossEncounterSpec>(&text) {
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

    #[test]
    fn load_boss_specs_from_disk_finds_gnu_ton() {
        let specs = load_boss_specs_from_disk();
        let gnu_ton = specs
            .iter()
            .find(|s| s.id == "gnu_ton")
            .expect("gnu_ton.ron should load");
        // The on-disk RON should match the hardcoded constructor's
        // values so a future runtime swap is a no-op.
        let hardcoded = ae::BossEncounterSpec::gnu_ton();
        assert_eq!(gnu_ton.id, hardcoded.id);
        assert_eq!(gnu_ton.name, hardcoded.name);
        assert_eq!(gnu_ton.max_hp, hardcoded.max_hp);
        assert_eq!(gnu_ton.phase1_to_transition_hp, hardcoded.phase1_to_transition_hp);
        assert_eq!(gnu_ton.transition_to_phase2_hp, hardcoded.transition_to_phase2_hp);
        assert_eq!(gnu_ton.phase2_to_enrage_hp, hardcoded.phase2_to_enrage_hp);
        assert_eq!(gnu_ton.intro_seconds, hardcoded.intro_seconds);
        assert_eq!(gnu_ton.transition_seconds, hardcoded.transition_seconds);
        assert_eq!(gnu_ton.stagger_seconds, hardcoded.stagger_seconds);
        assert_eq!(gnu_ton.death_seconds, hardcoded.death_seconds);
        assert_eq!(gnu_ton.stagger_threshold, hardcoded.stagger_threshold);
        assert_eq!(gnu_ton.stagger_window_seconds, hardcoded.stagger_window_seconds);
        assert_eq!(gnu_ton.music_intro, hardcoded.music_intro);
        assert_eq!(gnu_ton.music_phase1, hardcoded.music_phase1);
        assert_eq!(gnu_ton.music_phase2, hardcoded.music_phase2);
        assert_eq!(gnu_ton.music_enrage, hardcoded.music_enrage);
    }
}
