//! Cross-content validation for authored sandbox data.
//!
//! This module checks relationships that live *between* content systems: LDtk
//! room links, NPC dialogue ids, quest conditions, encounter/boss ids, and
//! music references. The intent is to catch content typos at startup/test time
//! instead of letting string ids silently fall back or never fire.

use std::collections::{BTreeMap, BTreeSet};

use ambition_platformer2d::content::MusicRegistry;
use ambition_encounter::encounter_reward_looted_flag;
use ambition_platformer2d_ldtk::{field_string, field_text, LdtkProject};

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
    let ldtk_accepts = ldtk_report.is_ok();
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

    // The rooms and the links that the runtime builds its room set from. Each
    // check below that asks about a room, a zone or a link reads these. It does
    // not read the LDtk fields a second time with a rule of its own.
    let (rooms, links) = match project.to_room_parts(
        &crate::worlds::world_manifest(),
        &ambition_platformer2d_ldtk::LdtkVocabulary::engine(),
    ) {
        Ok(parts) => parts,
        Err(errors) => {
            // The LDtk owner's refusals are in the report already. Other
            // refusals (a converter, a baked room) are told here.
            if ldtk_accepts {
                report.extend_errors(
                    errors
                        .into_iter()
                        .map(|error| format!("the world does not compose: {error}")),
                );
            }
            Default::default()
        }
    };

    validate_room_links(&rooms, &links, &mut report);
    validate_room_music_tracks(project, music, &mut report);
    validate_npc_dialogue_ids(project, character_catalog, &mut report);
    validate_npc_brain_overrides(project, character_catalog, &mut report);
    validate_quest_conditions(project, &rooms, music, &mut report);
    validate_cutscene_bindings(project, &mut report);
    let boss_catalog = crate::bosses::authored_boss_catalog();
    validate_boss_music_tracks(music, &boss_catalog, &mut report);

    report
}

/// Each link of the complete game names a room and a zone that exist.
///
/// ONE JUDGE PER RULE. The LDtk owner (`LdtkProject::validate`) judges a zone
/// by itself: its id, a landing pad, half a target. The world owner
/// (`unresolved_links`) says which end of a link does not resolve. A room set
/// drops such a link with a warning, because a partial set keeps the exits of
/// its rooms. This validator holds the complete game, so here each one is an
/// error.
///
/// The links are those that the runtime builds. This validator had its own
/// scan of the `LoadingZone` fields, which trimmed a target that the converter
/// did not trim. A target with a space after it then passed here and was a
/// dead door in play.
fn validate_room_links(
    rooms: &[ambition_platformer2d::world::rooms::RoomSpec],
    links: &[ambition_platformer2d::world::rooms::RoomLink],
    report: &mut ContentValidationReport,
) {
    report.extend_errors(
        ambition_platformer2d::world::rooms::unresolved_links(rooms, links)
            .into_iter()
            .map(|unresolved| unresolved.to_string()),
    );
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
            let character_id = field_text(entity, "character_id");
            let brain_override = field_text(entity, "brain_override");

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
    rooms: &[ambition_platformer2d::world::rooms::RoomSpec],
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
    // The encounter loader reads composed rooms. It must not read an
    // `LdtkProject`, which would put the map format back in the actor monolith.
    let loaded_encounters =
        ambition_encounter_features::load_encounter_specs_from_rooms(
            rooms,
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

/// A room names its entry cutscene in its world file (the `entry_cutscene`
/// level field). The field is on the room, so the room half of the check holds
/// by construction. The other two halves do not: a misspelled script id loads,
/// and two levels of one active area can both set the field, where the area
/// merge keeps the first and drops the other with no message.
fn validate_cutscene_bindings(project: &LdtkProject, report: &mut ContentValidationReport) {
    let room_ids = active_area_ids(project);
    // Both endpoints. `drain_cutscene_triggers` does
    // `let Some(script) = library.get(&id) else { continue; }`, so a binding to
    // a missing cutscene is silent at runtime.
    let library = crate::dialogue::cutscene_defaults::default_cutscene_library();

    let bound = authored_entry_cutscenes(project);
    let rows: Vec<(&str, &str, &str)> = bound
        .iter()
        .map(|(what, room, cutscene)| (what.as_str(), room.as_str(), cutscene.as_str()))
        .collect();
    check_cutscene_bindings(&room_ids, &library, &rows, report);
}

/// One row per LEVEL that sets `entry_cutscene`, keyed by its active area. Rows
/// are read before the area merge so that two levels of one area both appear
/// and the one-per-room rule can see them.
fn authored_entry_cutscenes(project: &LdtkProject) -> Vec<(String, String, String)> {
    project
        .levels
        .iter()
        .filter_map(|level| {
            let cutscene = level.level_metadata().entry_cutscene?;
            Some((
                format!("entry_cutscene of level '{}'", level.identifier),
                level.active_area(),
                cutscene,
            ))
        })
        .collect()
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
    // room → how many cutscenes are bound to it, across all its levels.
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

    // One cutscene per room. The rows are levels, and the area merge
    // (`RoomMetadata::merge`) keeps the first level's value, so a second
    // value in the same area never reaches the trigger.
    for (room, bound) in &per_room {
        if bound.len() > 1 {
            report.push_error(format!(
                "room '{room}' has {} cutscene bindings ({}) — the area merge keeps the \
                 first level's `entry_cutscene` and drops the others, so they never play. \
                 Set `entry_cutscene` on one level of the area",
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
    use super::*;

    /// The embedded project, with one zone that another zone arrives through
    /// re-authored by `edit`. Returns the content report's errors about that
    /// zone, by its iid or its `area:id` name.
    fn errors_about_an_arrival_zone_after(
        edit: impl Fn(&mut Vec<ambition_platformer2d_ldtk::LdtkFieldInstance>),
    ) -> Vec<String> {
        let music = crate::audio_registries::load_music_registry();
        let character_catalog = crate::character_catalog::load_catalog();
        let mut project = LdtkProject::load_default_for_dev(&crate::worlds::world_manifest())
            .expect("embedded LDtk loads");
        // A zone that some exit targets: it has an arrival, so the LDtk owner
        // accepts it as a landing pad once its own targets are cleared.
        let targets: BTreeSet<(String, String)> = project
            .levels
            .iter()
            .flat_map(|level| level.all_entity_instances())
            .filter(|entity| entity.identifier == "LoadingZone")
            .filter_map(|entity| {
                Some((
                    field_text(entity, "target_room")?,
                    field_text(entity, "target_zone")?,
                ))
            })
            .collect();
        let (iid, name) = project
            .levels
            .iter()
            .find_map(|level| {
                let area = level.active_area();
                level.all_entity_instances().find_map(|entity| {
                    let id = field_string(entity, "id")?;
                    (entity.identifier == "LoadingZone"
                        && targets.contains(&(area.clone(), id.clone())))
                    .then(|| (entity.iid.clone(), format!("{area}:{id}")))
                })
            })
            .expect("the embedded world has a zone that an exit arrives through");
        let fields = project
            .levels
            .iter_mut()
            .flat_map(|level| level.layer_instances.iter_mut())
            .flat_map(|layer| layer.entity_instances.iter_mut())
            .find(|entity| entity.iid == iid)
            .map(|entity| &mut entity.field_instances)
            .expect("the zone is still there");
        edit(fields);
        validate_content_graph(&music, &project, &character_catalog)
            .errors
            .into_iter()
            .filter(|error| error.contains(&iid) || error.contains(&name))
            .collect()
    }

    fn set_field(
        fields: &mut [ambition_platformer2d_ldtk::LdtkFieldInstance],
        name: &str,
        value: serde_json::Value,
    ) {
        fields
            .iter_mut()
            .find(|field| field.identifier == name)
            .expect("a LoadingZone carries every declared field")
            .value = value;
    }

    /// A LANDING PAD AND HALF A TARGET HAVE ONE JUDGE: THE LDtk OWNER.
    ///
    /// `LdtkProject::validate` allows a zone with no target when an exit
    /// arrives through it, and refuses a zone with half a target. This
    /// validator folds that report in, so it must not judge either case
    /// again. It used to refuse every zone without both targets, which
    /// refused a landing pad the owner allows and reported half a target
    /// twice.
    #[test]
    fn a_landing_pad_is_allowed_and_half_a_target_is_refused_once() {
        let pad = errors_about_an_arrival_zone_after(|fields| {
            set_field(fields, "target_room", serde_json::Value::Null);
            set_field(fields, "target_zone", serde_json::Value::Null);
        });
        assert!(pad.is_empty(), "a landing pad that an exit arrives through was refused: {pad:?}");

        let half = errors_about_an_arrival_zone_after(|fields| {
            set_field(fields, "target_zone", serde_json::Value::Null);
        });
        assert_eq!(half.len(), 1, "half a target is reported once, by the owner: {half:?}");

        // Control: a target that names no room is this validator's own check.
        let unknown = errors_about_an_arrival_zone_after(|fields| {
            set_field(fields, "target_room", serde_json::Value::String("no_such_room".into()));
        });
        assert!(
            unknown.iter().any(|error| error.contains("targets unknown room 'no_such_room'")),
            "a target naming no room was not refused: {unknown:?}"
        );
    }

    /// THE VALIDATOR JUDGES THE LINKS THAT THE RUNTIME BUILDS.
    ///
    /// This validator had its own scan of the `LoadingZone` fields. It trimmed
    /// a target, and the converter did not. A target with a space after it
    /// passed here, and the room set dropped the link: a dead door that the
    /// validator accepted. The LDtk owner trims now (`field_text`), and this
    /// validator reads the links that the owner builds, so the two agree: the
    /// target is accepted and the door is in the graph.
    #[test]
    fn a_target_with_a_stray_space_is_one_door_for_the_validator_and_the_runtime() {
        let music = crate::audio_registries::load_music_registry();
        let character_catalog = crate::character_catalog::load_catalog();
        let mut project = LdtkProject::load_default_for_dev(&crate::worlds::world_manifest())
            .expect("embedded LDtk loads");
        let exit = project
            .levels
            .iter_mut()
            .find_map(|level| {
                let area = level.active_area();
                level
                    .layer_instances
                    .iter_mut()
                    .flat_map(|layer| layer.entity_instances.iter_mut())
                    .find(|entity| {
                        entity.identifier == "LoadingZone"
                            && field_text(entity, "target_room").is_some()
                            && field_text(entity, "target_zone").is_some()
                            && field_text(entity, "id").is_some()
                    })
                    .map(|entity| (area, entity))
            })
            .expect("the embedded world has an exit");
        let (area, entity) = exit;
        let id = field_text(entity, "id").unwrap();
        let room = field_text(entity, "target_room").unwrap();
        let iid = entity.iid.clone();
        set_field(
            &mut entity.field_instances,
            "target_room",
            serde_json::Value::String(format!("{room} ")),
        );

        let name = format!("{area}:{id}");
        let errors: Vec<String> = validate_content_graph(&music, &project, &character_catalog)
            .errors
            .into_iter()
            .filter(|error| error.contains(&iid) || error.contains(&name))
            .collect();
        let set = project
            .to_room_set(
                &crate::worlds::world_manifest(),
                &ambition_platformer2d_ldtk::LdtkVocabulary::engine(),
            )
            .expect("the edited world composes");
        let has_door = set
            .canonical_links()
            .iter()
            .any(|link| link.from_room == area && link.from_zone == id && link.to_room == room);
        assert!(
            errors.is_empty() == has_door,
            "the validator and the runtime disagree about '{name}' -> '{room} ': \
             validator errors {errors:?}, door in the graph: {has_door}"
        );
        assert!(has_door, "a target with a space after it is its trimmed value: '{name}'");
    }

    /// A link to a zone that its target room does not have is refused once.
    /// The world owner only warns of it, because a partial room set keeps the
    /// exits of its rooms. This validator holds the complete game.
    #[test]
    fn a_target_zone_that_does_not_exist_is_refused_once() {
        let missing = errors_about_an_arrival_zone_after(|fields| {
            set_field(fields, "target_zone", serde_json::Value::String("no_such_zone".into()));
        });
        assert_eq!(
            missing
                .iter()
                .filter(|error| error.contains("targets missing zone"))
                .count(),
            1,
            "a target zone that does not exist: {missing:?}"
        );
    }

    /// AN NPC MAY NAME ONLY A NODE THAT CAN START.
    ///
    /// `oiler_post_stabilizer` exists only as `oiler_post_stabilizer__1` and
    /// `__2`, which another node reaches by `<<jump>>`. The runtime starts a
    /// dialogue id exactly as named (`DialogueNodeIndex::entry_node`), so a spawn
    /// that names the root warns and closes. The validator must refuse it.
    #[test]
    fn a_spawn_naming_a_root_that_exists_only_as_variants_is_refused() {
        let music = crate::audio_registries::load_music_registry();
        let character_catalog = crate::character_catalog::load_catalog();
        let mut project = LdtkProject::load_default_for_dev(&crate::worlds::world_manifest())
            .expect("embedded LDtk loads");
        let spawn = project
            .levels
            .iter_mut()
            .flat_map(|level| level.layer_instances.iter_mut())
            .flat_map(|layer| layer.entity_instances.iter_mut())
            .find(|entity| {
                entity.identifier == "NpcSpawn"
                    && field_string(entity, "dialogue_id").is_some_and(|id| !id.trim().is_empty())
            })
            .expect("the embedded world has an NPC with a dialogue");
        let iid = spawn.iid.clone();
        set_field(
            &mut spawn.field_instances,
            "dialogue_id",
            serde_json::Value::String("oiler_post_stabilizer".into()),
        );
        let errors: Vec<String> = validate_content_graph(&music, &project, &character_catalog)
            .errors
            .into_iter()
            .filter(|error| error.contains(&iid))
            .collect();
        assert_eq!(
            errors.len(),
            1,
            "a dialogue id that no node is titled was accepted: {errors:?}"
        );
    }

    #[test]
    fn embedded_content_graph_validates() {
        let report = validate_embedded_content_graph();
        report.panic_if_errors();
    }

    /// A room's `fight_music_track` is read into its metadata, and an id the
    /// music registry does not have is refused with the field named, the same
    /// as `music_track`.
    #[test]
    fn a_rooms_fight_music_track_is_read_and_checked() {
        let music = crate::audio_registries::load_music_registry();
        let mut project = LdtkProject::load_default_for_dev(&crate::worlds::world_manifest())
            .expect("embedded LDtk loads");
        let set_fight_track = |project: &mut LdtkProject, track: &str| {
            let level = project
                .levels
                .iter_mut()
                .find(|level| level.identifier == "mode_collapse_arena")
                .expect("the sandbox has a Mode Collapse arena");
            let field = level
                .field_instances
                .iter_mut()
                .find(|field| field.identifier == "fight_music_track")
                .expect("every level carries the declared fight_music_track field");
            field.value = serde_json::Value::String(track.to_owned());
            level.level_metadata()
        };

        let metadata = set_fight_track(&mut project, "crooked_ascent_boss");
        assert_eq!(metadata.fight_music_track.as_deref(), Some("crooked_ascent_boss"));
        let mut report = ContentValidationReport::default();
        validate_room_music_tracks(&project, &music, &mut report);
        assert!(report.errors.is_empty(), "a known track was refused: {:?}", report.errors);

        set_fight_track(&mut project, "no_such_track");
        let mut report = ContentValidationReport::default();
        validate_room_music_tracks(&project, &music, &mut report);
        assert!(
            report
                .errors
                .iter()
                .any(|error| error.contains("unknown fight_music_track 'no_such_track'")),
            "an unknown fight track must be refused with its field named: {:?}",
            report.errors
        );
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

    /// The shipped worlds name their room cutscenes in data, and the rows the
    /// validator reads are those rows. A reader that finds no rows passes every
    /// rule, so this also pins that the validator sees the six rooms.
    #[test]
    fn each_shipped_room_cutscene_is_read_from_its_world_file() {
        let project = LdtkProject::load_default_for_dev(&crate::worlds::world_manifest())
            .expect("embedded LDtk loads");
        let room_ids = active_area_ids(&project);
        let bound: BTreeSet<(String, String)> = authored_entry_cutscenes(&project)
            .into_iter()
            .map(|(_, room, cutscene)| (room, cutscene))
            .collect();
        let expected: BTreeSet<(String, String)> = [
            ("central_hub_complex", "test_intro"),
            ("basement_boss", "boss_intro_gradient_sentinel"),
            ("cutscene_lab", "cutscene_lab_intro"),
            ("intro_wake_room", "intro_wake"),
            ("intro_raid_corridor", "intro_raid"),
            ("drain_alley", "drain_market_arrival"),
        ]
        .iter()
        .map(|(room, cutscene)| (room.to_string(), cutscene.to_string()))
        .collect();
        assert_eq!(bound, expected);
        for (room, _) in &bound {
            assert!(room_ids.contains(room), "{room} is not a room: {room_ids:?}");
        }
        let mut report = ContentValidationReport::default();
        validate_cutscene_bindings(&project, &mut report);
        assert!(report.errors.is_empty(), "{:?}", report.errors);
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
