//! LDtk → `EncounterSpec` loader plus the content-installed wave book.
//!
//! `load_encounter_specs_from_ldtk` scans `EncounterTrigger`/`LockWall` markers
//! and builds one spec per area. Authored multi-wave timelines live in content
//! (`ambition_content/.../encounters/*.ron`) and are installed into
//! `ambition_encounter`'s wave book, keyed by trigger id; any unbooked
//! encounter falls back to a single wave from its `EnemySpawn` markers. The
//! loader names no specific encounter — that's the content seam.

use ambition_persistence::save_data::PersistedEncounterState;
#[cfg(test)]
use std::collections::HashMap;

use ambition_encounter::{EncounterMobSpec, EncounterSpec, EncounterWaveSpec, LockWallSpec};

/// Test fixture: the lib's own loader tests read content's authoritative
/// `encounters/goblin_encounter.ron` at compile time (cfg(test) only —
/// production embeds no encounter wave data and requires the content install).
#[cfg(test)]
static ENCOUNTER_WAVE_BOOK_FIXTURE: std::sync::LazyLock<HashMap<String, Vec<EncounterWaveSpec>>> =
    std::sync::LazyLock::new(|| {
        ron::from_str(include_str!(
            "../../../game/ambition_content/assets/data/encounters/goblin_encounter.ron"
        ))
        .expect("goblin_encounter.ron should parse as an encounter wave book")
    });

/// The authored multi-wave timeline for a trigger id, or `None` to fall back to
/// one wave from the level's LDtk `EnemySpawn` markers. Production reads the
/// content-installed encounter wave book in `ambition_encounter`; lib tests
/// fall back to content's authored RON fixture when no book is installed.
fn authored_encounter_waves(
    book: Option<&ambition_encounter::EncounterWaveBook>,
    id: &str,
) -> Option<Vec<EncounterWaveSpec>> {
    if let Some(waves) = ambition_encounter::authored_encounter_waves(book, id) {
        return Some(waves);
    }
    #[cfg(test)]
    {
        return ENCOUNTER_WAVE_BOOK_FIXTURE.get(id).cloned();
    }
    #[cfg(not(test))]
    None
}

/// Read every room's authored `EncounterTrigger` + `LockWall` and build matching
/// `EncounterSpec`s.
///
/// Runs once after startup (or after a hot reload). An encounter whose trigger
/// id has an authored entry in the content-installed wave book gets that
/// multi-wave timeline — the spawn cadence is data in `encounters/*.ron`, not in
/// the map. Any other encounter falls back to one wave assembled from the room's
/// own `EnemySpawn` placements. The loader names no specific encounter.
pub fn load_encounter_specs_from_rooms(
    rooms: &[ambition_platformer2d_world::rooms::RoomSpec],
    save: &ambition_persistence::save_data::AmbitionGameSaveData,
    // `None` is a composition with no authored encounters, which falls back to the room's own spawn
    // markers exactly as an unrecognised trigger id always did.
    waves: Option<&ambition_encounter::EncounterWaveBook>,
) -> Vec<(String, EncounterSpec, PersistedEncounterState)> {
    let mut out = Vec::new();
    for room in rooms {
        let Some(trigger) = room.encounter_triggers.first() else {
            continue;
        };
        // An unset `id` means "name it after the area", which is a fact the room
        // has and the IR deliberately does not.
        let trigger_id = if trigger.id.trim().is_empty() {
            room.id.clone()
        } else {
            trigger.id.trim().to_string()
        };
        let camera_zoom = trigger.camera_zoom.unwrap_or(1.2);
        let trigger_min = [trigger.min.x, trigger.min.y];
        let trigger_size = [trigger.size.x, trigger.size.y];

        // The lock wall marker (one per area, optional).
        let lock_wall = room.lock_walls.first().map(|wall| LockWallSpec {
            min: [wall.min.x, wall.min.y],
            size: [wall.size.x, wall.size.y],
        });

        let authored = authored_encounter_waves(waves, &trigger_id);
        let waves = authored
            .clone()
            .unwrap_or_else(|| fallback_waves_from_enemy_spawns(room));

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
            reward: ambition_encounter::spec::default_encounter_reward(),
        };
        let persisted = save.encounter(&trigger_id);
        out.push((trigger_id, spec, persisted));
    }
    out
}

/// The mob `kind` string for an authored brain — the inverse of the map
/// reader's `parse_enemy_brain`.
///
/// `Passive` has two pre-images and this collapses them. The map reader turns an EMPTY
/// `brain` field into `Passive`, and the literal `"Passive"` into `Passive` too; the old
/// project-reading fallback distinguished them, defaulting an empty field to
/// `"medium_striker"`.
fn wave_mob_kind(brain: &ambition_entity_catalog::placements::CharacterBrain) -> String {
    use ambition_entity_catalog::placements::CharacterBrain;
    match brain {
        CharacterBrain::Custom(kind) => kind.clone(),
        CharacterBrain::Guard { leash_radius } => format!("Guard:{leash_radius}"),
        CharacterBrain::Patrol { .. } => "Patrol".to_string(),
        CharacterBrain::Passive => "Passive".to_string(),
    }
}

fn fallback_waves_from_enemy_spawns(
    room: &ambition_platformer2d_world::rooms::RoomSpec,
) -> Vec<EncounterWaveSpec> {
    let mut wave_mobs = Vec::new();
    for spawn in &room.enemy_spawns {
        // The project reader computed `px + size * 0.5`; the IR's authored
        // footprint is min/max, so the same point is the midpoint of the two.
        let centre = (spawn.aabb.min + spawn.aabb.max) * 0.5;
        let mut mob =
            EncounterMobSpec::new(wave_mob_kind(&spawn.payload.brain), [centre.x, centre.y]);
        // the marker's own art identity. A marker-derived wave mob is the
        // same body reached by a different road, so it must not be the one path
        // left wearing its instance id. The IR REQUIRES `character_id`, so
        // unlike the project reader there is no absent case to fall back from.
        let character = spawn.payload.character_id.as_str().trim();
        if !character.is_empty() {
            mob = mob.with_character(character.to_string());
        }
        wave_mobs.push(mob);
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
        let waves = authored_encounter_waves(None, "goblin_encounter")
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
        //
        // the heavy is a CHARACTER now, not a role. The shape this asserts — the third wave is
        // uniformly the heavy — is unchanged; what the heavy IS finally has an answer.
        assert!(waves[0].mobs.iter().all(|m| m.kind == "medium_striker"));
        assert!(waves[2].mobs.iter().all(|m| m.kind == "npc_goblin_brute"));

        // Wave 2 carries a timed heavy reinforcement (positive delay).
        assert!(
            waves[1]
                .mobs
                .iter()
                .any(|m| m.kind == "npc_goblin_brute" && m.delay > 0.0),
            "wave 2 should include a delayed heavy",
        );
    }
}
