//! LDtk → `EncounterSpec` loader plus the content-installed wave book.
//!
//! `load_encounter_specs_from_ldtk` scans `EncounterTrigger`/`LockWall` markers
//! and builds one spec per area. Authored multi-wave timelines live in content
//! (`ambition_content/.../encounters/*.ron`) and are installed via
//! `install_encounter_waves` into the `ENCOUNTER_WAVE_BOOK`, keyed by trigger
//! id; any unbooked encounter falls back to a single wave from its `EnemySpawn`
//! markers. The loader names no specific encounter — that's the content seam.

use crate::persistence::save_data::PersistedEncounterState;

use crate::ldtk_world::LdtkProject;

use super::{EncounterMobSpec, EncounterSpec, EncounterWaveSpec, LockWallSpec};

use std::collections::HashMap;

/// Content-installed encounter wave timelines, keyed by trigger id. An encounter
/// whose id is in this book gets its authored multi-wave sequence; any other
/// encounter falls back to one wave assembled from its LDtk `EnemySpawn`
/// markers. This is the seam that keeps the engine's encounter loader from
/// naming any specific encounter — the goblin (and future) wave data is content
/// (`ambition_content/assets/data/encounters/*.ron`).
///
/// §5 classification (restructuring-blueprint): **content registry** —
/// install-once seam, immutable after install, read from the pure
/// `authored_encounter_waves` helper. Deliberately a process-global `OnceLock`,
/// not a Bevy `Resource` (the reader is the non-system LDtk loader);
/// `install_encounter_waves` + the `cfg(test)` fixture ARE the test-override.
static ENCOUNTER_WAVE_BOOK: std::sync::OnceLock<HashMap<String, Vec<EncounterWaveSpec>>> =
    std::sync::OnceLock::new();

/// Install the authored encounter wave timelines — `ambition_content` calls this
/// at plugin-build time (before the first `load_encounter_specs_from_ldtk`).
pub fn install_encounter_waves(book: HashMap<String, Vec<EncounterWaveSpec>>) {
    let _ = ENCOUNTER_WAVE_BOOK.set(book);
}

/// Test fixture: the lib's own loader tests read content's authoritative
/// `encounters/goblin_encounter.ron` at compile time (cfg(test) only —
/// production embeds no encounter wave data and requires the content install).
#[cfg(test)]
static ENCOUNTER_WAVE_BOOK_FIXTURE: std::sync::LazyLock<HashMap<String, Vec<EncounterWaveSpec>>> =
    std::sync::LazyLock::new(|| {
        ron::from_str(include_str!(
            "../../../ambition_content/assets/data/encounters/goblin_encounter.ron"
        ))
        .expect("goblin_encounter.ron should parse as an encounter wave book")
    });

/// The authored multi-wave timeline for a trigger id, or `None` to fall back to
/// one wave from the level's LDtk `EnemySpawn` markers. Production reads the
/// content install; lib tests read content's authored RON via the fixture.
fn authored_encounter_waves(id: &str) -> Option<Vec<EncounterWaveSpec>> {
    if let Some(book) = ENCOUNTER_WAVE_BOOK.get() {
        return book.get(id).cloned();
    }
    #[cfg(test)]
    {
        return ENCOUNTER_WAVE_BOOK_FIXTURE.get(id).cloned();
    }
    #[cfg(not(test))]
    None
}

/// Read all `EncounterTrigger` + `LockWall` markers in the active
/// LDtk project, build matching `EncounterSpec`s, and register them.
///
/// Runs once after startup (or after a hot reload). An encounter whose trigger
/// id has an authored entry in the content-installed wave book (see
/// [`install_encounter_waves`]) gets that multi-wave timeline — the spawn
/// cadence (delays between/within waves) is data in `encounters/*.ron`, not
/// LDtk JSON. Any other encounter falls back to one wave assembled from its
/// LDtk `EnemySpawn` markers. The loader names no specific encounter.
pub fn load_encounter_specs_from_ldtk(
    project: &LdtkProject,
    save: &crate::persistence::save_data::SandboxSaveData,
) -> Vec<(String, EncounterSpec, PersistedEncounterState)> {
    let mut out = Vec::new();
    for level in &project.levels {
        let area_id = level.active_area();
        let Some(trigger) = level
            .all_entity_instances()
            .find(|e| e.identifier == "EncounterTrigger")
        else {
            continue;
        };
        let trigger_id = crate::ldtk_world::field_string(trigger, "id")
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| area_id.clone());
        let camera_zoom = crate::ldtk_world::field_f32(trigger, "camera_zoom").unwrap_or(1.2);
        let trigger_min = [trigger.px[0] as f32, trigger.px[1] as f32];
        let trigger_size = [trigger.width as f32, trigger.height as f32];

        // Pick up the LockWall marker (one per area, optional).
        let lock_wall = level
            .all_entity_instances()
            .find(|e| e.identifier == "LockWall")
            .map(|e| LockWallSpec {
                min: [e.px[0] as f32, e.px[1] as f32],
                size: [e.width as f32, e.height as f32],
            });

        // Authored waves come from the content-installed wave book (keyed by
        // trigger id); any encounter without an authored timeline falls back to
        // one wave assembled from its LDtk EnemySpawn markers. The engine names
        // no specific encounter.
        let authored = authored_encounter_waves(&trigger_id);
        let waves = authored
            .clone()
            .unwrap_or_else(|| fallback_waves_from_enemy_spawns(level));

        let spec = EncounterSpec {
            id: trigger_id.clone(),
            waves,
            trigger_min,
            trigger_size,
            camera_zoom,
            lock_wall,
            intro_seconds: 2.5,
            // Authored encounters (those with a wave-book entry) are driven by
            // generated_music.rs (intro → adaptive stem loops → outro), signalled
            // by an empty track id; marker-only encounters use the shared loop.
            music_track: if authored.is_some() {
                String::new()
            } else {
                "pulse_drift_voyage".into()
            },
            reward: crate::encounter::spec::default_encounter_reward(),
        };
        let persisted = save.encounter(&trigger_id);
        out.push((trigger_id, spec, persisted));
    }
    out
}

fn fallback_waves_from_enemy_spawns(
    level: &crate::ldtk_world::LdtkLevel,
) -> Vec<EncounterWaveSpec> {
    let mut wave_mobs = Vec::new();
    for entity in level.all_entity_instances() {
        if entity.identifier != "EnemySpawn" {
            continue;
        }
        let kind = crate::ldtk_world::field_string(entity, "brain")
            .unwrap_or_else(|| "medium_striker".into());
        wave_mobs.push(EncounterMobSpec::new(
            kind,
            [
                entity.px[0] as f32 + entity.width as f32 * 0.5,
                entity.px[1] as f32 + entity.height as f32 * 0.5,
            ],
        ));
    }
    if wave_mobs.is_empty() {
        Vec::new()
    } else {
        vec![EncounterWaveSpec {
            label: "wave 1".into(),
            mobs: wave_mobs,
        }]
    }
}

#[cfg(test)]
mod loading_tests {
    use super::*;

    #[test]
    fn goblin_waves_escalate_and_spawn_past_the_trigger() {
        let waves = authored_encounter_waves("goblin_encounter")
            .expect("goblin_encounter has an authored wave book entry");
        assert_eq!(waves.len(), 3, "three authored waves");

        // Documented spatial invariant: every wave mob sits past the
        // encounter trigger's right edge (~1160) so it is on-screen after
        // the camera zooms out and the player has entered the arena.
        const TRIGGER_RIGHT: f32 = 1160.0;
        for wave in &waves {
            assert!(!wave.mobs.is_empty(), "wave '{}' has no mobs", wave.label);
            for mob in &wave.mobs {
                assert!(
                    mob.spawn[0] > TRIGGER_RIGHT,
                    "mob {:?} at x={} should spawn past the trigger",
                    mob.kind,
                    mob.spawn[0],
                );
                assert!(mob.delay >= 0.0, "negative spawn delay for {:?}", mob.kind);
                assert!(
                    mob.size[0] > 0.0 && mob.size[1] > 0.0,
                    "non-positive mob size"
                );
            }
        }

        // Escalation: wave 1 is light strikers, wave 3 is all heavies.
        assert!(waves[0].mobs.iter().all(|m| m.kind == "medium_striker"));
        assert!(waves[2].mobs.iter().all(|m| m.kind == "large_brute"));

        // Wave 2 carries a timed heavy reinforcement (positive delay).
        assert!(
            waves[1]
                .mobs
                .iter()
                .any(|m| m.kind == "large_brute" && m.delay > 0.0),
            "wave 2 should include a delayed heavy",
        );
    }
}
