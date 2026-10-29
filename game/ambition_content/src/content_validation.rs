//! Cross-content validation for authored sandbox data.
//!
//! This module checks relationships that live *between* content systems: LDtk
//! room links, NPC dialogue ids, quest conditions, encounter/boss ids, and
//! music references. The intent is to catch content typos at startup/test time
//! instead of letting string ids silently fall back or never fire.

use std::collections::BTreeSet;

use ambition_platformer2d::content::MusicRegistry;
use ambition_encounter::encounter_reward_looted_flag;
use ambition_platformer2d_ldtk::{field_text, LdtkProject};

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

/// Validate the checked-in sandbox content graph, against Ambition's SHIPPED
/// pack. The subject is the shipped product, not a composition.
pub fn validate_embedded_content_graph(sprite_content: &ambition_platformer2d::sprite_sheet::SpriteContent) -> ContentValidationReport {
    let pack = crate::pack::shipped();
    let music = crate::audio_registries::music_registry_of(pack);
    let project = match LdtkProject::load_default_for_dev(&crate::worlds::world_manifest()) {
        Ok(project) => project,
        Err(error) => {
            let mut report = ContentValidationReport::default();
            report.push_error(format!("failed to load embedded LDtk project: {error}"));
            return report;
        }
    };
    let character_catalog = crate::character_catalog::catalog_of(pack);
    validate_content_graph(&sprite_content, pack, music.as_ref(), &project, &character_catalog)
}

/// Validate relationships among the music registry and the LDtk world
/// (room/encounter/boss music references, dialogue, quests, patrols).
///
/// `music` is `None` when the music is not generated yet. The music
/// references are then not checked, and the report has one warning that
/// says so. Missing generated content does not stop the game.
///
/// `pack` is the pack under validation: a composition passes its selected pack,
/// so the boss roster, encounter waves, quests and cutscenes judged here are
/// the ones the App installed.
pub fn validate_content_graph(sprite_content: &ambition_platformer2d::sprite_sheet::SpriteContent,
    pack: &ambition_content_pack::PreparedContentPack,
    music: Option<&MusicRegistry>,
    project: &LdtkProject,
    character_catalog: &ambition_characters::actor::character_catalog::CharacterCatalog,
) -> ContentValidationReport {
    let mut report = ContentValidationReport::default();

    match music {
        Some(music) => {
            if let Err(error) = music.validate() {
                report.push_error(format!("music registry invalid: {error}"));
            }
        }
        None => report.warnings.push(
            "the music registry is not generated, so music references are not checked \
             (make it: scripts/regen/music.sh; query: python3 scripts/content_status.py)"
                .to_owned(),
        ),
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
    if let Some(music) = music {
        validate_room_music_tracks(project, music, &mut report);
    }
    validate_room_phase_tracks(pack, &rooms, music, &mut report);
    validate_npc_dialogue_ids(project, &mut report);
    validate_npc_brain_overrides(project, character_catalog, &mut report);
    validate_quest_conditions(&sprite_content, pack, &rooms, music, &mut report);
    validate_cutscene_bindings(pack, project, &mut report);
    let boss_catalog = crate::bosses::boss_catalog_of(&sprite_content, pack);
    if let Some(music) = music {
        validate_boss_music_tracks(music, &boss_catalog, &mut report);
    }

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

/// A room's track by the time of day (`music_cues.ron` `room_phase_tracks`)
/// names a room of the game and a track of the music registry, as a room's
/// own `music_track` does.
fn validate_room_phase_tracks(
    pack: &ambition_content_pack::PreparedContentPack,
    rooms: &[ambition_platformer2d::world::rooms::RoomSpec],
    music: Option<&MusicRegistry>,
    report: &mut ContentValidationReport,
) {
    let Some(authored) = ambition_audio::content_schema::lowered_music_cues(pack) else {
        return;
    };
    for row in &authored.room_phase_tracks {
        if !rooms.iter().any(|room| room.id == row.room_id) {
            report.extend_errors([format!("room music by the time of day: no room is called {:?}", row.room_id)]);
        }
        if music.is_some_and(|music| !music.tracks.iter().any(|track| track.id == row.track)) {
            report.extend_errors([format!(
                "room music by the time of day: room {:?} plays unknown track {:?} in the {}",
                row.room_id, row.track, row.phase
            )]);
        }
    }
}

fn validate_npc_dialogue_ids(project: &LdtkProject, report: &mut ContentValidationReport) {
    let known_ids = crate::dialogue::known_dialogue_ids();
    let known = known_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    for level in &project.levels {
        for entity in level.all_entity_instances() {
            if entity.identifier != "NpcSpawn" {
                continue;
            }
            // `field_text`, the rule the converter reads this id by.
            let Some(dialogue_id) = field_text(entity, "dialogue_id") else {
                continue;
            };
            let dialogue_id = dialogue_id.as_str();
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
    use ambition_characters::control::brain_binding::BrainBuildError;
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
                    match ambition_characters::control::brain_binding::validate_brain_override(&(character_catalog), &character_id, brain_override.as_deref())
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

fn validate_quest_conditions(sprite_content: &ambition_platformer2d::sprite_sheet::SpriteContent,
    pack: &ambition_content_pack::PreparedContentPack,
    rooms: &[ambition_platformer2d::world::rooms::RoomSpec],
    music: Option<&MusicRegistry>,
    report: &mut ContentValidationReport,
) {
    // Without the generated music there is no track list to check against.
    let valid_tracks = music.map(|music| music
        .tracks
        .iter()
        .map(|track| track.id.as_str())
        .collect::<BTreeSet<_>>());

    // The same book the plugin installs, read from the pack under validation,
    // not from a process-global shared with whichever App ran first.
    let waves =
        ambition_encounter::content_schema::lowered_encounter_waves(pack)
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
    for (id, spec, _) in &loaded_encounters {
        // Exactly-empty, matching `encounter/systems.rs`'s own
        // `!spec.music_track.is_empty()` gate, for the same reason as boss phases.
        if !spec.music_track.is_empty()
            && valid_tracks.as_ref().is_some_and(|tracks| !tracks.contains(spec.music_track.as_str()))
        {
            report.push_error(format!(
                "encounter '{}' references unknown music track '{}'",
                id, spec.music_track
            ));
        }
    }

    wave_spawns_start_clear_of_solid_ground(rooms, &loaded_encounters, report);

    let boss_catalog = crate::bosses::boss_catalog_of(&sprite_content, pack);
    let ids = QuestTargets::of(rooms, &loaded_encounters, &boss_catalog);
    check_quest_steps(&crate::quest::quest_specs_of(pack), &ids, report);
}

/// Every wave mob starts clear of its room's solid ground (Jon, 2026-10-18:
/// "enemies will spawn in the floor ... we shouldn't let that happen").
///
/// A mob starts on the ground under its authored box
/// (`RoomSpec::settled_on_ground`, at the spawn server), so a box a little in
/// the floor, or over it, is fine. A box that cannot be settled and is inside
/// a full-collision block starts in the floor: an error, naming the mob.
fn wave_spawns_start_clear_of_solid_ground(
    rooms: &[ambition_platformer2d::world::rooms::RoomSpec],
    loaded_encounters: &[(
        String,
        ambition_encounter::EncounterSpec,
        ambition_persistence::save_data::PersistedEncounterState,
    )],
    report: &mut ContentValidationReport,
) {
    use ambition_platformer2d::engine_core as ae;
    for (id, spec, _) in loaded_encounters {
        let Some(room) = rooms.iter().find(|room| room.id == spec.room_id) else {
            continue;
        };
        for wave in &spec.waves {
            for mob in &wave.mobs {
                let body = ae::Aabb::new(ae::Vec2::from(mob.spawn), ae::Vec2::from(mob.size) * 0.5);
                if room.settled_on_ground(body).is_some() {
                    continue;
                }
                let inside = room.embeds(body);
                if !inside.is_empty() {
                    report.push_error(format!(
                        "encounter '{id}' ({}): a mob of wave '{}' ({}) spawns at {:?} inside solid ground {:?} of room '{}'",
                        spec.room_id,
                        wave.label,
                        mob.character.as_deref().unwrap_or(&mob.kind),
                        mob.spawn,
                        inside,
                        room.id
                    ));
                }
            }
        }
    }
}

/// What a quest step may name, each set read from the owner of its id.
struct QuestTargets {
    rooms: BTreeSet<String>,
    encounters: BTreeSet<String>,
    bosses: BTreeSet<String>,
    items: BTreeSet<String>,
    npcs: BTreeSet<String>,
    flags: BTreeSet<String>,
}

impl QuestTargets {
    /// The targets of a composed world. No set is read from the LDtk fields:
    /// each comes from the rooms that the runtime builds, the encounters that
    /// the loader builds, and the boss owner.
    fn of(
        rooms: &[ambition_platformer2d::world::rooms::RoomSpec],
        loaded_encounters: &[(
            String,
            ambition_encounter::EncounterSpec,
            ambition_persistence::save_data::PersistedEncounterState,
        )],
        boss_catalog: &ambition_boss_encounter::BossCatalog,
    ) -> Self {
        let (flags, npcs, items) = composed_quest_ids(rooms, loaded_encounters);
        Self {
            // A room is entered by its composed id (`RoomSpec::id`).
            rooms: rooms.iter().map(|room| room.id.clone()).collect(),
            // The encounters that the loader builds, by the id it gives each one.
            encounters: loaded_encounters
                .iter()
                .map(|(id, _, _)| id.clone())
                .collect(),
            bosses: boss_defeat_ids(rooms, boss_catalog),
            items,
            npcs,
            flags,
        }
    }
}

/// The id that each authored boss reports when it is defeated
/// (`QuestAdvanceEvent::BossDefeated`): the id of the behaviour that the boss
/// owner resolves for its placement.
///
/// This validator used to slug the display name of each `BossSpawn`. Measured
/// on the shipped world, that id was not the reported one for 9 of the 11
/// bosses (`system_boss` against `clockwork_warden`, `t_rex` against
/// `trex_boss`). A quest that named the reported id was refused, and a quest
/// that named the slug was accepted and could not complete.
fn boss_defeat_ids(
    rooms: &[ambition_platformer2d::world::rooms::RoomSpec],
    boss_catalog: &ambition_boss_encounter::BossCatalog,
) -> BTreeSet<String> {
    rooms
        .iter()
        .flat_map(|room| &room.boss_spawns)
        .map(|boss| {
            ambition_boss_encounter::behavior::authored_boss_behavior(
                boss_catalog,
                &boss.name,
                &boss.payload,
            )
            .id
        })
        .collect()
}

/// The rules take their inputs as arguments so a test can plant a step. See
/// `check_cutscene_bindings`.
fn check_quest_steps(
    specs: &[ambition_persistence::quest::QuestSpec],
    ids: &QuestTargets,
    report: &mut ContentValidationReport,
) {
    use ambition_persistence::quest::QuestStepCondition as Condition;
    // A quest's payout sets its flag (`QuestSpec::reward_flag`), so a step
    // may wait for another quest's payout.
    let mut flags = ids.flags.clone();
    flags.extend(specs.iter().filter(|spec| spec.reward.is_some()).map(|spec| spec.reward_flag()));
    for spec in specs {
        if spec.steps.is_empty() {
            report.push_error(format!("quest '{}' has no steps", spec.id));
        }
        for (index, step) in spec.steps.iter().enumerate() {
            let (known, what, id) = match &step.condition {
                Condition::RoomEntered(room) => (&ids.rooms, "room", room),
                Condition::EncounterCleared(encounter) => (&ids.encounters, "encounter", encounter),
                Condition::BossDefeated(boss) => {
                    (&ids.bosses, "authored boss encounter", boss)
                }
                Condition::FlagSet(flag) => (&flags, "authored flag", flag),
                Condition::ItemCollected(item) => (&ids.items, "pickup/item id", item),
                // Gameplay emits the runtime NPC object id for NpcTalked. Most
                // quests use flags, but future ones may use this.
                Condition::NpcTalked(npc) => (&ids.npcs, "NPC id", npc),
            };
            if !known.contains(id.as_str()) {
                report.push_error(format!(
                    "quest '{}'/step {} references unknown {what} '{id}'",
                    spec.id, index
                ));
            }
        }
    }
}

/// A room names its entry cutscene in its world file (the `entry_cutscene`
/// level field). The field is on the room, so the room end of the binding
/// holds by construction, and the LDtk owner (`LdtkProject::validate`) refuses
/// two values in one area. The script end does not hold: a misspelled script
/// id loads.
fn validate_cutscene_bindings(
    pack: &ambition_content_pack::PreparedContentPack,
    project: &LdtkProject,
    report: &mut ContentValidationReport,
) {
    // `drain_cutscene_triggers` does
    // `let Some(script) = library.get(&id) else { continue; }`, so a binding to
    // a missing cutscene is silent at runtime.
    let library = crate::dialogue::cutscene_defaults::cutscene_library_of(pack);

    let bound = authored_entry_cutscenes(project);
    let rows: Vec<(&str, &str, &str)> = bound
        .iter()
        .map(|(what, room, cutscene)| (what.as_str(), room.as_str(), cutscene.as_str()))
        .collect();
    check_cutscene_bindings(&library, &rows, report);
}

/// One row per LEVEL that sets `entry_cutscene`, keyed by its active area, so
/// an error names the level that holds the bad id.
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
    library: &ambition_cutscene::CutsceneLibrary,
    rows: &[(&str, &str, &str)],
    report: &mut ContentValidationReport,
) {
    for &(what, room, cutscene) in rows {
        if library.get(cutscene).is_none() {
            report.push_error(format!(
                "{what} for room '{room}' references unknown cutscene '{cutscene}' — \
                 the runtime skips a binding whose script is missing, so this is a \
                 permanently dead row rather than a failure anybody would see"
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

/// The flags that the shipped world can set, and the ids of its NPCs and its
/// pickups, each read from the composed rooms: the placements that the runtime
/// lowers. Each id is made by the function that the runtime makes it with.
fn composed_quest_ids(
    rooms: &[ambition_platformer2d::world::rooms::RoomSpec],
    loaded_encounters: &[(
        String,
        ambition_encounter::EncounterSpec,
        ambition_persistence::save_data::PersistedEncounterState,
    )],
) -> (BTreeSet<String>, BTreeSet<String>, BTreeSet<String>) {
    use ambition_platformer2d::entity_catalog::placements::PlacementSchema;
    use ambition_platformer2d::world::rooms::{InteractionKindSpec, PickupKind};
    let mut flags = BTreeSet::from([
        "met_any_hub_npc".to_string(),
        "test_switch_toggled".to_string(),
    ]);
    let (mut npcs, mut pickups) = (BTreeSet::new(), BTreeSet::new());
    for record in rooms.iter().flat_map(|room| &room.placements) {
        match &record.schema {
            PlacementSchema::Interactable(interactable) => match &interactable.kind {
                InteractionKindSpec::Npc { dialogue_id, .. } => {
                    npcs.insert(record.id.as_str().to_string());
                    flags.insert(ambition_platformer2d::persistence::save_data::npc_talked_flag(
                        ambition_platformer2d::conversation::npc_talk_dialogue_id(
                            dialogue_id.as_deref(),
                        ),
                    ));
                }
                InteractionKindSpec::Custom(payload) => {
                    if let Some(switch) =
                        ambition_encounter::SwitchActivation::parse_custom(payload)
                    {
                        flags.insert(ambition_encounter::switches::switch_used_flag(&switch.id));
                    }
                }
                _ => {}
            },
            PlacementSchema::Pickup(pickup) => {
                pickups.insert(record.id.as_str().to_string());
                if let PickupKind::StoryFlag { flag } = &pickup.kind {
                    flags.insert(flag.clone());
                }
            }
            _ => {}
        }
    }
    // A dialogue choice can set a flag (`world.set_flag`): the hand-over of
    // Alice's note is the one writer of the flag its quest step reads.
    flags.extend(crate::dialogue::dialogue_set_flags());
    // The reward chest of an encounter is keyed by the id that the loader gives
    // the encounter.
    for (encounter_id, _, _) in loaded_encounters {
        flags.insert(encounter_reward_looted_flag(encounter_id));
    }
    // The reward chest of a boss is keyed by its PLACEMENT id
    // (`ambition_boss_encounter::rewards`), not by its display name.
    for boss in rooms.iter().flat_map(|room| &room.boss_spawns) {
        flags.insert(encounter_reward_looted_flag(&boss.id));
    }
    (flags, npcs, pickups)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ambition_platformer2d_ldtk::field_string;

    /// The embedded project, with one zone that another zone arrives through
    /// re-authored by `edit`. Returns the content report's errors about that
    /// zone, by its iid or its `area:id` name.
    fn errors_about_an_arrival_zone_after(sprite_content: &ambition_platformer2d::sprite_sheet::SpriteContent,
        edit: impl Fn(&mut Vec<ambition_platformer2d_ldtk::LdtkFieldInstance>),
    ) -> Vec<String> {
        let music = crate::audio_registries::shipped_music_registry();
        let character_catalog = crate::character_catalog::shipped_catalog();
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
        validate_content_graph(&sprite_content, crate::pack::shipped(), music.as_ref(), &project, &character_catalog)
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
    let sprite_content = ambition_platformer2d::sprite_sheet::test_support::shared_sprites();
        let pad = errors_about_an_arrival_zone_after(&sprite_content, |fields| {
            set_field(fields, "target_room", serde_json::Value::Null);
            set_field(fields, "target_zone", serde_json::Value::Null);
        });
        assert!(pad.is_empty(), "a landing pad that an exit arrives through was refused: {pad:?}");

        let half = errors_about_an_arrival_zone_after(&sprite_content, |fields| {
            set_field(fields, "target_zone", serde_json::Value::Null);
        });
        assert_eq!(half.len(), 1, "half a target is reported once, by the owner: {half:?}");

        // Control: a target that names no room is this validator's own check.
        let unknown = errors_about_an_arrival_zone_after(&sprite_content, |fields| {
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
    let sprite_content = ambition_platformer2d::sprite_sheet::test_support::shared_sprites();
        let music = crate::audio_registries::shipped_music_registry();
        let character_catalog = crate::character_catalog::shipped_catalog();
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
        let errors: Vec<String> = validate_content_graph(&sprite_content, crate::pack::shipped(), music.as_ref(), &project, &character_catalog)
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
    let sprite_content = ambition_platformer2d::sprite_sheet::test_support::shared_sprites();
        let missing = errors_about_an_arrival_zone_after(&sprite_content, |fields| {
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
    let sprite_content = ambition_platformer2d::sprite_sheet::test_support::shared_sprites();
        let music = crate::audio_registries::shipped_music_registry();
        let character_catalog = crate::character_catalog::shipped_catalog();
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
        let errors: Vec<String> = validate_content_graph(&sprite_content, crate::pack::shipped(), music.as_ref(), &project, &character_catalog)
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

    /// The Yarn titles are the one authority for a dialogue id. A catalog
    /// that names a Hall dialogue with no node does not make the id valid: the
    /// runtime starts a title, and there is none.
    #[test]
    fn a_hall_dialogue_the_catalog_names_with_no_node_is_refused() {
    let sprite_content = ambition_platformer2d::sprite_sheet::test_support::shared_sprites();
        let music = crate::audio_registries::shipped_music_registry();
        let mut data = crate::character_catalog::shipped_catalog().data().clone();
        let entry = data.characters.values_mut().next().expect("the catalog has a character");
        entry.hall_dialogue_id = Some("hall_nobody".into());
        let character_catalog =
            ambition_characters::actor::character_catalog::CharacterCatalog::from_data(data);
        let errors_naming = |dialogue_id: &str| {
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
                serde_json::Value::String(dialogue_id.into()),
            );
            validate_content_graph(&sprite_content, crate::pack::shipped(), music.as_ref(), &project, &character_catalog)
                .errors
                .into_iter()
                .filter(|error| error.contains(&iid))
                .collect::<Vec<_>>()
        };
        let nobody = errors_naming("hall_nobody");
        assert!(
            nobody.len() == 1 && nobody[0].contains("unknown dialogue_id 'hall_nobody'"),
            "a Hall dialogue with no node was accepted: {nobody:?}"
        );
        // Control: a Hall dialogue that has its node is accepted.
        assert_eq!(errors_naming("hall_player"), Vec::<String>::new());
    }

    #[test]
    fn embedded_content_graph_validates() {
    let sprite_content = ambition_platformer2d::sprite_sheet::test_support::shared_sprites();
        let report = validate_embedded_content_graph(&sprite_content);
        report.panic_if_errors();
    }

    /// A room's `fight_music_track` is read into its metadata, and an id the
    /// music registry does not have is refused with the field named, the same
    /// as `music_track`.
    #[test]
    fn a_rooms_fight_music_track_is_read_and_checked() {
        let music = crate::audio_registries::shipped_music_registry()
            .expect("the music registry is generated content: run scripts/regen/music.sh");
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
    let sprite_content = ambition_platformer2d::sprite_sheet::test_support::shared_sprites();
        let music = crate::audio_registries::shipped_music_registry();
        let project = LdtkProject::load_default_for_dev(&crate::worlds::world_manifest())
            .expect("embedded LDtk loads");
        let character_catalog = crate::character_catalog::shipped_catalog();
        let report = validate_content_graph(&sprite_content, crate::pack::shipped(), music.as_ref(), &project, &character_catalog);
        assert!(
            report
                .errors
                .iter()
                .all(|error| !error.contains("LoadingZone")),
            "loading zone validation failed: {:?}",
            report.errors
        );
    }

    /// The cutscenes a binding is checked against.
    fn binding_fixture() -> ambition_cutscene::CutsceneLibrary {
        let mut library = ambition_cutscene::CutsceneLibrary::default();
        library.insert(ambition_cutscene::CutsceneScript::new("intro", vec![]));
        library.insert(ambition_cutscene::CutsceneScript::new("second", vec![]));
        library
    }

    /// The control, and the key test: every assertion below is "this input
    /// produces an error", which a function that always errors would also pass.
    #[test]
    fn a_binding_naming_a_real_room_and_a_real_cutscene_is_accepted() {
        let library = binding_fixture();
        let mut report = ContentValidationReport::default();
        check_cutscene_bindings(&library, &[("b", "hub", "intro")], &mut report);
        assert!(report.errors.is_empty(), "{:?}", report.errors);
    }

    #[test]
    fn a_binding_naming_a_cutscene_that_does_not_exist_is_refused() {
        let library = binding_fixture();
        let mut report = ContentValidationReport::default();
        // The room resolves, so the room half passes it, and
        // `drain_cutscene_triggers` skips a missing script silently: a permanently
        // dead binding.
        check_cutscene_bindings(&library, &[("b", "hub", "intr0")], &mut report);
        assert_eq!(report.errors.len(), 1, "{:?}", report.errors);
        assert!(
            report.errors[0].contains("unknown cutscene 'intr0'"),
            "the error must name the cutscene, not just the row: {:?}",
            report.errors
        );
    }

    /// The shipped worlds name their room cutscenes in data, and the rows the
    /// validator reads are those rows. A reader that finds no rows passes every
    /// rule, so this also pins that the validator sees the five rooms.
    #[test]
    fn each_shipped_room_cutscene_is_read_from_its_world_file() {
        let project = LdtkProject::load_default_for_dev(&crate::worlds::world_manifest())
            .expect("embedded LDtk loads");
        let bound: BTreeSet<(String, String)> = authored_entry_cutscenes(&project)
            .into_iter()
            .map(|(_, room, cutscene)| (room, cutscene))
            .collect();
        let expected: BTreeSet<(String, String)> = [
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
        let mut report = ContentValidationReport::default();
        validate_cutscene_bindings(crate::pack::shipped(), &project, &mut report);
        assert!(report.errors.is_empty(), "{:?}", report.errors);
    }

    /// The composed rooms, the boss catalog and the quest targets of the
    /// shipped world.
    fn shipped_quest_targets(sprite_content: &ambition_platformer2d::sprite_sheet::SpriteContent) -> (
        Vec<ambition_platformer2d::world::rooms::RoomSpec>,
        ambition_boss_encounter::BossCatalog,
        QuestTargets,
    ) {
        let project = LdtkProject::load_default_for_dev(&crate::worlds::world_manifest())
            .expect("embedded LDtk loads");
        let (rooms, _) = project
            .to_room_parts(
                &crate::worlds::world_manifest(),
                &ambition_platformer2d_ldtk::LdtkVocabulary::engine(),
            )
            .expect("the embedded world composes");
        let boss_catalog = crate::bosses::shipped_boss_catalog(&sprite_content);
        let loaded = ambition_encounter_features::load_encounter_specs_from_rooms(
            &rooms,
            &ambition_persistence::save_data::AmbitionGameSaveData::default(),
            None,
        );
        let ids = QuestTargets::of(&rooms, &loaded, &boss_catalog);
        (rooms, boss_catalog, ids)
    }

    fn errors_for_a_quest_that_wants(
        ids: &QuestTargets,
        condition: ambition_persistence::quest::QuestStepCondition,
    ) -> Vec<String> {
        let quest = ambition_persistence::quest::QuestSpec::new(
            "planted",
            "Planted",
            "A quest with one planted step.",
            vec![ambition_persistence::quest::QuestStepSpec::new("Do it.", condition)],
        );
        let mut report = ContentValidationReport::default();
        check_quest_steps(&[quest], ids, &mut report);
        report.errors
    }

    /// A QUEST NAMES A BOSS BY THE ID THAT ITS DEFEAT REPORTS.
    ///
    /// The validator's id of each shipped boss is the id of the behaviour
    /// that the production constructor gives it, which is the id in
    /// `QuestAdvanceEvent::BossDefeated`. The validator used to slug the
    /// display name: `System Boss` gave `system_boss`, and the boss reports
    /// `clockwork_warden`.
    #[test]
    fn a_quest_names_a_boss_by_the_id_its_defeat_reports() {

    let sprite_content = ambition_platformer2d::sprite_sheet::test_support::shared_sprites();
        use ambition_persistence::quest::QuestStepCondition as Condition;
        let (rooms, boss_catalog, ids) = shipped_quest_targets(&sprite_content);

        let mut bosses = 0;
        for boss in rooms.iter().flat_map(|room| &room.boss_spawns) {
            bosses += 1;
            let built = ambition_boss_encounter::BossClusterScratch::new(&sprite_content,
                &boss_catalog,
                boss.id.clone(),
                boss.name.clone(),
                boss.aabb,
                boss.payload.clone(),
            );
            assert!(
                ids.bosses.contains(&built.config.behavior.id),
                "the boss '{}' reports '{}' when defeated, and the validator does not know that id: {:?}",
                boss.name,
                built.config.behavior.id,
                ids.bosses
            );
        }
        assert!(bosses >= 10, "the shipped world has its bosses: {bosses}");

        // `System Boss` is driven by `PhaseScript:clockwork_warden`. The slug
        // of its name is an id that no defeat reports.
        assert!(
            errors_for_a_quest_that_wants(&ids, Condition::BossDefeated("overflow_boss".into()))
                .is_empty(),
            "a quest that names the id the Overflow boss reports was refused"
        );
        assert_eq!(
            errors_for_a_quest_that_wants(&ids, Condition::BossDefeated("system_boss".into())).len(),
            1,
            "a quest that names the slug of a display name was accepted, and no defeat reports it"
        );

        // The shipped quests.
        let mut report = ContentValidationReport::default();
        check_quest_steps(&crate::quest::shipped_quest_specs(), &ids, &mut report);
        assert!(report.errors.is_empty(), "{:?}", report.errors);
    }

    /// The reward chest of a boss is keyed by its placement id, so that is
    /// the looted flag a quest may name. The slug of the display name keys
    /// no chest.
    #[test]
    fn a_boss_looted_flag_is_keyed_by_its_placement() {
    let sprite_content = ambition_platformer2d::sprite_sheet::test_support::shared_sprites();
        use ambition_persistence::quest::QuestStepCondition as Condition;
        let (rooms, _, ids) = shipped_quest_targets(&sprite_content);
        let boss = rooms
            .iter()
            .flat_map(|room| &room.boss_spawns)
            .find(|boss| boss.name == "Overflow")
            .expect("the shipped world has the Overflow boss");
        assert!(
            errors_for_a_quest_that_wants(
                &ids,
                Condition::FlagSet(encounter_reward_looted_flag(&boss.id))
            )
            .is_empty(),
            "the looted flag of the placement '{}' is not a known flag",
            boss.id
        );
        assert_eq!(
            errors_for_a_quest_that_wants(
                &ids,
                Condition::FlagSet(encounter_reward_looted_flag("overflow"))
            )
            .len(),
            1,
            "the looted flag of a display-name slug keys no chest"
        );
    }

    /// A talk to an NPC that authors no dialogue starts the generic one, and
    /// sets its talk flag. The flags are made by the function the runtime
    /// makes them with (`npc_talk_dialogue_id`), so a quest may name that
    /// flag. The validator's own scan knew only the authored dialogue ids.
    #[test]
    fn the_talk_flag_of_an_npc_with_no_dialogue_is_a_known_flag() {
    let sprite_content = ambition_platformer2d::sprite_sheet::test_support::shared_sprites();
        use ambition_persistence::quest::QuestStepCondition as Condition;
        let (rooms, _, ids) = shipped_quest_targets(&sprite_content);
        let silent = rooms
            .iter()
            .flat_map(|room| &room.placements)
            .any(|record| {
                matches!(
                    &record.schema,
                    ambition_platformer2d::entity_catalog::placements::PlacementSchema::Interactable(
                        interactable
                    ) if matches!(
                        &interactable.kind,
                        ambition_platformer2d::world::rooms::InteractionKindSpec::Npc {
                            dialogue_id: None,
                            ..
                        }
                    )
                )
            });
        assert!(silent, "the fixture needs a shipped NPC that authors no dialogue");
        let flag = ambition_platformer2d::persistence::save_data::npc_talked_flag(
            ambition_platformer2d::conversation::npc_talk_dialogue_id(None),
        );
        assert!(
            errors_for_a_quest_that_wants(&ids, Condition::FlagSet(flag.clone())).is_empty(),
            "a talk sets '{flag}', and the validator does not know it"
        );
    }
}
