use crate::engine_core as ae;
use crate::save::PersistedEncounterState;

use crate::ldtk_world::LdtkProject;

use super::{EncounterMobSpec, EncounterSpec, EncounterWaveSpec, LockWallSpec};

/// Read all `EncounterTrigger` + `LockWall` markers in the active
/// LDtk project, build matching `EncounterSpec`s, and register them.
///
/// Runs once after startup (or after a hot reload). The goblin_encounter area
/// gets its waves from a hard-coded `goblin_encounter_wave_specs()` rather
/// than from LDtk EnemySpawn markers, so the spawn timeline (delays
/// between waves and within waves) lives in code where it's easier to
/// tune than in the LDtk JSON.
pub fn load_encounter_specs_from_ldtk(
    project: &LdtkProject,
    save: &crate::save::SandboxSaveData,
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

        // Hard-coded waves for known encounters. Falls back to one
        // wave assembled from LDtk EnemySpawn markers for areas the
        // sandbox doesn't have a builder for yet.
        let waves = match trigger_id.as_str() {
            "goblin_encounter" => goblin_encounter_wave_specs(),
            _ => fallback_waves_from_enemy_spawns(level),
        };

        let spec = EncounterSpec {
            id: trigger_id.clone(),
            waves,
            trigger_min,
            trigger_size,
            camera_zoom,
            lock_wall,
            intro_seconds: 2.5,
            // goblin_encounter is now driven by generated_music.rs: intro -> adaptive stem loops -> outro.
            music_track: if trigger_id == "goblin_encounter" {
                String::new()
            } else {
                "pulse_drift_voyage".into()
            },
        };
        let persisted = save.encounter(&trigger_id);
        out.push((trigger_id, spec, persisted));
    }
    out
}

/// Build the canonical mob-lab wave spec — the user-authored fight
/// sequence:
///
/// - Wave 1: 2 mid-tier enemies, one each side (no sandbag respawn).
/// - Wave 2: 2 goblins immediately + 1 big goblin after a few seconds
///   (delay-based sub-spawn — wave 2 doesn't clear until all three
///   are down).
/// - Wave 3: 2 big goblins.
///
/// Positions assume the goblin_encounter arena floor (y=608) and span from
/// the divider-jamb edge (~x=720) to the back wall (~x=1584). The
/// arena is roughly 850x600 of usable space.
pub fn goblin_encounter_wave_specs() -> Vec<EncounterWaveSpec> {
    // Active-area-local coords. The arena floor is y=608 and the
    // doorway opening is at x=480-704. The encounter trigger spans
    // x=920-1160, so wave mobs sit deeper still — past the trigger
    // so they're visible after the camera zooms out and so the
    // player has crossed into the arena before the wall slams.
    let left_x: f32 = 1180.0;
    let right_x: f32 = 1500.0;
    let floor_y: f32 = 580.0; // ~30 px above the floor (mob centered)
    let goblin_size = [22.0, 38.0];
    let big_size = [32.0, 56.0];
    vec![
        EncounterWaveSpec {
            label: "wave 1 — flank the doorway".into(),
            mobs: vec![
                EncounterMobSpec::new("medium_striker", [left_x, floor_y]).with_size(goblin_size),
                EncounterMobSpec::new("medium_striker", [right_x, floor_y]).with_size(goblin_size),
            ],
        },
        EncounterWaveSpec {
            label: "wave 2 — goblins + heavy".into(),
            mobs: vec![
                EncounterMobSpec::new("medium_striker", [left_x, floor_y]).with_size(goblin_size),
                EncounterMobSpec::new("medium_striker", [right_x, floor_y])
                    .with_size(goblin_size)
                    .with_delay(0.70),
                // Big goblin reinforcement on a timer, fires whether
                // or not the goblins are still up.
                EncounterMobSpec::new("large_brute", [(left_x + right_x) * 0.5, floor_y - 18.0])
                    .with_size(big_size)
                    .with_delay(2.60),
            ],
        },
        EncounterWaveSpec {
            label: "wave 3 — heavy duo".into(),
            mobs: vec![
                EncounterMobSpec::new("large_brute", [left_x, floor_y - 18.0]).with_size(big_size),
                EncounterMobSpec::new("large_brute", [right_x, floor_y - 18.0]).with_size(big_size),
            ],
        },
    ]
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
