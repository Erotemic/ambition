//! Cross-content validation for authored sandbox data.
//!
//! This module checks relationships that live *between* content systems: LDtk
//! room links, NPC dialogue ids, quest conditions, encounter/boss ids, and
//! music references. The intent is to catch content typos at startup/test time
//! instead of letting string ids silently fall back or never fire.

use std::collections::{BTreeMap, BTreeSet};

use ambition_platformer2d::content::MusicRegistry;
use ambition_encounter::encounter_reward_looted_flag;
use ambition_platformer2d_ldtk::{field_string, LdtkProject};

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
/// Normalize one authored optional string: trim it, and treat blank as absent.
///
/// A field left as `"  "` in the editor means the author left it empty. A site
/// that kept `Some("")` would report *"targets unknown room ''"* instead of
/// treating the field as unset. Loading-zone `target_room` / `target_zone` and
/// placement `character_id` / `brain_override` all use this one rule.
fn authored_optional(value: String) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

pub fn validate_embedded_content_graph() -> ContentValidationReport {
    let music = crate::audio_registries::load_music_registry();
    let project = match LdtkProject::load_default_for_dev(&crate::worlds::world_manifest()) {
        Ok(project) => project,
        Err(error) => {
            let mut report = ContentValidationReport::default();
            report.push_error(format!("failed to load embedded LDtk project: {error}"));
            return report;
        }
    };
    let character_catalog = crate::character_catalog::load_catalog();
    validate_content_graph(&music, &project, &character_catalog)
}

/// Validate relationships among the music registry and the LDtk world
/// (room/encounter/boss music references, dialogue, quests, patrols).
pub fn validate_content_graph(
    music: &MusicRegistry,
    project: &LdtkProject,
    character_catalog: &ambition_characters::actor::character_catalog::CharacterCatalog,
) -> ContentValidationReport {
    let mut report = ContentValidationReport::default();

    if let Err(error) = music.validate() {
        report.push_error(format!("music registry invalid: {error}"));
    }

    let ldtk_report = project
        .validate(&ambition_platformer2d_ldtk::LdtkVocabulary::engine());
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
    validate_room_music_tracks(project, music, &mut report);
    validate_npc_dialogue_ids(project, character_catalog, &mut report);
    validate_npc_brain_overrides(project, character_catalog, &mut report);
    validate_quest_conditions(project, music, &mut report);
    validate_cutscene_bindings(project, &mut report);
    let boss_catalog = crate::bosses::authored_boss_catalog();
    validate_boss_music_tracks(music, &boss_catalog, &mut report);

    #[cfg(feature = "audio")]
    validate_adaptive_music_catalog(&mut report);

    report
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
            .and_then(authored_optional);
        let target_zone = target_zone
            .and_then(authored_optional);
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
    music: &MusicRegistry,
    report: &mut ContentValidationReport,
) {
    let valid_tracks = music.tracks.iter().map(|track| track.id.as_str());
    report.extend_errors(
        project
            .music_track_warnings(valid_tracks)
            .into_iter()
            .map(|warning| format!("room music reference: {warning}")),
    );
}

fn validate_npc_dialogue_ids(
    project: &LdtkProject,
    character_catalog: &ambition_characters::actor::character_catalog::CharacterCatalog,
    report: &mut ContentValidationReport,
) {
    let known_ids = crate::dialogue::known_dialogue_ids(character_catalog);
    let known = known_ids
        .iter()
        .map(String::as_str)
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

/// Validate authored NPC brain overrides against the assembled character catalog.
///
/// Raw override names resolve within the character's provider namespace;
/// qualified names resolve exactly. An override without a `character_id` is an
/// error. Unknown characters are skipped so partial provider compositions remain
/// valid, while every known character's override must resolve before spawning.
fn validate_npc_brain_overrides(
    project: &LdtkProject,
    character_catalog: &ambition_characters::actor::character_catalog::CharacterCatalog,
    report: &mut ContentValidationReport,
) {
    use ambition_characters::actor::character_catalog::BrainBuildError;
    for level in &project.levels {
        for entity in level.all_entity_instances() {
            if entity.identifier != "NpcSpawn" {
                continue;
            }
            let character_id = field_string(entity, "character_id")
                .and_then(authored_optional);
            let brain_override = field_string(entity, "brain_override")
                .and_then(authored_optional);

            match (character_id, brain_override) {
                // Anonymous placement, no brain authority — nothing to check.
                (None, None) => {}
                // An override with no character to qualify it against.
                (None, Some(brain_override)) => report.push_error(format!(
                    "level '{}' NpcSpawn '{}' has brain_override '{}' but no character_id \
                     (a brain preset can only be qualified inside a character's provider namespace)",
                    level.identifier, entity.iid, brain_override
                )),
                // A catalog-backed NPC: the character must exist and the override (if any)
                // must resolve. `validate_brain_override` checks both.
                (Some(character_id), brain_override) => {
                    match character_catalog
                        .validate_brain_override(&character_id, brain_override.as_deref())
                    {
                        Ok(_) => {}
                        // A character owned by a provider not loaded in this composition:
                        // skipped (the full host validates it).
                        Err(BrainBuildError::UnknownCharacter(_)) => {}
                        Err(error) => report.push_error(format!(
                            "level '{}' NpcSpawn '{}': {}",
                            level.identifier, entity.iid, error
                        )),
                    }
                }
            }
        }
    }
}

fn validate_quest_conditions(
    project: &LdtkProject,
    music: &MusicRegistry,
    report: &mut ContentValidationReport,
) {
    let room_ids = active_area_ids(project);
    let encounter_ids = authored_encounter_ids(project);
    let boss_ids = authored_boss_encounter_ids(project);
    let item_ids = authored_pickup_ids(project);
    let known_flags = authored_flag_ids(project);
    let valid_tracks = music
        .tracks
        .iter()
        .map(|track| track.id.as_str())
        .collect::<BTreeSet<_>>();

    // The same book the plugin installs, read from the prepared pack, not from
    // a process-global shared with whichever App ran first.
    let waves =
        ambition_encounter::content_schema::lowered_encounter_waves(crate::pack::prepared())
            .cloned()
            .map(ambition_encounter::EncounterWaveBook);
    // Holding an `LdtkProject` is correct here: validating the map is this
    // function's job. The encounter loader must not read one, which would put
    // the map format back in the actor monolith.
    let rooms = project
        .to_room_set(
            &crate::worlds::world_manifest(),
            &ambition_platformer2d_ldtk::LdtkVocabulary::engine(),
        )
        .map(|set| set.rooms)
        .unwrap_or_default();
    let loaded_encounters =
        ambition_encounter_features::load_encounter_specs_from_rooms(
            &rooms,
            &ambition_persistence::save_data::AmbitionGameSaveData::default(),
            waves.as_ref(),
        );
    for (id, spec, _) in loaded_encounters {
        // Exactly-empty, matching `encounter/systems.rs`'s own
        // `!spec.music_track.is_empty()` gate, for the same reason as boss phases.
        if !spec.music_track.is_empty() && !valid_tracks.contains(spec.music_track.as_str()) {
            report.push_error(format!(
                "encounter '{}' references unknown music track '{}'",
                id, spec.music_track
            ));
        }
    }

    for spec in crate::quest::default_quest_specs() {
        if spec.steps.is_empty() {
            report.push_error(format!("quest '{}' has no steps", spec.id));
        }
        for (index, step) in spec.steps.iter().enumerate() {
            match &step.condition {
                ambition_persistence::quest::QuestStepCondition::RoomEntered(room) => {
                    if !room_ids.contains(room.as_str()) {
                        report.push_error(format!(
                            "quest '{}'/step {} references unknown room '{}'",
                            spec.id, index, room
                        ));
                    }
                }
                ambition_persistence::quest::QuestStepCondition::EncounterCleared(encounter) => {
                    if !encounter_ids.contains(encounter.as_str()) {
                        report.push_error(format!(
                            "quest '{}'/step {} references unknown encounter '{}'",
                            spec.id, index, encounter
                        ));
                    }
                }
                ambition_persistence::quest::QuestStepCondition::BossDefeated(boss) => {
                    if !boss_ids.contains(boss.as_str()) {
                        report.push_error(format!(
                            "quest '{}'/step {} references unknown authored boss encounter '{}'",
                            spec.id, index, boss
                        ));
                    }
                }
                ambition_persistence::quest::QuestStepCondition::FlagSet(flag) => {
                    if !known_flags.contains(flag.as_str()) {
                        report.push_error(format!(
                            "quest '{}'/step {} references unknown authored flag '{}'",
                            spec.id, index, flag
                        ));
                    }
                }
                ambition_persistence::quest::QuestStepCondition::ItemCollected(item) => {
                    if !item_ids.contains(item.as_str()) {
                        report.push_error(format!(
                            "quest '{}'/step {} references unknown pickup/item id '{}'",
                            spec.id, index, item
                        ));
                    }
                }
                ambition_persistence::quest::QuestStepCondition::NpcTalked(npc) => {
                    // Gameplay emits the runtime NPC object id for NpcTalked. Most quests use
                    // flags, but future ones may use this.
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

/// The content compiler also checks this, for pack-supplied content. No other
/// validator reads `RoomCutsceneBindings` against the real room population,
/// so a binding to an LDtk level id (for example `"central_hub_main"`)
/// instead of a runtime room id would never fire and nothing would report it.
/// This works like `validate_quest_conditions` above.
fn validate_cutscene_bindings(project: &LdtkProject, report: &mut ContentValidationReport) {
    let room_ids = active_area_ids(project);
    // Both endpoints. `drain_cutscene_triggers` does
    // `let Some(script) = library.get(&id) else { continue; }`, so a binding to
    // a missing cutscene is silent at runtime.
    //
    // The library must be the assembled one. The intro installs five scripts
    // (`install_intro_cutscenes`), and validating against the defaults alone
    // would reject every intro binding.
    let mut library = crate::dialogue::cutscene_defaults::default_cutscene_library();
    crate::intro::cutscene::install_intro_cutscenes(&mut library);

    let defaults = crate::dialogue::cutscene_defaults::default_room_cutscene_bindings();
    let intro = crate::intro::cutscene::intro_room_cutscene_bindings();
    let rows: Vec<(&str, &str, &str)> = defaults
        .bindings
        .iter()
        .map(|(room, cutscene)| ("cutscene binding", room.as_str(), cutscene.as_str()))
        .chain(
            intro
                .iter()
                .map(|(room, cutscene)| ("intro cutscene binding", *room, *cutscene)),
        )
        .collect();
    check_cutscene_bindings(&room_ids, &library, &rows, report);
}

/// The rules take their inputs as arguments so a test can plant a violation.
/// A content-validation error aborts the process, so a bad shipped binding
/// fails every test in the target and none says which rule caught it; and the
/// real tables cannot exercise a rule they do not violate. The caller above
/// supplies the shipped inputs.
fn check_cutscene_bindings(
    room_ids: &BTreeSet<String>,
    library: &ambition_cutscene::CutsceneLibrary,
    rows: &[(&str, &str, &str)],
    report: &mut ContentValidationReport,
) {
    // room → how many cutscenes are bound to it, across BOTH tables.
    let mut per_room: BTreeMap<&str, Vec<&str>> = BTreeMap::new();

    for &(what, room, cutscene) in rows {
        if !room_ids.contains(room) {
            report.push_error(format!(
                "{what} for '{cutscene}' references unknown room '{room}'"
            ));
        }
        if library.get(cutscene).is_none() {
            report.push_error(format!(
                "{what} for room '{room}' references unknown cutscene '{cutscene}' — \
                 the runtime skips a binding whose script is missing, so this is a \
                 permanently dead row rather than a failure anybody would see"
            ));
        }
        per_room.entry(room).or_default().push(cutscene);
    }

    // One cutscene per room, because a second one does not queue.
    // `auto_trigger_room_cutscenes` enqueues every matching row, but only when
    // the room changes (the `LastCutsceneRoom` latch). `drain_cutscene_triggers`
    // then `mem::take`s the whole queue and `break`s on the first admissible
    // script, so the rest are dropped. The second binding would play only on a
    // later visit, after the first sets its seen flag.
    for (room, bound) in &per_room {
        if bound.len() > 1 {
            report.push_error(format!(
                "room '{room}' has {} cutscene bindings ({}) — the runtime starts only \
                 the first admissible one and discards the queue, so the others play on a \
                 later visit or never. Bind one cutscene per room",
                bound.len(),
                bound.join(", ")
            ));
        }
    }
}

/// `boss_encounter` emits a `music_track` reference per non-empty phase field,
/// so an unknown track in Ambition's own encounters is refused at reference
/// resolution — before startup, with the field named.
///
/// It reads the assembled catalog, so it also covers a provider that
/// contributes bosses without a content pack.
fn validate_boss_music_tracks(
    music: &MusicRegistry,
    boss_catalog: &ambition_boss_encounter::BossCatalog,
    report: &mut ContentValidationReport,
) {
    let tracks = music
        .tracks
        .iter()
        .map(|track| track.id.as_str())
        .collect::<BTreeSet<_>>();
    for spec in
        ambition_boss_encounter::default_boss_specs(boss_catalog)
    {
        for (field, track) in [
            ("music_intro", spec.music_intro.as_str()),
            ("music_phase1", spec.music_phase1.as_str()),
            ("music_phase2", spec.music_phase2.as_str()),
            ("music_enrage", spec.music_enrage.as_str()),
        ] {
            // Exactly-empty, matching `phase_music`'s own gate: the runtime requests a
            // whitespace-only field, so it must not pass here.
            if !track.is_empty() && !tracks.contains(track) {
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
    let catalog = crate::music::ambition_music_cue_catalog();
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
                ids.insert(
                    ambition_boss_encounter::encounter_id_from_name(
                        &name,
                    ),
                );
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
        crate::quest::PIRATE_TREASURE_REWARD_FLAG.to_string(),
    ]);
    for level in &project.levels {
        for entity in level.all_entity_instances() {
            if entity.identifier == "NpcSpawn" {
                if let Some(dialogue_id) = field_string(entity, "dialogue_id") {
                    let dialogue_id = dialogue_id.trim();
                    if !dialogue_id.is_empty() {
                        flags.insert(ambition_platformer2d::actors::features::npc_talked_flag(
                            dialogue_id,
                        ));
                    }
                }
            }
            if entity.identifier == "EncounterTrigger" {
                let encounter_id = field_string(entity, "id")
                    .map(|id| id.trim().to_string())
                    .filter(|id| !id.is_empty())
                    .unwrap_or_else(|| level.active_area());
                flags.insert(encounter_reward_looted_flag(&encounter_id));
            }
            if entity.identifier == "Switch" {
                if let Some(id) = field_string(entity, "id") {
                    let id = id.trim();
                    if !id.is_empty() {
                        flags.insert(ambition_encounter::switches::switch_used_flag(id));
                    }
                }
            }
            // PickupSpawn entities with `kind: "flag:<id>"` set the named flag in
            // save state when collected. This mirrors the runtime parse rule in
            // `world/ldtk_world/fields.rs::parse_pickup_kind`, so quest steps that
            // depend on a story-flag pickup validate without the flag listed elsewhere.
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
        flags.insert(encounter_reward_looted_flag(&boss));
    }
    flags
}

#[cfg(test)]
mod tests {
    /// Blank means absent.
    ///
    /// A field left as `"  "` in the editor means the author left it empty.
    /// Carrying `Some("")` into a lookup reports *"targets unknown room ''"*, which
    /// sends the author looking for a room they never named.
    #[test]
    fn a_blank_authored_field_is_absent_not_empty() {
        assert_eq!(super::authored_optional("  ".to_string()), None);
        assert_eq!(super::authored_optional(String::new()), None);
        assert_eq!(
            super::authored_optional("  cove  ".to_string()),
            Some("cove".to_string()),
            "a value with surrounding whitespace is the value, trimmed"
        );
    }

    use super::*;

    #[test]
    fn embedded_content_graph_validates() {
        let report = validate_embedded_content_graph();
        report.panic_if_errors();
    }

    #[test]
    fn validates_ldtk_loading_zone_targets() {
        let music = crate::audio_registries::load_music_registry();
        let project = LdtkProject::load_default_for_dev(&crate::worlds::world_manifest())
            .expect("embedded LDtk loads");
        let character_catalog = crate::character_catalog::load_catalog();
        let report = validate_content_graph(&music, &project, &character_catalog);
        assert!(
            report
                .errors
                .iter()
                .all(|error| !error.contains("LoadingZone")),
            "loading zone validation failed: {:?}",
            report.errors
        );
    }

    /// The inputs the three binding rules are checked against, so each arm
    /// plants exactly one violation and nothing else.
    fn binding_fixture() -> (BTreeSet<String>, ambition_cutscene::CutsceneLibrary) {
        let rooms: BTreeSet<String> = ["hub", "lab"].iter().map(|r| r.to_string()).collect();
        let mut library = ambition_cutscene::CutsceneLibrary::default();
        library.insert(ambition_cutscene::CutsceneScript::new("intro", vec![]));
        library.insert(ambition_cutscene::CutsceneScript::new("second", vec![]));
        (rooms, library)
    }

    /// The control, and the key test: every assertion below is "this input
    /// produces an error", which a function that always errors would also pass.
    #[test]
    fn a_binding_naming_a_real_room_and_a_real_cutscene_is_accepted() {
        let (rooms, library) = binding_fixture();
        let mut report = ContentValidationReport::default();
        check_cutscene_bindings(&rooms, &library, &[("b", "hub", "intro")], &mut report);
        assert!(report.errors.is_empty(), "{:?}", report.errors);
    }

    #[test]
    fn a_binding_naming_a_cutscene_that_does_not_exist_is_refused() {
        let (rooms, library) = binding_fixture();
        let mut report = ContentValidationReport::default();
        // The room resolves, so the room half passes it, and
        // `drain_cutscene_triggers` skips a missing script silently: a permanently
        // dead binding.
        check_cutscene_bindings(&rooms, &library, &[("b", "hub", "intr0")], &mut report);
        assert_eq!(report.errors.len(), 1, "{:?}", report.errors);
        assert!(
            report.errors[0].contains("unknown cutscene 'intr0'"),
            "the error must name the cutscene, not just the row: {:?}",
            report.errors
        );
    }

    #[test]
    fn a_room_bound_to_two_cutscenes_is_refused() {
        let (rooms, library) = binding_fixture();
        let mut report = ContentValidationReport::default();
        check_cutscene_bindings(
            &rooms,
            &library,
            &[("b", "hub", "intro"), ("b", "hub", "second")],
            &mut report,
        );
        assert_eq!(report.errors.len(), 1, "{:?}", report.errors);
        assert!(
            report.errors[0].contains("room 'hub' has 2 cutscene bindings"),
            "{:?}",
            report.errors
        );
    }

    /// Two rooms with one cutscene each is the ordinary case and must not trip
    /// the cardinality rule: the count is per room, not per table.
    #[test]
    fn one_cutscene_each_for_two_rooms_is_accepted() {
        let (rooms, library) = binding_fixture();
        let mut report = ContentValidationReport::default();
        check_cutscene_bindings(
            &rooms,
            &library,
            &[("b", "hub", "intro"), ("b", "lab", "second")],
            &mut report,
        );
        assert!(report.errors.is_empty(), "{:?}", report.errors);
    }

    #[test]
    fn cutscene_bindings_reference_rooms_that_exist() {
        let project = LdtkProject::load_default_for_dev(&crate::worlds::world_manifest())
            .expect("embedded LDtk loads");
        let room_ids = active_area_ids(&project);
        // Non-vacuity: an LDtk level id, not a runtime room id, must not resolve.
        // This is the defect `validate_cutscene_bindings` catches
        // (`central_hub_main` vs the runtime room `central_hub_complex`). If this
        // starts passing, the level/room merge changed shape; re-check the
        // assertions below.
        assert!(
            !room_ids.contains("central_hub_main"),
            "central_hub_main is an LDtk level id, not a room id -- room_ids: {room_ids:?}"
        );
        for (room, cutscene) in
            &crate::dialogue::cutscene_defaults::default_room_cutscene_bindings().bindings
        {
            assert!(
                room_ids.contains(room.as_str()),
                "cutscene binding for '{cutscene}' references unknown room '{room}'; \
                 known rooms: {room_ids:?}"
            );
        }
        for (room, cutscene) in crate::intro::cutscene::intro_room_cutscene_bindings() {
            assert!(
                room_ids.contains(*room),
                "intro cutscene binding for '{cutscene}' references unknown room '{room}'; \
                 known rooms: {room_ids:?}"
            );
        }
    }

    #[test]
    fn quest_boss_conditions_point_at_authored_bosses() {
        let project = LdtkProject::load_default_for_dev(&crate::worlds::world_manifest())
            .expect("embedded LDtk loads");
        let boss_ids = authored_boss_encounter_ids(&project);
        assert!(boss_ids.contains("clockwork_warden"));
        for spec in crate::quest::default_quest_specs() {
            for step in &spec.steps {
                if let ambition_persistence::quest::QuestStepCondition::BossDefeated(id) =
                    &step.condition
                {
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
