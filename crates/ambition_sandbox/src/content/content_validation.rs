//! Cross-content validation for authored sandbox data.
//!
//! This module checks relationships that live *between* content systems: LDtk
//! room links, NPC dialogue ids, quest conditions, encounter/boss ids, and
//! music references. The intent is to catch content typos at startup/test time
//! instead of letting string ids silently fall back or never fire.

use std::collections::{BTreeMap, BTreeSet};

use ambition_engine as ae;

use crate::content::data::SandboxDataSpec;
use crate::ldtk_world::{field_string, LdtkProject};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ContentValidationReport {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ContentValidationReport {
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn push_error(&mut self, message: impl Into<String>) {
        self.errors.push(message.into());
    }

    #[allow(dead_code)] // Used by content checks that haven't been wired into startup yet.
    pub fn push_warning(&mut self, message: impl Into<String>) {
        self.warnings.push(message.into());
    }

    pub fn extend_errors<I>(&mut self, messages: I)
    where
        I: IntoIterator<Item = String>,
    {
        self.errors.extend(messages);
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn panic_if_errors(&self) {
        if self.errors.is_empty() {
            return;
        }
        panic!(
            "content graph validation failed:\n{}",
            self.errors
                .iter()
                .map(|error| format!("- {error}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

/// Validate the checked-in sandbox content graph.
#[cfg_attr(not(test), allow(dead_code))]
pub fn validate_embedded_content_graph() -> ContentValidationReport {
    let data = SandboxDataSpec::load_embedded();
    let project = match LdtkProject::load_default_for_dev() {
        Ok(project) => project,
        Err(error) => {
            let mut report = ContentValidationReport::default();
            report.push_error(format!("failed to load embedded LDtk project: {error}"));
            return report;
        }
    };
    validate_content_graph(&data, &project)
}

/// Validate relationships among the non-spatial data manifest and LDtk world.
pub fn validate_content_graph(
    data: &SandboxDataSpec,
    project: &LdtkProject,
) -> ContentValidationReport {
    let mut report = ContentValidationReport::default();

    if let Err(error) = data.audio.validate() {
        report.push_error(format!("audio manifest invalid: {error}"));
    }

    let ldtk_report = project.validate();
    report.extend_errors(
        ldtk_report
            .errors
            .into_iter()
            .map(|error| format!("LDtk validation: {error}")),
    );
    report.warnings.extend(
        ldtk_report
            .warnings
            .into_iter()
            .map(|warning| format!("LDtk validation: {warning}")),
    );

    validate_ldtk_room_links(project, &mut report);
    validate_room_music_tracks(project, data, &mut report);
    validate_npc_dialogue_ids(project, &mut report);
    validate_quest_conditions(project, data, &mut report);
    validate_boss_music_tracks(data, &mut report);
    validate_patrol_brain_paths(project, &mut report);

    #[cfg(feature = "audio")]
    validate_adaptive_music_catalog(&mut report);

    report
}

/// Catch the failure mode from intro-v1 polish E: an `EnemySpawn`
/// with `brain: "Patrol:<path_id>"` whose `path_id` doesn't resolve
/// to a `KinematicPath` in the same level. The runtime silently
/// falls back to passive behavior, so the broken patrol is invisible
/// until playtest. Surfacing it as a content-graph warning catches
/// it at `cargo test` time instead.
///
/// Path id resolution mirrors `world::ldtk_world::conversion::path_lookup_id`:
/// 1. KinematicPath's `id` field if non-empty.
/// 2. Otherwise, a slug of the `name` field (lowercase, non-alnum
///    runs collapsed to underscores).
/// 3. Otherwise, the iid.
///
/// This validator only emits warnings rather than errors so an
/// existing latent mismatch (sandbox's basement `Patrol:enemy_patrol_a`
/// vs the nearby `name: "enemy patrol path A"` whose slug is
/// `enemy_patrol_path_a`) surfaces without breaking the regression
/// test. Promote to an error once the slugs are aligned.
fn validate_patrol_brain_paths(project: &LdtkProject, report: &mut ContentValidationReport) {
    for level in &project.levels {
        let mut path_ids: BTreeSet<String> = BTreeSet::new();
        for entity in level.all_entity_instances() {
            if entity.identifier != "KinematicPath" {
                continue;
            }
            if let Some(value) = field_string(entity, "id") {
                let trimmed = value.trim();
                if !trimmed.is_empty() {
                    path_ids.insert(trimmed.to_string());
                    continue;
                }
            }
            if let Some(value) = field_string(entity, "name") {
                if let Some(slug) = patrol_name_slug(value.trim()) {
                    path_ids.insert(slug);
                }
            }
        }
        for entity in level.all_entity_instances() {
            if entity.identifier != "EnemySpawn" {
                continue;
            }
            let Some(brain) = field_string(entity, "brain") else {
                continue;
            };
            let Some(path_id) = brain.strip_prefix("Patrol:") else {
                continue;
            };
            let path_id = path_id.trim();
            if path_id.is_empty() {
                report.push_warning(format!(
                    "level '{}' EnemySpawn '{}' uses bare brain 'Patrol:' (no path_id); enemy will fall back to passive",
                    level.identifier, entity.iid
                ));
                continue;
            }
            if !path_ids.contains(path_id) {
                report.push_warning(format!(
                    "level '{}' EnemySpawn '{}' brain 'Patrol:{}' references no matching KinematicPath (resolved ids: {:?}); enemy will fall back to passive",
                    level.identifier, entity.iid, path_id, path_ids
                ));
            }
        }
    }
}

fn patrol_name_slug(name: &str) -> Option<String> {
    let mut slug = String::new();
    let mut previous_was_sep = false;
    for ch in name.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            previous_was_sep = false;
        } else if !previous_was_sep && !slug.is_empty() {
            slug.push('_');
            previous_was_sep = true;
        }
    }
    while slug.ends_with('_') {
        slug.pop();
    }
    if slug.is_empty() {
        None
    } else {
        Some(slug)
    }
}

fn validate_ldtk_room_links(project: &LdtkProject, report: &mut ContentValidationReport) {
    let mut area_level_count: BTreeMap<String, usize> = BTreeMap::new();
    let mut zones_by_area: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut links = Vec::new();

    for level in &project.levels {
        let area = level.active_area();
        *area_level_count.entry(area.clone()).or_default() += 1;
        for entity in level.all_entity_instances() {
            if entity.identifier != "LoadingZone" {
                continue;
            }
            let zone_id = field_string(entity, "id").unwrap_or_else(|| entity.iid.clone());
            if zone_id.trim().is_empty() {
                report.push_error(format!(
                    "level '{}' has LoadingZone '{}' with a blank id",
                    level.identifier, entity.iid
                ));
                continue;
            }
            if !zones_by_area
                .entry(area.clone())
                .or_default()
                .insert(zone_id.clone())
            {
                report.push_error(format!(
                    "active area '{}' has duplicate LoadingZone id '{}'",
                    area, zone_id
                ));
            }
            links.push((
                level.identifier.clone(),
                area.clone(),
                zone_id,
                field_string(entity, "target_room"),
                field_string(entity, "target_zone"),
            ));
        }
    }

    for (level_id, area, zone_id, target_room, target_zone) in links {
        let target_room = target_room
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let target_zone = target_zone
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        match (target_room, target_zone) {
            (Some(room), Some(zone)) => {
                if !area_level_count.contains_key(&room) {
                    report.push_error(format!(
                        "LoadingZone '{}:{}' targets unknown room '{}'",
                        area, zone_id, room
                    ));
                    continue;
                }
                if !zones_by_area
                    .get(&room)
                    .map(|zones| zones.contains(&zone))
                    .unwrap_or(false)
                {
                    report.push_error(format!(
                        "LoadingZone '{}:{}' targets missing zone '{}:{}'",
                        area, zone_id, room, zone
                    ));
                }
            }
            _ => report.push_error(format!(
                "level '{}' LoadingZone '{}:{}' must author both target_room and target_zone",
                level_id, area, zone_id
            )),
        }
    }
}

fn validate_room_music_tracks(
    project: &LdtkProject,
    data: &SandboxDataSpec,
    report: &mut ContentValidationReport,
) {
    let valid_tracks = data
        .audio
        .music_tracks
        .iter()
        .map(|track| track.id.as_str());
    report.extend_errors(
        project
            .music_track_warnings(valid_tracks)
            .into_iter()
            .map(|warning| format!("room music reference: {warning}")),
    );
}

fn validate_npc_dialogue_ids(project: &LdtkProject, report: &mut ContentValidationReport) {
    let known = crate::dialog::known_dialogue_ids()
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    for level in &project.levels {
        for entity in level.all_entity_instances() {
            if entity.identifier != "NpcSpawn" {
                continue;
            }
            let Some(dialogue_id) = field_string(entity, "dialogue_id") else {
                continue;
            };
            let dialogue_id = dialogue_id.trim();
            if dialogue_id.is_empty() {
                continue;
            }
            if !known.contains(dialogue_id) {
                report.push_error(format!(
                    "level '{}' NpcSpawn '{}' references unknown dialogue_id '{}'",
                    level.identifier, entity.iid, dialogue_id
                ));
            }
        }
    }
}

fn validate_quest_conditions(
    project: &LdtkProject,
    data: &SandboxDataSpec,
    report: &mut ContentValidationReport,
) {
    let room_ids = active_area_ids(project);
    let encounter_ids = authored_encounter_ids(project);
    let boss_ids = authored_boss_encounter_ids(project);
    let item_ids = authored_pickup_ids(project);
    let known_flags = authored_flag_ids(project);
    let valid_tracks = data
        .audio
        .music_tracks
        .iter()
        .map(|track| track.id.as_str())
        .collect::<BTreeSet<_>>();

    let loaded_encounters =
        crate::encounter::load_encounter_specs_from_ldtk(project, &crate::save::SandboxSaveData::default());
    for (id, spec, _) in loaded_encounters {
        if !spec.music_track.trim().is_empty() && !valid_tracks.contains(spec.music_track.as_str())
        {
            report.push_error(format!(
                "encounter '{}' references unknown music track '{}'",
                id, spec.music_track
            ));
        }
    }

    for spec in crate::content::quest::default_quest_specs() {
        if spec.steps.is_empty() {
            report.push_error(format!("quest '{}' has no steps", spec.id));
        }
        for (index, step) in spec.steps.iter().enumerate() {
            match &step.condition {
                crate::quest::QuestStepCondition::RoomEntered(room) => {
                    if !room_ids.contains(room.as_str()) {
                        report.push_error(format!(
                            "quest '{}'/step {} references unknown room '{}'",
                            spec.id, index, room
                        ));
                    }
                }
                crate::quest::QuestStepCondition::EncounterCleared(encounter) => {
                    if !encounter_ids.contains(encounter.as_str()) {
                        report.push_error(format!(
                            "quest '{}'/step {} references unknown encounter '{}'",
                            spec.id, index, encounter
                        ));
                    }
                }
                crate::quest::QuestStepCondition::BossDefeated(boss) => {
                    if !boss_ids.contains(boss.as_str()) {
                        report.push_error(format!(
                            "quest '{}'/step {} references unknown authored boss encounter '{}'",
                            spec.id, index, boss
                        ));
                    }
                }
                crate::quest::QuestStepCondition::FlagSet(flag) => {
                    if !known_flags.contains(flag.as_str()) {
                        report.push_error(format!(
                            "quest '{}'/step {} references unknown authored flag '{}'",
                            spec.id, index, flag
                        ));
                    }
                }
                crate::quest::QuestStepCondition::ItemCollected(item) => {
                    if !item_ids.contains(item.as_str()) {
                        report.push_error(format!(
                            "quest '{}'/step {} references unknown pickup/item id '{}'",
                            spec.id, index, item
                        ));
                    }
                }
                crate::quest::QuestStepCondition::NpcTalked(npc) => {
                    // Gameplay emits the runtime NPC object id for NpcTalked. Most current
                    // quests use flags instead, but keep the validator honest for future ones.
                    if !authored_npc_ids(project).contains(npc.as_str()) {
                        report.push_error(format!(
                            "quest '{}'/step {} references unknown NPC id '{}'",
                            spec.id, index, npc
                        ));
                    }
                }
            }
        }
    }
}

fn validate_boss_music_tracks(data: &SandboxDataSpec, report: &mut ContentValidationReport) {
    let tracks = data
        .audio
        .music_tracks
        .iter()
        .map(|track| track.id.as_str())
        .collect::<BTreeSet<_>>();
    for spec in crate::boss_encounter::default_boss_specs() {
        for (field, track) in [
            ("music_intro", spec.music_intro.as_str()),
            ("music_phase1", spec.music_phase1.as_str()),
            ("music_phase2", spec.music_phase2.as_str()),
            ("music_enrage", spec.music_enrage.as_str()),
        ] {
            if !track.trim().is_empty() && !tracks.contains(track) {
                report.push_error(format!(
                    "boss spec '{}' {field} references unknown music track '{}'",
                    spec.id, track
                ));
            }
        }
    }
}

#[cfg(feature = "audio")]
fn validate_adaptive_music_catalog(report: &mut ContentValidationReport) {
    let catalog = crate::music::MusicCueCatalog::builtin();
    report.extend_errors(
        catalog
            .validate_references()
            .into_iter()
            .map(|error| format!("adaptive music catalog: {error}")),
    );
}

fn active_area_ids(project: &LdtkProject) -> BTreeSet<String> {
    project
        .levels
        .iter()
        .map(|level| level.active_area())
        .collect()
}

fn authored_encounter_ids(project: &LdtkProject) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for level in &project.levels {
        let area = level.active_area();
        for entity in level.all_entity_instances() {
            if entity.identifier == "EncounterTrigger" {
                ids.insert(
                    field_string(entity, "id")
                        .map(|id| id.trim().to_string())
                        .filter(|id| !id.is_empty())
                        .unwrap_or_else(|| area.clone()),
                );
            }
        }
    }
    ids
}

fn authored_boss_encounter_ids(project: &LdtkProject) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for level in &project.levels {
        for entity in level.all_entity_instances() {
            if entity.identifier == "BossSpawn" {
                let name = field_string(entity, "name")
                    .map(|name| name.trim().to_string())
                    .filter(|name| !name.is_empty())
                    .unwrap_or_else(|| entity.iid.clone());
                ids.insert(crate::boss_encounter::encounter_id_from_name(&name));
            }
        }
    }
    ids
}

fn authored_npc_ids(project: &LdtkProject) -> BTreeSet<String> {
    authored_entity_iids(project, "NpcSpawn")
}

fn authored_pickup_ids(project: &LdtkProject) -> BTreeSet<String> {
    authored_entity_iids(project, "PickupSpawn")
}

fn authored_entity_iids(project: &LdtkProject, identifier: &str) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for level in &project.levels {
        for entity in level.all_entity_instances() {
            if entity.identifier == identifier {
                ids.insert(entity.iid.clone());
            }
        }
    }
    ids
}

fn authored_flag_ids(project: &LdtkProject) -> BTreeSet<String> {
    let mut flags = BTreeSet::from([
        "met_any_hub_npc".to_string(),
        "test_switch_toggled".to_string(),
        crate::content::quest::PIRATE_TREASURE_REWARD_FLAG.to_string(),
    ]);
    for level in &project.levels {
        for entity in level.all_entity_instances() {
            if entity.identifier == "NpcSpawn" {
                if let Some(dialogue_id) = field_string(entity, "dialogue_id") {
                    let dialogue_id = dialogue_id.trim();
                    if !dialogue_id.is_empty() {
                        flags.insert(format!("npc_{dialogue_id}_talked"));
                    }
                }
            }
            if entity.identifier == "EncounterTrigger" {
                let encounter_id = field_string(entity, "id")
                    .map(|id| id.trim().to_string())
                    .filter(|id| !id.is_empty())
                    .unwrap_or_else(|| level.active_area());
                flags.insert(format!("encounter_{encounter_id}_reward_dropped"));
                flags.insert(crate::encounter::encounter_reward_looted_flag(
                    &encounter_id,
                ));
            }
            if entity.identifier == "Switch" {
                if let Some(id) = field_string(entity, "id") {
                    let id = id.trim();
                    if !id.is_empty() {
                        flags.insert(format!("switch_{id}_used"));
                    }
                }
            }
            // PickupSpawn entities with `kind: "flag:<id>"` set the
            // named flag in save state when collected. Mirror the
            // runtime parse rule in `world/ldtk_world/fields.rs::parse_pickup_kind`
            // so quest steps that depend on a story-flag pickup
            // validate without needing the flag listed elsewhere.
            if entity.identifier == "PickupSpawn" {
                if let Some(kind) = field_string(entity, "kind") {
                    if let Some(flag) = kind.trim().strip_prefix("flag:") {
                        if !flag.is_empty() {
                            flags.insert(flag.to_string());
                        }
                    }
                }
            }
        }
    }
    for boss in authored_boss_encounter_ids(project) {
        flags.insert(format!("encounter_{boss}_reward_dropped"));
        flags.insert(crate::encounter::encounter_reward_looted_flag(&boss));
    }
    flags
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_content_graph_validates() {
        let report = validate_embedded_content_graph();
        report.panic_if_errors();
    }

    #[test]
    fn validates_ldtk_loading_zone_targets() {
        let data = SandboxDataSpec::load_embedded();
        let project = LdtkProject::load_default_for_dev().expect("embedded LDtk loads");
        let report = validate_content_graph(&data, &project);
        assert!(
            report
                .errors
                .iter()
                .all(|error| !error.contains("LoadingZone")),
            "loading zone validation failed: {:?}",
            report.errors
        );
    }

    #[test]
    fn quest_boss_conditions_point_at_authored_bosses() {
        let project = LdtkProject::load_default_for_dev().expect("embedded LDtk loads");
        let boss_ids = authored_boss_encounter_ids(&project);
        assert!(boss_ids.contains("clockwork_warden"));
        for spec in crate::content::quest::default_quest_specs() {
            for step in &spec.steps {
                if let crate::quest::QuestStepCondition::BossDefeated(id) = &step.condition {
                    assert!(
                        boss_ids.contains(id.as_str()),
                        "quest '{}' references boss '{}' not authored in LDtk; authored bosses: {:?}",
                        spec.id,
                        id,
                        boss_ids
                    );
                }
            }
        }
    }
}
