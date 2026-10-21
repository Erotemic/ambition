//! Every telegraph cue a boss pattern names is a sound the SFX renderer makes.
//!
//! A telegraph's `cue` plays once on the tell's rising edge, through the
//! open-ended `SfxMessage::Play { id, .. }` path. An id the bank does not hold
//! plays NOTHING and says nothing, so a typo'd or never-authored cue is a
//! silent tell, and half of what tells two moves apart (`TelegraphSpec`) is
//! gone. The bank is a build product (`scripts/regen/sfx.sh`); its source is
//! one recipe per cue under `tools/ambition_sfx_renderer/sounds/active/`.

use std::collections::BTreeSet;
use std::path::PathBuf;

use ambition_boss_encounter::pattern::profile::BossBehaviorProfile;
use ambition_characters::brain::boss_pattern::{BossAttackPattern, BossPatternStep};
use ambition_content::bosses::boss_profiles_ron;

fn recipes() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tools/ambition_sfx_renderer/sounds/active")
}

fn cues_in(steps: &[BossPatternStep], out: &mut BTreeSet<String>) {
    for step in steps {
        match step {
            BossPatternStep::Telegraph { telegraph: Some(spec), .. } => {
                out.extend(spec.cue.iter().cloned());
            }
            BossPatternStep::Select { table } => {
                for arm in table {
                    cues_in(&arm.steps, out);
                }
            }
            _ => {}
        }
    }
}

#[test]
fn every_boss_telegraph_cue_has_a_recipe() {
    let profiles: std::collections::BTreeMap<String, BossBehaviorProfile> =
        ron::from_str(&boss_profiles_ron()).expect("boss_profiles.ron parses");
    let mut cues = BTreeSet::new();
    for profile in profiles.values() {
        if let BossAttackPattern::Scripted { phase1, phase2, enrage } = &profile.attack_pattern {
            for pattern in [phase1, phase2, enrage] {
                cues_in(&pattern.steps, &mut cues);
                for stance in pattern.stances.values() {
                    cues_in(stance, &mut cues);
                }
            }
        }
    }
    // Anti-vacuity: the T-rex alone tells with a dozen.
    assert!(cues.len() >= 12, "only {} telegraph cues were found: the walk is broken", cues.len());
    if !recipes().is_dir() {
        // A checkout without the renderer submodule cannot answer.
        eprintln!("no SFX recipes at {}: nothing to compare", recipes().display());
        return;
    }
    let missing: Vec<&String> =
        cues.iter().filter(|cue| !recipes().join(format!("{cue}.sfx.yaml")).is_file()).collect();
    assert!(missing.is_empty(), "telegraph cues with no recipe, which play silence: {missing:?}");
}

/// The T-rex's own sounds (his module's cues, beyond his pattern's).
#[test]
fn every_cue_the_trex_plays_has_a_recipe() {
    if !recipes().is_dir() {
        eprintln!("no SFX recipes at {}: nothing to compare", recipes().display());
        return;
    }
    let missing: Vec<&str> = ambition_content_modules::trex::VOICE
        .iter()
        .chain(ambition_content_modules::trex::BODY.iter())
        .copied()
        .filter(|cue| !recipes().join(format!("{cue}.sfx.yaml")).is_file())
        .collect();
    assert!(missing.is_empty(), "cues the T-rex plays with no recipe, which play silence: {missing:?}");
}

/// The Mockingbird's own sounds (its module's cues, beyond its pattern's).
#[test]
fn every_cue_the_mockingbird_plays_has_a_recipe() {
    if !recipes().is_dir() {
        eprintln!("no SFX recipes at {}: nothing to compare", recipes().display());
        return;
    }
    let missing: Vec<&str> = ambition_content_modules::mockingbird::SOUNDS
        .iter()
        .copied()
        .filter(|cue| !recipes().join(format!("{cue}.sfx.yaml")).is_file())
        .collect();
    assert!(missing.is_empty(), "cues the Mockingbird plays with no recipe, which play silence: {missing:?}");
}

/// The Inugami's own sounds (its howl at each size, and the rest), and those
/// of Kagero, whom it calls.
#[test]
fn every_cue_the_inugami_and_kagero_play_has_a_recipe() {
    if !recipes().is_dir() {
        eprintln!("no SFX recipes at {}: nothing to compare", recipes().display());
        return;
    }
    let ids = recipe_ids();
    let missing: Vec<&str> = ambition_content_modules::inugami::SOUNDS
        .iter()
        .chain(ambition_content_modules::kagero::SOUNDS.iter())
        .copied()
        .filter(|cue| !ids.contains(*cue))
        .collect();
    assert!(missing.is_empty(), "cues the Inugami or Kagero play with no recipe, which play silence: {missing:?}");
}

/// The `id` of every recipe under `sounds/active`, at any depth: a VFX's
/// companion cues live in `vfx/<owner>/`, named by their file, not their id.
fn recipe_ids() -> BTreeSet<String> {
    fn walk(dir: &std::path::Path, out: &mut BTreeSet<String>) {
        for entry in std::fs::read_dir(dir).expect("the recipes are readable").flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.to_string_lossy().ends_with(".sfx.yaml") {
                let text = std::fs::read_to_string(&path).expect("a recipe is readable");
                if let Some(id) = text.lines().find_map(|line| line.strip_prefix("id: ")) {
                    out.insert(id.trim().to_string());
                }
            }
        }
    }
    let mut out = BTreeSet::new();
    walk(&recipes(), &mut out);
    out
}
